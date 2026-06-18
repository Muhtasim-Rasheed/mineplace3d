use crate::{
    direction::Direction,
    entity::{Entity, PlayerEntity},
    world::World,
};

pub mod explode;
pub mod facing;
pub mod slab;
pub mod stairs;

fn player_cardinal(world: &World, id: u64) -> Direction {
    let player_fwd = world
        .get_entity::<PlayerEntity>(id)
        .unwrap()
        .forward()
        .with_y(0.0)
        .normalize_or_zero();
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
