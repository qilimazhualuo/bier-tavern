use anyhow::{Context, Result};
use serde_json::Value;

const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

pub struct WbiKeys {
    pub img_key: String,
    pub sub_key: String,
}

pub async fn fetch_wbi_keys(http: &reqwest::Client, cookie: &str) -> Result<WbiKeys> {
    let mut request = http
        .get("https://api.bilibili.com/x/web-interface/nav")
        .header("Referer", "https://www.bilibili.com/");
    if !cookie.is_empty() {
        request = request.header("Cookie", cookie);
    }
    let payload: Value = request.send().await?.json().await?;
    let wbi_img = payload
        .pointer("/data/wbi_img")
        .context("nav 响应里没有 wbi_img")?;
    let img_url = wbi_img
        .get("img_url")
        .and_then(Value::as_str)
        .context("缺少 img_url")?;
    let sub_url = wbi_img
        .get("sub_url")
        .and_then(Value::as_str)
        .context("缺少 sub_url")?;
    Ok(WbiKeys {
        img_key: extract_wbi_key(img_url),
        sub_key: extract_wbi_key(sub_url),
    })
}

pub fn sign_query(params: &mut Vec<(String, String)>, keys: &WbiKeys) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    params.push(("wts".to_string(), timestamp.to_string()));
    params.sort_by(|left, right| left.0.cmp(&right.0));

    let mixin_key = mixin_key(&format!("{}{}", keys.img_key, keys.sub_key));
    let query = params
        .iter()
        .map(|(key, value)| {
            let filtered = value.replace(['!', '\'', '(', ')', '*'], "");
            format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(&filtered)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let digest = md5::compute(format!("{query}{mixin_key}"));
    format!("{query}&w_rid={digest:x}")
}

fn extract_wbi_key(url: &str) -> String {
    let file_name = url.rsplit('/').next().unwrap_or(url);
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| file_name.to_string())
}

fn mixin_key(origin: &str) -> String {
    MIXIN_KEY_ENC_TAB
        .iter()
        .filter_map(|index| origin.chars().nth(*index))
        .take(32)
        .collect()
}
