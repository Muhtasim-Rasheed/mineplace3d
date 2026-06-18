use glam::IVec3;

use crate::{
    block::{BlockState, blocks},
    world::chunk::{CHUNK_SIZE, Chunk},
};

use super::Generator;

impl Generator {
    /// Generates a chunk for V01 at the given position.
    pub(super) fn generate_chunk_v01(
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
                        chunk.set_block(local, *blocks::STONE, BlockState::none());
                    } else if global_y < height - 1 {
                        chunk.set_block(local, *blocks::DIRT, BlockState::none());
                    } else if global_y < height {
                        chunk.set_block(local, *blocks::GRASS, BlockState::none());
                    }
                }
            }
        }
    }
}
