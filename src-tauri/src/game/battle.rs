use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

static UNIT_SEQ: AtomicU64 = AtomicU64::new(1);

const MAP_WIDTH: f32 = 1600.0;
const MAP_HEIGHT: f32 = 900.0;
const LANE_Y: f32 = 450.0;
const BASE_RADIUS: f32 = 54.0;
const UNIT_RADIUS: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Team {
    Red,
    Blue,
}

impl Team {
    pub fn from_danmaku(uid: Option<u64>, content: &str) -> Self {
        let lowered = content.to_lowercase();
        if content.contains('红') || lowered.contains("red") || content.contains("左") {
            return Self::Red;
        }
        if content.contains('蓝') || lowered.contains("blue") || content.contains("右") {
            return Self::Blue;
        }
        match uid.unwrap_or(0) % 2 {
            0 => Self::Red,
            _ => Self::Blue,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BattleStatus {
    Idle,
    Running,
    RedWin,
    BlueWin,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseView {
    pub team: Team,
    pub hp: f32,
    pub max_hp: f32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitView {
    pub id: u64,
    pub team: Team,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleSnapshot {
    pub status: BattleStatus,
    pub tick: u64,
    pub map_width: f32,
    pub map_height: f32,
    pub red_base: BaseView,
    pub blue_base: BaseView,
    pub units: Vec<UnitView>,
    pub red_spawned: u64,
    pub blue_spawned: u64,
}

#[derive(Debug, Clone)]
struct Base {
    hp: f32,
    max_hp: f32,
    x: f32,
    y: f32,
}

#[derive(Debug, Clone)]
struct Unit {
    id: u64,
    team: Team,
    x: f32,
    y: f32,
    hp: f32,
    max_hp: f32,
    speed: f32,
    damage: f32,
    attack_range: f32,
    attack_cooldown: f32,
    attack_timer: f32,
    label: String,
}

pub struct Battle {
    status: BattleStatus,
    tick: u64,
    red_base: Base,
    blue_base: Base,
    units: Vec<Unit>,
    red_spawned: u64,
    blue_spawned: u64,
}

impl Battle {
    pub fn new() -> Self {
        Self {
            status: BattleStatus::Idle,
            tick: 0,
            red_base: Base {
                hp: 1000.0,
                max_hp: 1000.0,
                x: 120.0,
                y: LANE_Y,
            },
            blue_base: Base {
                hp: 1000.0,
                max_hp: 1000.0,
                x: MAP_WIDTH - 120.0,
                y: LANE_Y,
            },
            units: Vec::new(),
            red_spawned: 0,
            blue_spawned: 0,
        }
    }

    pub fn reset_and_start(&mut self) {
        *self = Self::new();
        self.status = BattleStatus::Running;
    }

    pub fn stop(&mut self) {
        self.status = BattleStatus::Idle;
        self.units.clear();
    }

    pub fn is_running(&self) -> bool {
        self.status == BattleStatus::Running
    }

    pub fn snapshot(&self) -> BattleSnapshot {
        BattleSnapshot {
            status: self.status,
            tick: self.tick,
            map_width: MAP_WIDTH,
            map_height: MAP_HEIGHT,
            red_base: BaseView {
                team: Team::Red,
                hp: self.red_base.hp,
                max_hp: self.red_base.max_hp,
                x: self.red_base.x,
                y: self.red_base.y,
            },
            blue_base: BaseView {
                team: Team::Blue,
                hp: self.blue_base.hp,
                max_hp: self.blue_base.max_hp,
                x: self.blue_base.x,
                y: self.blue_base.y,
            },
            units: self
                .units
                .iter()
                .map(|unit| UnitView {
                    id: unit.id,
                    team: unit.team,
                    x: unit.x,
                    y: unit.y,
                    hp: unit.hp,
                    max_hp: unit.max_hp,
                    label: unit.label.clone(),
                })
                .collect(),
            red_spawned: self.red_spawned,
            blue_spawned: self.blue_spawned,
        }
    }

    pub fn feed_danmaku(&mut self, uid: Option<u64>, username: Option<&str>, content: &str) {
        if self.status != BattleStatus::Running {
            return;
        }
        let team = Team::from_danmaku(uid, content);
        let label = username
            .map(str::to_string)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| content.chars().take(6).collect());
        self.spawn_unit(team, label, content);
    }

    pub fn spawn_unit(&mut self, team: Team, label: String, content: &str) {
        if self.status != BattleStatus::Running {
            return;
        }
        if self.units.len() >= 120 {
            return;
        }

        let strength = content.chars().count().clamp(1, 24) as f32;
        let hp = 40.0 + strength * 2.5;
        let damage = 6.0 + strength * 0.35;
        let speed = 70.0 + (24.0 - strength).max(0.0);

        let (spawn_x, lane_offset) = match team {
            Team::Red => (self.red_base.x + 70.0, ((self.red_spawned % 5) as f32 - 2.0) * 18.0),
            Team::Blue => (
                self.blue_base.x - 70.0,
                ((self.blue_spawned % 5) as f32 - 2.0) * 18.0,
            ),
        };

        match team {
            Team::Red => self.red_spawned += 1,
            Team::Blue => self.blue_spawned += 1,
        }

        self.units.push(Unit {
            id: UNIT_SEQ.fetch_add(1, Ordering::Relaxed),
            team,
            x: spawn_x,
            y: LANE_Y + lane_offset,
            hp,
            max_hp: hp,
            speed,
            damage,
            attack_range: 36.0,
            attack_cooldown: 0.55,
            attack_timer: 0.0,
            label,
        });
    }

    pub fn tick(&mut self, delta_seconds: f32) {
        if self.status != BattleStatus::Running {
            return;
        }
        self.tick = self.tick.wrapping_add(1);

        self.move_units(delta_seconds);
        self.resolve_combat(delta_seconds);
        self.units.retain(|unit| unit.hp > 0.0);

        if self.red_base.hp <= 0.0 {
            self.red_base.hp = 0.0;
            self.status = BattleStatus::BlueWin;
            self.units.clear();
        } else if self.blue_base.hp <= 0.0 {
            self.blue_base.hp = 0.0;
            self.status = BattleStatus::RedWin;
            self.units.clear();
        }
    }

    fn move_units(&mut self, delta_seconds: f32) {
        let red_base = self.red_base.clone();
        let blue_base = self.blue_base.clone();
        let snapshot: Vec<(u64, Team, f32, f32)> = self
            .units
            .iter()
            .map(|unit| (unit.id, unit.team, unit.x, unit.y))
            .collect();

        for unit in &mut self.units {
            if unit.hp <= 0.0 {
                continue;
            }
            let target = find_target(unit, &snapshot, &red_base, &blue_base);
            let dx = target.0 - unit.x;
            let dy = target.1 - unit.y;
            let distance = (dx * dx + dy * dy).sqrt().max(0.0001);
            if distance <= unit.attack_range {
                continue;
            }
            let step = unit.speed * delta_seconds;
            unit.x += dx / distance * step;
            unit.y += dy / distance * step;
            unit.y = unit.y.clamp(80.0, MAP_HEIGHT - 80.0);
        }
    }

    fn resolve_combat(&mut self, delta_seconds: f32) {
        let red_base_pos = (self.red_base.x, self.red_base.y);
        let blue_base_pos = (self.blue_base.x, self.blue_base.y);
        let positions: Vec<(u64, Team, f32, f32, f32)> = self
            .units
            .iter()
            .map(|unit| (unit.id, unit.team, unit.x, unit.y, unit.damage))
            .collect();

        let mut base_damage_red = 0.0_f32;
        let mut base_damage_blue = 0.0_f32;
        let mut unit_damage: Vec<(u64, f32)> = Vec::new();

        for unit in &mut self.units {
            if unit.hp <= 0.0 {
                continue;
            }
            unit.attack_timer = (unit.attack_timer - delta_seconds).max(0.0);
            if unit.attack_timer > 0.0 {
                continue;
            }

            let enemy_unit = positions
                .iter()
                .filter(|item| item.1 != unit.team && item.0 != unit.id)
                .map(|item| {
                    let dx = item.2 - unit.x;
                    let dy = item.3 - unit.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    (item.0, distance)
                })
                .filter(|item| item.1 <= unit.attack_range)
                .min_by(|left, right| left.1.partial_cmp(&right.1).unwrap());

            if let Some((target_id, _)) = enemy_unit {
                unit_damage.push((target_id, unit.damage));
                unit.attack_timer = unit.attack_cooldown;
                continue;
            }

            let (enemy_base_x, enemy_base_y) = match unit.team {
                Team::Red => blue_base_pos,
                Team::Blue => red_base_pos,
            };
            let dx = enemy_base_x - unit.x;
            let dy = enemy_base_y - unit.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance <= unit.attack_range + BASE_RADIUS - UNIT_RADIUS {
                match unit.team {
                    Team::Red => base_damage_blue += unit.damage,
                    Team::Blue => base_damage_red += unit.damage,
                }
                unit.attack_timer = unit.attack_cooldown;
            }
        }

        self.red_base.hp = (self.red_base.hp - base_damage_red).max(0.0);
        self.blue_base.hp = (self.blue_base.hp - base_damage_blue).max(0.0);
        for (target_id, damage) in unit_damage {
            if let Some(target) = self.units.iter_mut().find(|unit| unit.id == target_id) {
                target.hp = (target.hp - damage).max(0.0);
            }
        }
    }
}

fn find_target(
    unit: &Unit,
    others: &[(u64, Team, f32, f32)],
    red_base: &Base,
    blue_base: &Base,
) -> (f32, f32) {
    let nearest_enemy = others
        .iter()
        .filter(|item| item.1 != unit.team && item.0 != unit.id)
        .map(|item| {
            let dx = item.2 - unit.x;
            let dy = item.3 - unit.y;
            let distance = (dx * dx + dy * dy).sqrt();
            (item.2, item.3, distance)
        })
        .min_by(|left, right| left.2.partial_cmp(&right.2).unwrap());

    if let Some((x, y, distance)) = nearest_enemy {
        if distance < 220.0 {
            return (x, y);
        }
    }

    match unit.team {
        Team::Red => (blue_base.x, blue_base.y),
        Team::Blue => (red_base.x, red_base.y),
    }
}
