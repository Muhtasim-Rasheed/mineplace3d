use glam::IVec3;

use crate::{
    block::{Block, BlockState},
    world::{
        chunk::{CHUNK_SIZE, Chunk},
        generation::structure::{Structure, StructureData},
    },
};

use super::Generator;

impl Generator {
    /// Gets the height of the terrain at the given global position.
    pub(super) fn get_height_v02(
        noise: &fastnoise_lite::FastNoiseLite,
        global_x: i32,
        global_z: i32,
    ) -> f32 {
        noise
            .get_noise_2d(global_x as f32 * 5.0, global_z as f32 * 5.0)
            .powi(2)
            * 60.0
            + 15.0
    }

    /// Generates a chunk (with only terrain) for V02 at the given position.
    pub(super) fn generate_chunk_v02(
        chunk: &mut Chunk,
        noise1: &fastnoise_lite::FastNoiseLite,
        noise2: &fastnoise_lite::FastNoiseLite,
        chunk_pos: IVec3,
    ) {
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let global_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                let global_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;

                let height = noise1
                    .get_noise_2d(global_x as f32 * 5.0, global_z as f32 * 5.0)
                    .powi(2)
                    * 60.0
                    + 15.0;

                let should_spawn_short_grass = noise2.get_noise_2d(
                    global_x as f32 * 45.0 + 100.0,
                    global_z as f32 * 45.0 + 100.0,
                ) > 0.4;

                for y in 0..CHUNK_SIZE {
                    let global_y = chunk_pos.y * CHUNK_SIZE as i32 + y as i32;
                    let local = IVec3::new(x as i32, y as i32, z as i32);

                    if global_y < -48 {
                        continue;
                    }
                    let is_cave = noise1.get_noise_3d(
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
                    } else if global_y == height && should_spawn_short_grass {
                        chunk.set_block(local, Block::SHORT_GRASS, BlockState::none());
                    }
                }
            }
        }
    }

    pub(super) fn generate_structures_around_v02(
        noise1: &fastnoise_lite::FastNoiseLite,
        noise2: &fastnoise_lite::FastNoiseLite,
        center_chunk: IVec3,
    ) -> Vec<Structure> {
        let mut structures = Vec::new();

        for cx in -1..=1 {
            for cy in -1..=1 {
                for cz in -1..=1 {
                    let neighbor_chunk = center_chunk + IVec3::new(cx, cy, cz);
                    structures.extend(Self::generate_structures_in_chunk_v02(
                        noise1,
                        noise2,
                        neighbor_chunk,
                    ));
                }
            }
        }

        structures
    }

    fn generate_structures_in_chunk_v02(
        noise1: &fastnoise_lite::FastNoiseLite,
        noise2: &fastnoise_lite::FastNoiseLite,
        chunk_pos: IVec3,
    ) -> Vec<Structure> {
        let mut structures = Vec::new();

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let global_x = chunk_pos.x * CHUNK_SIZE as i32 + x as i32;
                let global_z = chunk_pos.z * CHUNK_SIZE as i32 + z as i32;

                let n = noise2.get_noise_2d(global_x as f32 * 45.0, global_z as f32 * 45.0);

                if n > 0.7 {
                    let height = Self::get_height_v02(noise1, global_x, global_z) as i32;

                    structures.push(Structure {
                        data: StructureData::Tree {
                            trunk_height: 4 + ((n * 3.0) as u8),
                        },
                        pos: IVec3::new(global_x, height, global_z),
                    });
                }
            }
        }

        structures
    }
}
