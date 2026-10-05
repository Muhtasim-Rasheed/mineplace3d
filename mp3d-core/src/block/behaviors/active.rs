use glam::IVec3;

use crate::{
    block::BlockState, direction::Direction, entity::EntityId, protocol::BlockUpdateKind,
    world::World,
};

pub fn on_click(
    world: &mut World,
    _entity_id: EntityId,
    block_pos: IVec3,
    state: BlockState,
    _face: Direction,
) -> bool {
    world.urgent_set_block_at(
        block_pos,
        state.maybe_with("active", !state.get::<bool>("active").unwrap()),
        BlockUpdateKind::Interaction,
    );
    true
}
