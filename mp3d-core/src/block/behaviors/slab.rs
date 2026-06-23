use glam::IVec3;

use crate::{
    block::{BlockId, BlockState},
    direction::Direction,
    entity::PlayerEntity,
    item::item_registry,
    world::World,
};

pub fn on_click(
    id: BlockId,
    world: &mut World,
    entity_id: u64,
    block_pos: IVec3,
    state: BlockState,
    face: Direction,
) -> bool {
    let (item_count, place_block) = match world.get_entity::<PlayerEntity>(entity_id) {
        Some(p) => {
            let stack = p.inventory.hotbar_slot(p.hotbar_index);
            let assoc_block = item_registry().get(stack.item).unwrap().assoc_block;
            (stack.count, assoc_block)
        }
        None => return false,
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

pub fn on_place(_: BlockId, _: &mut World, _: u64, _: IVec3, face: Direction) -> BlockState {
    if face == Direction::Down {
        BlockState::slab(1)
    } else {
        BlockState::slab(0)
    }
}
