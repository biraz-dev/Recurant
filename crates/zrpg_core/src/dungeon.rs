//! Seeded floor generator: small rectangular rooms joined by L-shaped
//! corridors. Every room is linked to the one before it, so the whole floor
//! is always connected.

use alloc::vec::Vec;

use crate::grid::{Facing, GridPos};
use crate::map::{Map, Tile};
use crate::rng::{Rng, mix64};

#[derive(Clone, Copy, Debug)]
pub struct GenParams {
    /// Total size including the outer wall ring.
    pub width: i32,
    pub height: i32,
    pub max_rooms: usize,
    pub room_min: i32,
    pub room_max: i32,
}

impl Default for GenParams {
    fn default() -> Self {
        Self {
            width: 12,
            height: 12,
            max_rooms: 5,
            room_min: 2,
            room_max: 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Room {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Room {
    fn center(&self) -> GridPos {
        GridPos::new(self.x + self.w / 2, self.y + self.h / 2)
    }

    /// Overlap test with a one-cell gap so rooms keep a wall between them.
    fn touches(&self, o: &Room) -> bool {
        self.x - 1 < o.x + o.w
            && o.x - 1 < self.x + self.w
            && self.y - 1 < o.y + o.h
            && o.y - 1 < self.y + self.h
    }
}

/// One generated dungeon floor.
#[derive(Clone, Debug)]
pub struct Floor {
    pub map: Map,
    pub start: GridPos,
    pub start_facing: Facing,
    pub stairs: GridPos,
}

impl Floor {
    /// Floor `depth` of the run identified by `run_seed`. Same inputs, same floor.
    pub fn for_depth(run_seed: u64, depth: u32, params: &GenParams) -> Floor {
        generate(mix64(run_seed ^ mix64(depth as u64 + 1)), params)
    }
}

pub fn generate(seed: u64, params: &GenParams) -> Floor {
    assert!(params.width >= 5 && params.height >= 5, "map too small");
    let mut rng = Rng::new(seed);
    let mut map = Map::filled(params.width, params.height, Tile::Wall);

    let rooms = place_rooms(&mut rng, params);
    for room in &rooms {
        for y in room.y..room.y + room.h {
            for x in room.x..room.x + room.w {
                map.set(GridPos::new(x, y), Tile::Floor);
            }
        }
    }
    for pair in rooms.windows(2) {
        carve_corridor(&mut map, &mut rng, pair[0].center(), pair[1].center());
    }
    // An occasional extra link makes loops, so floors aren't pure trees.
    if rooms.len() > 2 && rng.chance(50) {
        let a = rng.range(0, rooms.len() as i32 - 1) as usize;
        let b = rng.range(0, rooms.len() as i32 - 1) as usize;
        if a != b {
            carve_corridor(&mut map, &mut rng, rooms[a].center(), rooms[b].center());
        }
    }

    let start = rooms[0].center();
    let stairs = farthest_cell(&map, start);
    map.set(stairs, Tile::StairsDown);
    let start_facing = most_open_facing(&map, start);

    Floor {
        map,
        start,
        start_facing,
        stairs,
    }
}

fn place_rooms(rng: &mut Rng, p: &GenParams) -> Vec<Room> {
    let mut rooms: Vec<Room> = Vec::new();
    for _ in 0..p.max_rooms * 8 {
        if rooms.len() >= p.max_rooms {
            break;
        }
        let w = rng.range(p.room_min, p.room_max.min(p.width - 2));
        let h = rng.range(p.room_min, p.room_max.min(p.height - 2));
        let room = Room {
            x: rng.range(1, p.width - 1 - w),
            y: rng.range(1, p.height - 1 - h),
            w,
            h,
        };
        if rooms.iter().all(|r| !r.touches(&room)) {
            rooms.push(room);
        }
    }
    // The first attempt always fits on an empty map, so `rooms` is never
    // empty; with a single room add a second one so there is somewhere to go.
    if rooms.len() == 1 {
        let r = rooms[0];
        let x = if r.x > p.width / 2 { 1 } else { p.width - 2 };
        let y = if r.y > p.height / 2 { 1 } else { p.height - 2 };
        rooms.push(Room { x, y, w: 1, h: 1 });
    }
    rooms
}

fn carve_corridor(map: &mut Map, rng: &mut Rng, a: GridPos, b: GridPos) {
    let horizontal_first = rng.chance(50);
    let corner = if horizontal_first {
        GridPos::new(b.x, a.y)
    } else {
        GridPos::new(a.x, b.y)
    };
    carve_line(map, a, corner);
    carve_line(map, corner, b);
}

fn carve_line(map: &mut Map, a: GridPos, b: GridPos) {
    let (dx, dy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let mut p = a;
    loop {
        if map.get(p) == Tile::Wall {
            map.set(p, Tile::Floor);
        }
        if p == b {
            break;
        }
        p = GridPos::new(p.x + dx, p.y + dy);
    }
}

fn farthest_cell(map: &Map, from: GridPos) -> GridPos {
    let dist = map.distances(from);
    map.positions()
        .filter_map(|p| dist[map.index(p)].map(|d| (d, p)))
        .max_by_key(|&(d, p)| (d, -p.y, -p.x))
        .map(|(_, p)| p)
        .unwrap_or(from)
}

/// The facing with the longest clear line of sight: a nice first view.
fn most_open_facing(map: &Map, from: GridPos) -> Facing {
    let sight = |dir: Facing| {
        let mut p = from + dir;
        let mut n = 0;
        while map.is_walkable(p) {
            n += 1;
            p = p + dir;
        }
        n
    };
    Facing::ALL
        .into_iter()
        .max_by_key(|&d| (sight(d), -(d as i32)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walkable_count(map: &Map) -> usize {
        map.positions().filter(|&p| map.is_walkable(p)).count()
    }

    #[test]
    fn floors_are_connected_and_sealed() {
        let params = GenParams::default();
        for seed in 0..2000u64 {
            let f = generate(seed, &params);
            let m = &f.map;
            assert_eq!((m.width(), m.height()), (12, 12));
            // Outer ring is solid.
            for p in m.positions() {
                if p.x == 0 || p.y == 0 || p.x == m.width() - 1 || p.y == m.height() - 1 {
                    assert_eq!(m.get(p), Tile::Wall, "seed {seed}: hole in border at {p:?}");
                }
            }
            // Every walkable cell is reachable from the start.
            let dist = m.distances(f.start);
            let reachable = dist.iter().filter(|d| d.is_some()).count();
            assert_eq!(
                reachable,
                walkable_count(m),
                "seed {seed}: disconnected floor"
            );
            assert_eq!(m.get(f.stairs), Tile::StairsDown);
            assert_ne!(f.start, f.stairs, "seed {seed}");
            assert!(
                m.is_walkable(f.start + f.start_facing),
                "seed {seed}: facing a wall"
            );
            assert!(walkable_count(m) >= 8, "seed {seed}: floor too cramped");
        }
    }

    #[test]
    fn same_seed_same_floor() {
        let p = GenParams::default();
        let (a, b) = (Floor::for_depth(99, 3, &p), Floor::for_depth(99, 3, &p));
        assert_eq!(a.start, b.start);
        assert_eq!(a.stairs, b.stairs);
        assert!(a.map.positions().all(|q| a.map.get(q) == b.map.get(q)));
    }

    #[test]
    fn depths_differ() {
        let p = GenParams::default();
        let a = Floor::for_depth(1, 1, &p);
        let b = Floor::for_depth(1, 2, &p);
        assert!(a.map.positions().any(|q| a.map.get(q) != b.map.get(q)));
    }

    #[test]
    fn other_sizes_work() {
        for (w, h) in [(5, 5), (8, 20), (32, 32)] {
            let params = GenParams {
                width: w,
                height: h,
                ..GenParams::default()
            };
            for seed in 0..200 {
                let f = generate(seed, &params);
                let dist = f.map.distances(f.start);
                assert!(dist[f.map.index(f.stairs)].is_some());
            }
        }
    }
}
