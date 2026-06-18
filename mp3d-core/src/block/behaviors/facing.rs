use glam::IVec3;

use crate::{
    block::{BlockId, BlockState, behaviors::player_cardinal},
    direction::Direction,
    world::World,
};

pub fn on_place(
    _: BlockId,
    world: &mut World,
    entity_id: u64,
    _: IVec3,
    _: Direction,
) -> BlockState {
    BlockState::facing(player_cardinal(world, entity_id))
}
