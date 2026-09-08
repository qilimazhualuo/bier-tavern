use anyhow::{Result, bail};
use serde_json::Value;

use super::login::CookieBag;
use super::{UserSession, json_field_str, json_field_u64, json_u64, wbi};

#[derive(Debug)]
pub enum AuthError {
    /// Cookie 明确失效，该清本地登录态
    Unauthorized(String),
    /// 网络 / 解析等临时问题，别乱动本地文件
    Transient(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized(message) | Self::Transient(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for AuthError {}

impl AuthError {
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Self::Unauthorized(_))
    }
}

#[derive(Default, Clone)]
pub struct BuvidPair {
    pub buvid3: String,
    pub buvid4: String,
}

pub async fn fetch_buvid(http: &reqwest::Client, cookie: &str) -> Result<BuvidPair> {
    let mut request = http
        .get("https://api.bilibili.com/x/frontend/finger/spi/")
        .header("Referer", "https://www.bilibili.com/");
    if !cookie.is_empty() {
        request = request.header("Cookie", cookie);
    }
    let payload: Value = request.send().await?.json().await?;
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    Ok(BuvidPair {
        buvid3: json_field_str(&data, &["b_3", "buvid3"]).unwrap_or_default(),
        buvid4: json_field_str(&data, &["b_4", "buvid4"]).unwrap_or_default(),
    })
}

pub async fn fetch_session_profile(
    http: &reqwest::Client,
    cookies: &CookieBag,
) -> Result<UserSession, AuthError> {
    let cookie = cookies.header();
    let response = http
        .get("https://api.bilibili.com/x/web-interface/nav")
        .header("Referer", "https://www.bilibili.com/")
        .header("Cookie", &cookie)
        .send()
        .await
        .map_err(|error| AuthError::Transient(error.to_string()))?;
    let payload: Value = response
        .json()
        .await
        .map_err(|error| AuthError::Transient(error.to_string()))?;
    let code = payload.get("code").and_then(Value::as_i64).unwrap_or(-1);
    let message = payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("登录态无效")
        .to_string();
    // -101 未登录；其它非 0 多数是风控/网络，别当成退出登录
    if code == -101 {
        return Err(AuthError::Unauthorized(message));
    }
    if code != 0 {
        return Err(AuthError::Transient(format!("nav 失败({code}): {message}")));
    }
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    if data.get("isLogin").and_then(Value::as_bool) == Some(false) {
        return Err(AuthError::Unauthorized("登录态无效".to_string()));
    }
    let uid = json_field_u64(&data, &["mid", "uid"])
        .ok_or_else(|| AuthError::Unauthorized("nav 没有 uid".to_string()))?;
    let uname =
        json_field_str(&data, &["uname", "username"]).unwrap_or_else(|| "未知用户".to_string());
    let face = json_field_str(&data, &["face"]).unwrap_or_default();
    let room = fetch_room_by_uid(http, uid, &cookie)
        .await
        .unwrap_or_default();

    Ok(UserSession {
        uid,
        uname,
        face,
        room_id: room.room_id,
        short_id: room.short_id,
        title: room.title,
        live_status: room.live_status,
        sessdata: cookies.sessdata.clone(),
        bili_jct: cookies.bili_jct.clone(),
        dede_user_id: if cookies.dede_user_id.is_empty() {
            uid.to_string()
        } else {
            cookies.dede_user_id.clone()
        },
        buvid3: cookies.buvid3.clone(),
        buvid4: cookies.buvid4.clone(),
    })
}

#[derive(Default)]
pub struct RoomBrief {
    pub room_id: u64,
    pub short_id: u64,
    pub title: String,
    pub live_status: u8,
}

pub async fn fetch_room_by_uid(
    http: &reqwest::Client,
    uid: u64,
    cookie: &str,
) -> Result<RoomBrief> {
    let mut request = http
        .get("https://api.live.bilibili.com/room/v1/Room/getRoomInfoOld")
        .query(&[("mid", uid.to_string())])
        .header("Referer", "https://live.bilibili.com/");
    if !cookie.is_empty() {
        request = request.header("Cookie", cookie);
    }
    let payload: Value = request.send().await?.json().await?;
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    let room_status = json_field_u64(&data, &["roomStatus", "room_status"]).unwrap_or(0);
    if room_status == 0 {
        return Ok(RoomBrief::default());
    }
    let raw_room_id = json_field_u64(&data, &["roomid", "room_id"]).unwrap_or(0);
    let title = json_field_str(&data, &["title"]).unwrap_or_default();
    let live_status = json_field_u64(&data, &["live_status", "liveStatus"]).unwrap_or(0) as u8;
    if raw_room_id == 0 {
        return Ok(RoomBrief {
            title,
            live_status,
            ..RoomBrief::default()
        });
    }

    let resolved = resolve_real_room_id(http, raw_room_id, cookie)
        .await
        .unwrap_or(RoomIds {
            room_id: raw_room_id,
            short_id: raw_room_id,
        });
    Ok(RoomBrief {
        room_id: resolved.room_id,
        short_id: resolved.short_id,
        title,
        live_status,
    })
}

struct RoomIds {
    room_id: u64,
    short_id: u64,
}

async fn resolve_real_room_id(
    http: &reqwest::Client,
    room_id: u64,
    cookie: &str,
) -> Result<RoomIds> {
    let mut request = http
        .get("https://api.live.bilibili.com/room/v1/Room/room_init")
        .query(&[("id", room_id.to_string())])
        .header("Referer", "https://live.bilibili.com/");
    if !cookie.is_empty() {
        request = request.header("Cookie", cookie);
    }
    let payload: Value = request.send().await?.json().await?;
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    let real_id = json_field_u64(&data, &["room_id", "roomid"]).unwrap_or(room_id);
    let short_id = json_field_u64(&data, &["short_id"]).unwrap_or(0);
    Ok(RoomIds {
        room_id: real_id,
        short_id: if short_id == 0 { real_id } else { short_id },
    })
}

pub async fn fetch_danmu_info(
    http: &reqwest::Client,
    room_id: u64,
    cookie: &str,
) -> Result<DanmuServer> {
    let wbi_keys = wbi::fetch_wbi_keys(http, cookie).await?;
    let mut params = vec![
        ("id".to_string(), room_id.to_string()),
        ("type".to_string(), "0".to_string()),
        ("web_location".to_string(), "444.8".to_string()),
    ];
    let query = wbi::sign_query(&mut params, &wbi_keys);
    let url = format!("https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?{query}");
    let mut request = http.get(url).header("Referer", "https://live.bilibili.com/");
    if !cookie.is_empty() {
        request = request.header("Cookie", cookie);
    }
    let payload: Value = request.send().await?.json().await?;
    if payload.get("code").and_then(Value::as_i64).unwrap_or(-1) != 0 {
        bail!(
            payload
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("获取弹幕服务器失败")
                .to_string()
        );
    }
    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    let token = json_field_str(&data, &["token"]).unwrap_or_default();
    let host_list = data
        .get("host_list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let host_item = host_list.first().cloned().unwrap_or(Value::Null);
    let host = json_field_str(&host_item, &["host"])
        .unwrap_or_else(|| "broadcastlv.chat.bilibili.com".to_string());
    let wss_port = json_u64(&host_item["wss_port"]).unwrap_or(443);
    Ok(DanmuServer {
        token,
        host,
        wss_port,
    })
}

pub struct DanmuServer {
    pub token: String,
    pub host: String,
    pub wss_port: u64,
}

impl DanmuServer {
    pub fn ws_url(&self) -> String {
        format!("wss://{}:{}/sub", self.host, self.wss_port)
    }
}
