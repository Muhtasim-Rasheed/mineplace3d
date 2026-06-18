//! A 16x16x16 chunk in a voxel engine.

use glam::IVec3;

use crate::{
    block::{BlockId, BlockState, CollisionShape, block_registry, blocks},
    direction::Direction,
};

pub const CHUNK_SIZE: usize = 16;

/// A 16x16x16 chunk of blocks.
#[derive(Clone, Debug)]
pub struct Chunk {
    block_palette: Vec<BlockId>,
    blocks: [u16; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    block_states: [BlockState; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

impl Chunk {
    /// Creates a new empty chunk.
    pub fn new() -> Self {
        Chunk {
            block_palette: vec![*blocks::AIR],
            blocks: [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
            block_states: [BlockState::none(); CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    /// Gets a reference to the block and block state at the given local position within the chunk.
    pub fn get_block(&self, local_pos: IVec3) -> Option<(BlockId, &BlockState)> {
        let index = local_pos.x as usize
            + CHUNK_SIZE * (local_pos.y as usize + CHUNK_SIZE * local_pos.z as usize);
        let palette_index = *self.blocks.get(index)? as usize;
        Some((
            self.block_palette.get(palette_index).copied()?,
            self.block_states.get(index)?,
        ))
    }

    /// Sets the block at the given local position within the chunk.
    pub fn set_block(&mut self, local_pos: IVec3, block: BlockId, state: BlockState) {
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
    ) -> Vec<(IVec3, BlockId, BlockState)> {
        let neighbors = [
            IVec3::new(-1, -1, -1), // Y -1 Z -1
            IVec3::new(0, -1, -1),
            IVec3::new(1, -1, -1),
            IVec3::new(-1, 0, -1), // Y 0 Z -1
            IVec3::new(0, 0, -1),
            IVec3::new(1, 0, -1),
            IVec3::new(-1, 1, -1), // Y 1 Z -1
            IVec3::new(0, 1, -1),
            IVec3::new(1, 1, -1),
            IVec3::new(-1, -1, 0), // Y -1 Z 0
            IVec3::new(0, -1, 0),
            IVec3::new(1, -1, 0),
            IVec3::new(-1, 0, 0), // Y 0 Z 0
            // IVec3::new(0, 0, 0), // This is us, which is accessed separately
            IVec3::new(1, 0, 0),
            IVec3::new(-1, 1, 0), // Y 1 Z 0
            IVec3::new(0, 1, 0),
            IVec3::new(1, 1, 0),
            IVec3::new(-1, -1, 1), // Y -1 Z 1
            IVec3::new(0, -1, 1),
            IVec3::new(1, -1, 1),
            IVec3::new(-1, 0, 1), // Y 0 Z 1
            IVec3::new(0, 0, 1),
            IVec3::new(1, 0, 1),
            IVec3::new(-1, 1, 1), // Y 1 Z 1
            IVec3::new(0, 1, 1),
            IVec3::new(1, 1, 1),
        ]
        .map(|dir| chunks.get(&(chunk_pos + dir)));

        fn get_block_global<'a>(
            me: &'a Chunk,
            neighbors: [Option<&'a Chunk>; 26],
            global_pos: IVec3,
            chunk_pos: IVec3,
        ) -> Option<(BlockId, &'a BlockState)> {
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
            match <(i32, i32, i32)>::from(get_chunk_pos) {
                (-1, -1, -1) => neighbors[0]?.get_block(local_pos),
                (0, -1, -1) => neighbors[1]?.get_block(local_pos),
                (1, -1, -1) => neighbors[2]?.get_block(local_pos),

                (-1, 0, -1) => neighbors[3]?.get_block(local_pos),
                (0, 0, -1) => neighbors[4]?.get_block(local_pos),
                (1, 0, -1) => neighbors[5]?.get_block(local_pos),

                (-1, 1, -1) => neighbors[6]?.get_block(local_pos),
                (0, 1, -1) => neighbors[7]?.get_block(local_pos),
                (1, 1, -1) => neighbors[8]?.get_block(local_pos),

                (-1, -1, 0) => neighbors[9]?.get_block(local_pos),
                (0, -1, 0) => neighbors[10]?.get_block(local_pos),
                (1, -1, 0) => neighbors[11]?.get_block(local_pos),

                (-1, 0, 0) => neighbors[12]?.get_block(local_pos),
                (0, 0, 0) => me.get_block(local_pos),
                (1, 0, 0) => neighbors[13]?.get_block(local_pos),

                (-1, 1, 0) => neighbors[14]?.get_block(local_pos),
                (0, 1, 0) => neighbors[15]?.get_block(local_pos),
                (1, 1, 0) => neighbors[16]?.get_block(local_pos),

                (-1, -1, 1) => neighbors[17]?.get_block(local_pos),
                (0, -1, 1) => neighbors[18]?.get_block(local_pos),
                (1, -1, 1) => neighbors[19]?.get_block(local_pos),

                (-1, 0, 1) => neighbors[20]?.get_block(local_pos),
                (0, 0, 1) => neighbors[21]?.get_block(local_pos),
                (1, 0, 1) => neighbors[22]?.get_block(local_pos),

                (-1, 1, 1) => neighbors[23]?.get_block(local_pos),
                (0, 1, 1) => neighbors[24]?.get_block(local_pos),
                (1, 1, 1) => neighbors[25]?.get_block(local_pos),
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
            let above_global_pos = global_pos + Direction::Up;
            let above_block = get_block_global(self, neighbors, above_global_pos, chunk_pos)
                .and_then(|(id, bs)| block_registry().get(id).map(|v| (v, bs)));
            let below_global_pos = global_pos + Direction::Down;
            let below_block = get_block_global(self, neighbors, below_global_pos, chunk_pos);
            if block == &*blocks::DIRT
                && let Some((above_block, _)) = above_block
                && above_block.collision_shape == CollisionShape::None
            {
                // DIRT -> GRASS if above cannot be collided with (e.g. AIR)
                updates.push((global_pos, *blocks::GRASS, BlockState::none()));
            }
            if block == &*blocks::GRASS
                && let Some((above_block, _)) = above_block
                && above_block.collision_shape != CollisionShape::None
            {
                // GRASS -> DIRT if above can be collided with (e.g. GRASS or LOG)
                updates.push((global_pos, *blocks::DIRT, BlockState::none()));
            }
            if block == &*blocks::LEAVES {
                // LEAVES -> AIR if no LOG in radius of 6 blocks
                let mut should_become_air = true;
                for dx in -6..=6 {
                    for dy in -6..=6 {
                        for dz in -6..=6 {
                            let delta = IVec3::new(dx, dy, dz);
                            if delta.length_squared() > 36 {
                                continue;
                            }
                            let pos = global_pos + delta;
                            let block = get_block_global(self, neighbors, pos, chunk_pos);
                            if let Some((block, _)) = block
                                && block == *blocks::LOG
                            {
                                should_become_air = false;
                            }
                        }
                    }
                }
                if should_become_air {
                    updates.push((global_pos, *blocks::AIR, BlockState::none()));
                }
            }
            if block == &*blocks::SHORT_GRASS
                && let Some((below_block, _)) = below_block
                && below_block != *blocks::GRASS
            {
                // SHORT_GRASS -> AIR if below is not GRASS
                updates.push((global_pos, *blocks::AIR, BlockState::none()));
            }
        }
        updates
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
