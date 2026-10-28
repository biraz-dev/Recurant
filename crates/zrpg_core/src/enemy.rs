//! Enemies and their tiny brain.
//!
//! An enemy sleeps until the player comes within walking range, then chases.
//! Once adjacent it winds up (a visible tell), strikes if the player is still
//! next to it, and recovers. Hitting it mid-wind-up staggers it.

use crate::grid::GridPos;

/// The zombie's tuning (the first enemy, and the yardstick for the rest),
/// in seconds and hit points.
pub mod tuning {
    pub const MAX_HP: i32 = 8;
    /// Walking distance (in cells) at which a sleeping enemy notices you.
    pub const NOTICE_RANGE: u32 = 5;
    pub const STEP_INTERVAL: f32 = 0.6;
    pub const WIND_UP: f32 = 0.6;
    pub const RECOVER: f32 = 0.9;
    pub const STAGGER: f32 = 0.7;
    pub const DAMAGE_MIN: i32 = 3;
    pub const DAMAGE_MAX: i32 = 5;
}

pub type EnemyId = u32;

/// Who an enemy is. Each kind has its own [`Stats`]; the brain is shared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    #[default]
    Zombie,
    /// Smouldering undead: embers in its cracked flesh. A little sturdier
    /// than a zombie and quicker to close in.
    Undead,
}

/// One kind's numbers, in seconds, cells and hit points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub max_hp: i32,
    /// Walking distance at which a sleeping enemy notices you.
    pub notice_range: u32,
    pub step_interval: f32,
    pub wind_up: f32,
    pub recover: f32,
    pub damage_min: i32,
    pub damage_max: i32,
}

impl EnemyKind {
    pub const fn stats(self) -> Stats {
        use tuning::*;
        let zombie = Stats {
            max_hp: MAX_HP,
            notice_range: NOTICE_RANGE,
            step_interval: STEP_INTERVAL,
            wind_up: WIND_UP,
            recover: RECOVER,
            damage_min: DAMAGE_MIN,
            damage_max: DAMAGE_MAX,
        };
        match self {
            EnemyKind::Zombie => zombie,
            EnemyKind::Undead => Stats {
                max_hp: 10,
                step_interval: 0.5,
                ..zombie
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnemyState {
    Dormant,
    Chasing,
    /// Preparing a strike; it lands when `left` reaches zero.
    WindUp {
        left: f32,
    },
    /// After a strike (or a stagger), unable to act until `left` runs out.
    Recover {
        left: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enemy {
    pub id: EnemyId,
    pub kind: EnemyKind,
    pub pos: GridPos,
    pub hp: i32,
    pub max_hp: i32,
    pub state: EnemyState,
    /// Counts down to the next chase step.
    pub(crate) step_timer: f32,
}

impl Enemy {
    /// A zombie.
    pub fn new(id: EnemyId, pos: GridPos) -> Self {
        Self::of_kind(id, pos, EnemyKind::Zombie)
    }

    pub fn of_kind(id: EnemyId, pos: GridPos, kind: EnemyKind) -> Self {
        let stats = kind.stats();
        Self {
            id,
            kind,
            pos,
            hp: stats.max_hp,
            max_hp: stats.max_hp,
            state: EnemyState::Dormant,
            step_timer: stats.step_interval,
        }
    }

    pub fn stats(&self) -> Stats {
        self.kind.stats()
    }

    pub fn is_winding_up(&self) -> bool {
        matches!(self.state, EnemyState::WindUp { .. })
    }
}
