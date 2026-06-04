//! Blocks for a voxel engine.

use glam::{IVec3, Vec3};

use crate::direction::Direction;

mod save_impls;

/// A struct used for declaring different types of blocks on the fly. Mineplace provides some
/// already defined blocks and an array of the already defined blocks.
#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub visible: bool,
    pub collision_shape: CollisionShape,
    pub interact_shape: Option<CollisionShape>,
    pub ident: &'static str,
    pub state_type: u16,
}

macro_rules! blocks {
    (
        $(
            $name:ident => {
                ident: $ident:expr
                $(, visible: $visible:expr)?
                $(, collision_shape: $collision_shape:expr)?
                $(, interact_shape: $interact_shape:expr)?
                $(, state_type: $state_type:expr)?
                $(,)?
            }
        ),* $(,)?
    ) => {
        impl Block {
            $(
                pub const $name: Block = Block {
                    visible: blocks!(@visible $( $visible )?),
                    collision_shape: blocks!(@collision_shape $( $collision_shape )?),
                    interact_shape: blocks!(@interact_shape $( $interact_shape )?),
                    ident: $ident,
                    state_type: blocks!(@state_type $( $state_type )?),
                };
            )*

            pub const ALL_BLOCKS: &'static [Block] = &[
                $(Self::$name),*
            ];
        }
    };

    (@visible $visible:expr) => { $visible };
    (@visible) => { true };

    (@collision_shape $collision_shape:expr) => { $collision_shape };
    (@collision_shape) => { CollisionShape::FullBlock };

    (@interact_shape $interact_shape:expr) => { Some($interact_shape) };
    (@interact_shape) => { None };

    (@state_type $state_type:expr) => { $state_type };
    (@state_type) => { BlockState::NONE_TYPE };
}

blocks! {
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

impl Block {
    pub fn collides_with_player(
        &self,
        player_width: f32,
        player_height: f32,
        player_pos_local: Vec3,
        block_state: BlockState,
    ) -> bool {
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
        match self.collision_shape {
            CollisionShape::None => false,
            CollisionShape::FullBlock => {
                let block_min = Vec3::new(0.0, 0.0, 0.0);
                let block_max = Vec3::new(1.0, 1.0, 1.0);
                crate::aabb_overlap(player_min, player_max, block_min, block_max)
            }
            CollisionShape::Slab => {
                if let Some(shape) = block_state.is_slab() {
                    let block_min;
                    let block_max;
                    match shape {
                        0x0000 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 0.5, 1.0);
                        }
                        0x0001 => {
                            block_min = Vec3::new(0.0, 0.5, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        0x0002 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::aabb_overlap(player_min, player_max, block_min, block_max)
                } else {
                    false
                }
            }
            CollisionShape::Stairs => {
                if let Some(shape) = block_state.is_stairs() {
                    let element_a_min = Vec3::new(0.0, 0.0, 0.0);
                    let element_a_max = Vec3::new(1.0, 0.5, 1.0);
                    let element_b_min;
                    let element_b_max;
                    match shape {
                        Direction::North => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.5);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            element_b_min = Vec3::new(0.5, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::aabb_overlap(player_min, player_max, element_a_min, element_a_max)
                        || crate::aabb_overlap(player_min, player_max, element_b_min, element_b_max)
                } else {
                    false
                }
            }
            CollisionShape::VSlab => {
                if let Some(shape) = block_state.is_facing() {
                    let block_min;
                    let block_max;
                    match shape {
                        Direction::North => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            block_min = Vec3::new(0.0, 0.0, 0.5);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            block_min = Vec3::new(0.5, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::aabb_overlap(player_min, player_max, block_min, block_max)
                } else {
                    false
                }
            }
        }
    }

    /// Returns the normal of the hit face, if it hit anything.
    pub fn ray_intersect(
        &self,
        ray_origin_local: Vec3,
        ray_direction_local: Vec3,
        block_state: BlockState,
    ) -> Option<IVec3> {
        match self.interact_shape.unwrap_or(self.collision_shape) {
            CollisionShape::None => None,
            CollisionShape::FullBlock => {
                let block_min = Vec3::new(0.0, 0.0, 0.0);
                let block_max = Vec3::new(1.0, 1.0, 1.0);
                crate::ray_intersect_aabb(
                    ray_origin_local,
                    ray_direction_local,
                    block_min,
                    block_max,
                )
            }
            CollisionShape::Slab => {
                if let Some(shape) = block_state.is_slab() {
                    let block_min;
                    let block_max;
                    match shape {
                        0x0000 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 0.5, 1.0);
                        }
                        0x0001 => {
                            block_min = Vec3::new(0.0, 0.5, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        0x0002 => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::ray_intersect_aabb(
                        ray_origin_local,
                        ray_direction_local,
                        block_min,
                        block_max,
                    )
                } else {
                    None
                }
            }
            CollisionShape::Stairs => {
                if let Some(shape) = block_state.is_stairs() {
                    let element_a_min = Vec3::new(0.0, 0.0, 0.0);
                    let element_a_max = Vec3::new(1.0, 0.5, 1.0);
                    let element_b_min;
                    let element_b_max;
                    match shape {
                        Direction::North => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.5);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            element_b_min = Vec3::new(0.5, 0.5, 0.0);
                            element_b_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            element_b_min = Vec3::new(0.0, 0.5, 0.0);
                            element_b_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::ray_intersect_aabb(
                        ray_origin_local,
                        ray_direction_local,
                        element_a_min,
                        element_a_max,
                    )
                    .or_else(|| {
                        crate::ray_intersect_aabb(
                            ray_origin_local,
                            ray_direction_local,
                            element_b_min,
                            element_b_max,
                        )
                    })
                } else {
                    None
                }
            }
            CollisionShape::VSlab => {
                if let Some(shape) = block_state.is_facing() {
                    let block_min;
                    let block_max;
                    match shape {
                        Direction::North => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 0.5);
                        }
                        Direction::South => {
                            block_min = Vec3::new(0.0, 0.0, 0.5);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::East => {
                            block_min = Vec3::new(0.5, 0.0, 0.0);
                            block_max = Vec3::new(1.0, 1.0, 1.0);
                        }
                        Direction::West => {
                            block_min = Vec3::new(0.0, 0.0, 0.0);
                            block_max = Vec3::new(0.5, 1.0, 1.0);
                        }
                        _ => unreachable!(),
                    }
                    crate::ray_intersect_aabb(
                        ray_origin_local,
                        ray_direction_local,
                        block_min,
                        block_max,
                    )
                } else {
                    None
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
    /// A stair (the facing direction is determined by the block state).
    Stairs = 3,
    /// A vertical slab (the facing direction is determined by the block state).
    VSlab = 4,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockState(u32);

impl BlockState {
    pub const NONE_TYPE: u16 = 0x0000;
    pub const SLAB_TYPE: u16 = 0x0001;
    pub const STAIR_TYPE: u16 = 0x0002;
    pub const FACING_TYPE: u16 = 0x0003;

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
        BlockState::new(Self::NONE_TYPE, 0x0000)
    }

    /// Creates a slab block state with the given top/bottom value.
    #[inline]
    pub const fn slab(data: u16) -> BlockState {
        BlockState::new(Self::SLAB_TYPE, data)
    }

    /// Creates a stair block state with the given facing direction
    #[inline]
    pub const fn stairs(dir: Direction) -> BlockState {
        assert!(!matches!(dir, Direction::Up | Direction::Down));
        BlockState::new(Self::STAIR_TYPE, dir as u16)
    }

    /// Creates a vertical slab block state with the given facing direction.
    #[inline]
    pub const fn facing(dir: Direction) -> BlockState {
        assert!(!matches!(dir, Direction::Up | Direction::Down));
        BlockState::new(Self::FACING_TYPE, dir as u16)
    }

    /// Checks if the block state is empty (i.e. has no data).
    #[inline]
    pub const fn is_none(&self) -> bool {
        self.state_type() == Self::NONE_TYPE
    }

    /// Checks if the block state is a slab and returns whether it is the top or bottom half of the
    /// block if it is.
    #[inline]
    pub const fn is_slab(&self) -> Option<u16> {
        if self.state_type() == Self::SLAB_TYPE {
            Some(self.data())
        } else {
            None
        }
    }

    /// Checks if the block state is stairs and returns the facing direction if it is.
    #[inline]
    pub const fn is_stairs(&self) -> Option<Direction> {
        if self.state_type() == Self::STAIR_TYPE {
            Direction::from_u8(self.data() as u8)
        } else {
            None
        }
    }

    /// Checks if the block state is a vertical slab and returns the facing direction if it is.
    #[inline]
    pub const fn is_facing(&self) -> Option<Direction> {
        if self.state_type() == Self::FACING_TYPE {
            Direction::from_u8(self.data() as u8)
        } else {
            None
        }
    }

    /// Returns all possible data values for the given block state type. If the slice is empty,
    /// then the block state of that type can have any data value (i.e. the data value is not used
    /// for that block state type). If the block state type is not recognized, then `None` is
    /// returned.
    #[inline]
    pub const fn possible_data_values(state_type: u16) -> Option<&'static [u16]> {
        match state_type {
            Self::NONE_TYPE => Some(&[0x0000]),
            Self::SLAB_TYPE => Some(&[0x0000, 0x0001, 0x0002]),
            Self::STAIR_TYPE => Some(&[0x0000, 0x0001, 0x0002, 0x0003]),
            Self::FACING_TYPE => Some(&[0x0000, 0x0001, 0x0002, 0x0003]),
            _ => None,
        }
    }

    /// Returns a default block state that can be used for displaying blocks in inventories and
    /// such.
    #[inline]
    pub const fn default_state(state_type: u16) -> Option<BlockState> {
        match state_type {
            Self::NONE_TYPE => Some(BlockState::none()),
            Self::SLAB_TYPE => Some(BlockState::slab(0)),
            Self::STAIR_TYPE => Some(BlockState::stairs(Direction::North)),
            Self::FACING_TYPE => Some(BlockState::facing(Direction::North)),
            _ => None,
        }
    }
}
