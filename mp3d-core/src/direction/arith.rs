use glam::{IVec3, Vec3};

use crate::direction::Direction;

impl std::ops::Add<Direction> for IVec3 {
    type Output = Self;

    fn add(self, dir: Direction) -> Self::Output {
        self + IVec3::from(dir)
    }
}

impl std::ops::Sub<Direction> for IVec3 {
    type Output = Self;

    fn sub(self, dir: Direction) -> Self::Output {
        self - IVec3::from(dir)
    }
}

impl std::ops::Neg for Direction {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.opposite()
    }
}

impl std::ops::Mul<i32> for Direction {
    type Output = IVec3;

    fn mul(self, rhs: i32) -> Self::Output {
        IVec3::from(self) * rhs
    }
}

impl std::ops::Mul<f32> for Direction {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3::from(self) * rhs
    }
}
