//! A 16x16x16 chunk in a voxel engine.

use glam::{IVec3, Vec3};

use crate::block::Block;

pub const CHUNK_SIZE: usize = 16;

/// A 16x16x16 chunk of blocks.
#[derive(Clone, Debug)]
pub struct Chunk {
    block_palette: Vec<Block>,
    blocks: [u16; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    /// Creates a new empty chunk with all blocks set to non-full blocks, except for blocks below
    /// y=3 which are full.
    pub fn new(chunk_pos: IVec3) -> Self {
        let mut blocks = [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let global_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
                    // use a lot of sine waves to create a wavy terrain for now
                    let height = (5.0
                        + (chunk_pos.x as f32 * CHUNK_SIZE as f32 + x as f32 / 4.0).sin() * 2.0
                        + (chunk_pos.z as f32 * CHUNK_SIZE as f32 + z as f32 / 4.0).sin() * 2.0
                        + ((chunk_pos.x as f32 * CHUNK_SIZE as f32 + x as f32 / 4.0) * 0.5).sin()
                            * 1.5
                        + ((chunk_pos.z as f32 * CHUNK_SIZE as f32 + z as f32 / 4.0) * 0.5).sin()
                            * 1.5) as i32;
                    if global_y < height as i32 - 3 {
                        blocks[x + CHUNK_SIZE * (y + CHUNK_SIZE * z)] = 3;
                    } else if global_y < height as i32 - 1 {
                        blocks[x + CHUNK_SIZE * (y + CHUNK_SIZE * z)] = 2;
                    } else if global_y < height as i32 {
                        blocks[x + CHUNK_SIZE * (y + CHUNK_SIZE * z)] = 1;
                    }
                }
            }
        }
        Chunk {
            block_palette: vec![
                Block::AIR,
                Block::GRASS,
                Block::DIRT,
                Block::STONE,
            ],
            blocks,
        }
    }

    /// Gets a reference to the block at the given local position within the chunk.
    pub fn get_block(&self, local_pos: IVec3) -> &Block {
        let index = local_pos.x as usize
            + CHUNK_SIZE * (local_pos.y as usize + CHUNK_SIZE * local_pos.z as usize);
        let palette_index = self.blocks[index] as usize;
        &self.block_palette[palette_index]
    }

    /// Sets the block at the given local position within the chunk.
    pub fn set_block(&mut self, local_pos: IVec3, block: Block) {
        let index = local_pos.x as usize
            + CHUNK_SIZE * (local_pos.y as usize + CHUNK_SIZE * local_pos.z as usize);
        if let Some(palette_index) = self.block_palette.iter().position(|b| *b == block) {
            self.blocks[index] = palette_index as u16;
        } else {
            self.block_palette.push(block);
            self.blocks[index] = (self.block_palette.len() - 1) as u16;
        }
    }
}
