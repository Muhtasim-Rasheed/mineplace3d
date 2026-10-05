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
        let props = self.named_props();
        let mut w = writer.save(&self.block).u8(props.len() as u8);
        for (name, value) in props {
            w = w
                .u8(name.len() as u8)
                .string(name)
                .u8(value.len() as u8)
                .string(&value);
        }
        w
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError> {
        let block: BlockId = reader.load(version)?;
        let mut state = BlockState::default_for(block);

        if version < 0x01 {
            return Ok(state);
        }

        if version < 0x09 {
            // legacy u32: 16 bits data | 16 bits type
            let bits = reader.u32().ctx("BlockState::bits")?;
            let data = (bits >> 16) as u16;
            if let Some(f) = block_registry().get(block).unwrap().from_legacy_state {
                state = f(state, data);
            }
            return Ok(state);
        }

        let count = reader.u8().ctx("BlockState::prop_count")?;
        for _ in 0..count {
            let nlen = reader.u8().ctx("BlockState::prop_name_len")? as usize;
            let name = reader.string(nlen).ctx("BlockState::prop_name")?;
            let vlen = reader.u8().ctx("BlockState::prop_value_len")? as usize;
            let value = reader.string(vlen).ctx("BlockState::prop_value")?;
            if !state.set_str(&name, &value) {
                log::warn!(
                    "{}: ignoring unknown property {}={}",
                    block_registry().get(block).unwrap().ident,
                    name,
                    value
                );
            }
        }
        Ok(state)
    }
}
