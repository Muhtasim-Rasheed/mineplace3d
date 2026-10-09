use glam::IVec3;

use crate::{
    block::{BlockState, behaviors::needs_support},
    direction::Direction,
    world::World,
};

pub fn on_update(world: &mut World, pos: IVec3, _: BlockState) -> bool {
    needs_support::on_update(world, pos, Direction::Down)
}
