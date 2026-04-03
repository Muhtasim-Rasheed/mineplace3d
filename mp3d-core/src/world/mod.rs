//! A world consisting of multiple chunks.
//!
//! The `World` struct manages a collection of `Chunk`s, each representing a
//! 16x16x16 section of the world. It provides methods for loading, unloading,
//! and accessing chunks, as well as handling world generation and updates.

pub mod chunk;

use std::collections::HashMap;

use glam::{IVec3, Vec3};

use crate::{
    block::{Block, BlockState},
    entity::{Entity, EntityType, PlayerEntity},
    saving::{io::*, Saveable, WorldLoadError, SAVE_VERSION},
    world::chunk::{Chunk, CHUNK_SIZE}, UniqueQueue,
};

const PRELOAD_RADIUS: i32 = 8;

/// A world consisting of multiple chunks. Each chunk contains a 16x16x16 grid of blocks.
pub struct World {
    pub chunks: fxhash::FxHashMap<IVec3, Chunk>,
    pub entities: fxhash::FxHashMap<u64, Box<dyn Entity>>,
    pub noise: fastnoise_lite::FastNoiseLite,
    
    // Storage of player data, keyed by username. This is used to store player data when they are
    // not currently in the world.
    pub(super) player_cache: HashMap<String, PlayerEntity>,

    /// Stores pending changes to blocks in the world. This is used to track changes that need to
    /// be sent to players.
    pub(super) pending_changes: PendingChanges,
    
    /// A map of chunk positions to a map of local block positions to the new block and block
    /// state. This is used to track changes to chunks that have been modified by the player or
    /// other entities.
    changes: HashMap<IVec3, HashMap<IVec3, (Block, BlockState)>>,
}

impl World {
    /// Creates a new empty world.
    pub fn new(seed: i32) -> Self {
        let mut noise = fastnoise_lite::FastNoiseLite::new();
        noise.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
        noise.set_seed(Some(seed));
        let mut chunks = fxhash::FxHashMap::default();
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
            entities: fxhash::FxHashMap::default(),
            noise,
            player_cache: HashMap::new(),
            pending_changes: PendingChanges::default(),
            changes: HashMap::new(),
        }
    }

    /// Gets a block at the given world position.
    pub fn get_block_at(&self, world_pos: IVec3) -> Option<(&Block, &BlockState)> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.chunks
            .get(&chunk_pos)
            .and_then(|c| c.get_block(local_pos))
    }

    /// Gets a block at the given world position, or generates a new chunk and returns the block if
    /// it doesn't exist.
    pub fn get_block_or_new(&mut self, world_pos: IVec3) -> Option<(&Block, &BlockState)> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.get_chunk_or_new(chunk_pos).get_block(local_pos)
    }

    /// Sets a block at the given world position.
    ///
    /// **Urgent version**: The change is added to the urgent changes queue, which will be drained
    /// first when sending updates to players, and then cleared.
    pub fn urgent_set_block_at(&mut self, world_pos: IVec3, block: Block, state: BlockState) {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.changes
            .entry(chunk_pos)
            .or_default()
            .insert(local_pos, (block, state));
        self.pending_changes.push(chunk_pos, local_pos, block, state, true);
        let chunk = self.get_chunk_mut_or_new(chunk_pos);
        chunk.set_block(local_pos, block, state);
    }

    /// Sets a block at the given world position.
    ///
    /// **Normal version**: The change is added to the normal changes queue, which will be sent to
    /// players after the urgent changes, and then cleared.
    pub fn normal_set_block_at(&mut self, world_pos: IVec3, block: Block, state: BlockState) {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.changes
            .entry(chunk_pos)
            .or_default()
            .insert(local_pos, (block, state));
        self.pending_changes.push(chunk_pos, local_pos, block, state, false);
        let chunk = self.get_chunk_mut_or_new(chunk_pos);
        chunk.set_block(local_pos, block, state);
    }

    /// Gets a reference to a chunk at the given chunk position, or loads it if it doesn't exist.
    pub fn get_chunk_or_new(&mut self, chunk_pos: IVec3) -> &Chunk {
        self.chunks.entry(chunk_pos).or_insert_with(|| {
            let mut chunk = Chunk::new(chunk_pos, &self.noise);
            if let Some(changes) = self.changes.get(&chunk_pos) {
                for (local_pos, (block, state)) in changes {
                    chunk.set_block(*local_pos, *block, *state);
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
                for (local_pos, (block, state)) in changes {
                    chunk.set_block(*local_pos, *block, *state);
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
        let mut updates = Vec::new();
        for (pos, chunk) in &self.chunks {
            updates.extend_from_slice(&chunk.random_tick(5, &self.chunks, *pos));
        }
        for update in updates {
            self.normal_set_block_at(update.0, update.1, update.2);
        }

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
                    if let Some((block, block_state)) = self.get_block_at(block_pos)
                        && block.collides_with_player(
                            entity_width,
                            entity_height,
                            entity_pos - block_pos.as_vec3(),
                            *block_state,
                        )
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Handles a block interaction at the given world position and face index. If the block is not
    /// interactive, this will attempt to place a block on the face that was clicked.
    pub fn block_interaction(&mut self, player_entity_id: u64, block_pos: IVec3, face: u8) {
        match self.get_block_at(block_pos).map(|(b, s)| (b.ident, *s)) {
            Some(("glungus", _)) => {
                self.interact_glungus(block_pos);
                return;
            }
            Some((ident, state)) if state == BlockState::slab(false) && face == 4 => {
                let player = match self.get_entity::<PlayerEntity>(player_entity_id) {
                    Some(p) => p,
                    None => return,
                };

                let place_block = player
                    .inventory
                    .hotbar_slot(player.hotbar_index)
                    .item
                    .assoc_block;

                if let Some(item_block) = place_block
                    && item_block.ident == ident
                {
                    if ident == "stone_slab" {
                        self.urgent_set_block_at(block_pos, Block::STONE, BlockState::none())
                    }
                    return;
                }
            }
            Some((ident, state)) if state == BlockState::slab(true) && face == 5 => {
                let player = match self.get_entity::<PlayerEntity>(player_entity_id) {
                    Some(p) => p,
                    None => return,
                };

                let place_block = player
                    .inventory
                    .hotbar_slot(player.hotbar_index)
                    .item
                    .assoc_block;

                if let Some(item_block) = place_block
                    && item_block.ident == ident
                {
                    if ident == "stone_slab" {
                        self.urgent_set_block_at(block_pos, Block::STONE, BlockState::none())
                    }
                    return;
                }
            }
            _ => {}
        }

        // Normal block placement logic
        let place_pos = block_pos
            + match face {
                0 => IVec3::new(0, 0, -1),
                1 => IVec3::new(0, 0, 1),
                2 => IVec3::new(1, 0, 0),
                3 => IVec3::new(-1, 0, 0),
                4 => IVec3::new(0, 1, 0),
                5 => IVec3::new(0, -1, 0),
                _ => return,
            };

        let player = match self.get_entity_mut::<PlayerEntity>(player_entity_id) {
            Some(p) => p,
            None => return,
        };

        let player_pos = player.position;

        let place_block = player
            .inventory
            .hotbar_slot(player.hotbar_index)
            .item
            .assoc_block;

        if let Some(block) = place_block
            && let Some(state) = BlockState::default_state(block.state_type)
        {
            let old_block = *self
                .get_block_at(place_pos)
                .map(|(b, _)| b)
                .unwrap_or(&Block::AIR);

            self.urgent_set_block_at(place_pos, *block, state);

            if self.collides(player_pos, PlayerEntity::width(), PlayerEntity::height()) {
                self.urgent_set_block_at(place_pos, old_block, BlockState::none());
            }
        }
    }

    fn interact_glungus(&mut self, block_pos: IVec3) {
        let radius_sq = 4;
        for x in -2..=2 {
            for y in -2..=2 {
                for z in -2..=2 {
                    if x * x + y * y + z * z <= radius_sq {
                        let pos = block_pos + IVec3::new(x, y, z);
                        self.normal_set_block_at(pos, Block::AIR, BlockState::none());
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct PendingChanges {
    /// Also stores changes, but will be sent to players and then cleared.
    ///
    /// **Urgent version**: Changes that are added to this queue will be sent to players before the
    /// normal changes, and then cleared.
    pub urgent: UniqueQueue<(IVec3, IVec3)>,

    /// Also stores changes, but will be sent to players and then cleared.
    ///
    /// **Normal version**: Changes that are added to this queue will be sent to players after the
    /// urgent changes, and then cleared.
    pub normal: UniqueQueue<(IVec3, IVec3)>,

    /// Stores data for the two queues above. This makes sure that if a block is changed multiple
    /// times in a tick, only the final state is sent to the players.
    pub data: HashMap<(IVec3, IVec3), (Block, BlockState)>,
}

impl PendingChanges {
    pub fn push(&mut self, chunk_pos: IVec3, local_pos: IVec3, block: Block, state: BlockState, urgent: bool) {
        self.data.insert((chunk_pos, local_pos), (block, state));
        if urgent {
            self.urgent.push((chunk_pos, local_pos));
        } else {
            self.normal.push((chunk_pos, local_pos));
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Iterator for PendingChanges {
    type Item = (IVec3, IVec3, Block, BlockState);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((chunk_pos, local_pos)) = self.urgent.pop() {
            self.data.remove(&(chunk_pos, local_pos)).map(|(block, state)| (chunk_pos, local_pos, block, state))
        } else if let Some((chunk_pos, local_pos)) = self.normal.pop() {
            self.data.remove(&(chunk_pos, local_pos)).map(|(block, state)| (chunk_pos, local_pos, block, state))
        } else {
            None
        }
    }
}

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
    ///   - 2 bytes: block state type (u16)
    ///   - 4 bytes: block state data (u32)
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

        log::info!("Saved save.bin");

        std::fs::create_dir_all(path.join("chunks"))?;
        for (chunk_pos, changes) in &self.changes {
            let mut chunk = Chunk::new(*chunk_pos, &self.noise);
            for (local_pos, (block, state)) in changes {
                chunk.set_block(*local_pos, *block, *state);
            }
            let chunk_path = path.join("chunks").join(format!(
                "chunk_{}_{}_{}.bin",
                chunk_pos.x, chunk_pos.y, chunk_pos.z
            ));
            let chunk_data = chunk.save();
            let mut chunk_file = std::fs::File::create(chunk_path)?;
            let change_count = changes.len() as u16;
            std::io::Write::write_all(&mut chunk_file, &change_count.to_le_bytes())?;
            for (local_pos, (block, state)) in changes {
                std::io::Write::write_all(
                    &mut chunk_file,
                    &[local_pos.x as u8, local_pos.y as u8, local_pos.z as u8],
                )?;
                let data = (*block, *state).save();
                std::io::Write::write_all(&mut chunk_file, data.as_slice())?;
            }
            std::io::Write::write_all(&mut chunk_file, &chunk_data)?;
        }

        log::info!("Saved chunks");

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

        log::info!("Saved entities and logged-in players");

        for cached in self.player_cache.values() {
            let player_data = cached.save();
            let hashed_username = fxhash::hash64(cached.username.as_bytes());
            let player_path = path
                .join("players")
                .join(format!("{}.bin", hashed_username));
            let mut player_file = std::fs::File::create(player_path)?;
            std::io::Write::write_all(&mut player_file, &player_data)?;
        }

        log::info!("Saved logged-off players");

        Ok(())
    }

    /// Loads a world from a folder. The folder should have the same structure as described in the
    /// `save` method.
    pub fn load(path: &std::path::Path) -> Result<Self, WorldLoadError> {
        let save_content = std::fs::read(path.join("save.bin"))
            .map_err(|_| WorldLoadError::MissingSaveFile(path.join("save.bin")))?;
        let mut save_iter = save_content.into_iter();
        match save_iter.next() {
            Some(version) if version <= 2 => load_v0_1_2(path, &mut save_iter, version),
            Some(version) => Err(WorldLoadError::InvalidSaveFormat(format!(
                "Unsupported save version: {}",
                version
            ))),
            None => Err(WorldLoadError::InvalidSaveFormat(
                "Save file is empty".to_string(),
            )),
        }
    }
}

fn load_v0_1_2(
    path: &std::path::Path,
    save_iter: &mut impl Iterator<Item = u8>,
    version: u8,
) -> Result<World, WorldLoadError> {
    // SEED
    let seed = read_i32(save_iter, "World seed")?;
    let mut noise = fastnoise_lite::FastNoiseLite::new();
    noise.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
    noise.set_seed(Some(seed));

    let mut world = World {
        chunks: fxhash::FxHashMap::default(),
        entities: fxhash::FxHashMap::default(),
        noise,
        player_cache: HashMap::new(),
        pending_changes: PendingChanges::default(),
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
        let change_count = read_u16(&mut chunk_iter, "Chunk change count")?;
        for _ in 0..change_count {
            let local_pos = read_u8vec3(&mut chunk_iter, "Chunk change local position")?.as_ivec3();
            let block_and_state = <(Block, BlockState)>::load(&mut chunk_iter, version)?;
            world
                .changes
                .entry(chunk_pos)
                .or_default()
                .insert(local_pos, block_and_state);
        }
        let chunk = Chunk::load(&mut chunk_iter, version)?;
        world.chunks.insert(chunk_pos, chunk);
    }

    // ENTITIES
    let entities_path = path.join("entities.bin");
    if !entities_path.exists() {
        return Err(WorldLoadError::MissingSaveFile(entities_path));
    }
    let entities_data = std::fs::read(entities_path).unwrap();
    let mut entities_iter = entities_data.into_iter();
    let entity_count = read_u64(&mut entities_iter, "Entity count")?;
    #[allow(clippy::never_loop)]
    #[allow(unreachable_code, unused_variables)]
    for _ in 0..entity_count {
        let entity_type = read_u8(&mut entities_iter, "Entity type")?;
        let entity_data_len = read_u32(&mut entities_iter, "Entity data length")?;
        let entity_data = take_exact(&mut entities_iter, entity_data_len as usize, "Entity data")?;
        let entity: Box<dyn Entity> = match entity_type {
            x if x == EntityType::Player as u8 => {
                return Err(WorldLoadError::InvalidSaveFormat(
                    "Player entities should be stored in the players folder".to_string(),
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
        let mut player_iter = player_data.into_iter();
        let player = PlayerEntity::load(&mut player_iter, version).map_err(|e| {
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
