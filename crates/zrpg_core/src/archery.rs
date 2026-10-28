//! Archery: a bow and a quiver of arrows. An arrow is loosed down your line
//! of sight and really flies, cell by cell, so a creature can step into its
//! path or out of it. It strikes the first one it meets; one that misses
//! falls where it stops (into a wall, or spent at the end of its range) and
//! can usually be picked up again. Kills sometimes leave a few arrows.
//!
//! The flight itself is in `game.rs` ([`Game::tick`](crate::Game::tick)).

use crate::grid::{Facing, GridPos};

pub mod tuning {
    /// Arrows you start with, and the most the quiver holds.
    pub const START_ARROWS: u32 = 15;
    pub const MAX_ARROWS: u32 = 60;
    /// Cells per second, and how many cells before it falls.
    pub const SPEED: f32 = 14.0;
    pub const RANGE: u32 = 8;
    /// Damage of a hit (before the level bonus) and seconds between shots.
    /// Shooting costs no Chi: arrows are the bow's price.
    pub const DAMAGE_MIN: i32 = 3;
    pub const DAMAGE_MAX: i32 = 5;
    pub const COOLDOWN: f32 = 0.85;
    /// Percent chance an arrow that misses lies whole where it fell, and
    /// that one that hits drops out of its target to be picked up.
    pub const KEPT_MISS_PERCENT: u32 = 75;
    pub const KEPT_HIT_PERCENT: u32 = 40;
    /// Percent chance a kill leaves arrows, and how many (inclusive).
    pub const DROP_PERCENT: u32 = 30;
    pub const DROP_MIN: i32 = 1;
    pub const DROP_MAX: i32 = 3;
}

/// An arrow in the air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    /// Unique within the run, so the front-end can follow each one.
    pub id: u32,
    /// The cell it was loosed from, and the way it flies.
    pub from: GridPos,
    pub dir: Facing,
    /// Cells travelled, from the centre of `from`.
    pub dist: f32,
    /// The cell it is in now, and how many it has entered.
    pub cell: GridPos,
    pub(crate) entered: u32,
}

impl Flight {
    pub(crate) fn new(id: u32, from: GridPos, dir: Facing) -> Flight {
        Flight {
            id,
            from,
            dir,
            dist: 0.0,
            cell: from,
            entered: 0,
        }
    }

    /// How many cells it has crossed into by now: the first boundary is
    /// half a cell out.
    pub(crate) fn cells_reached(&self) -> u32 {
        (self.dist + 0.5) as u32
    }
}
