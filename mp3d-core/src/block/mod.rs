//! Blocks for a voxel engine.

use glam::Vec3;

/// A struct used for declaring different types of blocks on the fly. Mineplace provides some
/// already defined blocks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub full: bool,
    pub color: Vec3,
}

impl Block {
    pub const AIR: Block = Block {
        full: false,
        color: Vec3::ZERO,
    };

    pub const GRASS: Block = Block {
        full: true,
        color: Vec3::new(0.2, 0.9, 0.2),
    };

    pub const DIRT: Block = Block {
        full: true,
        color: Vec3::new(0.59, 0.29, 0.0),
    };

    pub const STONE: Block = Block {
        full: true,
        color: Vec3::new(0.5, 0.5, 0.55),
    };
}
