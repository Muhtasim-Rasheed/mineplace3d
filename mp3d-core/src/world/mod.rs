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
    world::chunk::{CHUNK_SIZE, Chunk},
};

/// A world consisting of multiple chunks. Each chunk contains a 16x16x16 grid of blocks.
pub struct World {
    pub chunks: HashMap<IVec3, Chunk>,
}

impl World {
    /// Creates a new empty world.
    pub fn new() -> Self {
        let mut chunks = HashMap::new();
        // Preload some chunks around the origin
        for x in -4..4 {
            for y in -1..1 {
                for z in -4..4 {
                    let chunk_pos = IVec3::new(x, y, z);
                    chunks.insert(chunk_pos, Chunk::new(chunk_pos));
                }
            }
        }
        World { chunks }
    }

    /// Gets a block at the given world position.
    pub fn get_block_at(&self, world_pos: IVec3) -> Option<&Block> {
        let chunk_pos = IVec3::new(
            world_pos.x.div_euclid(CHUNK_SIZE as i32),
            world_pos.y.div_euclid(CHUNK_SIZE as i32),
            world_pos.z.div_euclid(CHUNK_SIZE as i32),
        );
        let local_pos = IVec3::new(
            world_pos.x.rem_euclid(CHUNK_SIZE as i32),
            world_pos.y.rem_euclid(CHUNK_SIZE as i32),
            world_pos.z.rem_euclid(CHUNK_SIZE as i32),
        );

        self.chunks.get(&chunk_pos).map(|c| c.get_block(local_pos))
    }

    /// Sets a block at the given world position.
    pub fn set_block_at(&mut self, world_pos: IVec3, block: Block) {
        let chunk_pos = IVec3::new(
            world_pos.x.div_euclid(CHUNK_SIZE as i32),
            world_pos.y.div_euclid(CHUNK_SIZE as i32),
            world_pos.z.div_euclid(CHUNK_SIZE as i32),
        );
        let local_pos = IVec3::new(
            world_pos.x.rem_euclid(CHUNK_SIZE as i32),
            world_pos.y.rem_euclid(CHUNK_SIZE as i32),
            world_pos.z.rem_euclid(CHUNK_SIZE as i32),
        );

        let chunk = self.chunks.get_mut(&chunk_pos);

        if let Some(chunk) = chunk {
            chunk.set_block(local_pos, block);
        } else {
            let mut new_chunk = Chunk::new(chunk_pos);
            new_chunk.set_block(local_pos, block);
            self.chunks.insert(chunk_pos, new_chunk);
        }
    }
}
