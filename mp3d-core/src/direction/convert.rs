use glam::{IVec3, Vec3};

use crate::{axis::Axis, direction::Direction};

impl TryFrom<IVec3> for Direction {
    type Error = ();

    fn try_from(value: IVec3) -> Result<Self, Self::Error> {
        match value {
            IVec3 { x: 0, y: 0, z: -1 } => Ok(Direction::North),
            IVec3 { x: 0, y: 0, z: 1 } => Ok(Direction::South),
            IVec3 { x: 1, y: 0, z: 0 } => Ok(Direction::East),
            IVec3 { x: -1, y: 0, z: 0 } => Ok(Direction::West),
            IVec3 { x: 0, y: 1, z: 0 } => Ok(Direction::Up),
            IVec3 { x: 0, y: -1, z: 0 } => Ok(Direction::Down),
            _ => Err(()),
        }
    }
}

impl From<Direction> for IVec3 {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::North => IVec3::NEG_Z,
            Direction::South => IVec3::Z,
            Direction::East => IVec3::X,
            Direction::West => IVec3::NEG_X,
            Direction::Up => IVec3::Y,
            Direction::Down => IVec3::NEG_Y,
        }
    }
}

impl From<Vec3> for Direction {
    fn from(v: Vec3) -> Self {
        let a = v.abs();

        if a.x > a.y && a.x > a.z {
            if v.x > 0.0 {
                Direction::East
            } else {
                Direction::West
            }
        } else if a.y > a.z {
            if v.y > 0.0 {
                Direction::Up
            } else {
                Direction::Down
            }
        } else {
            if v.z > 0.0 {
                Direction::South
            } else {
                Direction::North
            }
        }
    }
}

impl From<Direction> for Vec3 {
    fn from(dir: Direction) -> Self {
        IVec3::from(dir).as_vec3()
    }
}

impl TryFrom<u8> for Direction {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or(())
    }
}

impl From<Direction> for u8 {
    fn from(dir: Direction) -> Self {
        dir as u8
    }
}

impl From<Direction> for Axis {
    fn from(dir: Direction) -> Self {
        dir.as_axis()
    }
}

impl std::str::FromStr for Direction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "north" | "-z" => Ok(Direction::North),
            "south" | "+z" => Ok(Direction::South),
            "east" | "+x" => Ok(Direction::East),
            "west" | "-x" => Ok(Direction::West),
            "up" | "+y" => Ok(Direction::Up),
            "down" | "-y" => Ok(Direction::Down),
            _ => Err(()),
        }
    }
}

impl std::fmt::Debug for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Direction::*;
        match self {
            North => write!(f, "-z"),
            South => write!(f, "+z"),
            East => write!(f, "+x"),
            West => write!(f, "-x"),
            Up => write!(f, "+y"),
            Down => write!(f, "-y"),
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Direction::*;
        match self {
            North => write!(f, "north"),
            South => write!(f, "south"),
            East => write!(f, "east"),
            West => write!(f, "west"),
            Up => write!(f, "up"),
            Down => write!(f, "down"),
        }
    }
}
