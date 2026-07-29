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
    saving::{GENERATOR_VERSION, SAVE_VERSION, Saveable, WorldLoadError, io::*},
    uniquequeue::UniqueQueue,
    world::{
        chunk::{CHUNK_SIZE, Chunk},
        generation::Generator,
    },
};

/// A world consisting of multiple chunks. Each chunk contains a 16x16x16 grid of blocks.
pub struct World {
    pub chunks: FxHashMap<IVec3, Chunk>,
    pub ecs: ECS,
    pub generator: Generator,
    pub time: u64,

    // Storage of player data, keyed by username. This is used to store player data when they are
    // not currently in the world. It stores the data as bytes.
    pub(super) player_cache: HashMap<String, EntityDetails>,

    /// Stores pending changes to blocks in the world. This is used to track changes that need to
    /// be sent to players.
    pub(super) pending_changes: PendingChanges,

    /// A map of chunk positions to a map of local block positions to the new block and block
    /// state. This is used to track changes to chunks that have been modified by the player or
    /// other entities.
    changes: FxHashMap<IVec3, FxHashMap<IVec3, (BlockId, BlockState)>>,

    game_data: GameData,
}

impl World {
    /// Creates a new empty world.
    pub fn new(seed: i32) -> Self {
        let generator = Generator::new(GENERATOR_VERSION, seed).unwrap();
        let chunks = FxHashMap::default();
        World {
            chunks,
            ecs: ECS::new(),
            generator,
            time: 0,
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

    /// Gets a block at the given world position, or generates a new chunk and returns the block if
    /// it doesn't exist.
    pub fn get_block_or_new(&mut self, world_pos: IVec3) -> Option<(BlockId, &BlockState)> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.get_chunk_or_new(chunk_pos).get_block(local_pos)
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
        let chunk = self.get_chunk_mut_or_new(chunk_pos);
        chunk.set_block(local_pos, block, state);
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
        let chunk = self.get_chunk_mut_or_new(chunk_pos);
        chunk.set_block(local_pos, block, state);
    }

    /// Creates a new chunk at the specified coordinates in chunk space, applying all changes done
    /// to the chunk. Note that this function doesnt automatically insert the new chunk into the
    /// world.
    pub fn load_chunk(
        generator: &Generator,
        changes: &FxHashMap<IVec3, FxHashMap<IVec3, (BlockId, BlockState)>>,
        chunk_pos: IVec3,
    ) -> Chunk {
        let mut chunk = generator.generate_chunk(chunk_pos);
        if let Some(changes) = changes.get(&chunk_pos) {
            for (local_pos, (block, state)) in changes {
                chunk.set_block(*local_pos, *block, *state);
            }
        }
        chunk
    }

    /// Gets a reference to a chunk at the given chunk position, or loads it if it doesn't exist.
    pub fn get_chunk_or_new(&mut self, chunk_pos: IVec3) -> &Chunk {
        self.get_chunk_mut_or_new(chunk_pos)
    }

    /// Gets a mutable reference to a chunk at the given chunk position, or loads it if it doesn't
    /// exist.
    pub fn get_chunk_mut_or_new(&mut self, chunk_pos: IVec3) -> &mut Chunk {
        self.chunks
            .entry(chunk_pos)
            .or_insert_with(|| Self::load_chunk(&self.generator, &self.changes, chunk_pos))
    }

    /// Loads around specified coordinates in world space.
    pub fn load_around(&mut self, pos: IVec3) {
        let cpos = pos / CHUNK_SIZE as i32;

        for dx in -1..=-1 {
            for dy in -1..=-1 {
                for dz in -1..=-1 {
                    let cpos = cpos + IVec3::new(dx, dy, dz);
                    self.get_chunk_or_new(cpos);
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
        let mut save_file = std::fs::File::create(path.join("save.bin"))?;
        std::io::Write::write_all(&mut save_file, &[SAVE_VERSION])?;
        std::io::Write::write_all(&mut save_file, &self.generator.save())?;
        std::io::Write::write_all(&mut save_file, &self.time.to_le_bytes())?;

        log::info!("Saved save.bin");

        std::fs::create_dir_all(path.join("chunks"))?;
        for (chunk_pos, changes) in &self.changes {
            let chunk_path = path.join("chunks").join(format!(
                "chunk_{}_{}_{}.bin",
                chunk_pos.x, chunk_pos.y, chunk_pos.z
            ));
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
        }

        log::info!("Saved chunks");

        let mut entities_file = std::fs::File::create(path.join("entities.bin"))?;
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
        std::io::Write::write_all(&mut entities_file, &entity_count.to_le_bytes())?;

        for entity in self.ecs.e_alloc.iter() {
            let details = self.ecs.entity_details(entity);

            if let Some(username) = self.ecs.get_component::<Username>(entity) {
                let player_data = details.to_bytes();
                let hashed_username = hash64(username.0.as_bytes());
                let player_path = path
                    .join("players")
                    .join(format!("{}.bin", hashed_username));
                let mut player_file = std::fs::File::create(player_path)?;
                std::io::Write::write_all(&mut player_file, &player_data)?;
            } else {
                let entity_data = details.to_bytes();
                let entity_data_len = entity_data.len() as u32;
                std::io::Write::write_all(&mut entities_file, &entity_data_len.to_le_bytes())?;
                std::io::Write::write_all(&mut entities_file, &entity_data)?;
            }
        }

        log::info!("Saved entities and logged-in players");

        for (username, cached) in self.player_cache.iter() {
            let player_data = cached.to_bytes();
            let hashed_username = hash64(username.as_bytes());
            let player_path = path
                .join("players")
                .join(format!("{}.bin", hashed_username));
            let mut player_file = std::fs::File::create(player_path)?;
            std::io::Write::write_all(&mut player_file, &player_data)?;
        }

        log::info!("Saved logged-out players");

        Ok(())
    }

    /// Loads a world from a folder. The folder should have the same structure as described in the
    /// `save` method.
    pub fn load(path: &std::path::Path) -> Result<Self, WorldLoadError> {
        let save_content = std::fs::read(path.join("save.bin"))
            .map_err(|_| WorldLoadError::MissingSaveFile(path.join("save.bin")))?;
        let mut save_iter = save_content.into_iter();
        match save_iter.next() {
            Some(version) if version <= 0x08 => load_v0_to_v8(path, &mut save_iter, version),
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

fn load_v0_to_v8(
    path: &std::path::Path,
    save_iter: &mut impl Iterator<Item = u8>,
    version: u8,
) -> Result<World, WorldLoadError> {
    // GENERATOR
    let generator = Generator::load(save_iter, version).map_err(|e| {
        WorldLoadError::InvalidSaveFormat(format!("Failed to load generator: {}", e))
    })?;

    // TIME
    let time = if version >= 0x05 {
        read_u64(save_iter, "World::time")
            .map_err(|e| WorldLoadError::InvalidSaveFormat(format!("Failed to load time: {}", e)))?
    } else {
        0
    };

    let mut world = World {
        chunks: FxHashMap::default(),
        ecs: ECS::new(),
        generator,
        time,
        player_cache: HashMap::new(),
        pending_changes: PendingChanges::default(),
        changes: FxHashMap::default(),
        game_data: GameData::new(),
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
            let block_and_state = <(BlockId, BlockState)>::load(&mut chunk_iter, version)?;
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
            return Err(WorldLoadError::MissingSaveFile(entities_path));
        }
        let entities_data = std::fs::read(entities_path).unwrap();
        let mut entities_iter = entities_data.into_iter();
        let entity_count = read_u64(&mut entities_iter, "Entity count")?;

        for _ in 0..entity_count {
            let entity_data_len = read_u32(&mut entities_iter, "Entity data length")?;
            let entity_data =
                take_exact(&mut entities_iter, entity_data_len as usize, "Entity data")?;

            let details = EntityDetails::from_bytes(&entity_data).map_err(|e| {
                WorldLoadError::InvalidSaveFormat(format!("Failed to load entity: {e}"))
            })?;

            world.ecs.spawn_from_details(&details);
        }
    }
    // versions before 0x08 predate the ECS entirely so no non-player entities existed to save, nothing to load

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

        let (username, details) = if version < 8 {
            let mut player_iter = player_data.into_iter();
            load_legacy_player_details(&mut player_iter, version).map_err(|e| {
                WorldLoadError::InvalidSaveFormat(format!(
                    "Failed to load player data from {}: {}",
                    entry.path().display(),
                    e
                ))
            })?
        } else {
            let details = EntityDetails::from_bytes(&player_data).map_err(|e| {
                WorldLoadError::InvalidSaveFormat(format!(
                    "Failed to load player data from {}: {}",
                    entry.path().display(),
                    e
                ))
            })?;
            let username = details
                .get::<Username>() // see note below — needs a typed accessor on EntityDetails
                .ok_or_else(|| {
                    WorldLoadError::InvalidSaveFormat(format!(
                        "Player save {} missing Username component",
                        entry.path().display()
                    ))
                })?
                .0
                .clone();
            (username, details)
        };

        world.player_cache.insert(username, details);
    }

    Ok(world)
}

fn load_legacy_player_details(
    data: &mut impl Iterator<Item = u8>,
    version: u8,
) -> Result<(String, EntityDetails), WorldLoadError> {
    let username_len = read_u8(data, "Player username length")? as usize;
    let username = read_string(data, username_len, "Player username")?;
    let position = read_vec3(data, "Player position")?;
    let velocity = read_vec3(data, "Player velocity")?;
    let yaw = read_f32(data, "Player yaw")?;
    let pitch = read_f32(data, "Player pitch")?;
    let inventory = if version < 2 {
        Inventory::new()
    } else {
        Inventory::load(data, version)?
    };
    let flying = read_u8(data, "Player flying state")? != 0;

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
