use glam::Vec3;

use crate::entity::{
    EntityDetails,
    components::{Hitbox, OnGround, Rotation, Velocity},
};

pub const PICKUP_RADIUS: f32 = 1.5;

pub fn item_template() -> EntityDetails {
    EntityDetails::builder()
        .with(Velocity(Vec3::ZERO))
        .with(Rotation {
            yaw: rand::random_range(0.0..=360.0),
            pitch: 0.0,
        })
        .with(OnGround(false))
        .with(Hitbox {
            width: 0.5,
            height: 0.1,
        })
        .build()
}
