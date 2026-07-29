use glam::IVec3;

use crate::{
    block::{BlockId, BlockState},
    direction::Direction,
    entity::EntityId,
    world::World,
};

pub fn on_click(
    id: BlockId,
    world: &mut World,
    entity_id: EntityId,
    block_pos: IVec3,
    state: BlockState,
    face: Direction,
) -> bool {
    let Some((item_count, place_block)) = world.hotbar_stack_info(entity_id) else {
        return false;
    };
    if state == BlockState::slab(0) && face == Direction::Up
        || state == BlockState::slab(1) && face == Direction::Down
    {
        if item_count == 0 {
            return false;
        }

        if let Some(block) = place_block
            && **block == id
        {
            world.try_place_block(entity_id, block_pos, **block, BlockState::slab(2));
        }
        true
    } else {
        false
    }
}

pub fn on_place(_: BlockId, _: &mut World, _: EntityId, _: IVec3, face: Direction) -> BlockState {
    if face == Direction::Down {
        BlockState::slab(1)
    } else {
        BlockState::slab(0)
    }
}
