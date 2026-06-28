use glam::{IVec3, Vec3};

use crate::axis::Axis;

impl TryFrom<IVec3> for Axis {
    type Error = ();

    fn try_from(value: IVec3) -> Result<Self, Self::Error> {
        match value {
            IVec3 { x: 0, y: 0, z: -1 } => Ok(Self::Z),
            IVec3 { x: 0, y: 0, z: 1 } => Ok(Self::Z),
            IVec3 { x: 1, y: 0, z: 0 } => Ok(Self::X),
            IVec3 { x: -1, y: 0, z: 0 } => Ok(Self::X),
            IVec3 { x: 0, y: 1, z: 0 } => Ok(Self::Y),
            IVec3 { x: 0, y: -1, z: 0 } => Ok(Self::Y),
            _ => Err(()),
        }
    }
}

impl From<Axis> for IVec3 {
    fn from(axis: Axis) -> Self {
        match axis {
            Axis::X => IVec3::X,
            Axis::Y => IVec3::Y,
            Axis::Z => IVec3::Z,
        }
    }
}

impl From<Vec3> for Axis {
    fn from(v: Vec3) -> Self {
        let a = v.abs();

        if a.x > a.y && a.x > a.z {
            Axis::X
        } else if a.y > a.z {
            Axis::Y
        } else {
            Axis::Z
        }
    }
}

impl From<Axis> for Vec3 {
    fn from(axis: Axis) -> Self {
        IVec3::from(axis).as_vec3()
    }
}
