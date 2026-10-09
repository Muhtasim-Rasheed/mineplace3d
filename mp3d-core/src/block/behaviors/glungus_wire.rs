use glam::IVec3;

use crate::{
    block::{BlockId, BlockState},
    direction::Direction,
    entity::EntityId,
    protocol::BlockUpdateKind,
    world::World,
};

pub fn on_place(id: BlockId, _: &mut World, _: EntityId, _: IVec3, face: Direction) -> BlockState {
    BlockState::default_for(id).maybe_with("facing", face)
}

pub fn on_update(world: &mut World, pos: IVec3, block_state: BlockState) -> bool {
    let is_active = block_state.get::<bool>("active").unwrap();
    let dir = block_state.get::<Direction>("facing").unwrap();
    if let Some(neighbor_state) = world.get_block_at(pos + dir) {
        world.urgent_set_block_at(
            pos + dir,
            neighbor_state.maybe_with("active", is_active),
            BlockUpdateKind::Tick,
        );
    }
    false // set_block already notifies neighbors
}
