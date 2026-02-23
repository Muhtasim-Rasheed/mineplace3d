//! Blocks for a voxel engine.

/// A struct used for declaring different types of blocks on the fly. Mineplace provides some
/// already defined blocks and an array of the already defined blocks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub visible: bool,
    pub ident: &'static str,
}

impl Block {
    pub const AIR: Block = Block {
        visible: false,
        ident: "air",
    };

    pub const GRASS: Block = Block {
        visible: true,
        ident: "grass",
    };

    pub const DIRT: Block = Block {
        visible: true,
        ident: "dirt",
    };

    pub const STONE: Block = Block {
        visible: true,
        ident: "stone",
    };

    pub const ALL_BLOCKS: [Block; 4] = [Block::GRASS, Block::DIRT, Block::STONE, Block::AIR];
}
