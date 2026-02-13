//! Client-side world representation.

use std::collections::HashMap;

use glam::IVec3;
use mp3d_core::{block::Block, world::chunk::CHUNK_SIZE};

use crate::client::chunk::ClientChunk;

/// Number of chunks to render around the player
const RENDER_DISTANCE: i32 = 8;

/// Client-side world representation.
///
/// This struct manages the client-side representation of the game world, including
/// chunks and other world-related data.
pub struct ClientWorld {
    /// A mapping of chunk positions to their corresponding client-side chunk data.
    pub chunks: HashMap<IVec3, ClientChunk>,
    /// Changes done to the world that haven't been sent to the server yet.
    pub pending_changes: Vec<(IVec3, Block)>,
}

impl ClientWorld {
    /// Creates a new, empty `ClientWorld`.
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            pending_changes: Vec::new(),
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
        }
        self.pending_changes.push((world_pos, block));

        // Mark neighboring chunks as dirty if the block is on the edge of the chunk
        if local_pos.x == 0 {
            if let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(-1, 0, 0))) {
                neighbor.dirty = true;
            }
        } else if local_pos.x == CHUNK_SIZE as i32 - 1
            && let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(1, 0, 0))) {
                neighbor.dirty = true;
            }

        if local_pos.y == 0 {
            if let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, -1, 0))) {
                neighbor.dirty = true;
            }
        } else if local_pos.y == CHUNK_SIZE as i32 - 1
            && let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, 1, 0))) {
                neighbor.dirty = true;
            }

        if local_pos.z == 0 {
            if let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, 0, -1))) {
                neighbor.dirty = true;
            }
        } else if local_pos.z == CHUNK_SIZE as i32 - 1
            && let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, 0, 1))) {
                neighbor.dirty = true;
            }
    }

    /// Checks if the client-side world requires more chunks, and if so returns their coordinates.
    pub fn needs_chunks(&self, pos: IVec3) -> Vec<IVec3> {
        let mut chunks = Vec::new();
        let chunk_pos = pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));

        for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for y in -RENDER_DISTANCE..=RENDER_DISTANCE {
                for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
                    let offset = IVec3::new(x, y, z);
                    let distance = offset.length_squared();
                    if distance > RENDER_DISTANCE * RENDER_DISTANCE
                        || self.chunks.contains_key(&(chunk_pos + offset))
                    {
                        continue;
                    }
                    let chunk_coord = chunk_pos + offset;
                    chunks.push(chunk_coord);
                }
            }
        }

        chunks
    }

    /// Unloads chunks that are outside the render distance.
    pub fn unload_chunks(&mut self, player_pos: IVec3) -> Vec<IVec3> {
        let chunk_pos = player_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let mut to_remove = Vec::new();

        for pos in self.chunks.keys() {
            let distance = pos.distance_squared(chunk_pos);
            if distance > RENDER_DISTANCE * RENDER_DISTANCE {
                to_remove.push(*pos);
            }
        }

        for pos in &to_remove {
            self.chunks.remove(pos);
        }

        to_remove
    }
}
