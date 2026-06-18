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
    GLUNGUS => { ident: "glungus" },
    STONE_SLAB => {
        ident: "stone_slab",
        collision_shape: CollisionShape::Slab,
        state_type: BlockState::SLAB_TYPE,
    },
    STONE_STAIRS => {
        ident: "stone_stairs",
        collision_shape: CollisionShape::Stairs,
        state_type: BlockState::STAIR_TYPE,
    },
    STONE_VSLAB => {
        ident: "stone_vslab",
        collision_shape: CollisionShape::VSlab,
        state_type: BlockState::FACING_TYPE,
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
