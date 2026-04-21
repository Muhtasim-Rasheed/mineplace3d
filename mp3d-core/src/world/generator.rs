//! A world generator that supports multiple versions of Mineplace3D
//!
//! This module provides a [`Generator`] struct that can be used to generate worlds for different
//! versions of Mineplace3D.

use glam::IVec3;

use crate::{
    block::{Block, BlockState},
    saving::{Saveable, io::*},
    world::chunk::{CHUNK_SIZE, Chunk},
};

pub enum Generator {
    /// Generator version 0x01. This is the first world generator for beta.
    V01 {
        seed: i32,
        noise: fastnoise_lite::FastNoiseLite,
    },
}

impl Generator {
    /// Creates a new generator with the given version and seed.
    pub fn new(version: u8, seed: i32) -> Result<Self, String> {
        match version {
            0x00 => todo!("Alpha generator not implemented yet"),
            0x01 => {
                let mut noise = fastnoise_lite::FastNoiseLite::new();
                noise.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
                noise.set_seed(Some(seed));
                Ok(Generator::V01 { seed, noise })
            }
            _ => Err(format!("Unsupported generator version: {version}")),
        }
    }

    /// Generates a chunk at the given position.
    pub fn generate_chunk(&self, chunk_pos: IVec3) -> Chunk {
        let mut chunk = Chunk::new();

        match self {
            Generator::V01 { noise, .. } => {
                Self::generate_chunk_v01(&mut chunk, noise, chunk_pos);
                chunk
            }
        }
    }

    /// Returns the version of the generator.
    pub fn version(&self) -> u8 {
        match self {
            Generator::V01 { .. } => 0x01,
        }
    }

    /// Returns the seed of the generator. A generator always has a seed even if it doesn't use it,
    /// so this function always returns a value.
    pub fn seed(&self) -> i32 {
        match self {
            Generator::V01 { seed, .. } => *seed,
        }
    }

    /// Generates a chunk for V01 at the given position.
    fn generate_chunk_v01(
        chunk: &mut Chunk,
        noise: &fastnoise_lite::FastNoiseLite,
        chunk_pos: IVec3,
    ) {
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let global_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                let global_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;

                let height = noise
                    .get_noise_2d(global_x as f32 * 5.0, global_z as f32 * 5.0)
                    .powi(2)
                    * 60.0
                    + 15.0;

                for y in 0..CHUNK_SIZE {
                    let global_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
                    let local = IVec3::new(x as i32, y as i32, z as i32);

                    if global_y < -48 {
                        continue;
                    }
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
                        chunk.set_block(local, Block::STONE, BlockState::none());
                    } else if global_y < height - 1 {
                        chunk.set_block(local, Block::DIRT, BlockState::none());
                    } else if global_y < height {
                        chunk.set_block(local, Block::GRASS, BlockState::none());
                    }
                }
            }
        }
    }
}

impl Saveable for Generator {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.version());
        data.extend(&self.seed().to_le_bytes());
        data
    }

    fn load<I: Iterator<Item = u8>>(
        data: &mut I,
        version: u8,
    ) -> Result<Self, crate::saving::WorldLoadError>
    where
        Self: Sized,
    {
        if version > 0x03 {
            let generator_version = read_u8(data, "Generator version")?;
            let seed = read_i32(data, "Generator seed")?;
            Self::new(generator_version, seed)
                .map_err(|e| crate::saving::WorldLoadError::InvalidSaveFormat(e))
        } else {
            let seed = read_i32(data, "Generator seed")?;
            Self::new(0x01, seed).map_err(|e| crate::saving::WorldLoadError::InvalidSaveFormat(e))
        }
    }
}
