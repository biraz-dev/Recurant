//! The sandstone ruins: a hand-built first floor, open to the sky, where a
//! stair at the statue's feet leads down into the dungeon.
//!
//! The layout comes from the ruins asset kit (`art/kits/sandstone-ruins-v1`),
//! converted by `tools/make_ruins.py` into `ruins.map`: which cells are
//! walkable, and which edges between walkable cells are still closed
//! (parapets between the bridge and the pool, the sides of the stairs).

use alloc::vec::Vec;

use crate::dungeon::{Floor, Place};
use crate::grid::{Facing, GridPos};
use crate::map::{Map, Tile};

const MAP: &str = include_str!("ruins.map");

/// Where the ruins' two sentries wait: one in the junction past the
/// corridor, one guarding the statue chamber.
const SENTRIES: [GridPos; 2] = [GridPos::new(4, 9), GridPos::new(15, 10)];

/// What a ruins cell is underfoot. Only the front-end cares (for heights).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ground {
    Solid,
    Dry,
    Bridge,
    Water,
    Stairs,
}

fn rows() -> (i32, i32, impl Iterator<Item = &'static str>) {
    // Comments come first; map rows start with '#' too, so stop at the size.
    let mut lines = MAP.lines().skip_while(|l| l.starts_with('#'));
    let mut size = lines
        .next()
        .expect("ruins.map has a size line")
        .split(' ')
        .map(|n| n.parse::<i32>().expect("ruins.map size"));
    let (w, h) = (size.next().unwrap(), size.next().unwrap());
    (w, h, lines)
}

/// The map character at `p`, or `'#'` outside the map.
fn cell(p: GridPos) -> char {
    let (w, h, mut lines) = rows();
    if p.x < 0 || p.y < 0 || p.x >= w || p.y >= h {
        return '#';
    }
    lines
        .nth(p.y as usize)
        .and_then(|row| row.chars().nth(p.x as usize))
        .unwrap_or('#')
}

pub fn ground(p: GridPos) -> Ground {
    match cell(p) {
        '.' | 'S' | '>' => Ground::Dry,
        '=' => Ground::Bridge,
        '~' => Ground::Water,
        '/' => Ground::Stairs,
        _ => Ground::Solid,
    }
}

pub fn floor() -> Floor {
    let (w, h, lines) = rows();
    let lines: Vec<&str> = lines.collect();
    assert_eq!(
        lines.len(),
        2 * h as usize,
        "ruins.map: map then edge masks"
    );
    let mut map = Map::filled(w, h, Tile::Wall);
    let (mut start, mut stairs) = (None, None);
    for (y, row) in lines[..h as usize].iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let p = GridPos::new(x as i32, y as i32);
            match c {
                '#' => continue,
                'S' => start = Some(p),
                '>' => stairs = Some(p),
                _ => {}
            }
            map.set(
                p,
                if c == '>' {
                    Tile::StairsDown
                } else {
                    Tile::Floor
                },
            );
        }
    }
    for (y, row) in lines[h as usize..].iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let mask = c.to_digit(16).expect("ruins.map: hex edge mask");
            for dir in Facing::ALL {
                if mask & (1 << dir as u8) != 0 {
                    map.block(GridPos::new(x as i32, y as i32), dir);
                }
            }
        }
    }
    Floor {
        map,
        start: start.expect("ruins.map has a start"),
        start_facing: Facing::North,
        stairs: stairs.expect("ruins.map has a way down"),
        // Walking up to the statue: the steps lead down beneath it.
        stairs_dir: Facing::North,
        enemy_spawns: SENTRIES.to_vec(),
        place: Place::Ruins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_walkable_cell_is_reachable_and_matches_the_kit() {
        let f = floor();
        let dist = f.map.distances(f.start);
        let walkable: Vec<GridPos> = f
            .map
            .positions()
            .filter(|&p| f.map.is_walkable(p))
            .collect();
        // The kit declares 95 walkable cells, all reachable from the start.
        assert_eq!(walkable.len(), 95);
        assert!(walkable.iter().all(|&p| dist[f.map.index(p)].is_some()));
        assert_eq!(f.start, GridPos::new(2, 17));
        assert_eq!(f.stairs, GridPos::new(13, 8));
        for &s in &f.enemy_spawns {
            assert!(f.map.is_walkable(s) && s != f.start && s != f.stairs);
        }
    }

    #[test]
    fn parapets_and_stair_sides_are_closed() {
        let f = floor();
        // Bridge deck to the pool below: a parapet.
        assert!(!f.map.can_step(GridPos::new(8, 10), Facing::South));
        assert!(!f.map.can_step(GridPos::new(8, 11), Facing::North));
        // Stairs out of the water run east-west only.
        assert!(f.map.can_step(GridPos::new(6, 11), Facing::East));
        assert!(f.map.can_step(GridPos::new(6, 11), Facing::West));
        assert!(!f.map.can_step(GridPos::new(6, 11), Facing::South));
        assert!(!f.map.can_step(GridPos::new(6, 11), Facing::North));
        // Across a parapet is not within reach.
        assert!(!f.map.within_reach(GridPos::new(8, 10), GridPos::new(8, 11)));
        assert!(f.map.within_reach(GridPos::new(8, 10), GridPos::new(8, 9)));
    }

    #[test]
    fn both_routes_reach_the_statue() {
        // The dry route over the bridge, and the wading route through the
        // southern pool, as drawn in the kit's top-down plan.
        let f = floor();
        let mut dry = f.map.clone();
        for x in 6..=10 {
            dry.set(GridPos::new(x, 11), Tile::Wall);
            dry.set(GridPos::new(x, 12), Tile::Wall);
        }
        let mut wet = f.map.clone();
        for x in 6..=10 {
            wet.set(GridPos::new(x, 9), Tile::Wall);
            wet.set(GridPos::new(x, 10), Tile::Wall);
        }
        for m in [dry, wet] {
            assert!(m.distances(f.start)[m.index(f.stairs)].is_some());
        }
    }

    #[test]
    fn ground_heights_follow_the_kit() {
        assert_eq!(ground(GridPos::new(2, 17)), Ground::Dry);
        assert_eq!(ground(GridPos::new(8, 10)), Ground::Bridge);
        assert_eq!(ground(GridPos::new(8, 12)), Ground::Water);
        assert_eq!(ground(GridPos::new(10, 11)), Ground::Stairs);
        assert_eq!(ground(GridPos::new(0, 0)), Ground::Solid);
        assert_eq!(ground(GridPos::new(-1, 40)), Ground::Solid);
    }
}
