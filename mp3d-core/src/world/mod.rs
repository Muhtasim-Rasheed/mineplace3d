//! A world consisting of multiple chunks.
//!
//! The `World` struct manages a collection of `Chunk`s, each representing a
//! 16x16x16 section of the world. It provides methods for loading, unloading,
//! and accessing chunks, as well as handling world generation and updates.

pub mod chunk;

use std::collections::HashMap;

use glam::IVec3;

use crate::{
    block::Block,
    entity::Entity,
    world::chunk::{CHUNK_SIZE, Chunk},
};

const PRELOAD_RADIUS: i32 = 8;

/// A world consisting of multiple chunks. Each chunk contains a 16x16x16 grid of blocks.
pub struct World {
    pub chunks: HashMap<IVec3, Chunk>,
    pub entities: HashMap<u64, Box<dyn Entity>>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Creates a new empty world.
    pub fn new() -> Self {
        let mut chunks = HashMap::new();
        // Preload some chunks around the origin
        for x in -PRELOAD_RADIUS..PRELOAD_RADIUS {
            for y in -1..1 {
                for z in -PRELOAD_RADIUS..PRELOAD_RADIUS {
                    let chunk_pos = IVec3::new(x, y, z);
                    chunks.insert(chunk_pos, Chunk::new(chunk_pos));
                }
            }
        }
        World {
            chunks,
            entities: HashMap::new(),
        }
    }

    /// Gets a block at the given world position.
    pub fn get_block_at(&self, world_pos: IVec3) -> Option<&Block> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.chunks.get(&chunk_pos).map(|c| c.get_block(local_pos))
    }

    /// Sets a block at the given world position.
    pub fn set_block_at(&mut self, world_pos: IVec3, block: Block) {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        let chunk = self.chunks.get_mut(&chunk_pos);

        if let Some(chunk) = chunk {
            chunk.set_block(local_pos, block);
        } else {
            let mut new_chunk = Chunk::new(chunk_pos);
            new_chunk.set_block(local_pos, block);
            self.chunks.insert(chunk_pos, new_chunk);
        }
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
    pub fn remove_entity(&mut self, entity_id: u64) {
        self.entities.remove(&entity_id);
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
}
