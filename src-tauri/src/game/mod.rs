mod battle;
mod loop_runner;

pub use battle::{BattleSnapshot, Team};
pub use loop_runner::{ensure_battle_loop, stop_battle_loop};

use battle::Battle;
use tokio::sync::{Mutex, watch};

pub struct BattleRuntime {
    pub battle: Mutex<Battle>,
    pub loop_stop: Mutex<Option<watch::Sender<bool>>>,
}

impl BattleRuntime {
    pub fn new() -> Self {
        Self {
            battle: Mutex::new(Battle::new()),
            loop_stop: Mutex::new(None),
        }
    }
}
