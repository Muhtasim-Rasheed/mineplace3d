use crate::{
    block::*,
    serialize::{
        Saveable,
        read::{ByteReader, ReadError, ReadErrorExt, ReadErrorKind},
        write::ByteWriter,
    },
};

impl Saveable for BlockId {
    fn save(&self, writer: ByteWriter) -> ByteWriter {
        let ident = block_registry().get(*self).unwrap().ident;
        writer.u8(ident.len() as u8).string(ident)
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized,
    {
        let ident_str = if version >= 0x06 {
            let ident_len = reader.u8().ctx("Block::ident_len")? as usize;
            let ident_str = reader.string(ident_len).ctx("Block::ident")?;
            ident_str
        } else {
            reader.u8().ctx("Block::visible")?;
            let ident_len = reader.u8().ctx("Block::ident_len")? as usize;
            let ident_str = reader.string(ident_len).ctx("Block::ident")?;
            reader.u8().ctx("Block::collision_shape")?;
            if version >= 4 {
                reader.u8().ctx("Block::interact_shape")?;
            }
            if version >= 1 {
                reader.u16().ctx("Block::state_type")?;
            }
            ident_str
        };

        if let Some(id) = block_registry().get_id(&ident_str) {
            Ok(id)
        } else {
            Err(ReadErrorKind::InvalidId(ident_str.to_string())).ctx("converting to BlockId")
        }
    }
}

impl Saveable for BlockState {
    fn save(&self, writer: ByteWriter) -> ByteWriter {
        writer.u32(self.bits())
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized,
    {
        if version < 0x01 {
            Ok(BlockState::none())
        } else {
            Ok(BlockState::from_bits(reader.u32().ctx("BlockState::bits")?))
        }
    }
}

impl Saveable for (BlockId, BlockState) {
    fn save(&self, writer: ByteWriter) -> ByteWriter {
        writer.save(&self.0).save(&self.1)
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized,
    {
        let block = BlockId::load(reader, version)?;
        let block_state = BlockState::load(reader, version)?;
        Ok((block, block_state))
    }
}
