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

static BLOCK_IDENTS: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
    std::sync::OnceLock::new();

fn get_block_idents() -> &'static std::collections::HashSet<&'static str> {
    BLOCK_IDENTS.get_or_init(|| {
        let mut set = std::collections::HashSet::new();
        for block in Block::ALL_BLOCKS {
            set.insert(block.ident);
        }
        set
    })
}

/// Nice little helper for the crate to convert from a `&str` to a `&'static str`, which is needed
/// for block identifiers as `Block` needs to be `Copy` and thus cannot contain owned `String`s.
/// This function will
pub(crate) fn get_block_ident(ident: &str) -> Option<&'static str> {
    let idents = get_block_idents();
    if let Some(&ident) = idents.get(ident) {
        Some(ident)
    } else {
        None
    }
}
