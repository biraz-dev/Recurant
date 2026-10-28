//! The randomizer. Below the ruins, a run passes through stages of four
//! floors each. Every run shuffles the order of the six stage themes, and
//! each theme has its own inhabitants; floor size, room count and enemy
//! count vary floor by floor, and grow with depth.

use crate::dungeon::GenParams;
use crate::enemy::EnemyKind;
use crate::rng::{Rng, mix64};

/// Floors per stage.
pub const FLOORS_PER_STAGE: u32 = 4;
/// The first generated floor (floor 1 is the ruins).
const FIRST_DEPTH: u32 = 2;

/// What a stage is: its name, look and who lives there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Theme {
    /// Damp burial vaults: the shambling dead.
    #[default]
    Crypt,
    /// Bone-lined tunnels: skeleton soldiers.
    Catacombs,
    /// Sealed royal tombs: mummies and their king.
    Tombs,
    /// Hot, cracked depths: things made of embers.
    EmberDeep,
    /// Cold halls where the dead drift.
    Haunted,
    /// A buried fortress: knights, living and otherwise.
    FallenKeep,
}

impl Theme {
    pub const ALL: [Theme; 6] = [
        Theme::Crypt,
        Theme::Catacombs,
        Theme::Tombs,
        Theme::EmberDeep,
        Theme::Haunted,
        Theme::FallenKeep,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Theme::Crypt => "THE CRYPT",
            Theme::Catacombs => "THE CATACOMBS",
            Theme::Tombs => "THE TOMBS",
            Theme::EmberDeep => "THE EMBER DEEP",
            Theme::Haunted => "THE HAUNTED HALLS",
            Theme::FallenKeep => "THE FALLEN KEEP",
        }
    }

    /// Who lives here, most common first.
    pub const fn kinds(self) -> &'static [EnemyKind] {
        use EnemyKind::*;
        match self {
            Theme::Crypt => &[Zombie, Undead, Mummy],
            Theme::Catacombs => &[SkeletonWarrior, SkeletonKnight, EmberSkeleton],
            Theme::Tombs => &[Mummy, RoyalMummy, Undead],
            Theme::EmberDeep => &[EmberSkeleton, InnerDemon, Undead],
            Theme::Haunted => &[Wraith, Zombie, Undead],
            Theme::FallenKeep => &[SkeletonKnight, DemonKnight, SkeletonWarrior],
        }
    }
}

/// Which stage a floor belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stage {
    pub theme: Theme,
    /// 1-based count of stages reached this run.
    pub number: u32,
    /// Whether this floor is the stage's first (worth announcing).
    pub first_floor: bool,
}

/// The stage of generated floor `depth` (2 and deeper) of a run.
pub fn stage_for(run_seed: u64, depth: u32) -> Stage {
    let index = depth.saturating_sub(FIRST_DEPTH) / FLOORS_PER_STAGE;
    let n = Theme::ALL.len() as u32;
    let cycle = index / n;
    let order = order_for(run_seed, cycle);
    let mut theme = order[(index % n) as usize];
    // Don't repeat a theme across the seam between two shuffles.
    if index.is_multiple_of(n)
        && cycle > 0
        && order_for(run_seed, cycle - 1)[n as usize - 1] == theme
    {
        theme = order[1];
    } else if index % n == 1
        && cycle > 0
        && order_for(run_seed, cycle - 1)[n as usize - 1] == order[0]
    {
        theme = order[0];
    }
    Stage {
        theme,
        number: index + 1,
        first_floor: depth
            .saturating_sub(FIRST_DEPTH)
            .is_multiple_of(FLOORS_PER_STAGE),
    }
}

/// One shuffle of every theme (Fisher-Yates), per run and cycle.
fn order_for(run_seed: u64, cycle: u32) -> [Theme; 6] {
    let mut order = Theme::ALL;
    let mut rng = Rng::new(mix64(run_seed ^ 0x57A6_E000 ^ (cycle as u64) << 32));
    for i in (1..order.len()).rev() {
        let j = rng.range(0, i as i32) as usize;
        order.swap(i, j);
    }
    order
}

/// This floor's generator settings: bigger, roomier and busier with depth,
/// with some floor-to-floor variety.
pub fn params_for(base: &GenParams, run_seed: u64, depth: u32) -> GenParams {
    let mut rng = Rng::new(mix64(run_seed ^ 0x9A4A_0000 ^ depth as u64));
    let grow = (depth.saturating_sub(FIRST_DEPTH) / 3).min(4) as i32;
    let width = base.width + grow / 2 + rng.range(-1, 2);
    let height = base.height + grow / 2 + rng.range(-1, 2);
    let enemies =
        base.enemies + (depth.saturating_sub(FIRST_DEPTH) / 3) as usize + rng.range(0, 1) as usize;
    GenParams {
        width,
        height,
        max_rooms: (base.max_rooms as i32 + grow / 2 + rng.range(-1, 1)).max(3) as usize,
        room_max: base.room_max + rng.range(0, 1),
        enemies: enemies.min(MAX_ENEMIES),
        ..*base
    }
}

/// A floor never holds more than this many enemies.
pub const MAX_ENEMIES: usize = 5;

/// An enemy kind for this stage and depth: one of the stage's own, among
/// those that have turned up by this depth; early on, the classic pair.
pub fn kind_for(theme: Theme, depth: u32, roll: u64) -> EnemyKind {
    let own: alloc::vec::Vec<EnemyKind> = theme
        .kinds()
        .iter()
        .copied()
        .filter(|k| k.first_depth() <= depth)
        .collect();
    let pool: alloc::vec::Vec<EnemyKind> = if own.is_empty() {
        EnemyKind::ALL
            .into_iter()
            .filter(|k| k.first_depth() <= depth.max(FIRST_DEPTH))
            .collect()
    } else {
        own
    };
    // The stage's signature kind turns up most.
    let r = (roll % 100) as usize;
    if r < 40 {
        pool[0]
    } else {
        pool[(roll / 100) as usize % pool.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_last_four_floors_and_runs_differ() {
        let s = |seed, depth| stage_for(seed, depth).theme;
        for seed in 0..50u64 {
            for d in [2, 6, 10] {
                assert_eq!(s(seed, d), s(seed, d + 3), "seed {seed} depth {d}");
                assert_ne!(s(seed, d), s(seed, d + 4), "seed {seed} depth {d}");
            }
            assert!(stage_for(seed, 2).first_floor && !stage_for(seed, 3).first_floor);
            // Six stages in a row are all different themes.
            let six: std::collections::HashSet<_> = (0..6).map(|i| s(seed, 2 + 4 * i)).collect();
            assert_eq!(six.len(), 6);
            // No theme repeats back to back, even between shuffles.
            for i in 0..30 {
                assert_ne!(
                    s(seed, 2 + 4 * i),
                    s(seed, 6 + 4 * i),
                    "seed {seed} stage {i}"
                );
            }
        }
        let firsts: std::collections::HashSet<_> = (0..40u64).map(|seed| s(seed, 2)).collect();
        assert!(firsts.len() >= 4, "runs should start in different stages");
    }

    #[test]
    fn floors_grow_but_stay_within_bounds() {
        let base = GenParams::default();
        for seed in 0..30u64 {
            for depth in 2..60 {
                let p = params_for(&base, seed, depth);
                assert!((10..=16).contains(&p.width) && (10..=16).contains(&p.height));
                assert!((1..=MAX_ENEMIES).contains(&p.enemies));
            }
            assert!(params_for(&base, seed, 40).enemies > params_for(&base, seed, 2).enemies);
        }
    }

    #[test]
    fn stages_bring_their_own_kinds_once_deep_enough() {
        for theme in Theme::ALL {
            for depth in 2..40 {
                for roll in 0..200u64 {
                    let k = kind_for(theme, depth, mix64(roll));
                    assert!(k.first_depth() <= depth.max(2), "{theme:?} {depth} {k:?}");
                }
            }
            // Deep down, every inhabitant appears.
            let seen: std::collections::HashSet<_> =
                (0..400u64).map(|r| kind_for(theme, 20, mix64(r))).collect();
            assert_eq!(seen.len(), theme.kinds().len(), "{theme:?}");
        }
    }
}
