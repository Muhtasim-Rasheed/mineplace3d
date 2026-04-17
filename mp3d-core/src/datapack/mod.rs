//! Module to control block drops (that's it for now).

use fxhash::FxHashMap;

use crate::datapack::files::DataSources;

pub mod files;

#[derive(serde::Deserialize)]
struct RawDropEntry(u32, f32, u32, f32);

#[derive(Clone)]
pub struct DropEntry {
    pub min: u32,
    pub min_chance: f32,
    pub max: u32,
    pub max_chance: f32,
}

impl TryFrom<RawDropEntry> for DropEntry {
    type Error = String;

    fn try_from(mut value: RawDropEntry) -> Result<Self, Self::Error> {
        if value.0 > value.2 {
            return Err("min > max".into());
        }

        value.1 = value.1.clamp(0.0, 1.0);
        value.3 = value.3.clamp(0.0, 1.0);

        Ok(Self {
            min: value.0,
            min_chance: value.1,
            max: value.2,
            max_chance: value.3,
        })
    }
}

#[derive(serde::Deserialize)]
struct RawLootTableEntry {
    drops: FxHashMap<String, RawDropEntry>,
}

pub struct LootTableEntry {
    pub drops: FxHashMap<String, DropEntry>,
}

impl TryFrom<RawLootTableEntry> for LootTableEntry {
    type Error = String;

    fn try_from(value: RawLootTableEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            drops: value
                .drops
                .into_iter()
                .map(|(k, rv)| {
                    rv.try_into()
                        .map_err(|e| format!("{}: {}", &k, e))
                        .map(|v| (k, v))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

pub struct LootTable {
    block_entries: FxHashMap<String, LootTableEntry>,
}

pub struct GameData {
    sources: DataSources,
    loot_table: LootTable,
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}

impl GameData {
    pub fn new() -> Self {
        Self {
            sources: DataSources::new(),
            loot_table: LootTable {
                block_entries: FxHashMap::default(),
            },
        }
    }

    pub fn get_block_drops(&mut self, ident: &'static str) -> Option<&LootTableEntry> {
        if self.loot_table.block_entries.contains_key(ident) {
            return self.loot_table.block_entries.get(ident);
        }

        let path = std::path::PathBuf::from(format!("loot_table/blocks/{}.json", ident));

        let contents = self.sources.read_utf8(&path)?;

        let parsed_raw = match serde_json::from_str::<RawLootTableEntry>(&contents) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to read block loot table {}: {}", ident, e);
                return None;
            }
        };

        let parsed = match LootTableEntry::try_from(parsed_raw) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to convert loot table {}: {:?}", ident, e);
                return None;
            }
        };

        self.loot_table
            .block_entries
            .insert(ident.to_string(), parsed);

        self.loot_table.block_entries.get(ident)
    }
}
