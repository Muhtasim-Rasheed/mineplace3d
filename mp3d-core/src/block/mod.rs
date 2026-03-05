//! Blocks for a voxel engine.

use glam::Vec3;

mod save_impls;

/// A struct used for declaring different types of blocks on the fly. Mineplace provides some
/// already defined blocks and an array of the already defined blocks.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub visible: bool,
    pub collision_shape: CollisionShape,
    pub ident: &'static str,
    pub state_type: u16,
}

impl Block {
    pub const AIR: Block = Block {
        visible: false,
        collision_shape: CollisionShape::None,
        ident: "air",
        state_type: BlockState::NONE.state_type(),
    };

    pub const GRASS: Block = Block {
        visible: true,
        collision_shape: CollisionShape::FullBlock,
        ident: "grass",
        state_type: BlockState::NONE.state_type(),
    };

    pub const DIRT: Block = Block {
        visible: true,
        collision_shape: CollisionShape::FullBlock,
        ident: "dirt",
        state_type: BlockState::NONE.state_type(),
    };

    pub const STONE: Block = Block {
        visible: true,
        collision_shape: CollisionShape::FullBlock,
        ident: "stone",
        state_type: BlockState::NONE.state_type(),
    };

    pub const GLUNGUS: Block = Block {
        visible: true,
        collision_shape: CollisionShape::FullBlock,
        ident: "glungus",
        state_type: BlockState::NONE.state_type(),
    };

    pub const STONE_SLAB: Block = Block {
        visible: true,
        collision_shape: CollisionShape::Slab,
        ident: "stone_slab",
        state_type: BlockState::SLAB_BOTTOM.state_type(),
    };

    pub const ALL_BLOCKS: &[Block] = &[
        Block::AIR,
        Block::GRASS,
        Block::DIRT,
        Block::STONE,
        Block::GLUNGUS,
        Block::STONE_SLAB,
    ];

    pub fn collides_with_player(
        &self,
        player_width: f32,
        player_height: f32,
        player_pos_local: Vec3,
        block_state: BlockState,
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
            CollisionShape::Slab => {
                if let Some(is_top) = block_state.is_slab() {
                    let block_min = Vec3::new(0.0, if is_top { 0.5 } else { 0.0 }, 0.0);
                    let block_max = Vec3::new(1.0, if is_top { 1.0 } else { 0.5 }, 1.0);
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
                    crate::aabb_overlap(player_min, player_max, block_min, block_max)
                } else {
                    false
                }
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
    /// A slab (whether it's top or bottom is determined by the block state).
    Slab = 2,
}

/// Struct to store the block state of a block in the world.
///
/// For example, slabs store whether they are the top or bottom half of a block, stairs store their
/// facing direction and whether they are upside down, etc. This data is not stored in the block
/// struct itself because it is not shared between all blocks of the same type, but rather is
/// stored in the chunk data.
///
/// Currently, the block state is stored as a 32 bit integer (u32) for simplicity and efficiency. The
/// type of the block state is stored in the lower 16 bits, and the data is stored in the upper 16
/// bits. This allows for up to 65536 different block state types, each with up to 65536 different
/// data values.
#[derive(Clone, Copy, Debug)]
pub struct BlockState(u32);
impl BlockState {
    pub const NONE: BlockState = BlockState::none();
    pub const SLAB_BOTTOM: BlockState = BlockState::slab(false);
    pub const SLAB_TOP: BlockState = BlockState::slab(true);

    /// Creates a new block state with the given type and data.
    #[inline]
    pub const fn new(state_type: u16, data: u16) -> BlockState {
        BlockState((state_type as u32) | ((data as u32) << 16))
    }

    /// Creates a new block state with the given bits.
    #[inline]
    pub const fn from_bits(bits: u32) -> BlockState {
        BlockState(bits)
    }

    /// Gets the bits of the block state.
    #[inline]
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Gets the type of the block state.
    #[inline]
    pub const fn state_type(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Gets the data of the block state.
    #[inline]
    pub const fn data(&self) -> u16 {
        (self.0 >> 16) as u16
    }

    /// Creates an empty block state with no data.
    #[inline]
    pub const fn none() -> BlockState {
        BlockState::new(0x0000, 0x0000)
    }

    /// Creates a slab block state with the given top/bottom value.
    #[inline]
    pub const fn slab(is_top: bool) -> BlockState {
        BlockState::new(0x0001, if is_top { 0x0001 } else { 0x0000 })
    }

    /// Checks if the block state is empty (i.e. has no data).
    #[inline]
    pub const fn is_none(&self) -> bool {
        self.state_type() == 0x0000
    }

    /// Checks if the block state is a slab and returns whether it is the top or bottom half of the
    /// block if it is.
    #[inline]
    pub const fn is_slab(&self) -> Option<bool> {
        if self.state_type() == 0x0001 {
            Some(self.data() != 0)
        } else {
            None
        }
    }

    /// Creates a string representation of the block state, that can be added to [`Block::ident`]
    /// to get a unique identifier for a block with the given state.
    ///
    /// For example, a block with identifier "stone_slab" and a block state of
    /// [`BlockState::SLAB_TOP`] would have a unique identifier of "stone_slab_top".
    #[inline]
    pub const fn to_ident(&self) -> Option<&'static str> {
        match (self.state_type(), self.data()) {
            (0x0000, 0x0000) => Some(""),
            (0x0001, 0x0000) => Some("_bot"),
            (0x0001, 0x0001) => Some("_top"),
            _ => None,
        }
    }

    /// Returns all possible data values for the given block state type. If the slice is empty,
    /// then the block state of that type can have any data value (i.e. the data value is not used
    /// for that block state type). If the block state type is not recognized, then `None` is
    /// returned.
    #[inline]
    pub const fn possible_data_values(state_type: u16) -> Option<&'static [u16]> {
        match state_type {
            0x0000 => Some(&[0x0000]),         // NONE
            0x0001 => Some(&[0x0000, 0x0001]), // SLAB
            _ => None,
        }
    }

    /// Returns a default block state that can be used for displaying blocks in inventories and
    /// such.
    #[inline]
    pub const fn default_state(state_type: u16) -> Option<BlockState> {
        match state_type {
            0x0000 => Some(BlockState::NONE),
            0x0001 => Some(BlockState::SLAB_BOTTOM),
            _ => None,
        }
    }
}
