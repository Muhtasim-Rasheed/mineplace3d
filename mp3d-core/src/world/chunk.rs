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
    /// Creates a new empty chunk.
    pub fn new() -> Self {
        Chunk {
            block_palette: vec![Block::AIR],
            blocks: [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
            block_states: [BlockState::none(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
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
        &self,
        n: usize,
        chunks: &fxhash::FxHashMap<IVec3, Chunk>,
        chunk_pos: IVec3,
    ) -> Vec<(IVec3, Block, BlockState)> {
        let neighbors = [
            IVec3::new(0, 0, -1),
            IVec3::new(0, 0, 1),
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(0, -1, 0),
        ]
        .map(|dir| chunks.get(&(chunk_pos + dir)));

        fn get_block_global<'a>(
            me: &'a Chunk,
            neighbors: [Option<&'a Chunk>; 6],
            global_pos: IVec3,
            chunk_pos: IVec3,
        ) -> Option<(&'a Block, &'a BlockState)> {
            let get_chunk_pos = IVec3::new(
                global_pos.x.div_euclid(CHUNK_SIZE as i32),
                global_pos.y.div_euclid(CHUNK_SIZE as i32),
                global_pos.z.div_euclid(CHUNK_SIZE as i32),
            ) - chunk_pos;
            let local_pos = IVec3::new(
                global_pos.x.rem_euclid(CHUNK_SIZE as i32),
                global_pos.y.rem_euclid(CHUNK_SIZE as i32),
                global_pos.z.rem_euclid(CHUNK_SIZE as i32),
            );
            match get_chunk_pos {
                IVec3 { z: -1, .. } => neighbors[0]?.get_block(local_pos),
                IVec3 { z: 1, .. } => neighbors[1]?.get_block(local_pos),
                IVec3 { x: 1, .. } => neighbors[2]?.get_block(local_pos),
                IVec3 { x: -1, .. } => neighbors[3]?.get_block(local_pos),
                IVec3 { y: 1, .. } => neighbors[4]?.get_block(local_pos),
                IVec3 { y: -1, .. } => neighbors[5]?.get_block(local_pos),
                IVec3 { x: 0, y: 0, z: 0 } => me.get_block(local_pos),
                _ => None,
            }
        }

        let mut updates = Vec::new();
        for _ in 0..n {
            let x = rand::random::<u8>() as usize % CHUNK_SIZE;
            let y = rand::random::<u8>() as usize % CHUNK_SIZE;
            let z = rand::random::<u8>() as usize % CHUNK_SIZE;
            let global_pos = IVec3::new(
                chunk_pos.x * CHUNK_SIZE as i32 + x as i32,
                chunk_pos.y * CHUNK_SIZE as i32 + y as i32,
                chunk_pos.z * CHUNK_SIZE as i32 + z as i32,
            );
            let index = x + CHUNK_SIZE * (y + CHUNK_SIZE * z);
            let palette_index = self.blocks[index] as usize;
            let block = &self.block_palette[palette_index];
            let above_global_pos = global_pos + IVec3::new(0, 1, 0);
            let above_block = get_block_global(self, neighbors, above_global_pos, chunk_pos);
            if block == &Block::DIRT
                && let Some((above_block, _)) = above_block
                && !above_block.visible
            {
                updates.push((global_pos, Block::GRASS, BlockState::none()));
            }
            if block == &Block::GRASS
                && let Some((above_block, _)) = above_block
                && above_block.visible
            {
                updates.push((global_pos, Block::DIRT, BlockState::none()));
            }
        }
        updates
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
