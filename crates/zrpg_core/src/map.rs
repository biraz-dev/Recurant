//! The tile map of one dungeon floor.

use alloc::vec;
use alloc::vec::Vec;

use crate::grid::GridPos;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    StairsDown,
}

impl Tile {
    pub fn is_walkable(self) -> bool {
        !matches!(self, Tile::Wall)
    }
}

#[derive(Clone, Debug)]
pub struct Map {
    width: i32,
    height: i32,
    tiles: Vec<Tile>,
}

impl Map {
    pub fn filled(width: i32, height: i32, tile: Tile) -> Self {
        assert!(width > 0 && height > 0);
        Self {
            width,
            height,
            tiles: vec![tile; (width * height) as usize],
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn in_bounds(&self, p: GridPos) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height
    }

    /// Out-of-bounds reads are solid wall, so callers never need bounds checks.
    pub fn get(&self, p: GridPos) -> Tile {
        if self.in_bounds(p) {
            self.tiles[(p.y * self.width + p.x) as usize]
        } else {
            Tile::Wall
        }
    }

    pub fn set(&mut self, p: GridPos, tile: Tile) {
        if self.in_bounds(p) {
            self.tiles[(p.y * self.width + p.x) as usize] = tile;
        }
    }

    pub fn is_walkable(&self, p: GridPos) -> bool {
        self.get(p).is_walkable()
    }

    /// Every cell position, row by row.
    pub fn positions(&self) -> impl Iterator<Item = GridPos> + '_ {
        (0..self.height).flat_map(move |y| (0..self.width).map(move |x| GridPos::new(x, y)))
    }

    /// Breadth-first walking distance from `from` to every cell
    /// (`None` = unreachable). Indexed like the tile array.
    pub fn distances(&self, from: GridPos) -> Vec<Option<u32>> {
        let mut dist = vec![None; self.tiles.len()];
        if !self.is_walkable(from) {
            return dist;
        }
        let mut queue = alloc::collections::VecDeque::new();
        dist[self.index(from)] = Some(0);
        queue.push_back(from);
        while let Some(p) = queue.pop_front() {
            let d = dist[self.index(p)].unwrap_or(0);
            for dir in crate::grid::Facing::ALL {
                let n = p + dir;
                if self.is_walkable(n) && dist[self.index(n)].is_none() {
                    dist[self.index(n)] = Some(d + 1);
                    queue.push_back(n);
                }
            }
        }
        dist
    }

    pub fn index(&self, p: GridPos) -> usize {
        (p.y * self.width + p.x) as usize
    }
}

/// ASCII view for debugging: `#` wall, `.` floor, `>` stairs.
impl core::fmt::Display for Map {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for y in 0..self.height {
            for x in 0..self.width {
                let c = match self.get(GridPos::new(x, y)) {
                    Tile::Wall => '#',
                    Tile::Floor => '.',
                    Tile::StairsDown => '>',
                };
                write!(f, "{c}")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
