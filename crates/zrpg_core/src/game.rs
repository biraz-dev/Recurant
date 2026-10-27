//! Game state and the command/event loop the front-end drives.
//!
//! Commands resolve instantly here; the front-end animates the resulting
//! events at whatever pace it likes.

use crate::dungeon::{Floor, GenParams};
use crate::grid::{Facing, GridPos, Step, Turn};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Step(Step),
    Turn(Turn),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Moved {
        from: GridPos,
        to: GridPos,
    },
    /// Tried to walk into something solid.
    Bumped {
        at: GridPos,
        dir: Facing,
    },
    Turned {
        from: Facing,
        to: Facing,
        turn: Turn,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Player {
    pub pos: GridPos,
    pub facing: Facing,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub run_seed: u64,
    /// 1-based floor number.
    pub depth: u32,
    pub params: GenParams,
    pub floor: Floor,
    pub player: Player,
}

impl Game {
    pub fn new(run_seed: u64) -> Self {
        Self::with_params(run_seed, GenParams::default())
    }

    pub fn with_params(run_seed: u64, params: GenParams) -> Self {
        let floor = Floor::for_depth(run_seed, 1, &params);
        let player = Player {
            pos: floor.start,
            facing: floor.start_facing,
        };
        Self {
            run_seed,
            depth: 1,
            params,
            floor,
            player,
        }
    }

    pub fn apply(&mut self, cmd: Command) -> Event {
        match cmd {
            Command::Step(step) => {
                let dir = step.world_dir(self.player.facing);
                let from = self.player.pos;
                let to = from + dir;
                if self.floor.map.is_walkable(to) {
                    self.player.pos = to;
                    Event::Moved { from, to }
                } else {
                    Event::Bumped { at: from, dir }
                }
            }
            Command::Turn(turn) => {
                let from = self.player.facing;
                self.player.facing = from.turned(turn);
                Event::Turned {
                    from,
                    to: self.player.facing,
                    turn,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walls_block_movement() {
        let mut g = Game::new(5);
        // Walk forward until blocked; position must stay walkable throughout.
        for _ in 0..20 {
            let before = g.player.pos;
            match g.apply(Command::Step(Step::Forward)) {
                Event::Moved { from, to } => {
                    assert_eq!(from, before);
                    assert_eq!(to, g.player.pos);
                }
                Event::Bumped { at, dir } => {
                    assert_eq!(at, before);
                    assert_eq!(g.player.pos, before);
                    assert!(!g.floor.map.is_walkable(at + dir));
                }
                Event::Turned { .. } => unreachable!(),
            }
            assert!(g.floor.map.is_walkable(g.player.pos));
        }
    }

    #[test]
    fn random_walk_never_leaves_floor() {
        let mut rng = crate::rng::Rng::new(1);
        for seed in 0..50 {
            let mut g = Game::new(seed);
            for _ in 0..500 {
                let cmd = match rng.range(0, 5) {
                    0 => Command::Step(Step::Forward),
                    1 => Command::Step(Step::Back),
                    2 => Command::Step(Step::StrafeLeft),
                    3 => Command::Step(Step::StrafeRight),
                    4 => Command::Turn(Turn::Left),
                    _ => Command::Turn(Turn::Right),
                };
                g.apply(cmd);
                assert!(g.floor.map.is_walkable(g.player.pos));
            }
        }
    }

    #[test]
    fn turning_changes_only_facing() {
        let mut g = Game::new(3);
        let pos = g.player.pos;
        let f = g.player.facing;
        g.apply(Command::Turn(Turn::Right));
        assert_eq!(g.player.facing, f.right());
        assert_eq!(g.player.pos, pos);
    }
}
