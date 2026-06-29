//! Blocks for a voxel engine.

use behaviors::*;
pub use blockstate::BlockState;
pub use registration::*;

use crate::define_blocks;

pub mod behaviors;
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
    STONE_SLAB => {
        ident: "stone_slab",
        collision_shape: CollisionShape::Slab,
        state_type: BlockState::SLAB_TYPE,
        on_click: Box::new(slab::on_click),
        on_place: Box::new(slab::on_place),
    },
    STONE_STAIRS => {
        ident: "stone_stairs",
        collision_shape: CollisionShape::Stairs,
        state_type: BlockState::STAIR_TYPE,
        on_place: Box::new(stairs::on_place),
    },
    STONE_VSLAB => {
        ident: "stone_vslab",
        collision_shape: CollisionShape::VSlab,
        state_type: BlockState::FACING_TYPE,
        on_place: Box::new(facing::on_place),
    },
    COBBLESTONE => { ident: "cobblestone" },
    GRANITE => { ident: "granite" },
    LOG => { ident: "log" },
    LEAVES => { ident: "leaves" },
    GLUNGUS => { ident: "glungus", on_click: Box::new(explode::on_click) },
    GLUNGUS_SLAB => {
        ident: "glungus_slab",
        collision_shape: CollisionShape::Slab,
        state_type: BlockState::SLAB_TYPE,
        on_click: and_then::on_click(
            slab::on_click,
            explode::on_click,
        ),
        on_place: Box::new(slab::on_place),
    },
    GLUNGUS_STAIRS => {
        ident: "glungus_stairs",
        collision_shape: CollisionShape::Stairs,
        state_type: BlockState::STAIR_TYPE,
        on_click: Box::new(explode::on_click),
        on_place: Box::new(stairs::on_place),
    },
    GLUNGUS_VSLAB => {
        ident: "glungus_vslab",
        collision_shape: CollisionShape::VSlab,
        state_type: BlockState::FACING_TYPE,
        on_click: Box::new(explode::on_click),
        on_place: Box::new(facing::on_place),
    },
    SHORT_GRASS => {
        ident: "short_grass",
        collision_shape: CollisionShape::None,
        interact_shape: CollisionShape::FullBlock,
    },
    GLASS => { ident: "glass" },
    BRICKS => { ident: "bricks" },
    BRICK_SLAB => {
        ident: "brick_slab",
        collision_shape: CollisionShape::Slab,
        state_type: BlockState::SLAB_TYPE,
        on_click: Box::new(slab::on_click),
        on_place: Box::new(slab::on_place),
    },
    BRICK_STAIRS => {
        ident: "brick_stairs",
        collision_shape: CollisionShape::Stairs,
        state_type: BlockState::STAIR_TYPE,
        on_place: Box::new(stairs::on_place),
    },
    BRICK_VSLAB => {
        ident: "brick_vslab",
        collision_shape: CollisionShape::VSlab,
        state_type: BlockState::FACING_TYPE,
        on_place: Box::new(facing::on_place),
    },
    GOLD => { ident: "gold" },
    DIAMOND => { ident: "diamond" },
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
