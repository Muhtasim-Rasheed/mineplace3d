//! A 16x16x16 chunk in a voxel engine.

use std::io::Write;

use glam::IVec3;

use crate::block::Block;

pub const CHUNK_SIZE: usize = 16;

/// A 16x16x16 chunk of blocks.
#[derive(Clone, Debug)]
pub struct Chunk {
    block_palette: Vec<Block>,
    blocks: [u16; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    /// Creates a new chunk.
    pub fn new(chunk_pos: IVec3, noise: &fastnoise_lite::FastNoiseLite) -> Self {
        let mut blocks = [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let global_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                    let global_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
                    let global_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;
                    let height = noise
                        .get_noise_2d(global_x as f32 * 5.0, global_z as f32 * 5.0)
                        .powi(2)
                        * 60.0
                        + 15.0;
                    let is_cave = noise.get_noise_3d(
                        global_x as f32 * 10.0,
                        global_y as f32 * 10.0,
                        global_z as f32 * 10.0,
                    ) > 0.4;
                    let height = height as i32;
                    if is_cave {
                        continue;
                    }
                    if global_y < height - 3 {
                        blocks[x + CHUNK_SIZE * (y + CHUNK_SIZE * z)] = 3;
                    } else if global_y < height - 1 {
                        blocks[x + CHUNK_SIZE * (y + CHUNK_SIZE * z)] = 2;
                    } else if global_y < height {
                        blocks[x + CHUNK_SIZE * (y + CHUNK_SIZE * z)] = 1;
                    }
                }
            }
        }
        Chunk {
            block_palette: vec![Block::AIR, Block::GRASS, Block::DIRT, Block::STONE],
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

impl Chunk {
    /// Saves the chunk to a file.
    ///
    /// The file format is as follows:
    /// - 1 byte: number of blocks in the palette (N)
    /// - N * 13 bytes: block data (1 byte for block shape, 12 bytes for block color)
    /// - 4096 * 2 bytes: block indices (u16) for each block in the chunk
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(&[self.block_palette.len() as u8])?;
        for block in &self.block_palette {
            file.write_all(&[block.full as u8])?;
            file.write_all(
                &block
                    .color
                    .to_array()
                    .iter()
                    .flat_map(|c| c.to_le_bytes())
                    .collect::<Vec<u8>>(),
            )?;
        }
        for block_index in &self.blocks {
            file.write_all(&block_index.to_le_bytes())?;
        }
        Ok(())
    }
}
