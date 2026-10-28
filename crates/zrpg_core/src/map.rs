//! The tile map of one dungeon floor.

use alloc::vec;
use alloc::vec::Vec;

use crate::grid::{Facing, GridPos};

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
    /// Per cell, edges closed even between two walkable cells (a parapet,
    /// the side of a stair): one bit per `Facing`.
    blocked: Vec<u8>,
}

impl Map {
    pub fn filled(width: i32, height: i32, tile: Tile) -> Self {
        assert!(width > 0 && height > 0);
        Self {
            width,
            height,
            tiles: vec![tile; (width * height) as usize],
            blocked: vec![0; (width * height) as usize],
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

    /// Close the edge between `p` and its neighbour in `dir`, both ways.
    pub fn block(&mut self, p: GridPos, dir: Facing) {
        for (cell, side) in [(p, dir), (p + dir, dir.back())] {
            if self.in_bounds(cell) {
                let i = self.index(cell);
                self.blocked[i] |= 1 << side as u8;
            }
        }
    }

    pub fn is_blocked(&self, p: GridPos, dir: Facing) -> bool {
        self.in_bounds(p) && self.blocked[self.index(p)] & (1 << dir as u8) != 0
    }

    /// Can something walk from `from` to its neighbour in `dir`?
    pub fn can_step(&self, from: GridPos, dir: Facing) -> bool {
        self.is_walkable(from + dir) && !self.is_blocked(from, dir)
    }

    /// Can `a` see `b`? Every cell strictly between them must be open:
    /// walls hide what is behind them, not themselves.
    pub fn in_sight(&self, a: GridPos, b: GridPos) -> bool {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let steps = dx.abs().max(dy.abs());
        (1..steps).all(|i| {
            // Round to the nearest cell along the line, in integers.
            let p = GridPos::new(
                a.x + (2 * dx * i + steps * dx.signum()) / (2 * steps),
                a.y + (2 * dy * i + steps * dy.signum()) / (2 * steps),
            );
            self.is_walkable(p)
        })
    }

    /// Are `a` and `b` neighbours with an open edge between them (close
    /// enough to strike each other)?
    pub fn within_reach(&self, a: GridPos, b: GridPos) -> bool {
        Facing::ALL
            .into_iter()
            .any(|d| a + d == b && !self.is_blocked(a, d))
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
            for dir in Facing::ALL {
                let n = p + dir;
                if self.can_step(p, dir) && dist[self.index(n)].is_none() {
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
