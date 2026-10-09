use glam::IVec3;

use crate::{
    block::{BlockId, BlockState, LimitedInt, behaviors::needs_support},
    direction::Direction,
    entity::EntityId,
    protocol::BlockUpdateKind,
    world::World,
};

pub fn on_click(
    world: &mut World,
    _: EntityId,
    block_pos: IVec3,
    block_state: BlockState,
    _: Direction,
) -> bool {
    world.urgent_set_block_at(
        block_pos,
        block_state.maybe_with("active_timer", LimitedInt::<24>(24)),
        BlockUpdateKind::Interaction,
    );
    true
}

pub fn on_place(id: BlockId, _: &mut World, _: EntityId, _: IVec3, face: Direction) -> BlockState {
    BlockState::default_for(id).maybe_with("facing", face.opposite())
}

pub fn on_update(world: &mut World, pos: IVec3, block_state: BlockState) -> bool {
    let face = block_state.get::<Direction>("facing").unwrap();
    let _ = needs_support::on_update(world, pos, face) && return true; // if block was removed, exit

    let timer = block_state.get::<LimitedInt<24>>("active_timer").unwrap();
    if timer.0 == 0 {
        return false;
    }
    world.urgent_set_block_at(
        pos,
        block_state.maybe_with("active_timer", LimitedInt::<24>(timer.0 - 1)),
        BlockUpdateKind::Interaction,
    );
    if timer.0 > 1 {
        world.schedule_update(pos, 1);
    }
    false // set_block already notifies neighbors
}
