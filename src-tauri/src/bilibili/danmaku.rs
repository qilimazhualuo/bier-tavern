use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderValue, header};
use tokio_tungstenite::tungstenite::Message;

use super::packet::{DecodedPacket, generate_packet};
use super::user::fetch_danmu_info;
use super::{
    DanmakuMessage, DanmakuStatus, RoomStats, UserSession, json_field_str, json_field_u64, json_u64,
    now_millis,
};
use crate::state::AppState;

static MESSAGE_SEQ: AtomicU64 = AtomicU64::new(1);

pub async fn run_danmaku_loop(
    app: AppHandle,
    http: reqwest::Client,
    session: UserSession,
    mut cancel_signal: watch::Receiver<bool>,
) {
    emit_status(&app, "connecting", "正在连接弹幕服务器");
    match connect_and_listen(&app, &http, &session, &mut cancel_signal).await {
        Ok(()) => emit_status(&app, "disconnected", "弹幕连接已断开"),
        Err(error) => emit_status(&app, "error", &error.to_string()),
    }
}

async fn connect_and_listen(
    app: &AppHandle,
    http: &reqwest::Client,
    session: &UserSession,
    cancel_signal: &mut watch::Receiver<bool>,
) -> Result<()> {
    if session.room_id == 0 {
        anyhow::bail!("当前账号没有直播间，连个屁的弹幕都没有");
    }

    let cookie = session.cookie_header();
    let server = fetch_danmu_info(http, session.room_id, &cookie).await?;
    let ws_url = server.ws_url();
    // 必须用 IntoClientRequest，让库自己生成 Sec-WebSocket-Key，
    // 手写 Request 缺这个头会直接炸。
    let mut request = ws_url
        .as_str()
        .into_client_request()
        .context("构造 WebSocket 请求失败")?;
    let headers = request.headers_mut();
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_static(super::USER_AGENT),
    );
    headers.insert(
        header::ORIGIN,
        HeaderValue::from_static("https://live.bilibili.com"),
    );
    if !cookie.is_empty() {
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&cookie).context("Cookie 头非法")?,
        );
    }

    let (socket, _) = tokio_tungstenite::connect_async(request).await?;
    let (writer, mut reader) = socket.split();
    let writer = Arc::new(Mutex::new(writer));

    let auth_body = json!({
        "uid": session.uid,
        "roomid": session.room_id,
        "protover": 3,
        "platform": "web",
        "type": 2,
        "buvid": session.buvid3,
        "key": server.token,
    });
    let auth_packet = generate_packet(7, auth_body.to_string().as_bytes());
    writer
        .lock()
        .await
        .send(Message::Binary(auth_packet.into()))
        .await?;

    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel_signal.changed() => {
                if *cancel_signal.borrow() {
                    break;
                }
            }
            _ = heartbeat.tick() => {
                let beat = generate_packet(2, b"");
                writer.lock().await.send(Message::Binary(beat.into())).await?;
            }
            incoming = reader.next() => {
                match incoming {
                    Some(Ok(Message::Binary(bytes))) => {
                        handle_packets(app, bytes.as_ref());
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(json) = serde_json::from_str::<Value>(text.as_ref()) {
                            if let Some(message) = map_command(&json) {
                                feed_battle_from_danmaku(app, &message);
                                let _ = app.emit("danmaku-message", message);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        writer.lock().await.send(Message::Pong(payload)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(error)) => anyhow::bail!(error),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn handle_packets(app: &AppHandle, buffer: &[u8]) {
    for packet in super::packet::decode_packets(buffer) {
        match packet {
            DecodedPacket::Popular { count } => {
                let _ = app.emit(
                    "room-stats",
                    RoomStats {
                        watched: count as u64,
                    },
                );
            }
            DecodedPacket::AuthOk => {
                emit_status(app, "connected", "弹幕已连接");
            }
            DecodedPacket::Message(json) => {
                let command = json.get("cmd").and_then(Value::as_str).unwrap_or("");
                if command == "WATCHED_CHANGE" {
                    if let Some(watched) = json.pointer("/data/num").and_then(json_u64) {
                        let _ = app.emit("room-stats", RoomStats { watched });
                    }
                    continue;
                }
                if let Some(message) = map_command(&json) {
                    feed_battle_from_danmaku(app, &message);
                    let _ = app.emit("danmaku-message", message);
                }
            }
        }
    }
}

fn feed_battle_from_danmaku(app: &AppHandle, message: &DanmakuMessage) {
    if message.cmd != "DANMU_MSG" {
        return;
    }
    let app_handle = app.clone();
    let uid = message.uid;
    let username = message.username.clone();
    let content = message.content.clone();
    tauri::async_runtime::spawn(async move {
        let Some(app_state) = app_handle.try_state::<AppState>() else {
            return;
        };
        let mut battle = app_state.battle.battle.lock().await;
        battle.feed_danmaku(uid, username.as_deref(), &content);
    });
}

fn map_command(json: &Value) -> Option<DanmakuMessage> {
    let command = json.get("cmd").and_then(Value::as_str).unwrap_or("");
    let command_name = command.split(':').next().unwrap_or(command);
    let data = json.get("data").cloned().unwrap_or(Value::Null);
    let (uid, username, content) = match command_name {
        "DANMU_MSG" => parse_danmu(json)?,
        "SEND_GIFT" => {
            let username = json_field_str(&data, &["uname"]);
            let gift_name = json_field_str(&data, &["giftName", "gift_name"]).unwrap_or_else(|| "礼物".to_string());
            let count = json_field_u64(&data, &["num"]).unwrap_or(1);
            (
                json_field_u64(&data, &["uid"]),
                username.clone(),
                format!(
                    "投喂 {} x{}",
                    gift_name,
                    count
                ),
            )
        }
        "COMBO_SEND" => {
            let username = json_field_str(&data, &["uname"]);
            let gift_name = json_field_str(&data, &["gift_name", "giftName"]).unwrap_or_else(|| "礼物".to_string());
            let count = json_field_u64(&data, &["combo_num", "comboNum"]).unwrap_or(1);
            (
                json_field_u64(&data, &["uid"]),
                username,
                format!("连击 {gift_name} x{count}"),
            )
        }
        "SUPER_CHAT_MESSAGE" | "SUPER_CHAT_MESSAGE_JPN" => {
            let user_info = data.get("user_info").cloned().unwrap_or(Value::Null);
            let username = json_field_str(&user_info, &["uname"])
                .or_else(|| json_field_str(&data, &["uname"]));
            let text = json_field_str(&data, &["message"]).unwrap_or_default();
            let price = json_field_u64(&data, &["price"]).unwrap_or(0);
            (
                json_field_u64(&data, &["uid"]),
                username,
                format!("醒目留言 ¥{price}：{text}"),
            )
        }
        "INTERACT_WORD" => {
            let username = json_field_str(&data, &["uname"]);
            let msg_type = json_field_u64(&data, &["msg_type"]).unwrap_or(1);
            let action = match msg_type {
                2 => "关注了直播间",
                3 => "分享了直播间",
                _ => "进入直播间",
            };
            (
                json_field_u64(&data, &["uid"]),
                username,
                action.to_string(),
            )
        }
        "GUARD_BUY" => {
            let username = json_field_str(&data, &["username", "uname"]);
            let gift_name = json_field_str(&data, &["gift_name"]).unwrap_or_else(|| "舰长".to_string());
            (
                json_field_u64(&data, &["uid"]),
                username,
                format!("开通了{gift_name}"),
            )
        }
        "ENTRY_EFFECT" => {
            let copy_writing = json_field_str(&data, &["copy_writing"]).unwrap_or_default();
            (
                json_field_u64(&data, &["uid"]),
                json_field_str(&data, &["uname"]),
                strip_tags(&copy_writing),
            )
        }
        "LIVE" => (None, None, "直播间开播了".to_string()),
        "WARNING" => (
            None,
            None,
            json_field_str(json, &["msg"]).unwrap_or_else(|| "直播警告".to_string()),
        ),
        "CUT_OFF" => (
            None,
            None,
            json_field_str(json, &["msg"]).unwrap_or_else(|| "直播被切断".to_string()),
        ),
        "ROOM_BLOCK_MSG" => {
            let username = json_field_str(&data, &["uname"]);
            (
                json_field_u64(&data, &["uid"]),
                username,
                "被禁言".to_string(),
            )
        }
        _ => return None,
    };

    if content.trim().is_empty() {
        return None;
    }

    Some(DanmakuMessage {
        id: MESSAGE_SEQ.fetch_add(1, Ordering::Relaxed),
        cmd: command_name.to_string(),
        uid,
        username,
        content,
        timestamp: now_millis(),
    })
}

fn parse_danmu(json: &Value) -> Option<(Option<u64>, Option<String>, String)> {
    let info = json.get("info")?.as_array()?;
    let content = info.get(1)?.as_str().unwrap_or("").to_string();
    let user = info.get(2)?.as_array();
    let uid = user.and_then(|item| item.first()).and_then(json_u64);
    let username = user
        .and_then(|item| item.get(1))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    Some((uid, username, content))
}

fn strip_tags(text: &str) -> String {
    let mut output = String::new();
    let mut inside_tag = false;
    for character in text.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    output
}

fn emit_status(app: &AppHandle, state: &str, detail: &str) {
    let _ = app.emit(
        "danmaku-status",
        DanmakuStatus {
            state: state.to_string(),
            detail: detail.to_string(),
        },
    );
}
