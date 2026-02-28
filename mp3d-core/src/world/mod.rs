//! A world consisting of multiple chunks.
//!
//! The `World` struct manages a collection of `Chunk`s, each representing a
//! 16x16x16 section of the world. It provides methods for loading, unloading,
//! and accessing chunks, as well as handling world generation and updates.

pub mod chunk;

use std::collections::HashMap;

use glam::{IVec3, Vec3};

use crate::{
    block::Block,
    entity::{Entity, EntityType, PlayerEntity},
    world::chunk::{CHUNK_SIZE, Chunk},
};

const PRELOAD_RADIUS: i32 = 8;

/// A world consisting of multiple chunks. Each chunk contains a 16x16x16 grid of blocks.
pub struct World {
    pub chunks: HashMap<IVec3, Chunk>,
    pub entities: HashMap<u64, Box<dyn Entity>>,
    pub noise: fastnoise_lite::FastNoiseLite,
    // Storage of player data, keyed by username. This is used to store player data when they are
    // not currently in the world.
    pub(super) player_cache: HashMap<String, PlayerEntity>,
    /// A map of chunk positions to a map of local block positions to the new block state. This is
    /// used to track changes to chunks that have been modified by the player or other entities.
    changes: HashMap<IVec3, HashMap<IVec3, Block>>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates a new empty world.
    pub fn new() -> Self {
        let mut noise = fastnoise_lite::FastNoiseLite::new();
        noise.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
        let mut chunks = HashMap::new();
        // Preload some chunks around the origin
        for x in -PRELOAD_RADIUS..PRELOAD_RADIUS {
            for y in -1..1 {
                for z in -PRELOAD_RADIUS..PRELOAD_RADIUS {
                    let chunk_pos = IVec3::new(x, y, z);
                    chunks.insert(chunk_pos, Chunk::new(chunk_pos, &noise));
                }
            }
        }
        World {
            chunks,
            entities: HashMap::new(),
            noise,
            player_cache: HashMap::new(),
            changes: HashMap::new(),
        }
    }

    /// Gets a block at the given world position.
    pub fn get_block_at(&self, world_pos: IVec3) -> Option<&Block> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.chunks.get(&chunk_pos).map(|c| c.get_block(local_pos))
    }

    /// Gets a block at the given world position, or generates a new chunk and returns the block if
    /// it doesn't exist.
    pub fn get_block_or_new(&mut self, world_pos: IVec3) -> &Block {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.get_chunk_or_new(chunk_pos).get_block(local_pos)
    }

    /// Sets a block at the given world position.
    pub fn set_block_at(&mut self, world_pos: IVec3, block: Block) {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.changes
            .entry(chunk_pos)
            .or_insert_with(HashMap::new)
            .insert(local_pos, block);
        let chunk = self.get_chunk_mut_or_new(chunk_pos);
        chunk.set_block(local_pos, block);
    }

    /// Gets a reference to a chunk at the given chunk position, or loads it if it doesn't exist.
    pub fn get_chunk_or_new(&mut self, chunk_pos: IVec3) -> &Chunk {
        self.chunks.entry(chunk_pos).or_insert_with(|| {
            let mut chunk = Chunk::new(chunk_pos, &self.noise);
            if let Some(changes) = self.changes.get(&chunk_pos) {
                for (local_pos, block) in changes {
                    chunk.set_block(*local_pos, *block);
                }
            }
            chunk
        })
    }

    /// Gets a mutable reference to a chunk at the given chunk position, or loads it if it doesn't
    /// exist.
    pub fn get_chunk_mut_or_new(&mut self, chunk_pos: IVec3) -> &mut Chunk {
        self.chunks.entry(chunk_pos).or_insert_with(|| {
            let mut chunk = Chunk::new(chunk_pos, &self.noise);
            if let Some(changes) = self.changes.get(&chunk_pos) {
                for (local_pos, block) in changes {
                    chunk.set_block(*local_pos, *block);
                }
            }
            chunk
        })
    }

    /// Gets the ID of the next available entity.
    fn next_entity_id(&self) -> u64 {
        let mut id = 1;
        while self.entities.contains_key(&id) {
            id += 1;
        }
        id
    }

    /// Adds an entity to the world, assigning it a unique ID.
    pub fn add_entity(&mut self, mut entity: Box<dyn Entity>) -> u64 {
        let entity_id = self.next_entity_id();
        entity.set_id(entity_id);
        self.entities.insert(entity_id, entity);
        entity_id
    }

    /// Removes an entity from the world by its ID.
    pub fn remove_entity(&mut self, entity_id: u64) -> Option<Box<dyn Entity>> {
        self.entities.remove(&entity_id)
    }

    /// Gets a reference to an entity by its ID.
    pub fn get_entity<E: Entity>(&self, entity_id: u64) -> Option<&E> {
        self.entities
            .get(&entity_id)
            .and_then(|e| e.as_any().downcast_ref::<E>())
    }

    /// Gets a mutable reference to an entity by its ID.
    pub fn get_entity_mut<E: Entity>(&mut self, entity_id: u64) -> Option<&mut E> {
        self.entities
            .get_mut(&entity_id)
            .and_then(|e| e.as_any_mut().downcast_mut::<E>())
    }

    /// Updates the world. The optimal TPS (Ticks Per Second) is 48.
    pub fn tick(&mut self, tps: u8) {
        let entity_ids: Vec<u64> = self.entities.keys().cloned().collect();
        for entity_id in entity_ids {
            if let Some(mut entity) = self.entities.remove(&entity_id) {
                entity.tick(self, tps);

                if !entity.requests_removal() {
                    self.entities.insert(entity_id, entity);
                }
            }
        }
    }

    /// Checks for collisions between an entity (using its position, width, and height) and the
    /// blocks in the world. This is used for player movement and other entity interactions with
    /// the world.
    pub fn collides(&self, entity_pos: Vec3, entity_width: f32, entity_height: f32) -> bool {
        let min_block_pos = (entity_pos - Vec3::splat(entity_width / 2.0))
            .floor()
            .as_ivec3();
        let max_block_pos = (entity_pos
            + Vec3::new(entity_width / 2.0, entity_height, entity_width / 2.0))
        .floor()
        .as_ivec3();

        for x in min_block_pos.x..=max_block_pos.x {
            for y in min_block_pos.y..=max_block_pos.y {
                for z in min_block_pos.z..=max_block_pos.z {
                    let block_pos = IVec3::new(x, y, z);
                    if let Some(block) = self.get_block_at(block_pos) {
                        let block_state = crate::block::BlockState::none();
                        if block.collides_with_player(
                            entity_width,
                            entity_height,
                            entity_pos - block_pos.as_vec3(),
                            block_state,
                        ) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

pub enum WorldLoadError {
    MissingSaveFile(std::path::PathBuf),
    InvalidSaveFormat(String),
}

impl std::fmt::Display for WorldLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorldLoadError::MissingSaveFile(path) => {
                write!(f, "Save file not found: {}", path.display())
            }
            WorldLoadError::InvalidSaveFormat(msg) => write!(f, "Invalid save format: {}", msg),
        }
    }
}

impl std::fmt::Debug for WorldLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorldLoadError: {}", self)
    }
}

impl std::error::Error for WorldLoadError {}

impl From<WorldLoadError> for std::io::Error {
    fn from(err: WorldLoadError) -> Self {
        match err {
            WorldLoadError::MissingSaveFile(path) => std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Save file not found: {}", path.display()),
            ),
            WorldLoadError::InvalidSaveFormat(msg) => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid save format: {}", msg),
            ),
        }
    }
}

/// The current version of the world save format (in beta).
pub const SAVE_VERSION: u8 = 0;

impl World {
    /// Saves the world to a folder.
    ///
    /// All modified chunks are saved to the "chunks" subfolder, with filenames in the format
    /// "chunk_x_y_z.bin". The entity data is saved to "entities.bin". The player data is contained
    /// in the "players" subfolder, with filenames in the format "{hashed_username}.bin", which
    /// contains the position, rotation, and other relevant data for each player. Note that the
    /// players, even though they are entities, aren't stored in the entities.bin file, since they
    /// are linked to user accounts and need to be loaded and linked to the accounts when they
    /// join, so they are stored separately in the "players" subfolder. The folder also contains a
    /// "save.bin" file with metadata about the world, such as the seed, generation settings, and
    /// also the version of the save format, so that future versions of the game can maintain
    /// compatibility with older saves. The entity IDs aren't stored in the world save, since they
    /// can be generated on load anyways.
    ///
    /// # chunks/chunk_x_y_z.bin
    /// - 2 bytes: number of changes in the chunk (N)
    /// - N times
    ///   - 3 bytes: local block position (x, y, z) within the chunk (0-15)
    ///   - 1 byte: whether the block is visible (0 or 1)
    ///   - 1 byte: length of the block identifier (M)
    ///   - M bytes: block identifier (UTF-8 string)
    ///   - 1 byte: collision shape
    /// - actual chunk data defined by [`Chunk::save`]
    ///
    /// # save.bin
    /// - 1 byte: save format version (u8)
    /// - 4 bytes: world seed (i32)
    ///
    /// # entities.bin
    /// - 8 bytes: number of entities (N)
    /// - N times
    ///   - 1 byte: entity type (u8)
    ///   - 4 bytes: length of entity data (M)
    ///   - M bytes: entity data (format defined by each entity type)
    ///
    /// # players/{hashed_username}.bin
    /// - 1 byte: length of username (U)
    /// - U bytes: username (UTF-8 string)
    /// - 12 bytes: position (3 f32 values for x, y, z)
    /// - 12 bytes: velocity (3 f32 values for x, y, z)
    /// - 4 bytes: yaw (f32)
    /// - 4 bytes: pitch (f32)
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut save_file = std::fs::File::create(path.join("save.bin"))?;
        std::io::Write::write_all(&mut save_file, &[SAVE_VERSION])?;
        std::io::Write::write_all(&mut save_file, &self.noise.seed.to_le_bytes())?;

        std::fs::create_dir_all(path.join("chunks"))?;
        for (chunk_pos, changes) in &self.changes {
            let mut chunk = Chunk::new(*chunk_pos, &self.noise);
            for (local_pos, block) in changes {
                chunk.set_block(*local_pos, *block);
            }
            let chunk_path = path.join("chunks").join(format!(
                "chunk_{}_{}_{}.bin",
                chunk_pos.x, chunk_pos.y, chunk_pos.z
            ));
            let chunk_data = chunk.save();
            let mut chunk_file = std::fs::File::create(chunk_path)?;
            let change_count = changes.len() as u16;
            std::io::Write::write_all(&mut chunk_file, &change_count.to_le_bytes())?;
            for (local_pos, block) in changes {
                std::io::Write::write_all(
                    &mut chunk_file,
                    &[local_pos.x as u8, local_pos.y as u8, local_pos.z as u8],
                )?;
                std::io::Write::write_all(&mut chunk_file, &[block.visible as u8])?;
                let block_id = block.ident;
                std::io::Write::write_all(&mut chunk_file, &(block_id.len() as u8).to_le_bytes())?;
                std::io::Write::write_all(&mut chunk_file, block_id.as_bytes())?;
                std::io::Write::write_all(&mut chunk_file, &[block.collision_shape as u8])?;
            }
            std::io::Write::write_all(&mut chunk_file, &chunk_data)?;
        }

        let mut entities_file = std::fs::File::create(path.join("entities.bin"))?;
        std::fs::create_dir_all(path.join("players"))?;
        let entity_count = self
            .entities
            .values()
            .filter(|e| e.entity_type() != EntityType::Player)
            .count() as u64;
        std::io::Write::write_all(&mut entities_file, &entity_count.to_le_bytes())?;
        for entity in self.entities.values() {
            let entity_type = entity.entity_type() as u8;
            if entity_type == EntityType::Player as u8 {
                let player = entity.as_any().downcast_ref::<PlayerEntity>().unwrap();
                let player_data = player.save();
                let hashed_username = fxhash::hash64(player.username.as_bytes());
                let player_path = path
                    .join("players")
                    .join(format!("{}.bin", hashed_username));
                let mut player_file = std::fs::File::create(player_path)?;
                std::io::Write::write_all(&mut player_file, &player_data)?;
            } else {
                let entity_data = entity.save();
                let entity_data_len = entity_data.len() as u32;
                std::io::Write::write_all(&mut entities_file, &[entity_type])?;
                std::io::Write::write_all(&mut entities_file, &entity_data_len.to_le_bytes())?;
                std::io::Write::write_all(&mut entities_file, &entity_data)?;
            }
        }
        for cached in self.player_cache.values() {
            let player_data = cached.save();
            let hashed_username = fxhash::hash64(cached.username.as_bytes());
            let player_path = path
                .join("players")
                .join(format!("{}.bin", hashed_username));
            let mut player_file = std::fs::File::create(player_path)?;
            std::io::Write::write_all(&mut player_file, &player_data)?;
        }

        Ok(())
    }

    /// Loads a world from a folder. The folder should have the same structure as described in the
    /// `save` method.
    pub fn load(path: &std::path::Path) -> Result<Self, WorldLoadError> {
        fn take_exact<I: Iterator<Item = u8>>(
            n: usize,
            iter: &mut I,
        ) -> Result<Vec<u8>, WorldLoadError> {
            let bytes: Vec<u8> = iter.take(n).collect();
            if bytes.len() == n {
                Ok(bytes)
            } else {
                Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Unexpected end of file while reading {} bytes",
                    n
                )))
            }
        }

        let save_content = std::fs::read(path.join("save.bin"))
            .map_err(|_| WorldLoadError::MissingSaveFile(path.join("save.bin")))?;
        let mut save_iter = save_content.iter();
        match save_iter.next() {
            Some(&version) if version == 0 => {
                // SEED
                let seed_bytes = take_exact(4, &mut save_iter.cloned())?;
                let seed = i32::from_le_bytes(seed_bytes.try_into().unwrap());
                let mut noise = fastnoise_lite::FastNoiseLite::new();
                noise.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
                noise.set_seed(Some(seed));

                let mut world = World {
                    chunks: HashMap::new(),
                    entities: HashMap::new(),
                    noise,
                    player_cache: HashMap::new(),
                    changes: HashMap::new(),
                };

                // CHUNKS
                let chunks_dir = path.join("chunks");
                if !chunks_dir.exists() {
                    return Err(WorldLoadError::MissingSaveFile(chunks_dir));
                }
                for entry in std::fs::read_dir(chunks_dir).unwrap() {
                    let entry = entry.unwrap();
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_str().unwrap();
                    if !file_name_str.starts_with("chunk_") || !file_name_str.ends_with(".bin") {
                        continue;
                    }
                    let parts: Vec<&str> = file_name_str[6..file_name_str.len() - 4]
                        .split('_')
                        .collect();
                    if parts.len() != 3 {
                        continue;
                    }
                    let chunk_pos = IVec3::new(
                        parts[0].parse().unwrap(),
                        parts[1].parse().unwrap(),
                        parts[2].parse().unwrap(),
                    );
                    let chunk_data = std::fs::read(entry.path()).unwrap();
                    let mut chunk_iter = chunk_data.into_iter();
                    let change_count_bytes = take_exact(2, &mut chunk_iter)?.try_into().unwrap();
                    let change_count = u16::from_le_bytes(change_count_bytes);
                    for _ in 0..change_count {
                        let local_pos_bytes = take_exact(3, &mut chunk_iter)?;
                        let local_pos = IVec3::new(
                            local_pos_bytes[0] as i32,
                            local_pos_bytes[1] as i32,
                            local_pos_bytes[2] as i32,
                        );
                        let visible = chunk_iter.next().unwrap() == 1;
                        let block_id_len = chunk_iter.next().unwrap() as usize;
                        let block_id_bytes = take_exact(block_id_len, &mut chunk_iter)?;
                        let block_id = String::from_utf8(block_id_bytes).unwrap();
                        let block_id_static =
                            if let Some(ident) = super::block::get_block_ident(&block_id) {
                                ident
                            } else {
                                return Err(WorldLoadError::InvalidSaveFormat(format!(
                                    "Unknown block identifier: {}",
                                    block_id
                                )));
                            };
                        let collision_shape_byte = chunk_iter.next().unwrap();
                        let collision_shape = match collision_shape_byte {
                            0 => crate::block::CollisionShape::None,
                            1 => crate::block::CollisionShape::FullBlock,
                            _ => {
                                return Err(WorldLoadError::InvalidSaveFormat(format!(
                                    "Unknown collision shape: {}",
                                    collision_shape_byte
                                )));
                            }
                        };
                        let block = Block {
                            ident: block_id_static,
                            visible,
                            collision_shape,
                        };
                        world
                            .changes
                            .entry(chunk_pos)
                            .or_insert_with(HashMap::new)
                            .insert(local_pos, block);
                    }
                }

                // ENTITIES
                let entities_path = path.join("entities.bin");
                if !entities_path.exists() {
                    return Err(WorldLoadError::MissingSaveFile(entities_path));
                }
                let entities_data = std::fs::read(entities_path).unwrap();
                let mut entities_iter = entities_data.into_iter();
                let entity_count_bytes = take_exact(8, &mut entities_iter)?.try_into().unwrap();
                let entity_count = u64::from_le_bytes(entity_count_bytes);
                #[allow(unreachable_code, unused_variables)]
                for _ in 0..entity_count {
                    let entity_type = entities_iter.next().unwrap();
                    let entity_data_len_bytes =
                        take_exact(4, &mut entities_iter)?.try_into().unwrap();
                    let entity_data_len = u32::from_le_bytes(entity_data_len_bytes);
                    let entity_data = take_exact(entity_data_len as usize, &mut entities_iter)?;
                    let entity: Box<dyn Entity> = match entity_type {
                        x if x == EntityType::Player as u8 => {
                            return Err(WorldLoadError::InvalidSaveFormat(
                                "Player entities should be stored in the players folder"
                                    .to_string(),
                            ));
                        }
                        _ => {
                            return Err(WorldLoadError::InvalidSaveFormat(format!(
                                "Unknown entity type: {}",
                                entity_type
                            )));
                        }
                    };
                    world.add_entity(entity);
                }

                let players_dir = path.join("players");
                if !players_dir.exists() {
                    return Err(WorldLoadError::MissingSaveFile(players_dir));
                }
                for entry in std::fs::read_dir(players_dir).unwrap() {
                    let entry = entry.unwrap();
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_str().unwrap();
                    if !file_name_str.ends_with(".bin") {
                        continue;
                    }
                    let player_data = std::fs::read(entry.path()).unwrap();
                    let player = PlayerEntity::load(&player_data, version).map_err(|e| {
                        WorldLoadError::InvalidSaveFormat(format!(
                            "Failed to load player data from {}: {}",
                            entry.path().display(),
                            e
                        ))
                    })?;
                    world.player_cache.insert(player.username.clone(), player);
                }

                Ok(world)
            }
            Some(&version) => {
                return Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Unsupported save version: {}",
                    version
                )));
            }
            None => {
                return Err(WorldLoadError::InvalidSaveFormat(
                    "Save file is empty".to_string(),
                ));
            }
        }
    }
}
