use glam::IVec3;

use crate::{
    block::{BlockId, BlockState},
    direction::Direction,
    world::World,
};

pub fn on_click(
    first: impl Fn(BlockId, &mut World, u64, IVec3, BlockState, Direction) -> bool
    + Send
    + Sync
    + 'static,
    second: impl Fn(BlockId, &mut World, u64, IVec3, BlockState, Direction) -> bool
    + Send
    + Sync
    + 'static,
) -> Box<dyn Fn(BlockId, &mut World, u64, IVec3, BlockState, Direction) -> bool + Send + Sync> {
    Box::new(move |id, world, entity, pos, state, face| {
        if first(id, world, entity, pos, state, face) {
            return true;
        }
        second(id, world, entity, pos, state, face)
    })
}
