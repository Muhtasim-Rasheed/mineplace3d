use crate::item::*;
use crate::saving::Saveable;
use crate::saving::WorldLoadError;
use crate::saving::io::*;

impl Saveable for Item {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let ident_bytes = self.ident.as_bytes();
        data.push(ident_bytes.len() as u8);
        data.extend_from_slice(ident_bytes);
        data.push(self.assoc_block.is_some() as u8);
        data.extend_from_slice(&self.max_stack.to_le_bytes());
        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, _version: u8) -> Result<Self, WorldLoadError>
    where
        Self: Sized,
    {
        let ident_len = read_u8(data, "Item::ident_len")? as usize;
        let ident_str = read_string(data, ident_len, "Item::ident")?;
        let ident = get_item_ident(&ident_str).ok_or_else(|| {
            WorldLoadError::InvalidSaveFormat(format!("Unknown item identifier: {}", ident_str))
        })?;
        let has_block = read_u8(data, "Item::has_block")? != 0;
        let assoc_block = if has_block {
            Some(get_item_block(ident).ok_or_else(|| {
                WorldLoadError::InvalidSaveFormat(format!(
                    "Item {} is supposed to have an associated block, but it was not found",
                    ident
                ))
            })?)
        } else {
            None
        };
        let max_stack = read_u16(data, "Item::max_stack")?;
        Ok(Item {
            ident,
            assoc_block,
            max_stack,
        })
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
        let item = Item::load(data, version)?;
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

static ITEM_IDENTS: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
    std::sync::OnceLock::new();

static ITEM_BLOCKS: std::sync::OnceLock<std::collections::HashMap<&'static str, &'static Block>> =
    std::sync::OnceLock::new();

fn get_item_idents() -> &'static std::collections::HashSet<&'static str> {
    ITEM_IDENTS.get_or_init(|| {
        let mut set = std::collections::HashSet::new();
        for item in Item::ALL_ITEMS {
            set.insert(item.ident);
        }
        set
    })
}

fn get_item_blocks() -> &'static std::collections::HashMap<&'static str, &'static Block> {
    ITEM_BLOCKS.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        for item in Item::ALL_ITEMS {
            if let Some(block) = item.assoc_block {
                map.insert(item.ident, block);
            }
        }
        map
    })
}

/// Nice little helper for the module to convert from a `&str` to a `&'static str`, which is needed
/// for item identifiers as `Item` needs to be `Copy` and thus cannot contain owned `String`s.
/// This function will return `None` if the given identifier is not a valid item identifier, and
/// `Some(&'static str)` if it is.
fn get_item_ident(ident: &str) -> Option<&'static str> {
    let idents = get_item_idents();
    if let Some(&ident) = idents.get(ident) {
        Some(ident)
    } else {
        None
    }
}

/// Helper function to get the associated block for an item identifier, if it exists.
fn get_item_block(ident: &str) -> Option<&'static Block> {
    let blocks = get_item_blocks();
    if let Some(&block) = blocks.get(ident) {
        Some(block)
    } else {
        None
    }
}
