//! A module for working with directions in a 3D space.
//!!
//! This module defines the [`Direction`] enum, which represents the six cardinal directions in a 3D
//! space: north, south, east, west, up, and down. It also provides methods for getting the opposite
//! direction, converting between directions and vectors, and performing arithmetic operations with
//! directions.

use crate::axis::Axis;

mod arith;
mod convert;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
    Up = 4,
    Down = 5,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
        Direction::Up,
        Direction::Down,
    ];

    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Direction::North),
            1 => Some(Direction::South),
            2 => Some(Direction::East),
            3 => Some(Direction::West),
            4 => Some(Direction::Up),
            5 => Some(Direction::Down),
            _ => None,
        }
    }

    pub const fn as_axis(self) -> Axis {
        match self {
            Direction::North => Axis::Z,
            Direction::South => Axis::Z,
            Direction::East => Axis::X,
            Direction::West => Axis::X,
            Direction::Up => Axis::Y,
            Direction::Down => Axis::Y,
        }
    }

    pub const fn opposite(self) -> Self {
        // No branches in the final compiled binary because of optimization

        match (self as u8) ^ 1 {
            0 => Direction::North,
            1 => Direction::South,
            2 => Direction::East,
            3 => Direction::West,
            4 => Direction::Up,
            5 => Direction::Down,
            _ => unreachable!(),
        }
    }
}
