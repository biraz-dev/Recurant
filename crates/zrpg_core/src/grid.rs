//! Grid coordinates and the four cardinal directions.
//!
//! Convention: `x` grows east, `y` grows south. North is `y - 1`.

use core::ops::Add;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn manhattan(self, other: GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

impl Add<Facing> for GridPos {
    type Output = GridPos;
    fn add(self, dir: Facing) -> GridPos {
        let (dx, dy) = dir.delta();
        GridPos::new(self.x + dx, self.y + dy)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Facing {
    #[default]
    North,
    East,
    South,
    West,
}

impl Facing {
    pub const ALL: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

    pub fn delta(self) -> (i32, i32) {
        match self {
            Facing::North => (0, -1),
            Facing::East => (1, 0),
            Facing::South => (0, 1),
            Facing::West => (-1, 0),
        }
    }

    pub fn right(self) -> Facing {
        Facing::ALL[(self as usize + 1) % 4]
    }

    pub fn left(self) -> Facing {
        Facing::ALL[(self as usize + 3) % 4]
    }

    pub fn back(self) -> Facing {
        Facing::ALL[(self as usize + 2) % 4]
    }

    pub fn turned(self, turn: Turn) -> Facing {
        match turn {
            Turn::Left => self.left(),
            Turn::Right => self.right(),
        }
    }

    /// Quarter turns clockwise from north (N=0, E=1, S=2, W=3).
    pub fn quarter_turns(self) -> i32 {
        self as i32
    }
}

/// A one-cell step relative to the way the actor is facing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Forward,
    Back,
    StrafeLeft,
    StrafeRight,
}

impl Step {
    pub fn world_dir(self, facing: Facing) -> Facing {
        match self {
            Step::Forward => facing,
            Step::Back => facing.back(),
            Step::StrafeLeft => facing.left(),
            Step::StrafeRight => facing.right(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_round_trips() {
        for f in Facing::ALL {
            assert_eq!(f.left().right(), f);
            assert_eq!(f.right().right(), f.back());
            assert_eq!(f.left().left().left().left(), f);
        }
        assert_eq!(Facing::North.right(), Facing::East);
        assert_eq!(Facing::North.left(), Facing::West);
    }

    #[test]
    fn steps_are_relative() {
        assert_eq!(Step::StrafeRight.world_dir(Facing::North), Facing::East);
        assert_eq!(Step::Back.world_dir(Facing::East), Facing::West);
        assert_eq!(GridPos::new(3, 3) + Facing::North, GridPos::new(3, 2));
    }
}
