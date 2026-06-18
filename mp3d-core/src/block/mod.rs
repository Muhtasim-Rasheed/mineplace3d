//! Blocks for a voxel engine.

pub use blockstate::BlockState;
pub use registration::*;

use crate::define_blocks;

mod blockstate;
mod registration;
mod save_impls;

// Definitions of all blocks
define_blocks! {
    AIR => {
        ident: "air",
        visible: false,
        collision_shape: CollisionShape::None,
        interact_shape: CollisionShape::None,
    },
    GRASS => { ident: "grass" },
    DIRT => { ident: "dirt" },
    STONE => { ident: "stone" },
    COBBLESTONE => { ident: "cobblestone" },
    GRANITE => { ident: "granite" },
    LOG => { ident: "log" },
    LEAVES => { ident: "leaves" },
    GLUNGUS => { ident: "glungus", on_click: on_click::glungus },
    STONE_SLAB => {
        ident: "stone_slab",
        collision_shape: CollisionShape::Slab,
        state_type: BlockState::SLAB_TYPE,
        on_click: on_click::slab,
        on_place: on_place::slab,
    },
    STONE_STAIRS => {
        ident: "stone_stairs",
        collision_shape: CollisionShape::Stairs,
        state_type: BlockState::STAIR_TYPE,
        on_place: on_place::stairs,
    },
    STONE_VSLAB => {
        ident: "stone_vslab",
        collision_shape: CollisionShape::VSlab,
        state_type: BlockState::FACING_TYPE,
        on_place: on_place::facing,
    },
    SHORT_GRASS => {
        ident: "short_grass",
        collision_shape: CollisionShape::None,
        interact_shape: CollisionShape::FullBlock,
    },
}

/// Collision shape used for collision detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CollisionShape {
    /// No collision.
    None = 0,
    /// A full cube.
    FullBlock = 1,
    /// A slab (whether it's top or bottom is determined by the block state).
    Slab = 2,
    /// A stair (the facing direction is determined by the block state).
    Stairs = 3,
    /// A vertical slab (the facing direction is determined by the block state).
    VSlab = 4,
}

mod on_click {
    use glam::IVec3;

    use super::*;
    use crate::{
        direction::Direction, entity::PlayerEntity, item::item_registry, protocol::BlockUpdateKind,
        world::World,
    };

    pub(super) fn glungus(
        _: BlockId,
        world: &mut World,
        _: u64,
        block_pos: IVec3,
        _: BlockState,
        _: Direction,
    ) -> bool {
        let radius_sq = 8 * 8;
        for x in -8..=8 {
            for y in -8..=8 {
                for z in -8..=8 {
                    if x * x + y * y + z * z <= radius_sq {
                        let pos = block_pos + IVec3::new(x, y, z);
                        world.urgent_set_block_at(
                            pos,
                            *blocks::AIR,
                            BlockState::none(),
                            BlockUpdateKind::Interaction,
                        );
                    }
                }
            }
        }
        true
    }

    pub(super) fn slab(
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
}

mod on_place {
    use glam::IVec3;

    use super::*;
    use crate::{
        direction::Direction,
        entity::{Entity, PlayerEntity},
        world::World,
    };

    pub(super) fn slab(_: BlockId, _: &mut World, _: u64, _: IVec3, face: Direction) -> BlockState {
        if face == Direction::Down {
            BlockState::slab(1)
        } else if face == Direction::Up {
            BlockState::slab(0)
        } else {
            unreachable!()
        }
    }

    pub(super) fn stairs(
        _: BlockId,
        world: &mut World,
        entity_id: u64,
        _: IVec3,
        _: Direction,
    ) -> BlockState {
        let player_fwd = world
            .get_entity::<PlayerEntity>(entity_id)
            .unwrap()
            .forward()
            .with_y(0.0)
            .normalize_or_zero();
        let player_dir = if player_fwd.x.abs() > player_fwd.z.abs() {
            if player_fwd.x > 0.0 {
                Direction::East
            } else {
                Direction::West
            }
        } else {
            if player_fwd.z > 0.0 {
                Direction::South
            } else {
                Direction::North
            }
        };
        BlockState::stairs(player_dir)
    }

    pub(super) fn facing(
        _: BlockId,
        world: &mut World,
        entity_id: u64,
        _: IVec3,
        _: Direction,
    ) -> BlockState {
        let player_fwd = world
            .get_entity::<PlayerEntity>(entity_id)
            .unwrap()
            .forward()
            .with_y(0.0)
            .normalize_or_zero();
        let player_dir = if player_fwd.x.abs() > player_fwd.z.abs() {
            if player_fwd.x > 0.0 {
                Direction::East
            } else {
                Direction::West
            }
        } else {
            if player_fwd.z > 0.0 {
                Direction::South
            } else {
                Direction::North
            }
        };
        BlockState::facing(player_dir)
    }
}
