//! Portable ZRPG game logic.
//!
//! This crate knows nothing about rendering, input devices or Bevy. It is
//! `no_std` (with `alloc`) and has no dependencies so that the same rules can
//! run on desktop, Android, the web and, one day, a Nintendo 64.
//!
//! The front-end feeds [`Command`]s into a [`Game`] and reacts to the
//! [`Event`]s that come back.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod auto;
pub mod dungeon;
pub mod enemy;
pub mod game;
pub mod grid;
pub mod items;
pub mod map;
pub mod progression;
pub mod rng;
pub mod ruins;
pub mod save;
pub mod spells;
pub mod stage;

pub use dungeon::{Floor, GenParams, Place};
pub use enemy::{Enemy, EnemyId, EnemyKind, EnemyState};
pub use game::{Command, Drop, Event, Game, Player};
pub use grid::{Facing, GridPos, Step, Turn};
pub use items::{Bag, Item};
pub use map::{Map, Tile};
pub use progression::{MAX_LEVEL, Perk, Perks};
pub use rng::Rng;
pub use spells::Spell;
pub use stage::{Stage, Theme};
