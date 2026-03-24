//! A 16x16x16 chunk in a voxel engine.

use glam::IVec3;

use crate::{
    block::{Block, BlockState},
    saving::{Saveable, WorldLoadError, io::*},
};

pub const CHUNK_SIZE: usize = 16;

/// A 16x16x16 chunk of blocks.
#[derive(Clone, Debug)]
pub struct Chunk {
    block_palette: Vec<Block>,
    blocks: [u16; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    block_states: [BlockState; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    /// Creates a new chunk.
    pub fn new(chunk_pos: IVec3, noise: &fastnoise_lite::FastNoiseLite) -> Self {
        let mut blocks = [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        #[allow(unused_mut)]
        let mut block_states = [BlockState::none(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let global_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                    let global_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
                    let global_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;

                    if global_y < -48 {
                        blocks[x + CHUNK_SIZE * (y + CHUNK_SIZE * z)] = 0;
                        continue;
                    }

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
            block_states,
        }
    }

    /// Gets a reference to the block and block state at the given local position within the chunk.
    pub fn get_block(&self, local_pos: IVec3) -> Option<(&Block, &BlockState)> {
        let index = local_pos.x as usize
            + CHUNK_SIZE * (local_pos.y as usize + CHUNK_SIZE * local_pos.z as usize);
        let palette_index = *self.blocks.get(index)? as usize;
        Some((
            self.block_palette.get(palette_index)?,
            self.block_states.get(index)?,
        ))
    }

    /// Sets the block at the given local position within the chunk.
    pub fn set_block(&mut self, local_pos: IVec3, block: Block, state: BlockState) {
        let index = local_pos.x as usize
            + CHUNK_SIZE * (local_pos.y as usize + CHUNK_SIZE * local_pos.z as usize);
        if let Some(palette_index) = self.block_palette.iter().position(|b| *b == block) {
            self.blocks[index] = palette_index as u16;
        } else {
            self.block_palette.push(block);
            self.blocks[index] = (self.block_palette.len() - 1) as u16;
        }
        self.block_states[index] = state;
    }

    /// Random ticks N random blocks in the chunk.
    pub fn random_tick(
        &mut self,
        n: usize,
        changes: &mut std::collections::HashMap<
            IVec3,
            std::collections::HashMap<IVec3, (Block, BlockState)>,
        >,
        pending_changes: &mut Vec<(IVec3, IVec3, Block, BlockState)>,
        chunk_pos: IVec3,
    ) {
        for _ in 0..n {
            let x = rand::random::<u8>() as usize % CHUNK_SIZE;
            let y = rand::random::<u8>() as usize % CHUNK_SIZE;
            let z = rand::random::<u8>() as usize % CHUNK_SIZE;
            let index = x + CHUNK_SIZE * (y + CHUNK_SIZE * z);
            let palette_index = self.blocks[index] as usize;
            let block = &self.block_palette[palette_index];
            if block == &Block::DIRT
                && let Some((above_block, _)) =
                    self.get_block(IVec3::new(x as i32, y as i32 + 1, z as i32))
                && above_block == &Block::AIR
            {
                self.set_block(
                    IVec3::new(x as i32, y as i32, z as i32),
                    Block::GRASS,
                    BlockState::none(),
                );
                changes.entry(chunk_pos).or_default().insert(
                    IVec3::new(x as i32, y as i32, z as i32),
                    (Block::GRASS, BlockState::none()),
                );
                pending_changes.push((
                    chunk_pos,
                    IVec3::new(x as i32, y as i32, z as i32),
                    Block::GRASS,
                    BlockState::none(),
                ));
            }
        }
    }
}

impl Saveable for Chunk {
    /// Serialises the chunk.
    ///
    /// The chunk format is as follows:
    /// - 1 byte: number of blocks in the palette (N)
    /// - N times
    ///   - 1 byte: whether the block is visible (0 or 1)
    ///   - 1 byte: length of the block identifier (M)
    ///   - M bytes: block identifier (UTF-8 string)
    ///   - 1 byte: collision shape
    ///   - 2 bytes: block state type (u16)
    /// - 4096 * 2 bytes: block indices (u16) for each block in the chunk
    /// - 4096 * 4 bytes: block states (u32) for each block in the chunk
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.block_palette.len() as u8);
        for block in &self.block_palette {
            let block_data = block.save();
            data.extend_from_slice(&block_data);
        }
        for block_index in &self.blocks {
            data.extend_from_slice(&block_index.to_le_bytes());
        }
        for block_state in &self.block_states {
            data.extend_from_slice(&block_state.save());
        }
        data
    }

    /// Loads a chunk from the given data.
    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError> {
        let palette_len = read_u8(data, "Chunk palette length")? as usize;
        let mut block_palette = Vec::with_capacity(palette_len);
        for _ in 0..palette_len {
            let block = Block::load(data, version)?;
            block_palette.push(block);
        }
        let mut blocks = [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        for block in &mut blocks {
            *block = read_u16(data, "Chunk blocks")?;
        }
        let mut block_states = [BlockState::none(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        // Even though BlockState::load returns BlockState::none() and doesn't consume any data for
        // version 0, this makes it faster to load version 0 chunks since we don't have to read any
        // data for block states.
        if version > 0 {
            for block_state in &mut block_states {
                *block_state = BlockState::load(data, version)?;
            }
        }
        Ok(Chunk {
            block_palette,
            blocks,
            block_states,
        })
    }
}
