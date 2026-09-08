mod bilibili;
mod commands;
mod game;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = state::AppState::new().expect("初始化 HTTP 客户端失败");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::generate_qrcode,
            commands::poll_qrcode,
            commands::get_session,
            commands::logout,
            commands::reconnect_danmaku,
            commands::start_battle,
            commands::stop_battle,
            commands::get_battle_state,
            commands::spawn_test_wave,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
