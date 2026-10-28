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
    /// Bones and a round shield: quick, but brittle.
    SkeletonWarrior,
    /// Slow and hard to put down; its blows are heavy.
    Mummy,
    /// A charred skeleton with coals for marrow: fast.
    EmberSkeleton,
    /// Drifts quickly and notices you from further off.
    Wraith,
    /// An armoured skeleton: tough, and hits hard.
    SkeletonKnight,
    /// Quick and vicious.
    InnerDemon,
    /// The strongest: slow to wind up, but every blow tells.
    DemonKnight,
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
            EnemyKind::SkeletonWarrior => Stats {
                max_hp: 7,
                step_interval: 0.5,
                wind_up: 0.55,
                damage_min: 2,
                damage_max: 4,
                ..zombie
            },
            EnemyKind::Mummy => Stats {
                max_hp: 13,
                step_interval: 0.8,
                wind_up: 0.7,
                recover: 1.0,
                damage_min: 4,
                damage_max: 6,
                ..zombie
            },
            EnemyKind::EmberSkeleton => Stats {
                step_interval: 0.45,
                wind_up: 0.5,
                ..zombie
            },
            EnemyKind::Wraith => Stats {
                max_hp: 7,
                notice_range: 7,
                step_interval: 0.4,
                damage_min: 3,
                damage_max: 6,
                ..zombie
            },
            EnemyKind::SkeletonKnight => Stats {
                max_hp: 12,
                damage_min: 4,
                damage_max: 6,
                ..zombie
            },
            EnemyKind::InnerDemon => Stats {
                max_hp: 11,
                step_interval: 0.45,
                wind_up: 0.55,
                damage_min: 4,
                damage_max: 7,
                ..zombie
            },
            EnemyKind::DemonKnight => Stats {
                max_hp: 16,
                wind_up: 0.7,
                recover: 0.8,
                damage_min: 5,
                damage_max: 8,
                ..zombie
            },
        }
    }

    /// Every kind, weakest first.
    pub const ALL: [EnemyKind; 9] = [
        EnemyKind::Zombie,
        EnemyKind::SkeletonWarrior,
        EnemyKind::Mummy,
        EnemyKind::Undead,
        EnemyKind::EmberSkeleton,
        EnemyKind::Wraith,
        EnemyKind::SkeletonKnight,
        EnemyKind::InnerDemon,
        EnemyKind::DemonKnight,
    ];

    /// The first dungeon floor where this kind can turn up.
    pub const fn first_depth(self) -> u32 {
        match self {
            EnemyKind::Zombie | EnemyKind::SkeletonWarrior => 2,
            EnemyKind::Mummy | EnemyKind::Undead => 3,
            EnemyKind::EmberSkeleton | EnemyKind::Wraith => 4,
            EnemyKind::SkeletonKnight | EnemyKind::InnerDemon => 5,
            EnemyKind::DemonKnight => 6,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_keeps_a_readable_tell() {
        for kind in EnemyKind::ALL {
            let s = kind.stats();
            assert!(s.wind_up >= 0.5, "{kind:?} winds up too fast to react");
            assert!(s.max_hp > 0 && s.damage_min <= s.damage_max, "{kind:?}");
        }
        // The zombie is unchanged.
        assert_eq!(EnemyKind::Zombie.stats().max_hp, tuning::MAX_HP);
    }
}
