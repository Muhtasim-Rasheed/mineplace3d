use glam::IVec3;

use crate::{
    block::{BlockId, BlockState, behaviors::player_cardinal},
    direction::Direction,
    entity::EntityId,
    world::World,
};

pub fn on_place(
    id: BlockId,
    world: &mut World,
    entity_id: EntityId,
    _: IVec3,
    _: Direction,
) -> BlockState {
    BlockState::default_for(id).maybe_with("facing", player_cardinal(world, entity_id))
}
