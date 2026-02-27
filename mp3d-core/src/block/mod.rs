//! Blocks for a voxel engine.

use glam::Vec3;

/// A struct used for declaring different types of blocks on the fly. Mineplace provides some
/// already defined blocks and an array of the already defined blocks.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub visible: bool,
    pub collision_shape: CollisionShape,
    pub ident: &'static str,
}

impl Block {
    pub const AIR: Block = Block {
        visible: false,
        collision_shape: CollisionShape::None,
        ident: "air",
    };

    pub const GRASS: Block = Block {
        visible: true,
        collision_shape: CollisionShape::FullBlock,
        ident: "grass",
    };

    pub const DIRT: Block = Block {
        visible: true,
        collision_shape: CollisionShape::FullBlock,
        ident: "dirt",
    };

    pub const STONE: Block = Block {
        visible: true,
        collision_shape: CollisionShape::FullBlock,
        ident: "stone",
    };

    pub const ALL_BLOCKS: [Block; 4] = [Block::AIR, Block::GRASS, Block::DIRT, Block::STONE];

    pub fn collides_with_player(
        &self,
        player_width: f32,
        player_height: f32,
        player_pos_local: Vec3,
        _block_state: BlockState,
    ) -> bool {
        match self.collision_shape {
            CollisionShape::None => false,
            CollisionShape::FullBlock => {
                let half_width = player_width / 2.0;
                let player_min = Vec3::new(
                    player_pos_local.x - half_width,
                    player_pos_local.y,
                    player_pos_local.z - half_width,
                );
                let player_max = Vec3::new(
                    player_pos_local.x + half_width,
                    player_pos_local.y + player_height,
                    player_pos_local.z + half_width,
                );
                let block_min = Vec3::new(0.0, 0.0, 0.0);
                let block_max = Vec3::new(1.0, 1.0, 1.0);
                crate::aabb_overlap(player_min, player_max, block_min, block_max)
            }
        }
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.ident == other.ident
    }
}

/// Collision shape used for collision detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CollisionShape {
    /// No collision.
    None = 0,
    /// A full cube.
    FullBlock = 1,
}

/// Struct to store the block state of a block in the world.
///
/// For example, slabs store whether they are the top or bottom half of a block, stairs store their
/// facing direction and whether they are upside down, etc. This data is not stored in the block
/// struct itself because it is not shared between all blocks of the same type, but rather is
/// stored in the chunk data.
#[derive(Clone, Copy, Debug)]
pub enum BlockState {
    /// No additional state.
    None,
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
