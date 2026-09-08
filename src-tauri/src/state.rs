use crate::game::BattleRuntime;
use tokio::sync::watch;

pub struct AppState {
    pub http: reqwest::Client,
    pub session: tokio::sync::Mutex<Option<crate::bilibili::UserSession>>,
    pub danmaku_stop: tokio::sync::Mutex<Option<watch::Sender<bool>>>,
    pub battle: BattleRuntime,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .user_agent(crate::bilibili::USER_AGENT)
            .timeout(std::time::Duration::from_secs(20))
            .build()?;

        Ok(Self {
            http,
            session: tokio::sync::Mutex::new(None),
            danmaku_stop: tokio::sync::Mutex::new(None),
            battle: BattleRuntime::new(),
        })
    }
}
