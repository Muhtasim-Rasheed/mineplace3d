//! A 16x16x16 chunk in a voxel engine.

use glam::IVec3;

use crate::{block::Block, world::WorldLoadError};

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
    pub fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.block_palette.len() as u8);
        for block in &self.block_palette {
            let ident_len = block.ident.len() as u8;
            data.push(block.visible as u8);
            data.push(ident_len);
            data.extend(block.ident.as_bytes());
            data.extend(&[block.collision_shape as u8]);
            data.extend(&block.state_type.to_le_bytes());
        }
        for block_index in &self.blocks {
            data.extend(&block_index.to_le_bytes());
        }
        data
    }

    /// Loads a chunk from the given data.
    pub fn load<I: Iterator<Item = u8>>(version: u8, data_iter: &mut I) -> Result<Self, WorldLoadError> {
        match version {
            0 => load_v0(data_iter),
            1 => load_v1(data_iter),
            _ => {
                return Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Unsupported save version: {}",
                    version
                )));
            }
        }
    }
}

fn load_v0(data_iter: &mut impl Iterator<Item = u8>) -> Result<Chunk, WorldLoadError> {
    let palette_len = super::take_exact(1, data_iter)?[0] as usize;
    let mut block_palette = Vec::with_capacity(palette_len);
    for _ in 0..palette_len {
        let visible = super::take_exact(1, data_iter)?[0] != 0;
        let block_id_len = data_iter.next().unwrap() as usize;
        let block_id_bytes = super::take_exact(block_id_len, data_iter)?;
        let block_id = String::from_utf8(block_id_bytes).unwrap();
        let Some(ident) = crate::block::get_block_ident(&block_id) else {
            return Err(WorldLoadError::InvalidSaveFormat(format!(
                "Unknown block identifier: {}",
                block_id
            )));
        };
        let collision_shape = super::take_exact(1, data_iter)?[0];
        let collision_shape = match collision_shape {
            0 => crate::block::CollisionShape::None,
            1 => crate::block::CollisionShape::FullBlock,
            2 => crate::block::CollisionShape::Slab,
            _ => {
                return Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Invalid collision shape: {}",
                    collision_shape
                )))
            }
        };
        let Some(ident) = crate::block::get_block_ident(&ident) else {
            return Err(WorldLoadError::InvalidSaveFormat(format!(
                "Unknown block identifier: {}",
                ident
            )));
        };
        block_palette.push(Block {
            visible,
            ident,
            collision_shape,
            state_type: crate::block::BlockState::none().state_type(),
        });
    }
    let mut blocks = [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
    for block in &mut blocks {
        let block_bytes = super::take_exact(2, data_iter)?;
        *block = u16::from_le_bytes([block_bytes[0], block_bytes[1]]);
    }
    Ok(Chunk {
        block_palette,
        blocks,
    })
}

fn load_v1(data_iter: &mut impl Iterator<Item = u8>) -> Result<Chunk, WorldLoadError> {
    let palette_len = super::take_exact(1, data_iter)?[0] as usize;
    let mut block_palette = Vec::with_capacity(palette_len);
    for _ in 0..palette_len {
        let visible = super::take_exact(1, data_iter)?[0] != 0;
        let block_id_len = data_iter.next().unwrap() as usize;
        let block_id_bytes = super::take_exact(block_id_len, data_iter)?;
        let block_id = String::from_utf8(block_id_bytes).unwrap();
        let Some(ident) = crate::block::get_block_ident(&block_id) else {
            return Err(WorldLoadError::InvalidSaveFormat(format!(
                "Unknown block identifier: {}",
                block_id
            )));
        };
        let collision_shape = super::take_exact(1, data_iter)?[0];
        let collision_shape = match collision_shape {
            0 => crate::block::CollisionShape::None,
            1 => crate::block::CollisionShape::FullBlock,
            2 => crate::block::CollisionShape::Slab,
            _ => {
                return Err(WorldLoadError::InvalidSaveFormat(format!(
                    "Invalid collision shape: {}",
                    collision_shape
                )))
            }
        };
        let state_type_bytes = super::take_exact(2, data_iter)?;
        let state_type = u16::from_le_bytes([state_type_bytes[0], state_type_bytes[1]]);
        block_palette.push(Block {
            visible,
            ident,
            collision_shape,
            state_type,
        });
    }
    let mut blocks = [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
    for block in &mut blocks {
        let block_bytes = super::take_exact(2, data_iter)?;
        *block = u16::from_le_bytes([block_bytes[0], block_bytes[1]]);
    }
    Ok(Chunk {
        block_palette,
        blocks,
    })
}
