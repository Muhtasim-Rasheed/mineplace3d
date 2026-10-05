use glam::IVec3;

use crate::{block::BlockState, direction::Direction, entity::EntityId, world::World};

pub fn on_click(
    first: impl Fn(&mut World, EntityId, IVec3, BlockState, Direction) -> bool + Send + Sync + 'static,
    second: impl Fn(&mut World, EntityId, IVec3, BlockState, Direction) -> bool + Send + Sync + 'static,
) -> Box<dyn Fn(&mut World, EntityId, IVec3, BlockState, Direction) -> bool + Send + Sync> {
    Box::new(move |world, entity, pos, state, face| {
        if first(world, entity, pos, state, face) {
            return true;
        }
        second(world, entity, pos, state, face)
    })
}
