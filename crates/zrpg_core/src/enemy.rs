//! Enemies and their tiny brain.
//!
//! An enemy sleeps until the player comes within walking range, then chases.
//! Once adjacent it winds up (a visible tell), strikes if the player is still
//! next to it, and recovers. Hitting it mid-wind-up staggers it.

use crate::grid::GridPos;

/// Enemy tuning, in seconds and hit points.
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
    pub pos: GridPos,
    pub hp: i32,
    pub max_hp: i32,
    pub state: EnemyState,
    /// Counts down to the next chase step.
    pub(crate) step_timer: f32,
}

impl Enemy {
    pub fn new(id: EnemyId, pos: GridPos) -> Self {
        Self {
            id,
            pos,
            hp: tuning::MAX_HP,
            max_hp: tuning::MAX_HP,
            state: EnemyState::Dormant,
            step_timer: tuning::STEP_INTERVAL,
        }
    }

    pub fn is_winding_up(&self) -> bool {
        matches!(self.state, EnemyState::WindUp { .. })
    }
}
