//! Client-side world representation.

use std::collections::{HashMap, HashSet};

use glam::{IVec3, Vec3};
use mp3d_core::{
    block::{Block, BlockState},
    world::chunk::CHUNK_SIZE,
};

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
    pub pending_changes: Vec<(IVec3, (Block, BlockState))>,
    /// Queue of chunks that need to be remeshed.
    pub remesh_queue: HashSet<IVec3>,
}

impl ClientWorld {
    /// Creates a new, empty `ClientWorld`.
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            pending_changes: Vec::new(),
            remesh_queue: HashSet::new(),
        }
    }

    /// Gets a block at the given world position.
    pub fn get_block_at(&self, world_pos: IVec3) -> Option<(&Block, &BlockState)> {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        self.chunks.get(&chunk_pos).map(|c| c.get_block(local_pos))
    }

    /// Sets a block at the given world position.
    pub fn set_block_at(&mut self, world_pos: IVec3, block: Block, state: BlockState) {
        let chunk_pos = world_pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let local_pos = world_pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

        let chunk = self.chunks.get_mut(&chunk_pos);

        if let Some(chunk) = chunk {
            chunk.set_block(local_pos, block, state);
            chunk.dirty = true;
            self.remesh_queue.insert(chunk_pos);
        }
        self.pending_changes.push((world_pos, (block, state)));

        // Mark neighboring chunks as dirty if the block is on the edge of the chunk
        if local_pos.x == 0 {
            if let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(-1, 0, 0))) {
                neighbor.dirty = true;
                self.remesh_queue.insert(chunk_pos + IVec3::new(-1, 0, 0));
            }
        } else if local_pos.x == CHUNK_SIZE as i32 - 1
            && let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(1, 0, 0)))
        {
            neighbor.dirty = true;
            self.remesh_queue.insert(chunk_pos + IVec3::new(1, 0, 0));
        }

        if local_pos.y == 0 {
            if let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, -1, 0))) {
                neighbor.dirty = true;
                self.remesh_queue.insert(chunk_pos + IVec3::new(0, -1, 0));
            }
        } else if local_pos.y == CHUNK_SIZE as i32 - 1
            && let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, 1, 0)))
        {
            neighbor.dirty = true;
            self.remesh_queue.insert(chunk_pos + IVec3::new(0, 1, 0));
        }

        if local_pos.z == 0 {
            if let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, 0, -1))) {
                neighbor.dirty = true;
                self.remesh_queue.insert(chunk_pos + IVec3::new(0, 0, -1));
            }
        } else if local_pos.z == CHUNK_SIZE as i32 - 1
            && let Some(neighbor) = self.chunks.get_mut(&(chunk_pos + IVec3::new(0, 0, 1)))
        {
            neighbor.dirty = true;
            self.remesh_queue.insert(chunk_pos + IVec3::new(0, 0, 1));
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
            self.remesh_queue.remove(pos);
        }

        to_remove
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
}
