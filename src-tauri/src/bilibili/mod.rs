pub mod danmaku;
pub mod login;
pub mod packet;
pub mod persist;
pub mod user;
pub mod wbi;

use serde::{Deserialize, Serialize};

pub const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSession {
    pub uid: u64,
    pub uname: String,
    pub face: String,
    pub room_id: u64,
    pub short_id: u64,
    pub title: String,
    pub live_status: u8,
    pub sessdata: String,
    pub bili_jct: String,
    pub dede_user_id: String,
    pub buvid3: String,
    pub buvid4: String,
}

impl UserSession {
    pub fn cookie_header(&self) -> String {
        let mut parts = Vec::new();
        if !self.sessdata.is_empty() {
            parts.push(format!("SESSDATA={}", self.sessdata));
        }
        if !self.bili_jct.is_empty() {
            parts.push(format!("bili_jct={}", self.bili_jct));
        }
        if !self.dede_user_id.is_empty() {
            parts.push(format!("DedeUserID={}", self.dede_user_id));
        }
        if !self.buvid3.is_empty() {
            parts.push(format!("buvid3={}", self.buvid3));
        }
        if !self.buvid4.is_empty() {
            parts.push(format!("buvid4={}", self.buvid4));
        }
        parts.join("; ")
    }

    pub fn to_profile(&self) -> UserProfile {
        UserProfile {
            uid: self.uid,
            uname: self.uname.clone(),
            face: self.face.clone(),
            room_id: self.room_id,
            short_id: self.short_id,
            title: self.title.clone(),
            live_status: self.live_status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub uid: u64,
    pub uname: String,
    pub face: String,
    pub room_id: u64,
    pub short_id: u64,
    pub title: String,
    pub live_status: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DanmakuMessage {
    pub id: u64,
    pub cmd: String,
    pub uid: Option<u64>,
    pub username: Option<String>,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DanmakuStatus {
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomStats {
    pub watched: u64,
}

pub fn json_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        .or_else(|| value.as_str()?.parse().ok())
}

pub fn json_field_u64(object: &serde_json::Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(found) = object.get(key).and_then(json_u64) {
            return Some(found);
        }
    }
    None
}

pub fn json_field_str(object: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(text) = object.get(*key).and_then(|value| value.as_str()) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
