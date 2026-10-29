//! Dream floors, built from the Dreamscape modular kits
//! (`art/kits/dreamscape-modular-v1`): the run's interludes, on floor 2 and
//! between every two stages (see `dungeon.rs`).
//!
//! There are two kinds. The Mindglass Sanctum is hand-laid from one kit by
//! `tools/make_dreamscape.py`: dark violet stone and crystal. The patchworks,
//! from `tools/make_patchwork.py`, chain rooms from contrasting kits through
//! ordinary doorways, with the odd exhibit strayed in from elsewhere and a
//! room that comes back, one thing changed. Both write maps of walkable
//! cells, the start, the way down and where sentries wait; exhibit cells are
//! solid to the core.
//!
//! Each run shuffles every dream and takes them in turn, so its interludes
//! differ from the last run's and don't repeat until it has seen them all.

use alloc::vec::Vec;

use crate::dungeon::{Floor, Place};
use crate::grid::{Facing, GridPos};
use crate::rng::{Rng, mix64};

pub mod patchwork;

const MINDGLASS: &str = include_str!("dream/mindglass-sanctum.map");

/// The sanctum's sentries: one in the colonnaded hall, one in the reliquary
/// by the way down.
const SANCTUM_SENTRIES: [GridPos; 2] = [GridPos::new(7, 8), GridPos::new(6, 3)];

/// Which dream: the sanctum, or patchwork `id - 1`.
pub type DreamId = u8;
pub const SANCTUM: DreamId = 0;

/// How many dreams there are.
pub fn count() -> usize {
    1 + patchwork::FLOORS.len()
}

/// The dream for a run's interlude `n` (0 on floor 2).
pub fn pick(run_seed: u64, n: u32) -> DreamId {
    let mut order: Vec<DreamId> = (0..count() as DreamId).collect();
    let mut rng = Rng::new(mix64(run_seed ^ 0xD2EA_D2EA));
    for i in (1..order.len()).rev() {
        let j = rng.range(0, i as i32) as usize;
        order.swap(i, j);
    }
    order[n as usize % order.len()]
}

pub fn floor(id: DreamId) -> Floor {
    if id == SANCTUM {
        let (map, start, stairs, _) = crate::ruins::parse_map(MINDGLASS);
        return Floor {
            map,
            start,
            start_facing: Facing::North,
            stairs,
            // Up the reliquary to the ring: the steps lead down before it.
            stairs_dir: Facing::North,
            enemy_spawns: SANCTUM_SENTRIES.to_vec(),
            place: Place::Dream(id),
            stage: None,
        };
    }
    let p = &patchwork::FLOORS[id as usize - 1];
    let (map, start, stairs, sentries) = crate::ruins::parse_map(p.map);
    Floor {
        map,
        start,
        start_facing: Facing::North,
        stairs,
        stairs_dir: p.stairs_dir,
        enemy_spawns: sentries,
        place: Place::Dream(id),
        stage: None,
    }
}

/// The sanctum, as the run that only has it knows it.
pub fn sanctum() -> Floor {
    floor(SANCTUM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sanctum_is_whole_and_reachable() {
        let f = sanctum();
        let walkable: Vec<GridPos> = f
            .map
            .positions()
            .filter(|&p| f.map.is_walkable(p))
            .collect();
        assert_eq!(walkable.len(), 65);
        let dist = f.map.distances(f.start);
        assert!(walkable.iter().all(|&p| dist[f.map.index(p)].is_some()));
        assert_eq!(f.start, GridPos::new(7, 14));
        assert_eq!(f.stairs, GridPos::new(7, 2));
        for &s in &f.enemy_spawns {
            assert!(f.map.is_walkable(s) && s != f.start && s != f.stairs);
        }
    }

    #[test]
    fn exhibits_are_solid_and_doorways_open() {
        let f = sanctum();
        // A column in the hall, and the ring behind the way down.
        assert!(!f.map.is_walkable(GridPos::new(4, 7)));
        assert!(!f.map.is_walkable(GridPos::new(7, 1)));
        // Through the doorways: corridor to hall, hall to reliquary.
        assert!(f.map.can_step(GridPos::new(7, 11), Facing::North));
        assert!(f.map.can_step(GridPos::new(7, 6), Facing::North));
        assert!(f.map.can_step(GridPos::new(7, 5), Facing::North));
    }

    /// Every dream: the way down is reachable from the start, every
    /// walkable cell is, sentries stand clear of both, and you start facing
    /// into the room.
    #[test]
    fn every_dream_is_whole() {
        for id in 0..count() as DreamId {
            let f = floor(id);
            assert_eq!(f.place, Place::Dream(id));
            let dist = f.map.distances(f.start);
            for p in f.map.positions().filter(|&p| f.map.is_walkable(p)) {
                assert!(
                    dist[f.map.index(p)].is_some(),
                    "dream {id}: {p:?} unreachable"
                );
            }
            assert!(!f.enemy_spawns.is_empty(), "dream {id} has no sentries");
            for &s in &f.enemy_spawns {
                assert!(
                    f.map.is_walkable(s) && s != f.start && s != f.stairs,
                    "dream {id}"
                );
            }
            assert!(
                f.map.can_step(f.start, f.start_facing),
                "dream {id} starts facing a wall"
            );
        }
    }

    /// Dreams fall on floor 2 and between every two stages; the generated
    /// floors keep the numbers they'd have had without them. Runs saved
    /// with only the sanctum, or no dreams, keep theirs.
    #[test]
    fn interludes_fall_between_stages() {
        use crate::dungeon::{Dreams, GenParams, Slot, slot};
        let p = GenParams::default();
        let got: Vec<Slot> = (1..=13).map(|d| slot(d, &p)).collect();
        let want = [
            Slot::Ruins,
            Slot::Dream(0),
            Slot::Generated(2),
            Slot::Generated(3),
            Slot::Generated(4),
            Slot::Generated(5),
            Slot::Dream(1),
            Slot::Generated(6),
            Slot::Generated(7),
            Slot::Generated(8),
            Slot::Generated(9),
            Slot::Dream(2),
            Slot::Generated(10),
        ];
        assert_eq!(got, want);
        // Generated floors 2-5 are the first stage, 6-9 the second.
        assert_eq!(crate::stage::stage_for(1, 5).number, 1);
        assert_eq!(crate::stage::stage_for(1, 6).number, 2);
        let sanctum = GenParams {
            dreams: Dreams::Sanctum,
            ..p
        };
        assert_eq!(slot(2, &sanctum), Slot::Dream(0));
        assert_eq!(slot(7, &sanctum), Slot::Generated(6));
        let off = GenParams {
            dreams: Dreams::Off,
            ..p
        };
        assert_eq!(slot(2, &off), Slot::Generated(2));
    }

    #[test]
    fn a_run_sees_every_dream_before_any_repeats() {
        for seed in [1u64, 2, 99] {
            let mut seen: Vec<DreamId> = (0..count() as u32).map(|n| pick(seed, n)).collect();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), count());
        }
        // And runs differ.
        let a: Vec<DreamId> = (0..4).map(|n| pick(1, n)).collect();
        let b: Vec<DreamId> = (0..4).map(|n| pick(2, n)).collect();
        assert_ne!(a, b);
    }
}
