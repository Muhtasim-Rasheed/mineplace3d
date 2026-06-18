use crate::direction::Direction;

/// Struct to store the block state of a block in the world.
///
/// For example, slabs store whether they are the top or bottom half of a block, stairs store their
/// facing direction, etc. This data is not stored in the block struct itself because it is not shared
/// between all blocks of the same type, but rather is stored in the chunk data.
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
