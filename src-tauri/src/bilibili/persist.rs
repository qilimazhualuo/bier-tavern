use anyhow::Result;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::UserSession;

fn session_file(app: &AppHandle) -> Result<PathBuf> {
    let directory = app.path().app_data_dir()?;
    std::fs::create_dir_all(&directory)?;
    Ok(directory.join("session.json"))
}

pub fn save_session(app: &AppHandle, session: &UserSession) -> Result<()> {
    let path = session_file(app)?;
    let json = serde_json::to_string_pretty(session)?;
    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, json)?;
    std::fs::rename(&temp_path, &path)?;
    Ok(())
}

pub fn load_session(app: &AppHandle) -> Result<Option<UserSession>> {
    let path = session_file(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(path)?;
    let session = serde_json::from_str(&json)?;
    Ok(Some(session))
}

pub fn clear_session(app: &AppHandle) -> Result<()> {
    let path = session_file(app)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
