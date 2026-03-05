use crate::block::*;
use crate::saving::Saveable;
use crate::saving::WorldLoadError;
use crate::saving::io::*;

impl Saveable for Block {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.visible as u8);

        let ident_bytes = self.ident.as_bytes();
        data.push(ident_bytes.len() as u8);
        data.extend(ident_bytes);

        data.push(self.collision_shape as u8);
        data.extend(&self.state_type.to_le_bytes());

        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError> {
        let visible = read_u8(data, "Block::visible")? != 0;
        let ident_len = read_u8(data, "Block::ident_len")? as usize;
        let ident_str = read_string(data, ident_len, "Block::ident")?;
        let ident = get_block_ident(&ident_str).ok_or_else(|| {
            WorldLoadError::InvalidSaveFormat(format!("Unknown block identifier: {}", ident_str))
        })?;
        let state_type;
        let collision_shape_byte = read_u8(data, "Block::collision_shape")?;
        let collision_shape = match collision_shape_byte {
            0 => CollisionShape::None,
            1 => CollisionShape::FullBlock,
            2 => CollisionShape::Slab,
            _ => {
                return Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Invalid collision shape: {}",
                    collision_shape_byte
                )));
            }
        };
        match version {
            0 => state_type = 0,
            1 => state_type = read_u16(data, "Block::state_type")?,
            _ => {
                return Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Unsupported save version: {}",
                    version
                )));
            }
        };
        Ok(Block {
            visible,
            collision_shape,
            ident,
            state_type,
        })
    }
}

impl Saveable for BlockState {
    fn save(&self) -> Vec<u8> {
        self.bits().to_le_bytes().to_vec()
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError> {
        match version {
            0 => Ok(BlockState::none()),
            1 => Ok(BlockState::from_bits(read_u32(data, "BlockState::bits")?)),
            _ => {
                return Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Unsupported save version: {}",
                    version
                )));
            }
        }
    }
}

impl Saveable for (Block, BlockState) {
    fn save(&self) -> Vec<u8> {
        let mut data = self.0.save();
        data.extend(self.1.save());
        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError> {
        let block = Block::load(data, version)?;
        let block_state = BlockState::load(data, version)?;
        Ok((block, block_state))
    }
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

/// Nice little helper for the module to convert from a `&str` to a `&'static str`, which is needed
/// for block identifiers as `Block` needs to be `Copy` and thus cannot contain owned `String`s.
/// This function will return `None` if the given identifier is not a valid block identifier, and
/// `Some(&'static str)` if it is.
fn get_block_ident(ident: &str) -> Option<&'static str> {
    let idents = get_block_idents();
    if let Some(&ident) = idents.get(ident) {
        Some(ident)
    } else {
        None
    }
}
