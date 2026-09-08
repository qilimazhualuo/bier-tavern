use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use super::BattleRuntime;
use crate::state::AppState;

pub async fn ensure_battle_loop(app: &AppHandle, runtime: &BattleRuntime) {
    let mut guard = runtime.loop_stop.lock().await;
    if guard.is_some() {
        return;
    }
    let (sender, receiver) = watch::channel(false);
    *guard = Some(sender);
    drop(guard);

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        run_loop(app_handle, receiver).await;
    });
}

pub async fn stop_battle_loop(runtime: &BattleRuntime) {
    if let Some(sender) = runtime.loop_stop.lock().await.take() {
        let _ = sender.send(true);
    }
}

async fn run_loop(app: AppHandle, mut cancel_signal: watch::Receiver<bool>) {
    let mut ticker = tokio::time::interval(Duration::from_millis(50));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancel_signal.changed() => {
                if *cancel_signal.borrow() {
                    break;
                }
            }
            _ = ticker.tick() => {
                let Some(app_state) = app.try_state::<AppState>() else {
                    continue;
                };
                let snapshot = {
                    let mut battle = app_state.battle.battle.lock().await;
                    if !battle.is_running() {
                        continue;
                    }
                    battle.tick(0.05);
                    let snapshot = battle.snapshot();
                    if !battle.is_running() {
                        // 终局再推一帧，然后停循环
                        drop(battle);
                        let _ = app.emit("game-state", &snapshot);
                        stop_battle_loop(&app_state.battle).await;
                        break;
                    }
                    snapshot
                };
                let _ = app.emit("game-state", snapshot);
            }
        }
    }
}
