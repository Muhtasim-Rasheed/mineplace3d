//! A 16x16x16 chunk in a voxel engine.

use glam::IVec3;

use crate::block::Block;

pub const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    pub blocks: [[[Block; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}

impl Chunk {
    /// Creates a new empty chunk with all blocks set to non-full.
    pub fn new(chunk_pos: IVec3) -> Self {
        let mut blocks = [[[Block { full: false }; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let global_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
                    if global_y < 0 {
                        blocks[x][y][z].full = true;
                    }
                }
            }
        }
        Chunk { blocks }
    }
}
