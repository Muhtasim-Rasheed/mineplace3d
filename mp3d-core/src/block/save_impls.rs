use crate::block::*;
use crate::saving::Saveable;
use crate::saving::WorldLoadError;
use crate::saving::io::*;

impl Saveable for BlockId {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();

        let ident = block_registry().get(*self).unwrap().ident;

        let ident_bytes = ident.as_bytes();
        data.push(ident_bytes.len() as u8);
        data.extend(ident_bytes);

        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError>
    where
        Self: Sized,
    {
        let ident_str = if version >= 0x06 {
            let ident_len = read_u8(data, "Block::ident_len")? as usize;
            let ident_str = read_string(data, ident_len, "Block::ident")?;
            ident_str
        } else {
            read_u8(data, "Block::visible")?;
            let ident_len = read_u8(data, "Block::ident_len")? as usize;
            let ident_str = read_string(data, ident_len, "Block::ident")?;
            read_u8(data, "Block::collision_shape")?;
            if version >= 4 {
                read_u8(data, "Block::interact_shape")?;
            }
            if version >= 1 {
                read_u16(data, "Block::state_type")?;
            }
            ident_str
        };

        if let Some(id) = block_registry().get_id(&ident_str) {
            Ok(id)
        } else {
            Err(WorldLoadError::InvalidSaveFormat(format!(
                "Unknown block identifier: {ident_str}"
            )))
        }
    }
}

impl Saveable for BlockState {
    fn save(&self) -> Vec<u8> {
        self.bits().to_le_bytes().to_vec()
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError> {
        if version < 1 {
            Ok(BlockState::none())
        } else {
            Ok(BlockState::from_bits(read_u32(data, "BlockState::bits")?))
        }
    }
}

impl Saveable for (BlockId, BlockState) {
    fn save(&self) -> Vec<u8> {
        let mut data = self.0.save();
        data.extend(self.1.save());
        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError> {
        let block = BlockId::load(data, version)?;
        let block_state = BlockState::load(data, version)?;
        Ok((block, block_state))
    }
}
