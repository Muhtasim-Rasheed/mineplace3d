//! A world generator that supports multiple versions of Mineplace3D
//!
//! This module provides a [`Generator`] struct that can be used to generate worlds for different
//! versions of Mineplace3D.

use glam::IVec3;

use crate::{
    block::{BlockId, BlockState, blocks},
    saving::{Saveable, io::*},
    world::{
        chunk::{CHUNK_SIZE, Chunk},
        generation::structure::{Structure, StructureData},
    },
};

pub enum Generator {
    /// Generator version 0x01. This is the first world generator for beta.
    V01 {
        seed: i32,
        noise: fastnoise_lite::FastNoiseLite,
    },
    /// Generator version 0x02. This generator adds structures.
    V02 {
        seed: i32,
        noise1: fastnoise_lite::FastNoiseLite,
        noise2: fastnoise_lite::FastNoiseLite,
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
            0x02 => {
                let mut noise1 = fastnoise_lite::FastNoiseLite::new();
                noise1.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
                noise1.set_seed(Some(seed));
                let mut noise2 = fastnoise_lite::FastNoiseLite::new();
                noise2.set_noise_type(Some(fastnoise_lite::NoiseType::Perlin));
                noise2.set_seed(Some(seed + 1));
                Ok(Generator::V02 {
                    seed,
                    noise1,
                    noise2,
                })
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
            Generator::V02 { noise1, noise2, .. } => {
                Self::generate_chunk_v02(&mut chunk, noise1, noise2, chunk_pos);
                let structures = Self::generate_structures_around_v02(noise1, noise2, chunk_pos);
                Self::apply_structures_to_chunk(&mut chunk, chunk_pos, structures);
                chunk
            }
        }
    }

    /// Returns the version of the generator.
    pub fn version(&self) -> u8 {
        match self {
            Generator::V01 { .. } => 0x01,
            Generator::V02 { .. } => 0x02,
        }
    }

    /// Returns the seed of the generator. A generator always has a seed even if it doesn't use it,
    /// so this function always returns a value.
    pub fn seed(&self) -> i32 {
        match self {
            Generator::V01 { seed, .. } => *seed,
            Generator::V02 { seed, .. } => *seed,
        }
    }

    fn apply_structures_to_chunk(chunk: &mut Chunk, chunk_pos: IVec3, structures: Vec<Structure>) {
        for structure in structures {
            match structure.data {
                StructureData::Tree { trunk_height } => {
                    Self::place_tree_filtered(chunk, chunk_pos, structure.pos, trunk_height);
                }
            }
        }
    }

    fn place_tree_filtered(chunk: &mut Chunk, chunk_pos: IVec3, origin: IVec3, trunk_height: u8) {
        let chunk_min = chunk_pos * CHUNK_SIZE as i32;
        let chunk_max = chunk_min + IVec3::splat(CHUNK_SIZE as i32);

        let mut try_place = |pos: IVec3, block: BlockId| {
            if pos.x >= chunk_min.x
                && pos.x < chunk_max.x
                && pos.y >= chunk_min.y
                && pos.y < chunk_max.y
                && pos.z >= chunk_min.z
                && pos.z < chunk_max.z
            {
                let local = pos - chunk_min;
                chunk.set_block(local, block, BlockState::none());
            }
        };

        // Leaves
        let top = origin + IVec3::new(0, trunk_height as i32, 0);

        for dx in -2..=2 {
            for dy in -2..0 {
                for dz in -2..=2 {
                    let p = top + IVec3::new(dx, dy, dz);
                    try_place(p, *blocks::LEAVES);
                }
            }
        }
        for dx in -1..=1 {
            for dy in 0..2 {
                for dz in -1..=1 {
                    let p = top + IVec3::new(dx, dy, dz);
                    try_place(p, *blocks::LEAVES);
                }
            }
        }

        // Trunk
        for i in 0..trunk_height {
            let p = origin + IVec3::new(0, i as i32, 0);
            try_place(p, *blocks::LOG);
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
        if version >= 0x03 {
            let generator_version = read_u8(data, "Generator version")?;
            let seed = read_i32(data, "Generator seed")?;
            Self::new(generator_version, seed)
                .map_err(crate::saving::WorldLoadError::InvalidSaveFormat)
        } else {
            let seed = read_i32(data, "Generator seed")?;
            Self::new(0x01, seed).map_err(crate::saving::WorldLoadError::InvalidSaveFormat)
        }
    }
}
