use tauri::{AppHandle, State};
use tokio::sync::watch;

use crate::bilibili::login::{QrGenerateResult, QrPollResult};
use crate::bilibili::{self, UserProfile, UserSession};
use crate::game::{self, BattleSnapshot};
use crate::state::AppState;

#[tauri::command]
pub async fn generate_qrcode(state: State<'_, AppState>) -> Result<QrGenerateResult, String> {
    bilibili::login::generate_qrcode(&state.http)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn poll_qrcode(
    qrcode_key: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<QrPollResult, String> {
    let outcome = bilibili::login::poll_qrcode(&state.http, &qrcode_key)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(session) = outcome.session {
        apply_session(&app, &state, session).await?;
    }
    Ok(outcome.result)
}

#[tauri::command]
pub async fn get_session(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Option<UserProfile>, String> {
    if let Some(session) = state.session.lock().await.clone() {
        return Ok(Some(session.to_profile()));
    }

    let stored = bilibili::persist::load_session(&app).map_err(|error| error.to_string())?;
    let Some(stored_session) = stored else {
        return Ok(None);
    };
    if stored_session.sessdata.is_empty() {
        let _ = bilibili::persist::clear_session(&app);
        return Ok(None);
    }

    let cookies = bilibili::login::CookieBag::from_session(&stored_session);
    match bilibili::user::fetch_session_profile(&state.http, &cookies).await {
        Ok(fresh_session) => {
            apply_session(&app, &state, fresh_session.clone()).await?;
            Ok(Some(fresh_session.to_profile()))
        }
        Err(error) if error.is_unauthorized() => {
            let _ = bilibili::persist::clear_session(&app);
            Ok(None)
        }
        Err(_) => {
            *state.session.lock().await = Some(stored_session.clone());
            start_danmaku(&app, &state, stored_session.clone()).await;
            Ok(Some(stored_session.to_profile()))
        }
    }
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    stop_danmaku(&state).await;
    *state.session.lock().await = None;
    bilibili::persist::clear_session(&app).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn reconnect_danmaku(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let session = state
        .session
        .lock()
        .await
        .clone()
        .ok_or_else(|| "还没登录，重连个屁".to_string())?;
    start_danmaku(&app, &state, session).await;
    Ok(())
}

#[tauri::command]
pub async fn start_battle(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<BattleSnapshot, String> {
    {
        let mut battle = state.battle.battle.lock().await;
        battle.reset_and_start();
    }
    game::ensure_battle_loop(&app, &state.battle).await;
    let snapshot = state.battle.battle.lock().await.snapshot();
    Ok(snapshot)
}

#[tauri::command]
pub async fn stop_battle(state: State<'_, AppState>) -> Result<BattleSnapshot, String> {
    game::stop_battle_loop(&state.battle).await;
    let mut battle = state.battle.battle.lock().await;
    battle.stop();
    Ok(battle.snapshot())
}

#[tauri::command]
pub async fn get_battle_state(state: State<'_, AppState>) -> Result<BattleSnapshot, String> {
    Ok(state.battle.battle.lock().await.snapshot())
}

#[tauri::command]
pub async fn spawn_test_wave(state: State<'_, AppState>) -> Result<BattleSnapshot, String> {
    let mut battle = state.battle.battle.lock().await;
    if !battle.is_running() {
        return Err("对局还没开始".to_string());
    }
    battle.spawn_unit(game::Team::Red, "测试红".to_string(), "红方测试弹幕");
    battle.spawn_unit(game::Team::Blue, "测试蓝".to_string(), "蓝方测试弹幕");
    Ok(battle.snapshot())
}

async fn apply_session(
    app: &AppHandle,
    state: &AppState,
    session: UserSession,
) -> Result<(), String> {
    bilibili::persist::save_session(app, &session).map_err(|error| error.to_string())?;
    *state.session.lock().await = Some(session.clone());
    start_danmaku(app, state, session).await;
    Ok(())
}

async fn start_danmaku(app: &AppHandle, state: &AppState, session: UserSession) {
    stop_danmaku(state).await;
    let (sender, receiver) = watch::channel(false);
    *state.danmaku_stop.lock().await = Some(sender);
    let http = state.http.clone();
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        bilibili::danmaku::run_danmaku_loop(app_handle, http, session, receiver).await;
    });
}

async fn stop_danmaku(state: &AppState) {
    if let Some(sender) = state.danmaku_stop.lock().await.take() {
        let _ = sender.send(true);
    }
}
