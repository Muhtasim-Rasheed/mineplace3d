use glam::Vec3;

use crate::entity::{
    EntityDetails,
    components::{Hitbox, OnGround, Rotation, Velocity},
};

pub fn cat_template() -> EntityDetails {
    EntityDetails::builder()
        .with(Velocity(Vec3::ZERO))
        .with(Rotation {
            yaw: 0.0,
            pitch: 0.0,
        })
        .with(OnGround(false))
        .with(Hitbox {
            width: 0.6,
            height: 0.7,
        })
        .build()
}
