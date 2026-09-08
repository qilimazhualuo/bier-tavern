use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{UserSession, user};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrGenerateResult {
    pub url: String,
    pub qrcode_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrPollResult {
    pub status: String,
    pub message: String,
    pub user: Option<super::UserProfile>,
}

pub struct QrPollOutcome {
    pub result: QrPollResult,
    pub session: Option<UserSession>,
}

#[derive(Debug, Deserialize)]
struct QrGeneratePayload {
    code: i32,
    message: Option<String>,
    data: Option<QrGenerateData>,
}

#[derive(Debug, Deserialize)]
struct QrGenerateData {
    url: String,
    qrcode_key: String,
}

pub async fn generate_qrcode(http: &reqwest::Client) -> Result<QrGenerateResult> {
    let payload: QrGeneratePayload = http
        .get("https://passport.bilibili.com/x/passport-login/web/qrcode/generate")
        .header("Referer", "https://www.bilibili.com/")
        .send()
        .await?
        .json()
        .await?;
    if payload.code != 0 {
        anyhow::bail!(
            payload.message.unwrap_or_else(|| "生成二维码失败".to_string())
        );
    }
    let data = payload.data.context("生成二维码没有返回 data")?;
    Ok(QrGenerateResult {
        url: data.url,
        qrcode_key: data.qrcode_key,
    })
}

pub async fn poll_qrcode(http: &reqwest::Client, qrcode_key: &str) -> Result<QrPollOutcome> {
    let response = http
        .get("https://passport.bilibili.com/x/passport-login/web/qrcode/poll")
        .query(&[("qrcode_key", qrcode_key)])
        .header("Referer", "https://passport.bilibili.com/login")
        .send()
        .await?;

    let mut cookies = CookieBag::default();
    for header_value in response.headers().get_all(reqwest::header::SET_COOKIE) {
        if let Ok(raw) = header_value.to_str() {
            cookies.ingest_set_cookie(raw);
        }
    }

    let payload: Value = response.json().await?;
    let outer_code = payload.get("code").and_then(Value::as_i64).unwrap_or(-1);
    if outer_code != 0 {
        return Ok(poll_outcome(
            "error",
            payload
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("轮询二维码失败"),
            None,
        ));
    }

    let data = payload.get("data").cloned().unwrap_or(Value::Null);
    let inner_code = data.get("code").and_then(Value::as_i64).unwrap_or(-1);
    let inner_message = data
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    match inner_code {
        86101 => Ok(poll_outcome("waiting", "等待扫码", None)),
        86090 => Ok(poll_outcome("scanned", "已扫码，请在手机上确认", None)),
        86038 => Ok(poll_outcome("expired", "二维码已过期", None)),
        0 => {
            if let Some(login_url) = data.get("url").and_then(Value::as_str) {
                cookies.ingest_login_url(login_url);
            }
            let session = build_session(http, cookies).await?;
            Ok(poll_outcome("success", "登录成功", Some(session)))
        }
        _ => Ok(poll_outcome(
            "error",
            if inner_message.is_empty() {
                format!("未知扫码状态: {inner_code}")
            } else {
                inner_message
            },
            None,
        )),
    }
}

fn poll_outcome(
    status: impl Into<String>,
    message: impl Into<String>,
    session: Option<UserSession>,
) -> QrPollOutcome {
    QrPollOutcome {
        result: QrPollResult {
            status: status.into(),
            message: message.into(),
            user: session.as_ref().map(UserSession::to_profile),
        },
        session,
    }
}

async fn build_session(http: &reqwest::Client, mut cookies: CookieBag) -> Result<UserSession> {
    if cookies.sessdata.is_empty() {
        anyhow::bail!("登录成功但没有拿到 SESSDATA，B 站又在抽风");
    }
    let fingerprint = user::fetch_buvid(http, &cookies.header())
        .await
        .unwrap_or_default();
    if cookies.buvid3.is_empty() {
        cookies.buvid3 = fingerprint.buvid3;
    }
    if cookies.buvid4.is_empty() {
        cookies.buvid4 = fingerprint.buvid4;
    }
    user::fetch_session_profile(http, &cookies)
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

#[derive(Debug, Default, Clone)]
pub struct CookieBag {
    pub sessdata: String,
    pub bili_jct: String,
    pub dede_user_id: String,
    pub buvid3: String,
    pub buvid4: String,
}

impl CookieBag {
    pub fn header(&self) -> String {
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

    pub fn ingest_set_cookie(&mut self, raw: &str) {
        let first_pair = raw.split(';').next().unwrap_or(raw);
        if let Some((key, value)) = first_pair.split_once('=') {
            self.assign(key.trim(), value.trim());
        }
    }

    pub fn ingest_login_url(&mut self, login_url: &str) {
        // SESSDATA 里常有 %2C，不能走 query_pairs 解码，否则 Cookie 可能失效
        let Some(query) = login_url.split_once('?').map(|(_, query)| query) else {
            return;
        };
        for pair in query.split('&') {
            let Some((raw_key, raw_value)) = pair.split_once('=') else {
                continue;
            };
            let key = urlencoding::decode(raw_key)
                .unwrap_or(std::borrow::Cow::Borrowed(raw_key))
                .into_owned();
            match key.as_str() {
                "SESSDATA" => self.sessdata = raw_value.to_string(),
                "bili_jct" => {
                    self.bili_jct = urlencoding::decode(raw_value)
                        .unwrap_or(std::borrow::Cow::Borrowed(raw_value))
                        .into_owned();
                }
                "DedeUserID" => {
                    self.dede_user_id = urlencoding::decode(raw_value)
                        .unwrap_or(std::borrow::Cow::Borrowed(raw_value))
                        .into_owned();
                }
                "buvid3" => {
                    self.buvid3 = urlencoding::decode(raw_value)
                        .unwrap_or(std::borrow::Cow::Borrowed(raw_value))
                        .into_owned();
                }
                "buvid4" => {
                    self.buvid4 = urlencoding::decode(raw_value)
                        .unwrap_or(std::borrow::Cow::Borrowed(raw_value))
                        .into_owned();
                }
                _ => {}
            }
        }
    }

    pub fn from_session(session: &UserSession) -> Self {
        Self {
            sessdata: session.sessdata.clone(),
            bili_jct: session.bili_jct.clone(),
            dede_user_id: session.dede_user_id.clone(),
            buvid3: session.buvid3.clone(),
            buvid4: session.buvid4.clone(),
        }
    }

    fn assign(&mut self, key: &str, value: &str) {
        match key {
            "SESSDATA" => self.sessdata = value.to_string(),
            "bili_jct" => self.bili_jct = value.to_string(),
            "DedeUserID" => self.dede_user_id = value.to_string(),
            "buvid3" => self.buvid3 = value.to_string(),
            "buvid4" => self.buvid4 = value.to_string(),
            _ => {}
        }
    }
}
