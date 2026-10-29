//! Dreamscape floors: hand-built from the Dreamscape modular kits
//! (`art/kits/dreamscape-modular-v1`), each a contrasting place the run
//! passes through. For now one, the Mindglass Sanctum: dark violet stone and
//! crystal, floor 2, between the sunlit ruins and the dungeon below.
//!
//! `tools/make_dreamscape.py` lays the floor out from the kit's modules and
//! writes its map (`dream/<theme>.map`): walkable cells, the start, the way
//! down, and edges closed between walkable cells (none here). Exhibit cells,
//! where the crystals stand, are solid to the core.

use crate::dungeon::{Floor, Place};
use crate::grid::{Facing, GridPos};

const MINDGLASS: &str = include_str!("dream/mindglass-sanctum.map");

/// Who walks the sanctum: one in the colonnaded hall, one in the reliquary
/// by the way down.
const SENTRIES: [GridPos; 2] = [GridPos::new(7, 8), GridPos::new(6, 3)];

pub fn floor() -> Floor {
    let (map, start, stairs) = crate::ruins::parse_map(MINDGLASS);
    Floor {
        map,
        start,
        start_facing: Facing::North,
        stairs,
        // Up the reliquary to the ring: the steps lead down before it.
        stairs_dir: Facing::North,
        enemy_spawns: SENTRIES.to_vec(),
        place: Place::Dream,
        stage: None,
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn the_sanctum_is_whole_and_reachable() {
        let f = floor();
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
        let f = floor();
        // A column in the hall, and the ring behind the way down.
        assert!(!f.map.is_walkable(GridPos::new(4, 7)));
        assert!(!f.map.is_walkable(GridPos::new(7, 1)));
        // Through the doorways: corridor to hall, hall to reliquary.
        assert!(f.map.can_step(GridPos::new(7, 11), Facing::North));
        assert!(f.map.can_step(GridPos::new(7, 6), Facing::North));
        assert!(f.map.can_step(GridPos::new(7, 5), Facing::North));
    }
}
