use glam::{IVec3, Vec3};

use crate::direction::Direction;

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
            Direction::North => IVec3::new(0, 0, -1),
            Direction::South => IVec3::new(0, 0, 1),
            Direction::East => IVec3::new(1, 0, 0),
            Direction::West => IVec3::new(-1, 0, 0),
            Direction::Up => IVec3::new(0, 1, 0),
            Direction::Down => IVec3::new(0, -1, 0),
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

impl std::str::FromStr for Direction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "north" => Ok(Direction::North),
            "south" => Ok(Direction::South),
            "east" => Ok(Direction::East),
            "west" => Ok(Direction::West),
            "up" => Ok(Direction::Up),
            "down" => Ok(Direction::Down),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Direction::North => "North",
            Direction::South => "South",
            Direction::East => "East",
            Direction::West => "West",
            Direction::Up => "Up",
            Direction::Down => "Down",
        };
        write!(f, "{}", s)
    }
}
