use crate::item::*;
use crate::saving::Saveable;
use crate::saving::WorldLoadError;
use crate::saving::io::*;

impl Saveable for ItemId {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let ident = item_registry().get(*self).unwrap().ident;

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
            let ident_len = read_u8(data, "Item::ident_len")? as usize;
            read_string(data, ident_len, "Item::ident")?
        } else {
            let ident_len = read_u8(data, "Item::ident_len")? as usize;
            let ident_str = read_string(data, ident_len, "Item::ident")?;
            read_u8(data, "Item::has_block")?;
            read_u16(data, "Item::max_stack")?;
            ident_str
        };

        if let Some(id) = item_registry().get_id(&ident_str) {
            Ok(id)
        } else {
            Err(WorldLoadError::InvalidSaveFormat(format!(
                "Unknown item identifier: {ident_str}"
            )))
        }
    }
}

impl Saveable for ItemStack {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let item_data = self.item.save();
        data.extend_from_slice(&item_data);
        data.extend_from_slice(&self.count.to_le_bytes());
        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError>
    where
        Self: Sized,
    {
        let item = ItemId::load(data, version)?;
        let count = read_u16(data, "ItemStack::count")?;
        Ok(ItemStack { item, count })
    }
}

impl Saveable for Inventory {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for slot in &self.slots() {
            let slot_data = slot.save();
            data.extend_from_slice(&slot_data);
        }
        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError>
    where
        Self: Sized,
    {
        let mut inventory = Inventory::new();
        for slot in inventory.slots_mut() {
            let slot_data = ItemStack::load(data, version)?;
            *slot = slot_data;
        }
        Ok(inventory)
    }
}
