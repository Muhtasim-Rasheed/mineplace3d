use glam::IVec3;

use crate::{
    block::{BlockId, BlockState, blockstate::SlabHalf},
    direction::Direction,
    entity::EntityId,
    world::World,
};

pub fn on_click(
    world: &mut World,
    entity_id: EntityId,
    block_pos: IVec3,
    state: BlockState,
    face: Direction,
) -> bool {
    let Some((item_count, place_block)) = world.hotbar_stack_info(entity_id) else {
        return false;
    };
    if state.get::<SlabHalf>("half").unwrap() == SlabHalf::Bottom && face == Direction::Up
        || state.get::<SlabHalf>("half").unwrap() == SlabHalf::Top && face == Direction::Down
    {
        if item_count == 0 {
            return false;
        }

        if let Some(block) = place_block
            && **block == state.block
        {
            world.try_place_block(
                entity_id,
                block_pos,
                BlockState::default_for(state.block).maybe_with("half", SlabHalf::Both),
            );
        }
        true
    } else {
        false
    }
}

pub fn on_place(id: BlockId, _: &mut World, _: EntityId, _: IVec3, face: Direction) -> BlockState {
    if face == Direction::Down {
        BlockState::default_for(id).maybe_with("half", SlabHalf::Top)
    } else {
        BlockState::default_for(id).maybe_with("half", SlabHalf::Bottom)
    }
}
