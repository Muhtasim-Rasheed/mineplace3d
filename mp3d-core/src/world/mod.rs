//! A world consisting of multiple chunks.
//!
//! The `World` struct manages a collection of `Chunk`s, each representing a
//! 16x16x16 section of the world. It provides methods for loading, unloading,
//! and accessing chunks, as well as handling world generation and updates.

pub mod chunk;
pub mod generation;

use std::collections::HashMap;

use fxhash::{FxHashMap, FxHashSet, hash64};
use glam::{IVec3, Vec3};

use crate::{
    block::{BlockId, BlockState, block_registry, blocks},
    datapack::GameData,
    direction::Direction,
    entity::{
        EntityDetails, EntityId, MoveInput,
        components::*,
        ecs::{ECS, Scheduler},
    },
    item::{Inventory, item_registry, items},
    physics::CollisionWorld,
    protocol::{BlockUpdate, BlockUpdateKind},
    registry::LazyId,
    serialize::{
        GENERATOR_VERSION, SAVE_VERSION,
        read::{ByteReader, ReadError, ReadErrorExt, ReadErrorKind},
        write::ByteWriter,
    },
    uniquequeue::UniqueQueue,
    world::{
        chunk::{CHUNK_SIZE, Chunk},
        generation::{Generator, pool::GenerationPool},
    },
};

/// A world consisting of multiple chunks. Each chunk contains a 16x16x16 grid of blocks.
pub struct World {
    pub chunks: FxHashMap<IVec3, Chunk>,
    pub ecs: ECS,
    pub generation_pool: GenerationPool,
    pub time: u64,

    // What clients were last told
    pub(crate) replicated_snapshots: FxHashMap<EntityId, EntityDetails>,

    // Storage of player data, keyed by username. This is used to store player data when they are
    // not currently in the world. It stores the data as bytes.
    pub(crate) player_cache: HashMap<String, EntityDetails>,

    /// Stores pending changes to blocks in the world. This is used to track changes that need to
    /// be sent to players.
    pub(crate) pending_changes: PendingChanges,

    /// A map of chunk positions to a map of local block positions to the new block and block
    /// state. This is used to track changes to chunks that have been modified by the player or
    /// other entities.
    changes: FxHashMap<IVec3, FxHashMap<IVec3, (BlockId, BlockState)>>,

    game_data: GameData,
}

impl World {
    /// Creates a new empty world.
    pub fn new(seed: i32) -> Self {
        let generation_pool =
            GenerationPool::new(Generator::new(GENERATOR_VERSION, seed).unwrap(), 2);
        let chunks = FxHashMap::default();
        World {
            chunks,
            ecs: ECS::new(),
            generation_pool,
            time: 0,
            replicated_snapshots: FxHashMap::default(),
            player_cache: HashMap::new(),
            pending_changes: PendingChanges::default(),
            changes: FxHashMap::default(),
            game_data: GameData::new(),
        }
    }

    /// Gets a block at the given world position.
    pub fn get_block_at(&self, world_pos: IVec3) -> Option<(BlockId, &BlockState)> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.chunks
            .get(&chunk_pos)
            .and_then(|c| c.get_block(local_pos))
    }

    /// Sets a block at the given world position.
    ///
    /// **Urgent version**: The change is added to the urgent changes queue, which will be drained
    /// first when sending updates to players, and then cleared.
    pub fn urgent_set_block_at(
        &mut self,
        world_pos: IVec3,
        block: BlockId,
        state: BlockState,
        kind: BlockUpdateKind,
    ) {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.changes
            .entry(chunk_pos)
            .or_default()
            .insert(local_pos, (block, state));
        self.pending_changes.push(BlockUpdate {
            position: world_pos,
            block,
            block_state: state,
            urgent: true,
            kind,
        });

        // Only update the in-memory chunk if it's already loaded.
        // If it isn't, the diff above will be applied automatically
        // whenever the chunk is generated later.
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_block(local_pos, block, state);
        }
    }

    /// Sets a block at the given world position.
    ///
    /// **Normal version**: The change is added to the normal changes queue, which will be sent to
    /// players after the urgent changes, and then cleared.
    pub fn normal_set_block_at(
        &mut self,
        world_pos: IVec3,
        block: BlockId,
        state: BlockState,
        kind: BlockUpdateKind,
    ) {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.changes
            .entry(chunk_pos)
            .or_default()
            .insert(local_pos, (block, state));
        self.pending_changes.push(BlockUpdate {
            position: world_pos,
            block,
            block_state: state,
            urgent: false,
            kind,
        });

        // Only update the in-memory chunk if it's already loaded.
        // If it isn't, the diff above will be applied automatically
        // whenever the chunk is generated later.
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_block(local_pos, block, state);
        }
    }

    /// Applies block changes on a chunk.
    pub fn apply_changes_on_chunk(&self, chunk: &mut Chunk, chunk_pos: IVec3) {
        if let Some(changes) = self.changes.get(&chunk_pos) {
            for (local_pos, (block, state)) in changes {
                chunk.set_block(*local_pos, *block, *state);
            }
        }
    }

    /// Loads around specified coordinates in world space.
    pub fn load_around(&mut self, session_id: u64, pos: IVec3) {
        let cpos = pos / CHUNK_SIZE as i32;

        for dx in -1..=-1 {
            for dy in -1..=-1 {
                for dz in -1..=-1 {
                    let cpos = cpos + IVec3::new(dx, dy, dz);
                    if !self.chunks.contains_key(&cpos) {
                        self.generation_pool.request_chunk(session_id, cpos);
                    }
                }
            }
        }
    }

    /// Updates the world. The optimal TPS (Ticks Per Second) is 48.
    pub fn tick(&mut self, scheduler: &mut Scheduler, tps: u8) {
        let mut updates = Vec::new();
        for (pos, chunk) in &self.chunks {
            updates.extend_from_slice(&chunk.random_tick(5, &self.chunks, *pos));
        }
        for update in updates {
            self.normal_set_block_at(update.0, update.1, update.2, BlockUpdateKind::RandomTick);
        }

        scheduler.run(self, 1.0 / tps as f32);
        self.time += 1;
    }

    pub fn entities_in_range(&self, center: Vec3, range: f32) -> Vec<EntityId> {
        self.ecs
            .query_ro::<&Position>()
            .filter(|(_, pos)| pos.0.distance_squared(center) <= range * range)
            .map(|(entity, _)| entity)
            .collect()
    }

    fn player_bounds(ecs: &ECS, entity: EntityId) -> Option<(Vec3, f32, f32)> {
        let pos = ecs.get_component_copied::<Position>(entity)?.0;
        let hb = ecs.get_component_copied::<Hitbox>(entity)?;
        Some((pos, hb.width, hb.height))
    }

    pub fn try_place_block(
        &mut self,
        player_entity_id: EntityId,
        pos: IVec3,
        block: BlockId,
        state: BlockState,
    ) -> bool {
        let Some((player_pos, player_width, player_height)) =
            Self::player_bounds(&self.ecs, player_entity_id)
        else {
            return false;
        };

        let old_block = self
            .get_block_at(pos)
            .map(|(b, _)| b)
            .unwrap_or(*blocks::AIR);

        self.urgent_set_block_at(pos, block, state, BlockUpdateKind::Placed);

        if self.collides(player_pos, player_width, player_height) {
            self.urgent_set_block_at(pos, old_block, BlockState::none(), BlockUpdateKind::Removed);
            return false;
        }

        let Some(hotbar) = self
            .ecs
            .get_component_copied::<SelectedHotbarSlot>(player_entity_id)
        else {
            return false;
        };

        let Some(inv) = self.ecs.get_component_mut::<Inventory>(player_entity_id) else {
            return false;
        };

        let slot = inv.hotbar_slot_mut(hotbar.0);
        slot.count -= 1;
        if slot.count == 0 {
            slot.item = *items::AIR;
        }
        inv.dirty = true;

        true
    }

    /// Handles a block interaction at the given world position and face index. If the block is not
    /// interactive, this will attempt to place a block on the face that was clicked.
    pub fn block_interaction(
        &mut self,
        player_entity_id: EntityId,
        block_pos: IVec3,
        face: Direction,
    ) {
        let Some((item_count, place_block)) = self.hotbar_stack_info(player_entity_id) else {
            return;
        };

        if let Some((id, state)) = self.get_block_at(block_pos).map(|(b, s)| (b, *s)) {
            let def = block_registry().get(id).unwrap();
            if let Some(on_click) = &def.on_click {
                if on_click(id, self, player_entity_id, block_pos, state, face) {
                    return; // hook fully handled the interaction
                }
            }
        }

        let place_pos = block_pos + face;
        if item_count == 0 {
            return;
        }
        if let Some(block) = place_block {
            let def = block_registry().get(**block).unwrap();
            let state = if let Some(on_place) = &def.on_place {
                (on_place)(**block, self, player_entity_id, place_pos, face)
            } else if let Some(bs) = BlockState::default_state(def.state_type) {
                bs
            } else {
                return;
            };
            self.try_place_block(player_entity_id, place_pos, **block, state);
        }
    }

    pub fn hotbar_stack_info(
        &self,
        entity: EntityId,
    ) -> Option<(u16, Option<&'static LazyId<BlockId>>)> {
        let inv = self.ecs.get_component::<Inventory>(entity)?;
        let hotbar = self.ecs.get_component::<SelectedHotbarSlot>(entity)?;
        let stack = inv.hotbar_slot(hotbar.0);
        let assoc_block = item_registry().get(stack.item).unwrap().assoc_block;
        Some((stack.count, assoc_block))
    }

    pub fn break_block(&mut self, player_entity_id: EntityId, block_pos: IVec3) {
        let (block, state) = match self.get_block_at(block_pos) {
            Some((b, s)) => (b, *s),
            None => return,
        };

        let block_def = block_registry().get(block).unwrap();
        if let Some(on_break) = &block_def.on_break {
            on_break(block, self, player_entity_id, block_pos, state);
        }

        let Some(loot_table_entry) = self.game_data.get_block_drops(block) else {
            return;
        };
        let drops = &loot_table_entry.drops;
        let drops = drops.get(&state.data()).cloned().unwrap_or_default();

        self.urgent_set_block_at(
            block_pos,
            *blocks::AIR,
            crate::block::BlockState::none(),
            crate::protocol::BlockUpdateKind::Removed,
        );

        let Some(inv) = self.ecs.get_component_mut::<Inventory>(player_entity_id) else {
            return;
        };

        for (item, drop_entry) in drops {
            let count = if drop_entry.max == drop_entry.min {
                drop_entry.min
            } else {
                let mut rng = rand::rng();
                let roll = rand::Rng::random_range(&mut rng, 0.0..1.0);
                if roll < drop_entry.min_chance {
                    drop_entry.min
                } else if roll < drop_entry.max_chance {
                    drop_entry.max
                } else {
                    0
                }
            };

            let item = match item_registry().get_id(&item) {
                Some(i) => i,
                None => {
                    log::warn!(
                        "Unknown item '{}' in loot table for block '{}'",
                        item,
                        block_def.ident
                    );
                    continue;
                }
            };

            // TODO: implement item entities, for now just add the items directly to the player's
            // inventory
            inv.add_stack(item, count as u16);
        }
    }
}

impl CollisionWorld for World {
    fn collides(&self, pos: Vec3, width: f32, height: f32) -> bool {
        self.chunks.collides(pos, width, height)
    }
}

impl CollisionWorld for FxHashMap<IVec3, Chunk> {
    fn collides(&self, pos: Vec3, width: f32, height: f32) -> bool {
        fn get_block_at(
            this: &FxHashMap<IVec3, Chunk>,
            world_pos: IVec3,
        ) -> Option<(BlockId, &BlockState)> {
            let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
            let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

            this.get(&chunk_pos).and_then(|c| c.get_block(local_pos))
        }
        let min_block_pos = (pos - Vec3::splat(width / 2.0)).floor().as_ivec3();
        let max_block_pos = (pos + Vec3::new(width / 2.0, height, width / 2.0))
            .floor()
            .as_ivec3();

        for x in min_block_pos.x..=max_block_pos.x {
            for y in min_block_pos.y..=max_block_pos.y {
                for z in min_block_pos.z..=max_block_pos.z {
                    let block_pos = IVec3::new(x, y, z);
                    if let Some((block, block_state)) = get_block_at(self, block_pos)
                        && let Some(block) = block_registry().get(block)
                        && block.collides_with_player(
                            width,
                            height,
                            pos - block_pos.as_vec3(),
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
}

/// Position-less and priority-less version of [`BlockUpdate`]
#[derive(Clone, Debug)]
pub struct BlockChangeKey {
    pub block: BlockId,
    pub block_state: BlockState,
    pub kind: BlockUpdateKind,
}

#[derive(Debug, Default)]
pub struct PendingChanges {
    /// Also stores changes, but will be sent to players and then cleared.
    ///
    /// **Urgent version**: Changes that are added to this queue will be sent to players before the
    /// normal changes, and then cleared.
    pub urgent: UniqueQueue<IVec3>,

    /// Also stores changes, but will be sent to players and then cleared.
    ///
    /// **Normal version**: Changes that are added to this queue will be sent to players after the
    /// urgent changes, and then cleared.
    pub normal: UniqueQueue<IVec3>,

    /// Stores data for the two queues above. This makes sure that if a block is changed multiple
    /// times in a tick, only the final state is sent to the players.
    pub data: HashMap<IVec3, BlockChangeKey>,
}

impl PendingChanges {
    pub fn push(&mut self, update: BlockUpdate) {
        self.data.insert(
            update.position,
            BlockChangeKey {
                block: update.block,
                block_state: update.block_state,
                kind: update.kind,
            },
        );
        if update.urgent {
            self.urgent.push(update.position);
        } else {
            self.normal.push(update.position);
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Iterator for PendingChanges {
    type Item = BlockUpdate;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pos) = self.urgent.pop() {
            self.data.remove(&pos).map(|change| BlockUpdate {
                position: pos,
                block: change.block,
                block_state: change.block_state,
                urgent: true,
                kind: change.kind,
            })
        } else if let Some(pos) = self.normal.pop() {
            self.data.remove(&pos).map(|change| BlockUpdate {
                position: pos,
                block: change.block,
                block_state: change.block_state,
                urgent: false,
                kind: change.kind,
            })
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
    ///   - 1 byte: length of the block identifier (M)
    ///   - M bytes: block identifier (UTF-8 string)
    ///   - 4 bytes: block state data (u32)
    ///
    /// # save.bin
    /// - 1 byte: save format version (u8)
    /// - 1 byte: generator version (u8)
    /// - 4 bytes: world seed (i32)
    /// - 8 bytes: current time in ticks (u64)
    ///
    /// # entities.bin
    /// - 8 bytes: number of entities (N)
    /// - N times
    ///   - 1 byte: entity type (u8)
    ///   - 4 bytes: length of entity data (M)
    ///   - M bytes: entity data (format defined by each entity type)
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(
            path.join("save.bin"),
            ByteWriter::new()
                .u8(SAVE_VERSION)
                .save(&self.generation_pool.generator)
                .u64(self.time)
                .into_bytes(),
        )?;

        log::info!("Saved save.bin");

        std::fs::create_dir_all(path.join("chunks"))?;
        for (chunk_pos, changes) in &self.changes {
            let chunk_path = path.join("chunks").join(format!(
                "chunk_{}_{}_{}.bin",
                chunk_pos.x, chunk_pos.y, chunk_pos.z
            ));
            let mut chunk_writer = ByteWriter::new().u16(changes.len() as u16);
            for (local_pos, (block, state)) in changes {
                chunk_writer = chunk_writer
                    .u8vec3(local_pos.as_u8vec3())
                    .save(&(*block, *state));
            }
            std::fs::write(chunk_path, chunk_writer.into_bytes())?;
        }

        log::info!("Saved chunks");

        std::fs::create_dir_all(path.join("players"))?;

        let player_ids: FxHashSet<EntityId> = self
            .ecs
            .query_ro::<&Username>()
            .map(|(entity, _)| entity)
            .collect();

        let entity_count = self
            .ecs
            .e_alloc
            .iter()
            .filter(|e| !player_ids.contains(e))
            .count() as u64;
        let mut entities_writer = ByteWriter::new().u64(entity_count);

        for entity in self.ecs.e_alloc.iter() {
            let details = self.ecs.entity_details(entity);

            if let Some(username) = self.ecs.get_component::<Username>(entity) {
                let player_data = details.to_bytes();
                let hashed_username = hash64(username.0.as_bytes());
                let player_path = path
                    .join("players")
                    .join(format!("{}.bin", hashed_username));
                std::fs::write(player_path, &player_data)?;
            } else {
                let entity_data = details.to_bytes();
                let entity_data_len = entity_data.len() as u32;
                entities_writer = entities_writer.u32(entity_data_len).bytes(&entity_data);
            }
        }

        std::fs::write(path.join("entities.bin"), entities_writer.into_bytes())?;

        log::info!("Saved entities and logged-in players");

        for (username, cached) in self.player_cache.iter() {
            let player_data = cached.to_bytes();
            let hashed_username = hash64(username.as_bytes());
            let player_path = path
                .join("players")
                .join(format!("{}.bin", hashed_username));
            std::fs::write(player_path, &player_data)?;
        }

        log::info!("Saved logged-out players");

        Ok(())
    }

    /// Loads a world from a folder. The folder should have the same structure as described in the
    /// `save` method.
    pub fn load(path: &std::path::Path) -> Result<Self, ReadError> {
        let save_content = std::fs::read(path.join("save.bin"))
            .map_err(|_| ReadErrorKind::MissingFile(path.join("save.bin")))
            .ctx("loading world")?;
        let mut save_reader = ByteReader::new(&save_content);
        match save_reader.u8().ctx("expected save version")? {
            version if version <= 0x08 => load_v0_to_v8(path, &mut save_reader, version),
            version => Err(ReadErrorKind::InvalidTag(version)).ctx("save version"),
        }
    }
}

fn load_v0_to_v8(
    path: &std::path::Path,
    save_reader: &mut ByteReader,
    version: u8,
) -> Result<World, ReadError> {
    // GENERATOR
    let generator = save_reader.load(version)?;
    let generation_pool = GenerationPool::new(generator, 2);

    // TIME
    let time = if version >= 0x05 {
        save_reader.u64().ctx("world::time")?
    } else {
        0
    };

    let mut world = World {
        chunks: FxHashMap::default(),
        ecs: ECS::new(),
        generation_pool,
        time,
        replicated_snapshots: FxHashMap::default(),
        player_cache: HashMap::new(),
        pending_changes: PendingChanges::default(),
        changes: FxHashMap::default(),
        game_data: GameData::new(),
    };

    // CHUNKS
    let chunks_dir = path.join("chunks");
    if !chunks_dir.exists() {
        return Err(ReadErrorKind::MissingFile(chunks_dir)).ctx("loading chunks");
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
        let mut chunk_reader = ByteReader::new(&chunk_data);
        let change_count = chunk_reader.u16().ctx("chunk change count")?;
        for _ in 0..change_count {
            let local_pos = chunk_reader
                .u8vec3()
                .ctx("chunk change local position")?
                .as_ivec3();
            let block_and_state = chunk_reader.load(version)?;
            world
                .changes
                .entry(chunk_pos)
                .or_default()
                .insert(local_pos, block_and_state);
        }

        // In 0x06 the redundant chunk data was removed. We don't handle loading the many many bytes
        // for versions before 0x06 simply because there was nothing else after the chunk data.
    }

    // ENTITIES
    if version >= 0x08 {
        let entities_path = path.join("entities.bin");
        if !entities_path.exists() {
            return Err(ReadErrorKind::MissingFile(entities_path)).ctx("loading entities");
        }
        let entities_data = std::fs::read(entities_path).unwrap();
        let mut entities_reader = ByteReader::new(&entities_data);
        let entity_count = entities_reader.u64().ctx("entity count")?;

        for _ in 0..entity_count {
            let entity_data_len = entities_reader.u32().ctx("entity data length")? as usize;
            let entity_data = entities_reader.take(entity_data_len).ctx("entity data")?;

            let details = EntityDetails::from_bytes(&entity_data)?;

            world.ecs.spawn_from_details(&details);
        }
    }
    // versions before 0x08 predate the ECS entirely so no non-player entities existed to save, nothing to load

    let players_dir = path.join("players");
    if !players_dir.exists() {
        return Err(ReadErrorKind::MissingFile(players_dir)).ctx("loading players");
    }
    for entry in std::fs::read_dir(players_dir).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_str().unwrap();
        if !file_name_str.ends_with(".bin") {
            continue;
        }
        let player_data = std::fs::read(entry.path()).unwrap();

        let (username, details) = if version < 8 {
            let mut player_reader = ByteReader::new(&player_data);
            load_legacy_player_details(&mut player_reader, version)?
        } else {
            let details = EntityDetails::from_bytes(&player_data)?;
            let username = details
                .get::<Username>()
                .ok_or_else(|| ReadErrorKind::MissingData("username".to_string()))
                .ctx("player save missing username component")?
                .0
                .clone();
            (username, details)
        };

        world.player_cache.insert(username, details);
    }

    Ok(world)
}

fn load_legacy_player_details(
    reader: &mut ByteReader,
    version: u8,
) -> Result<(String, EntityDetails), ReadError> {
    let username_len = reader.u8().ctx("player username length")? as usize;
    let username = reader.string(username_len).ctx("player username")?;
    let position = reader.vec3().ctx("player position")?;
    let velocity = reader.vec3().ctx("player velocity")?;
    let yaw = reader.f32().ctx("player yaw")?;
    let pitch = reader.f32().ctx("player pitch")?;
    let inventory = if version < 2 {
        Inventory::new()
    } else {
        reader.load(version)?
    };
    let flying = reader.u8().ctx("player flying state")? != 0;

    let details = EntityDetails::builder()
        .with(Position(position))
        .with(Velocity(velocity))
        .with(Rotation { yaw, pitch })
        .with(OnGround(false))
        .with(Username(username.clone()))
        .with(inventory)
        .with(SelectedHotbarSlot(0))
        .with(Flying(flying))
        .with(Hitbox {
            width: 0.8,
            height: 1.8,
        })
        .with(MoveInput::default())
        .build();

    Ok((username, details))
}
