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

pub mod dungeon;
pub mod game;
pub mod grid;
pub mod map;
pub mod rng;

pub use dungeon::{Floor, GenParams};
pub use game::{Command, Event, Game};
pub use grid::{Facing, GridPos, Step, Turn};
pub use map::{Map, Tile};
pub use rng::Rng;
