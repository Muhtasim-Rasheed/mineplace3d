//! Blocks for a voxel engine.

use behaviors::*;
pub use blockstate::BlockState;
pub use registration::*;

use crate::{
    block::blockstate::{HorizontalDir, SlabHalf},
    define_blocks,
    direction::Direction,
};

pub mod behaviors;
mod blockstate;
mod registration;
mod save_impls;

fn from_legacy_slab(mut state: BlockState, data: u16) -> BlockState {
    match data {
        0x0001 => state.set("half", SlabHalf::Top),
        0x0002 => state.set("half", SlabHalf::Both),
        _ => state.set("half", SlabHalf::Bottom),
    };
    state
}

fn from_legacy_facing(mut state: BlockState, data: u16) -> BlockState {
    match data {
        0x0001 => state.set("facing", HorizontalDir(Direction::South)),
        0x0002 => state.set("facing", HorizontalDir(Direction::East)),
        0x0003 => state.set("facing", HorizontalDir(Direction::West)),
        _ => state.set("facing", HorizontalDir(Direction::North)),
    };
    state
}

fn from_legacy_active(mut state: BlockState, data: u16) -> BlockState {
    match data {
        0x0001 => state.set("active", true),
        _ => state.set("active", false),
    };
    state
}

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
        state_properties: vec![PropertyDef::new("half", SlabHalf::Bottom)],
        from_legacy_state: from_legacy_slab,
        on_click: Box::new(slab::on_click),
        on_place: Box::new(slab::on_place),
    },
    STONE_STAIRS => {
        ident: "stone_stairs",
        collision_shape: CollisionShape::Stairs,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
        on_place: Box::new(stairs::on_place),
    },
    STONE_VSLAB => {
        ident: "stone_vslab",
        collision_shape: CollisionShape::VSlab,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
        on_place: Box::new(facing::on_place),
    },
    COBBLESTONE => { ident: "cobblestone" },
    GRANITE => { ident: "granite" },
    LOG => { ident: "log" },
    LEAVES => { ident: "leaves" },
    PLANKS => { ident: "planks" },
    PLANKS_SLAB => {
        ident: "planks_slab",
        collision_shape: CollisionShape::Slab,
        state_properties: vec![PropertyDef::new("half", SlabHalf::Bottom)],
        from_legacy_state: from_legacy_slab,
        on_click: Box::new(slab::on_click),
        on_place: Box::new(slab::on_place),
    },
    PLANKS_STAIRS => {
        ident: "planks_stairs",
        collision_shape: CollisionShape::Stairs,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
        on_place: Box::new(stairs::on_place),
    },
    PLANKS_VSLAB => {
        ident: "planks_vslab",
        collision_shape: CollisionShape::VSlab,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
        on_place: Box::new(facing::on_place),
    },
    GLUNGUS => { ident: "glungus", on_click: Box::new(explode::on_click) },
    GLUNGUS_SLAB => {
        ident: "glungus_slab",
        collision_shape: CollisionShape::Slab,
        state_properties: vec![PropertyDef::new("half", SlabHalf::Bottom)],
        from_legacy_state: from_legacy_slab,
        on_click: and_then::on_click(
            slab::on_click,
            explode::on_click,
        ),
        on_place: Box::new(slab::on_place),
    },
    GLUNGUS_STAIRS => {
        ident: "glungus_stairs",
        collision_shape: CollisionShape::Stairs,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
        on_click: Box::new(explode::on_click),
        on_place: Box::new(stairs::on_place),
    },
    GLUNGUS_VSLAB => {
        ident: "glungus_vslab",
        collision_shape: CollisionShape::VSlab,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
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
        state_properties: vec![PropertyDef::new("half", SlabHalf::Bottom)],
        from_legacy_state: from_legacy_slab,
        on_click: Box::new(slab::on_click),
        on_place: Box::new(slab::on_place),
    },
    BRICK_STAIRS => {
        ident: "brick_stairs",
        collision_shape: CollisionShape::Stairs,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
        on_place: Box::new(stairs::on_place),
    },
    BRICK_VSLAB => {
        ident: "brick_vslab",
        collision_shape: CollisionShape::VSlab,
        state_properties: vec![PropertyDef::new("facing", HorizontalDir(Direction::North))],
        from_legacy_state: from_legacy_facing,
        on_place: Box::new(facing::on_place),
    },
    GOLD => { ident: "gold" },
    DIAMOND => { ident: "diamond" },
    MACHINE_RUNNER => {
        ident: "machine_runner",
        state_properties: vec![PropertyDef::new("active", false)],
        from_legacy_state: from_legacy_active,
        on_click: Box::new(active::on_click),
    },
    MOVER => { ident: "mover" },
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
