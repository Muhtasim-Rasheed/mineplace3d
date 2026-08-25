use crate::item::*;
use crate::serialize::Saveable;
use crate::serialize::read::{ByteReader, ReadError, ReadErrorExt, ReadErrorKind};
use crate::serialize::write::ByteWriter;

impl Saveable for ItemId {
    fn save(&self, writer: ByteWriter) -> ByteWriter {
        let ident = item_registry().get(*self).unwrap().ident;
        writer.u8(ident.len() as u8).string(ident)
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized,
    {
        let ident_str = if version >= 0x06 {
            let ident_len = reader.u8().ctx("Item::ident_len")? as usize;
            reader.string(ident_len).ctx("Item::ident")?
        } else {
            let ident_len = reader.u8().ctx("Item::ident_len")? as usize;
            let ident_str = reader.string(ident_len).ctx("Item::ident")?;
            reader.u8().ctx("Item::has_block")?;
            reader.u16().ctx("Item::max_stack")?;
            ident_str
        };

        if let Some(id) = item_registry().get_id(&ident_str) {
            Ok(id)
        } else {
            Err(ReadErrorKind::InvalidId(ident_str.to_string())).ctx("converting to ItemId")
        }
    }
}

impl Saveable for ItemStack {
    fn save(&self, writer: ByteWriter) -> ByteWriter {
        writer.save(&self.item).u16(self.count)
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized,
    {
        Ok(ItemStack {
            item: ItemId::load(reader, version)?,
            count: reader.u16().ctx("ItemStack::count")?,
        })
    }
}

impl Saveable for Inventory {
    fn save(&self, mut writer: ByteWriter) -> ByteWriter {
        for slot in &self.slots() {
            writer = writer.save(slot);
        }
        writer
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized,
    {
        let mut inventory = Inventory::new();
        for slot in inventory.slots_mut() {
            let slot_data = ItemStack::load(reader, version)?;
            *slot = slot_data;
        }
        Ok(inventory)
    }
}
