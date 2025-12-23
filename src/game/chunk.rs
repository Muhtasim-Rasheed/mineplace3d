use std::{collections::HashMap, sync::Arc};

use fastnoise_lite::FastNoiseLite;
use glam::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::game::{Block, BlockType, BlockVertex, FACE_TEMPLATES, Face, ModelDefs, should_occlude};

pub const CHUNK_SIZE: usize = 16;

pub enum ChunkTask {
    Generate {
        cx: i32,
        cy: i32,
        cz: i32,
        noise: Arc<FastNoiseLite>,
        cave_noise: Arc<FastNoiseLite>,
        biome_noise: Arc<FastNoiseLite>,
    },
}

pub enum ChunkResult {
    Generated {
        cx: i32,
        cy: i32,
        cz: i32,
        chunk: Chunk,
        outside_blocks: HashMap<(IVec3, IVec3), Block>,
    },
}

pub struct NeighbourChunks<'a> {
    pub n: Option<&'a Chunk>,
    pub s: Option<&'a Chunk>,
    pub e: Option<&'a Chunk>,
    pub w: Option<&'a Chunk>,
    pub u: Option<&'a Chunk>,
    pub d: Option<&'a Chunk>,
}

impl<'a> NeighbourChunks<'a> {
    pub fn all<F>(&self, mut f: F) -> bool
    where
        F: FnMut(usize, &Chunk) -> bool,
    {
        if let Some(n) = self.n {
            if !f(0, n) {
                return false;
            }
        }
        if let Some(s) = self.s {
            if !f(1, s) {
                return false;
            }
        }
        if let Some(e) = self.e {
            if !f(2, e) {
                return false;
            }
        }
        if let Some(w) = self.w {
            if !f(3, w) {
                return false;
            }
        }
        if let Some(u) = self.u {
            if !f(4, u) {
                return false;
            }
        }
        if let Some(d) = self.d {
            if !f(5, d) {
                return false;
            }
        }
        true
    }
}

pub struct Chunk {
    pub is_dirty: bool,
    blocks: Vec<Block>,
    foliage_color: Vec<Vec3>,
}

impl Chunk {
    pub fn new(
        cx: i32,
        cy: i32,
        cz: i32,
        noise: &FastNoiseLite,
        cave_noise: &FastNoiseLite,
        biome_noise: &FastNoiseLite,
    ) -> (Self, HashMap<(IVec3, IVec3), Block>) {
        let mut rng = StdRng::seed_from_u64(noise.seed as u64);
        let mut blocks = vec![Block::Air; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        let mut foliage_color = vec![Vec3::splat(0.0); CHUNK_SIZE * CHUNK_SIZE];
        fn fractal_noise(
            noise: &FastNoiseLite,
            x: f32,
            y: f32,
            octaves: i32,
            persistence: f32,
            lacunarity: f32,
        ) -> f32 {
            let mut amplitude = 1.0;
            let mut frequency = 1.0;
            let mut value = 0.0;
            let mut max_value = 0.0;

            for _ in 0..octaves {
                value += noise.get_noise_2d(x * frequency, y * frequency) * amplitude;
                max_value += amplitude;
                amplitude *= persistence;
                frequency *= lacunarity;
            }

            value / max_value
        }
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let real_x = x as i32 + cx * CHUNK_SIZE as i32;
                let real_z = z as i32 + cz * CHUNK_SIZE as i32;
                let t = ((biome_noise.get_noise_2d(real_x as f32 * 0.1, real_z as f32 * 0.1)
                    + 1.0)
                    / 2.0)
                    .powi(2);

                let plains_noise_val = fractal_noise(
                    noise,
                    real_x as f32 * 0.05,
                    real_z as f32 * 0.05,
                    4,
                    0.5,
                    2.0,
                );
                let plains_height = plains_noise_val * 30.0;
                let plains_cave_thresh = 2.0;
                let plains_foliage_color = vec3(0.5, 1.0, 0.5);

                let mtn_noise_val = fractal_noise(
                    noise,
                    real_x as f32 * 0.04,
                    real_z as f32 * 0.04,
                    8,
                    0.5,
                    2.0,
                );
                let mtn_height = (mtn_noise_val * 7.0 + 10.0).powi(2) / 2.0;
                let mtn_cave_thresh = -0.3;
                let mtn_foliage_color = vec3(0.1, 0.7, 0.5);

                let height = (plains_height * (1.0 - t) + mtn_height * t) as i32;
                let cave_thresh = plains_cave_thresh * (1.0 - t) + mtn_cave_thresh * t;
                let foliage_color_val =
                    plains_foliage_color * (1.0 - t as f32) + mtn_foliage_color * t as f32;
                let snow_replace_grass_chance = if height <= 96 {
                    0.0
                } else if height >= 108 {
                    1.0
                } else {
                    (height - 96) as f64 / (108 - 96) as f64
                };
                let random_f64 = rng.random::<f64>();
                foliage_color[x * CHUNK_SIZE + z] = foliage_color_val;
                for y in 0..CHUNK_SIZE {
                    let real_y = y as i32 + cy * CHUNK_SIZE as i32;

                    let is_cave = cave_noise.get_noise_3d(
                        real_x as f32 * 0.1,
                        real_y as f32 * 0.1,
                        real_z as f32 * 0.1,
                    ) > cave_thresh as f32;

                    let ore_thresh = 0.7;
                    let ore_val = cave_noise.get_noise_3d(
                        real_x as f32 * 0.6 + 100.0,
                        real_y as f32 * 0.6 + 100.0,
                        real_z as f32 * 0.6 + 100.0,
                    );
                    let is_ore = ore_val > ore_thresh;

                    let block;
                    if real_y < -32 {
                        block = Block::Air;
                    } else if real_y == -31 {
                        block = Block::Bedrock;
                    } else if real_y == -30 && rng.random_bool(0.5) {
                        block = Block::Bedrock;
                    } else if is_cave {
                        block = Block::Air;
                    } else if real_y < height - 3 {
                        if is_ore {
                            block = Block::Glungus;
                        } else {
                            block = Block::Stone;
                        }
                    } else if real_y < height - 1 {
                        block = Block::Dirt;
                    } else if random_f64 < snow_replace_grass_chance && real_y < height {
                        block = Block::Snow;
                    } else if real_y < height {
                        block = Block::Grass;
                    } else {
                        block = Block::Air;
                    }
                    blocks[x * CHUNK_SIZE * CHUNK_SIZE + y * CHUNK_SIZE + z] = block;
                }
            }
        }

        let mut outside_blocks = HashMap::new();

        fn get_chunk_and_local_coords(x: i32, y: i32, z: i32) -> (IVec3, usize, usize, usize) {
            let chunk_x = x.div_euclid(CHUNK_SIZE as i32);
            let chunk_y = y.div_euclid(CHUNK_SIZE as i32);
            let chunk_z = z.div_euclid(CHUNK_SIZE as i32);

            let local_x = (x.rem_euclid(CHUNK_SIZE as i32)) as usize;
            let local_y = (y.rem_euclid(CHUNK_SIZE as i32)) as usize;
            let local_z = (z.rem_euclid(CHUNK_SIZE as i32)) as usize;

            (
                IVec3::new(chunk_x, chunk_y, chunk_z),
                local_x,
                local_y,
                local_z,
            )
        }

        fn place_block(
            // blocks: &mut Vec<Block>,
            blocks: &mut [Block],
            outside_blocks: &mut HashMap<(IVec3, IVec3), Block>,
            chunk_pos: IVec3,
            target_chunk: IVec3,
            local: IVec3,
            block: Block,
        ) {
            if target_chunk == chunk_pos {
                if local.y >= 0 && local.y < CHUNK_SIZE as i32 {
                    blocks[local.x as usize * CHUNK_SIZE * CHUNK_SIZE
                        + local.y as usize * CHUNK_SIZE
                        + local.z as usize] = block;
                }
            } else {
                outside_blocks.entry((target_chunk, local)).or_insert(block);
            }
        }

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for y in (0..CHUNK_SIZE).rev() {
                    let block = blocks[x * CHUNK_SIZE * CHUNK_SIZE + y * CHUNK_SIZE + z];
                    if block == Block::Grass && rng.random_bool(0.005) {
                        let tree_height = rng.random_range(4..7);
                        let global_x = cx * CHUNK_SIZE as i32 + x as i32;
                        let global_z = cz * CHUNK_SIZE as i32 + z as i32;
                        let global_y = cy * CHUNK_SIZE as i32 + y as i32;

                        for ty in 1..=tree_height {
                            let tree_y = global_y + ty;
                            let (target_chunk_pos, local_x, local_y, local_z) =
                                get_chunk_and_local_coords(global_x, tree_y, global_z);

                            place_block(
                                &mut blocks,
                                &mut outside_blocks,
                                IVec3::new(cx, cy, cz),
                                target_chunk_pos,
                                IVec3::new(local_x as i32, local_y as i32, local_z as i32),
                                Block::OakLog,
                            );
                        }

                        let leaf_start = global_y + tree_height;
                        for lx_offset in -2i32..=2 {
                            for lz_offset in -2i32..=2 {
                                for ly_offset in -2i32..=2 {
                                    if lx_offset.abs() + lz_offset.abs() + ly_offset.abs() <= 3 {
                                        let lx_global = global_x + lx_offset;
                                        let lz_global = global_z + lz_offset;
                                        let ly_global = leaf_start + ly_offset;

                                        let (target_chunk_pos, local_x, local_y, local_z) =
                                            get_chunk_and_local_coords(
                                                lx_global, ly_global, lz_global,
                                            );

                                        let is_air = if target_chunk_pos == IVec3::new(cx, cy, cz) {
                                            local_y < CHUNK_SIZE
                                                && blocks[local_x * CHUNK_SIZE * CHUNK_SIZE
                                                    + local_y * CHUNK_SIZE
                                                    + local_z]
                                                    == Block::Air
                                        } else {
                                            true
                                        };

                                        if is_air {
                                            place_block(
                                                &mut blocks,
                                                &mut outside_blocks,
                                                IVec3::new(cx, cy, cz),
                                                target_chunk_pos,
                                                IVec3::new(
                                                    local_x as i32,
                                                    local_y as i32,
                                                    local_z as i32,
                                                ),
                                                Block::Leaves,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    } else if block != Block::Air {
                        break;
                    }
                }
            }
        }

        (
            Chunk {
                is_dirty: true,
                blocks,
                foliage_color,
            },
            outside_blocks,
        )
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> &Block {
        &self.blocks[x * CHUNK_SIZE * CHUNK_SIZE + y * CHUNK_SIZE + z]
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: Block) {
        self.blocks[x * CHUNK_SIZE * CHUNK_SIZE + y * CHUNK_SIZE + z] = block;
        self.is_dirty = true;
    }

    pub fn is_empty(&self) -> bool {
        self.blocks
            .iter()
            .all(|&b| b.block_type() == BlockType::Air)
    }

    pub fn is_side_full(&self, side: u8) -> bool {
        match side {
            0 => {
                // -Z
                for x in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        let block = self.get_block(x, y, 0);
                        if block.block_type() != BlockType::FullOpaque {
                            return false;
                        }
                    }
                }
                true
            }
            1 => {
                // +Z
                for x in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        let block = self.get_block(x, y, CHUNK_SIZE - 1);
                        if block.block_type() != BlockType::FullOpaque {
                            return false;
                        }
                    }
                }
                true
            }
            2 => {
                // +X
                for y in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let block = self.get_block(CHUNK_SIZE - 1, y, z);
                        if block.block_type() != BlockType::FullOpaque {
                            return false;
                        }
                    }
                }
                true
            }
            3 => {
                // -X
                for y in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let block = self.get_block(0, y, z);
                        if block.block_type() != BlockType::FullOpaque {
                            return false;
                        }
                    }
                }
                true
            }
            4 => {
                // +Y
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let block = self.get_block(x, CHUNK_SIZE - 1, z);
                        if block.block_type() != BlockType::FullOpaque {
                            return false;
                        }
                    }
                }
                true
            }
            5 => {
                // -Y
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let block = self.get_block(x, 0, z);
                        if block.block_type() != BlockType::FullOpaque {
                            return false;
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn generate_chunk_mesh(
        &self,
        neighbour_chunks: &NeighbourChunks,
        model_defs: &ModelDefs,
    ) -> (Vec<BlockVertex>, Vec<u32>) {
        const STRIDE_X: usize = CHUNK_SIZE * CHUNK_SIZE; // N*N

        let mut vertices = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 24);
        let mut indices = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 36);
        let mut index_offset: u32 = 0;

        // Make local aliases for speed
        let blocks = &self.blocks;
        let foliage = &self.foliage_color;

        // Helper: read block at world-local coords (x,y,z) where coords are isize
        // Returns Block::Air for out-of-range or missing neighbour chunk
        #[inline(always)]
        fn neighbour_block_at(
            x: isize,
            y: isize,
            z: isize,
            blocks: &[Block],
            nei: &NeighbourChunks,
        ) -> BlockType {
            // inside main chunk?
            if (0..CHUNK_SIZE as isize).contains(&x)
                && (0..CHUNK_SIZE as isize).contains(&y)
                && (0..CHUNK_SIZE as isize).contains(&z)
            {
                let idx = (x as usize) * STRIDE_X + (y as usize) * CHUNK_SIZE + (z as usize);
                return blocks[idx].block_type();
            }

            // west
            if x < 0 {
                return nei
                    .w
                    .map(|c| {
                        c.get_block(CHUNK_SIZE - 1, y as usize, z as usize)
                            .block_type()
                    })
                    .unwrap_or(BlockType::Air);
            }

            // east
            if x >= CHUNK_SIZE as isize {
                return nei
                    .e
                    .map(|c| c.get_block(0, y as usize, z as usize).block_type())
                    .unwrap_or(BlockType::Air);
            }

            // down
            if y < 0 {
                return nei
                    .d
                    .map(|c| {
                        c.get_block(x as usize, CHUNK_SIZE - 1, z as usize)
                            .block_type()
                    })
                    .unwrap_or(BlockType::Air);
            }

            // up
            if y >= CHUNK_SIZE as isize {
                return nei
                    .u
                    .map(|c| c.get_block(x as usize, 0, z as usize).block_type())
                    .unwrap_or(BlockType::Air);
            }

            // north
            if z < 0 {
                return nei
                    .n
                    .map(|c| {
                        c.get_block(x as usize, y as usize, CHUNK_SIZE - 1)
                            .block_type()
                    })
                    .unwrap_or(BlockType::Air);
            }

            // south
            return nei
                .s
                .map(|c| c.get_block(x as usize, y as usize, 0).block_type())
                .unwrap_or(BlockType::Air);
        }

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let idx = x * STRIDE_X + y * CHUNK_SIZE + z;
                    let block = blocks[idx];
                    if block == Block::Air {
                        continue;
                    }

                    let cubes = block.cubes(model_defs);
                    let uvs_collection = block.uvs(model_defs);

                    for (cube, uvs) in cubes.iter().zip(uvs_collection.iter()) {
                        for (i, face_template) in FACE_TEMPLATES.iter().enumerate() {
                            let nx = x as isize + face_template.normal.x as isize;
                            let ny = y as isize + face_template.normal.y as isize;
                            let nz = z as isize + face_template.normal.z as isize;

                            let neighbour =
                                neighbour_block_at(nx, ny, nz, blocks, neighbour_chunks);
                            if should_occlude(block.block_type(), neighbour) {
                                continue;
                            }
                            let face = Face::use_template(*face_template, cube[0], cube[1], uvs[i]);

                            // Push 4 vertices
                            for j in 0..4 {
                                let vert_offset =
                                    face.vertices[j] + vec3(x as f32, y as f32, z as f32);

                                vertices.push(BlockVertex::new(
                                    uvec3(x as u32, y as u32, z as u32),
                                    vert_offset,
                                    i as u8,
                                    face.uvs[j],
                                    ((block as u32) & 0xFFFF) as u16,
                                    foliage[x * CHUNK_SIZE + z],
                                ));
                            }

                            indices.push(index_offset);
                            indices.push(index_offset + 1);
                            indices.push(index_offset + 2);
                            indices.push(index_offset);
                            indices.push(index_offset + 2);
                            indices.push(index_offset + 3);

                            index_offset += 4;
                        }
                    }
                }
            }
        }

        (vertices, indices)
    }
}
