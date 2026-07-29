use glam::Vec3;

use crate::{
    direction::Direction,
    entity::{EntityId, components::Rotation},
    world::World,
};

pub mod and_then;
pub mod explode;
pub mod facing;
pub mod slab;
pub mod stairs;

fn player_cardinal(world: &World, id: EntityId) -> Direction {
    let Some(yaw) = world
        .ecs
        .get_component_copied::<Rotation>(id)
        .map(|r| r.yaw)
    else {
        panic!("Called player_cardinal on an entity ({id}) without the Rotation component.")
    };
    let yaw_rad = yaw.to_radians();
    let player_fwd = Vec3::new(yaw_rad.sin(), 0.0, yaw_rad.cos());
    if player_fwd.x.abs() > player_fwd.z.abs() {
        if player_fwd.x > 0.0 {
            Direction::East
        } else {
            Direction::West
        }
    } else {
        if player_fwd.z > 0.0 {
            Direction::South
        } else {
            Direction::North
        }
    }
}
