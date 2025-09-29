use glam::*;
use glfw::MouseButton;
use noise::{NoiseFn, OpenSimplex, Seedable};
use rand::{Rng, SeedableRng, rngs::StdRng};
use rayon::prelude::*;
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate::{
    asset::{Key, KeyPart, ModelDefs, ResourceManager}, mesh::{BillboardVertex, BlockVertex, CloudPlaneVertex, DrawMode, Mesh, UIVertex}, shader::ShaderProgram, texture::Texture, PLACABLE_BLOCKS, WINDOW_HEIGHT, WINDOW_WIDTH
};

pub const CHUNK_SIZE: usize = 16;
pub const RENDER_DISTANCE: i32 = 8;

const FULL_BLOCK: u32 = 0x00000000;
const PARTIAL_SLAB_TOP: u32 = 0x00010000;
const PARTIAL_SLAB_BOTTOM: u32 = 0x00020000;
const PARTIAL_STAIRS_N: u32 = 0x00030000;
const PARTIAL_STAIRS_S: u32 = 0x00040000;
const PARTIAL_STAIRS_E: u32 = 0x00050000;
const PARTIAL_STAIRS_W: u32 = 0x00060000;
const BLOCK_MASK: u32 = 0x0000FFFF;

#[inline]
fn mask_partial(bits: u32) -> u32 {
    (bits >> 16) & 0x000F
}

#[inline]
fn collision_aabb(min_a: Vec3, max_a: Vec3, min_b: Vec3, max_b: Vec3) -> bool {
    (min_a.x <= max_b.x && max_a.x >= min_b.x)
        && (min_a.y <= max_b.y && max_a.y >= min_b.y)
        && (min_a.z <= max_b.z && max_a.z >= min_b.z)
}

#[inline]
fn extract_frustum_planes(pv: Mat4) -> [Vec4; 6] {
    let m = pv.to_cols_array_2d();

    let row0 = Vec4::new(m[0][0], m[1][0], m[2][0], m[3][0]);
    let row1 = Vec4::new(m[0][1], m[1][1], m[2][1], m[3][1]);
    let row2 = Vec4::new(m[0][2], m[1][2], m[2][2], m[3][2]);
    let row3 = Vec4::new(m[0][3], m[1][3], m[2][3], m[3][3]);

    let mut planes = [
        row3 + row0, // left
        row3 - row0, // right
        row3 + row1, // bottom
        row3 - row1, // top
        row3 + row2, // near
        row3 - row2, // far
    ];

    // normalize planes
    for plane in planes.iter_mut() {
        let n = plane.truncate().length();
        *plane /= n;
    }

    planes
}

#[inline]
fn aabb_in_frustum(min: Vec3, max: Vec3, planes: &[Vec4; 6]) -> bool {
    for plane in planes.iter() {
        let p = vec3(
            if plane.x >= 0.0 { max.x } else { min.x },
            if plane.y >= 0.0 { max.y } else { min.y },
            if plane.z >= 0.0 { max.z } else { min.z },
        );
        if plane.xyz().dot(p) + plane.w < 0.0 {
            return false;
        }
    }
    true
}

#[derive(Copy, Clone)]
struct Face {
    vertices: [Vec3; 4],
    uvs: [UVec2; 4],
}

#[derive(Copy, Clone)]
struct FaceTemplate {
    normal: IVec3,
    vertices: [IVec3; 4],
}

impl Face {
    fn use_template(template: FaceTemplate, from: Vec3, to: Vec3, uvs: [UVec2; 4]) -> Self {
        let min = from;
        let max = to;
        let vertices = template.vertices.map(|v| {
            vec3(
                if v.x == 0 { min.x } else { max.x },
                if v.y == 0 { min.y } else { max.y },
                if v.z == 0 { min.z } else { max.z },
            )
        });
        Self { vertices, uvs }
    }
}

const FACE_TEMPLATES: [FaceTemplate; 6] = [
    // +Z (front)
    FaceTemplate {
        normal: IVec3::new(0, 0, 1),
        vertices: [
            IVec3::new(0, 0, 1),
            IVec3::new(1, 0, 1),
            IVec3::new(1, 1, 1),
            IVec3::new(0, 1, 1),
        ],
    },
    // -Z (back)
    FaceTemplate {
        normal: IVec3::new(0, 0, -1),
        vertices: [
            IVec3::new(1, 0, 0),
            IVec3::new(0, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(1, 1, 0),
        ],
    },
    // +X (right)
    FaceTemplate {
        normal: IVec3::new(1, 0, 0),
        vertices: [
            IVec3::new(1, 0, 1),
            IVec3::new(1, 0, 0),
            IVec3::new(1, 1, 0),
            IVec3::new(1, 1, 1),
        ],
    },
    // -X (left)
    FaceTemplate {
        normal: IVec3::new(-1, 0, 0),
        vertices: [
            IVec3::new(0, 0, 0),
            IVec3::new(0, 0, 1),
            IVec3::new(0, 1, 1),
            IVec3::new(0, 1, 0),
        ],
    },
    // +Y (top)
    FaceTemplate {
        normal: IVec3::new(0, 1, 0),
        vertices: [
            IVec3::new(0, 1, 1),
            IVec3::new(1, 1, 1),
            IVec3::new(1, 1, 0),
            IVec3::new(0, 1, 0),
        ],
    },
    // -Y (bottom)
    FaceTemplate {
        normal: IVec3::new(0, -1, 0),
        vertices: [
            IVec3::new(0, 0, 0),
            IVec3::new(1, 0, 0),
            IVec3::new(1, 0, 1),
            IVec3::new(0, 0, 1),
        ],
    },
];

pub enum ChunkTask {
    Generate {
        cx: i32,
        cy: i32,
        cz: i32,
        noise: Arc<OpenSimplex>,
        cave_noise: Arc<OpenSimplex>,
        biome_noise: Arc<OpenSimplex>,
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

#[inline(always)]
fn should_occlude(self_block: BlockType, neighbour: BlockType) -> bool {
    match (self_block, neighbour) {
        // Full opaque
        (BlockType::FullOpaque, BlockType::FullOpaque) => true,
        (BlockType::FullOpaque, _) => false,

        // Translucent
        (BlockType::Translucent, BlockType::FullOpaque) => true,
        (BlockType::Translucent, BlockType::Translucent) => true,
        (BlockType::Translucent, _) => false,

        // Partial
        (BlockType::Partial, _) => false,

        // Air
        (BlockType::Air, _) => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockType {
    FullOpaque,
    Translucent,
    Partial,
    Air,
}

#[rustfmt::skip]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Block {
    Air              = FULL_BLOCK,
    Grass            = FULL_BLOCK          | 0x0001,
    Dirt             = FULL_BLOCK          | 0x0002,
    Planks           = FULL_BLOCK          | 0x0003,
    PlanksSlabTop    = PARTIAL_SLAB_TOP    | 0x0003,
    PlanksSlabBottom = PARTIAL_SLAB_BOTTOM | 0x0003,
    PlanksStairsN    = PARTIAL_STAIRS_N    | 0x0003,
    PlanksStairsS    = PARTIAL_STAIRS_S    | 0x0003,
    PlanksStairsE    = PARTIAL_STAIRS_E    | 0x0003,
    PlanksStairsW    = PARTIAL_STAIRS_W    | 0x0003,
    Stone            = FULL_BLOCK          | 0x0004,
    OakLog           = FULL_BLOCK          | 0x0005,
    Leaves           = FULL_BLOCK          | 0x0006,
    CobbleStone      = FULL_BLOCK          | 0x0007,
    StoneSlabTop     = PARTIAL_SLAB_TOP    | 0x0007,
    StoneSlabBottom  = PARTIAL_SLAB_BOTTOM | 0x0007,
    StoneStairsN     = PARTIAL_STAIRS_N    | 0x0007,
    StoneStairsS     = PARTIAL_STAIRS_S    | 0x0007,
    StoneStairsE     = PARTIAL_STAIRS_E    | 0x0007,
    StoneStairsW     = PARTIAL_STAIRS_W    | 0x0007,
    Glass            = FULL_BLOCK          | 0x0008,
    Brick            = FULL_BLOCK          | 0x0009,
    Snow             = FULL_BLOCK          | 0x000A,
    Glungus          = FULL_BLOCK          | 0x000B,
    Bedrock          = FULL_BLOCK          | 0x000C,
}

impl Block {
    pub fn is_solid_at(
        &self,
        model_defs: &ModelDefs,
        local_pos: Vec3,
        player_width: f32,
        player_height: f32,
    ) -> bool {
        let cubes = self.cubes(model_defs);
        for [min, max] in cubes.iter() {
            if collision_aabb(
                local_pos - vec3(player_width / 2.0, 0.0, player_width / 2.0),
                local_pos + vec3(player_width / 2.0, player_height, player_width / 2.0),
                *min,
                *max,
            ) {
                return true;
            }
        }
        false
    }

    pub fn is_transparent(&self) -> bool {
        matches!(self, Block::Air | Block::Leaves | Block::Glass)
    }

    pub fn block_type(&self) -> BlockType {
        if *self == Block::Air {
            return BlockType::Air;
        }
        let partial_bits = mask_partial(*self as u32);
        let is_partial = partial_bits > 0;
        if is_partial {
            BlockType::Partial
        } else if self.is_transparent() {
            BlockType::Translucent
        } else {
            BlockType::FullOpaque
        }
    }

    pub fn ui_mesh(&self, from: Vec2, to: Vec2, m: Mat4, model_defs: &ModelDefs) -> Mesh<UIVertex> {
        let cubes = self.cubes(model_defs);
        let uvs = self.uvs(model_defs);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset = 0;

        let size = to - from;

        for ([cube_from, cube_to], uvs) in cubes.iter().zip(uvs.iter()) {
            for (i, face_template) in FACE_TEMPLATES.iter().enumerate() {
                let face = Face::use_template(*face_template, *cube_from, *cube_to, uvs[i]);

                let v0 = m.transform_point3(face.vertices[0]);
                let v1 = m.transform_point3(face.vertices[1]);
                let v2 = m.transform_point3(face.vertices[2]);
                let v3 = m.transform_point3(face.vertices[3]);

                let quad = [
                    UIVertex {
                        position: (from + v0.xy() * size).extend(v0.z),
                        uv: vec2(face.uvs[0].x as f32 / 192.0, face.uvs[0].y as f32 / 192.0)
                            + self.uv_offset(),
                    },
                    UIVertex {
                        position: (from + v1.xy() * size).extend(v1.z),
                        uv: vec2(face.uvs[1].x as f32 / 192.0, face.uvs[1].y as f32 / 192.0)
                            + self.uv_offset(),
                    },
                    UIVertex {
                        position: (from + v2.xy() * size).extend(v2.z),
                        uv: vec2(face.uvs[2].x as f32 / 192.0, face.uvs[2].y as f32 / 192.0)
                            + self.uv_offset(),
                    },
                    UIVertex {
                        position: (from + v3.xy() * size).extend(v3.z),
                        uv: vec2(face.uvs[3].x as f32 / 192.0, face.uvs[3].y as f32 / 192.0)
                            + self.uv_offset(),
                    },
                ];

                vertices.extend_from_slice(&quad);

                indices.extend_from_slice(&[
                    index_offset,
                    index_offset + 1,
                    index_offset + 2,
                    index_offset,
                    index_offset + 2,
                    index_offset + 3,
                ]);

                index_offset += 4;
            }
        }

        Mesh::new(&vertices, &indices, DrawMode::Triangles)
    }

    pub fn uv_offset(&self) -> Vec2 {
        let tile_index = *self as u32 & BLOCK_MASK;
        let tile_x = tile_index % 12;
        let tile_y = tile_index / 12;

        let uv_unit = 1.0 / 12.0;
        let uv_row_unit = 1.0 / 12.0;

        vec2(tile_x as f32 * uv_unit, tile_y as f32 * uv_row_unit)
    }

    pub fn uvs(&self, model_defs: &ModelDefs) -> Vec<[[UVec2; 4]; 6]> {
        let partial_bits = mask_partial(*self as u32);

        fn face_uv(min: UVec2, max: UVec2) -> [UVec2; 4] {
            [
                uvec2(max.x, max.y),
                uvec2(min.x, max.y),
                uvec2(min.x, min.y),
                uvec2(max.x, min.y),
            ]
        }

        fn faces_uvs(minmax: &[[UVec2; 2]; 6]) -> [[UVec2; 4]; 6] {
            minmax.map(|[min, max]| face_uv(min, max))
        }

        macro_rules! get_uvs {
            ($name:ident) => {
                let $name = model_defs
                    .get(stringify!($name))
                    .unwrap()
                    .uvs
                    .iter()
                    .map(|face_uvs| faces_uvs(face_uvs))
                    .collect::<Vec<_>>();
            };
        }

        get_uvs!(full);
        get_uvs!(slab_top);
        get_uvs!(slab_bottom);
        get_uvs!(stairs_n);
        get_uvs!(stairs_s);
        get_uvs!(stairs_e);
        get_uvs!(stairs_w);

        match partial_bits {
            0 => full,           // Full block
            1 => slab_top,       // Slab top
            2 => slab_bottom,    // Slab bottom
            3 => stairs_n,       // Stairs north
            4 => stairs_s,       // Stairs south
            5 => stairs_e,       // Stairs east
            6 => stairs_w,       // Stairs west
            _ => unreachable!(), // Should not happen
        }
    }

    pub fn cubes(&self, model_defs: &ModelDefs) -> Vec<[Vec3; 2]> {
        macro_rules! get_cubes {
            ($name:ident) => {
                let $name = model_defs.get(stringify!($name)).unwrap().cubes.clone();
            };
        }

        get_cubes!(empty);

        if *self == Block::Air {
            return empty;
        }

        get_cubes!(full);
        get_cubes!(slab_top);
        get_cubes!(slab_bottom);
        get_cubes!(stairs_n);
        get_cubes!(stairs_s);
        get_cubes!(stairs_e);
        get_cubes!(stairs_w);

        let partial_bits = mask_partial(*self as u32);
        match partial_bits {
            0 => full,
            1 => slab_top,
            2 => slab_bottom,
            3 => stairs_n,
            4 => stairs_s,
            5 => stairs_e,
            6 => stairs_w,
            _ => unreachable!(),
        }
    }
}

impl From<Block> for Key {
    fn from(value: Block) -> Self {
        let lo = (value as u32 & BLOCK_MASK) as u16;
        let hi = (value as u32 >> 16) as u16;
        let parts;
        if hi == 0 {
            parts = vec![
                KeyPart::Text("block".to_string()),
                KeyPart::Numeric(lo as u32),
            ];
            return Key { parts };
        }
        let parts = vec![
            KeyPart::Text("block".to_string()),
            KeyPart::Numeric(lo as u32),
            KeyPart::Numeric(hi as u32),
        ];
        Key { parts }
    }
}

pub struct NeighbourChunks<'a> {
    pub n: Option<&'a Chunk>,
    pub s: Option<&'a Chunk>,
    pub e: Option<&'a Chunk>,
    pub w: Option<&'a Chunk>,
    pub u: Option<&'a Chunk>,
    pub d: Option<&'a Chunk>,
}

pub struct Chunk {
    is_dirty: bool,
    cached_mesh: Option<Arc<(Vec<BlockVertex>, Vec<u32>)>>,
    blocks: Vec<Block>,
    foliage_color: Vec<Vec3>,
}

impl Chunk {
    pub fn new(
        cx: i32,
        cy: i32,
        cz: i32,
        noise: &OpenSimplex,
        cave_noise: &OpenSimplex,
        biome_noise: &OpenSimplex,
    ) -> (Self, HashMap<(IVec3, IVec3), Block>) {
        let mut rng = StdRng::seed_from_u64(noise.seed() as u64);
        let mut blocks = vec![Block::Air; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
        let mut foliage_color = vec![Vec3::splat(0.0); CHUNK_SIZE * CHUNK_SIZE];
        fn fractal_noise(
            noise: &OpenSimplex,
            x: f64,
            y: f64,
            octaves: i32,
            persistence: f64,
            lacunarity: f64,
        ) -> f64 {
            let mut amplitude = 1.0;
            let mut frequency = 1.0;
            let mut value = 0.0;
            let mut max_value = 0.0;

            for _ in 0..octaves {
                value += noise.get([x * frequency, y * frequency]) * amplitude;
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
                let t = (biome_noise.get([real_x as f64 * 0.01, real_z as f64 * 0.01]) + 1.0) / 2.0;

                let plains_noise_val = fractal_noise(
                    noise,
                    real_x as f64 * 0.03,
                    real_z as f64 * 0.03,
                    3,
                    0.5,
                    2.0,
                );
                let plains_height = plains_noise_val * 30.0;
                let plains_cave_thresh = -1.0;
                let plains_foliage_color = vec3(0.5, 1.0, 0.5);

                let mtn_noise_val = fractal_noise(
                    noise,
                    real_x as f64 * 0.015,
                    real_z as f64 * 0.015,
                    5,
                    0.5,
                    2.0,
                );
                let mtn_height = (mtn_noise_val * 10.0).powi(4).max(plains_height + 15.0);
                let mtn_cave_thresh = 0.3;
                let mtn_foliage_color = vec3(0.1, 0.7, 0.5);

                let height = (plains_height * (1.0 - t) + mtn_height * t) as i32;
                let cave_thresh = plains_cave_thresh * (1.0 - t) + mtn_cave_thresh * t;
                let foliage_color_val =
                    plains_foliage_color * (1.0 - t as f32) + mtn_foliage_color * t as f32;
                foliage_color[x * CHUNK_SIZE + z] = foliage_color_val;
                for y in 0..CHUNK_SIZE {
                    let real_y = y as i32 + cy * CHUNK_SIZE as i32;

                    let is_cave = cave_noise.get([
                        real_x as f64 * 0.095,
                        real_y as f64 * 0.095,
                        real_z as f64 * 0.095,
                    ]) < cave_thresh;

                    let snow_replace_grass_chance = if height <= 96 {
                        0.0
                    } else if height >= 108 {
                        1.0
                    } else {
                        (height - 96) as f64 / (108 - 96) as f64
                    };

                    let ore_thresh = 0.3;
                    let ore_val = cave_noise.get([
                        real_x as f64 * 0.2 + 100.0,
                        real_y as f64 * 0.2 + 100.0,
                        real_z as f64 * 0.2 + 100.0,
                    ]);
                    let is_ore = ore_val > ore_thresh;

                    let random_f64 = rng.random::<f64>();

                    let block;
                    if real_y < -32 {
                        block = Block::Air;
                    } else if real_y < -30 {
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
            blocks: &mut Vec<Block>,
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
                cached_mesh: None,
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

    pub fn generate_chunk_mesh(
        &self,
        neighbour_chunks: &NeighbourChunks,
        model_defs: &ModelDefs,
    ) -> (Vec<BlockVertex>, Vec<u32>) {
        // Fast path: cached
        if !self.is_dirty {
            // if let Some((ref verts, ref idxs)) = self.cached_mesh {
            //     return (verts.clone(), idxs.clone());
            // }
            if let Some(cached) = &self.cached_mesh {
                return (cached.0.clone(), cached.1.clone());
            }
        }

        // Precompute sizes & capacities
        const STRIDE_X: usize = CHUNK_SIZE * CHUNK_SIZE; // N*N
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset: u32 = 0;

        // Make local aliases for speed
        let blocks = &self.blocks;
        let foliage = &self.foliage_color;
        let model_defs = model_defs; // local alias - cheap

        // Helper: read block at world-local coords (x,y,z) where coords are isize
        // Returns Block::Air for out-of-range or missing neighbour chunk
        #[inline(always)]
        fn neighbour_block_at(
            x: isize,
            y: isize,
            z: isize,
            blocks: &[Block],
            neighbour_chunks: &NeighbourChunks,
        ) -> BlockType {
            // in-chunk
            if (0..CHUNK_SIZE as isize).contains(&x)
                && (0..CHUNK_SIZE as isize).contains(&y)
                && (0..CHUNK_SIZE as isize).contains(&z)
            {
                let xi = x as usize;
                let yi = y as usize;
                let zi = z as usize;
                return blocks[xi * STRIDE_X + yi * CHUNK_SIZE + zi].block_type();
            }

            if x < 0 {
                if let Some(w) = neighbour_chunks.w {
                    return (*w.get_block(CHUNK_SIZE - 1, y as usize, z as usize)).block_type();
                } else {
                    return BlockType::Air;
                }
            }
            if x >= CHUNK_SIZE as isize {
                if let Some(e) = neighbour_chunks.e {
                    return (*e.get_block(0, y as usize, z as usize)).block_type();
                } else {
                    return BlockType::Air;
                }
            }
            if y < 0 {
                if let Some(d) = neighbour_chunks.d {
                    return (*d.get_block(x as usize, CHUNK_SIZE - 1, z as usize)).block_type();
                } else {
                    return BlockType::Air;
                }
            }
            if y >= CHUNK_SIZE as isize {
                if let Some(u) = neighbour_chunks.u {
                    return (*u.get_block(x as usize, 0, z as usize)).block_type();
                } else {
                    return BlockType::Air;
                }
            }
            if z < 0 {
                if let Some(n) = neighbour_chunks.n {
                    return (*n.get_block(x as usize, y as usize, CHUNK_SIZE - 1)).block_type();
                } else {
                    return BlockType::Air;
                }
            }
            if z >= CHUNK_SIZE as isize {
                if let Some(s) = neighbour_chunks.s {
                    return (*s.get_block(x as usize, y as usize, 0)).block_type();
                } else {
                    return BlockType::Air;
                }
            }
            BlockType::Air // should not happen
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
                                // local_pos as UVec3, cast once
                                let vert_offset = face.vertices[j].as_uvec3()
                                    + uvec3(x as u32, y as u32, z as u32);

                                vertices.push(BlockVertex::new(
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

pub trait Entity: 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn identity(&self) -> &'static str {
        std::any::type_name::<Self>().rsplit("::").next().unwrap()
    }
    fn position(&self) -> Vec3;
    fn velocity(&self) -> Vec3;
    fn apply_velocity(&mut self, delta: Vec3);
    fn width(&self) -> f32;
    fn height(&self) -> f32;
    fn eye_height(&self) -> f32;
    fn update(&mut self, world: &mut World, events: &[glfw::WindowEvent], dt: f64);
    fn draw(&self, _world: &World, _resource_manager: &ResourceManager) {
    }
}

#[derive(Clone)]
pub struct Player {
    pub old_position: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub jumping: bool,
    pub keys_down: HashSet<glfw::Key>,
    pub mouse_down: HashSet<glfw::MouseButton>,
    pub break_place_cooldown: u32,
    pub selected_block: Option<RayHit>,
    pub current_block: usize,
    pub projection: Mat4,
    pub cloud_projection: Mat4,
}

impl Player {
    pub fn new(position: Vec3) -> Self {
        Player {
            old_position: position,
            position,
            velocity: Vec3::ZERO,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
            yaw: -90.0,
            pitch: 0.0,
            jumping: false,
            keys_down: HashSet::new(),
            mouse_down: HashSet::new(),
            break_place_cooldown: 0,
            selected_block: None,
            current_block: 0,
            projection: Mat4::perspective_rh_gl(
                90f32.to_radians(),
                WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
                0.1,
                200.0,
            ),
            cloud_projection: Mat4::perspective_rh_gl(
                90f32.to_radians(),
                WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
                0.1,
                400.0,
            ),
        }
    }

    pub fn camera_pos(&self) -> Vec3 {
        self.position.with_y(self.position.y + self.eye_height())
    }
}

impl Entity for Player {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn velocity(&self) -> Vec3 {
        self.velocity
    }

    fn apply_velocity(&mut self, delta: Vec3) {
        self.velocity += delta;
    }

    fn width(&self) -> f32 {
        0.6
    }

    fn height(&self) -> f32 {
        1.8
    }

    fn eye_height(&self) -> f32 {
        1.7
    }

    fn update(&mut self, world: &mut World, events: &[glfw::WindowEvent], dt: f64) {
        for event in events {
            match event {
                glfw::WindowEvent::Key(glfw::Key::Left, _, glfw::Action::Press, _) => {
                    self.current_block =
                        (self.current_block + PLACABLE_BLOCKS.len() - 1) % PLACABLE_BLOCKS.len();
                }
                glfw::WindowEvent::Key(glfw::Key::Right, _, glfw::Action::Press, _) => {
                    self.current_block = (self.current_block + 1) % PLACABLE_BLOCKS.len();
                }
                glfw::WindowEvent::Key(key, _, action, _) => match action {
                    glfw::Action::Press => {
                        self.keys_down.insert(*key);
                    }
                    glfw::Action::Release => {
                        self.keys_down.remove(key);
                    }
                    _ => {}
                },
                glfw::WindowEvent::MouseButton(button, action, _) => match action {
                    glfw::Action::Press => {
                        self.mouse_down.insert(*button);
                    }
                    glfw::Action::Release => {
                        self.mouse_down.remove(button);
                    }
                    _ => {}
                },
                glfw::WindowEvent::Scroll(_, yoffset) => {
                    if *yoffset > 0.0 {
                        self.current_block = (self.current_block + PLACABLE_BLOCKS.len() - 1)
                            % PLACABLE_BLOCKS.len();
                    } else if *yoffset < 0.0 {
                        self.current_block = (self.current_block + 1) % PLACABLE_BLOCKS.len();
                    }
                }
                _ => {}
            }
        }
        self.selected_block = cast_ray(world, self.camera_pos(), self.forward, 5.0);

        let player_accel = 0.9;
        let jump_accel = 0.8 * 10.0;
        let sprint_player_accel = player_accel * 1.5;
        if self.keys_down.contains(&glfw::Key::W) {
            self.velocity += vec3(self.forward.x, 0.0, self.forward.z).normalize()
                * if self.keys_down.contains(&glfw::Key::LeftControl)
                    || self.keys_down.contains(&glfw::Key::Q)
                {
                    sprint_player_accel
                } else {
                    player_accel
                };
        }
        if self.keys_down.contains(&glfw::Key::S) {
            self.velocity -= vec3(self.forward.x, 0.0, self.forward.z).normalize() * player_accel;
        }
        if self.keys_down.contains(&glfw::Key::A) {
            self.velocity -= self.forward.cross(self.up).normalize() * player_accel;
        }
        if self.keys_down.contains(&glfw::Key::D) {
            self.velocity += self.forward.cross(self.up).normalize() * player_accel;
        }
        if self.keys_down.contains(&glfw::Key::Space) && !self.jumping {
            self.velocity.y += jump_accel;
        }
        self.old_position = self.position;
        if self.break_place_cooldown > 0 {
            self.break_place_cooldown -= 1;
        }
        if self.mouse_down.contains(&MouseButton::Button2) && self.break_place_cooldown <= 0 {
            if let Some(ref hit) = self.selected_block {
                let block_pos = hit.block_pos;
                let hit_normal = hit.face_normal;
                let new_pos = block_pos + hit_normal;
                world.set_block(
                    new_pos.x,
                    new_pos.y,
                    new_pos.z,
                    PLACABLE_BLOCKS[self.current_block],
                );
                let (collide_x, collide_y, collide_z) =
                    world.player_collision_mask(self.old_position, self.position, 0.5, 1.8);
                if collide_x || collide_y || collide_z {
                    world.set_block(new_pos.x, new_pos.y, new_pos.z, Block::Air);
                }
                self.break_place_cooldown = 12;
            }
        }
        if self.mouse_down.contains(&MouseButton::Button1) && self.break_place_cooldown <= 0 {
            if let Some(ref hit) = self.selected_block {
                if !(world.get_block(hit.block_pos.x, hit.block_pos.y, hit.block_pos.z)
                    == Block::Bedrock)
                {
                    let block_pos = hit.block_pos;
                    world.break_block(block_pos);
                    self.break_place_cooldown = 12;
                }
            }
        }
        self.velocity.y -= 36.0 * dt as f32 - dt as f32 * 10.0 * self.velocity.y;
        self.position += self.velocity * dt as f32;
        self.velocity *= 0.85;

        let (collide_x, collide_y, collide_z) = world.player_collision_mask(
            self.old_position,
            self.position,
            self.width(),
            self.height(),
        );

        if collide_y {
            self.position.y = self.old_position.y;
            self.jumping = false;
            self.velocity.y = 0.0;
        } else {
            self.jumping = true;
        }

        if collide_x || collide_z {
            let mut stepped_pos = self.old_position;
            stepped_pos.y += 0.55;
            stepped_pos.x = self.position.x;
            stepped_pos.z = self.position.z;

            if !world.is_player_colliding(stepped_pos, self.width(), self.height()) && !self.jumping
            {
                self.position = stepped_pos;
            } else {
                if collide_x {
                    self.position.x = self.old_position.x;
                    self.velocity.x = 0.0;
                }
                if collide_z {
                    self.position.z = self.old_position.z;
                    self.velocity.z = 0.0;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum BillboardType {
    Explosion = 0,
}

impl BillboardType {
    pub fn uvs(&self) -> [Vec2; 2] {
        let tile_index = *self as u32;
        let tile_x = tile_index % 12;
        let tile_y = tile_index / 12;

        let uv_unit = 1.0 / 12.0;
        let uv_row_unit = 1.0 / 12.0;

        [
            vec2(tile_x as f32 * uv_unit, tile_y as f32 * uv_row_unit),
            vec2((tile_x + 1) as f32 * uv_unit, (tile_y + 1) as f32 * uv_row_unit),
        ]
    }

    pub fn spherical_billboard(&self) -> bool {
        match self {
            BillboardType::Explosion => true,
        }
    }
}

#[derive(Clone)]
pub struct Billboard {
    pub position: Vec3,
    pub size: f32,
    pub life: u32,
    pub kind: BillboardType,
    start_size: f32,
    shader_key: String,
    atlas_key: String,
}

impl Billboard {
    pub fn new(position: Vec3, size: f32, life: u32, kind: BillboardType, shader_key: &str, atlas_key: &str) -> Self {
        Billboard {
            position,
            size,
            life,
            kind,
            start_size: size,
            shader_key: shader_key.to_string(),
            atlas_key: atlas_key.to_string(),
        }
    }
}

impl Entity for Billboard {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn velocity(&self) -> Vec3 {
        Vec3::ZERO
    }

    fn apply_velocity(&mut self, _delta: Vec3) {
        // Billboards do not move
    }

    fn width(&self) -> f32 {
        self.size
    }

    fn height(&self) -> f32 {
        self.size
    }

    fn eye_height(&self) -> f32 {
        self.size / 2.0
    }

    fn update(&mut self, _world: &mut World, _events: &[glfw::WindowEvent], _dt: f64) {
        if self.life > 0 {
            self.life -= 1;
        }
        // update the size based on remaining life
        if self.life > 0 {
            self.size = self.start_size * (self.life as f32 / 30.0);
        }
    }

    fn draw(&self, world: &World, resource_manager: &ResourceManager) {
        if self.life == 0 {
            return;
        }
        let shader = resource_manager.get::<ShaderProgram>(&self.shader_key).unwrap();
        let atlas = resource_manager.get::<Texture>(&self.atlas_key).unwrap();
        let uvs = self.kind.uvs();

        let view = Mat4::look_at_rh(
            world.get_player().camera_pos(),
            world.get_player().camera_pos() + world.get_player().forward,
            world.get_player().up,
        );
        let projection = world.get_player().projection;

        let vertices = vec![
            BillboardVertex {
                corner: vec2(-1.0, -1.0),
                uv: vec2(uvs[0].x, uvs[1].y),
            },
            BillboardVertex {
                corner: vec2(1.0, -1.0),
                uv: vec2(uvs[1].x, uvs[1].y),
            },
            BillboardVertex {
                corner: vec2(1.0, 1.0),
                uv: vec2(uvs[1].x, uvs[0].y),
            },
            BillboardVertex {
                corner: vec2(-1.0, 1.0),
                uv: vec2(uvs[0].x, uvs[0].y),
            },
        ];
        let indices = [0, 1, 2, 0, 2, 3];
        let mesh = Mesh::new(&vertices, &indices, DrawMode::Triangles);

        shader.use_program();
        shader.set_uniform("view", view);
        shader.set_uniform("projection", projection);
        shader.set_uniform("center", self.position);
        shader.set_uniform("size", self.size);
        shader.set_uniform("spherical", self.kind.spherical_billboard());
        atlas.bind_to_unit(0);
        shader.set_uniform("texture_sampler", 0);
        mesh.draw();
    }
}

pub struct World {
    chunks: HashMap<IVec3, Chunk>,
    changes: HashMap<(IVec3, IVec3), Block>,
    entities: Vec<Rc<RefCell<dyn Entity>>>,
    pub meshes: HashMap<IVec3, Mesh<BlockVertex>>,
    noise: OpenSimplex,
    cave_noise: OpenSimplex,
    biome_noise: OpenSimplex,
    pub resource_mgr: ResourceManager,
}

impl World {
    pub fn new(seed: u32, resource_mgr: ResourceManager) -> Self {
        let noise = OpenSimplex::new(seed);
        let cave_noise = OpenSimplex::new(seed.wrapping_add(u32::MAX / 3));
        const TWO_THIRDS_U32: u32 = (u32::MAX as f32 * (2.0 / 3.0)) as u32;
        let biome_noise = OpenSimplex::new(seed.wrapping_add(TWO_THIRDS_U32));

        let mut chunks = HashMap::new();
        for x in -3..=3 {
            for y in -1..=3 {
                for z in -3..=3 {
                    let res = Chunk::new(x, y, z, &noise, &cave_noise, &biome_noise);
                    chunks.insert(ivec3(x, y, z), res.0);
                }
            }
        }

        let player = Player::new(vec3(0.0, 10.0, 0.0));

        World {
            chunks,
            changes: HashMap::new(),
            entities: vec![Rc::new(RefCell::new(player))],
            meshes: HashMap::new(),
            noise,
            cave_noise,
            biome_noise,
            resource_mgr,
        }
    }

    pub fn get_player(&self) -> Ref<Player> {
        for entity in &self.entities {
            if entity.borrow().as_any().is::<Player>() {
                return Ref::map(entity.borrow(), |e| {
                    e.as_any().downcast_ref::<Player>().unwrap()
                });
            }
        }
        panic!("No player found");
    }

    pub fn get_player_mut(&mut self) -> RefMut<Player> {
        for entity in &self.entities {
            if entity.borrow().as_any().is::<Player>() {
                return RefMut::map(entity.borrow_mut(), |e| {
                    e.as_any_mut().downcast_mut::<Player>().unwrap()
                });
            }
        }
        panic!("No player found");
    }

    pub fn seed(&self) -> u32 {
        self.noise.seed()
    }

    pub fn noise(&self) -> Arc<OpenSimplex> {
        self.noise.into()
    }

    pub fn cave_noise(&self) -> Arc<OpenSimplex> {
        self.cave_noise.into()
    }

    pub fn biome_noise(&self) -> Arc<OpenSimplex> {
        self.biome_noise.into()
    }

    pub fn update(&mut self, events: &[glfw::WindowEvent], dt: f64) {
        let player_pos = self.get_player().position();
        self.chunks.retain(|pos, _| {
            let distance_squared = pos
                .as_vec3()
                .distance_squared(player_pos / CHUNK_SIZE as f32);
            distance_squared <= RENDER_DISTANCE as f32 * RENDER_DISTANCE as f32
        });
        for entity in self.entities.clone() {
            entity.borrow_mut().update(self, events, dt);
        }
    }

    pub fn chunk_exists(&self, x: i32, y: i32, z: i32) -> bool {
        self.chunks.contains_key(&ivec3(x, y, z))
    }

    pub fn add_chunk(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        chunk: Chunk,
        outside_blocks: HashMap<(IVec3, IVec3), Block>,
    ) {
        let mut chunk = chunk;
        for local_x in 0..CHUNK_SIZE {
            for local_y in 0..CHUNK_SIZE {
                for local_z in 0..CHUNK_SIZE {
                    if let Some(block) = self.changes.get(&(
                        ivec3(x, y, z),
                        ivec3(local_x as i32, local_y as i32, local_z as i32),
                    )) {
                        chunk.set_block(local_x, local_y, local_z, *block);
                    }
                }
            }
        }
        for ((chunk_pos, pos), block) in outside_blocks.into_iter() {
            let local_pos = pos.rem_euclid(IVec3::splat(CHUNK_SIZE as i32));

            self.get_chunk(chunk_pos.x, chunk_pos.y, chunk_pos.z)
                .set_block(
                    local_pos.x as usize,
                    local_pos.y as usize,
                    local_pos.z as usize,
                    block,
                );
        }
        self.chunks.insert(ivec3(x, y, z), chunk);
    }

    pub fn add_entity(&mut self, entity: impl Entity) {
        self.entities.push(Rc::new(RefCell::new(entity)));
    }

    pub fn get_chunk(&mut self, x: i32, y: i32, z: i32) -> &mut Chunk {
        self.chunks.entry(ivec3(x, y, z)).or_insert_with(|| {
            let res = Chunk::new(x, y, z, &self.noise, &self.cave_noise, &self.biome_noise);
            res.0
        })
    }

    pub fn get_block(&mut self, x: i32, y: i32, z: i32) -> Block {
        let chunk_x = x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = y.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = z.div_euclid(CHUNK_SIZE as i32);

        let chunk = self.get_chunk(chunk_x, chunk_y, chunk_z);
        let local_x = (x.rem_euclid(CHUNK_SIZE as i32)) as usize;
        let local_y = (y.rem_euclid(CHUNK_SIZE as i32)) as usize;
        let local_z = (z.rem_euclid(CHUNK_SIZE as i32)) as usize;

        chunk.get_block(local_x, local_y, local_z).to_owned()
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) {
        let chunk_x = x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = y.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = z.div_euclid(CHUNK_SIZE as i32);

        let chunk = self.get_chunk(chunk_x, chunk_y, chunk_z);
        let local_x = (x.rem_euclid(CHUNK_SIZE as i32)) as usize;
        let local_y = (y.rem_euclid(CHUNK_SIZE as i32)) as usize;
        let local_z = (z.rem_euclid(CHUNK_SIZE as i32)) as usize;

        chunk.set_block(local_x, local_y, local_z, block);
        self.changes.insert(
            (
                ivec3(chunk_x, chunk_y, chunk_z),
                ivec3(local_x as i32, local_y as i32, local_z as i32),
            ),
            block,
        );

        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                for dz in -1i32..=1 {
                    if dx.abs() + dy.abs() + dz.abs() != 1 {
                        continue;
                    }
                    self.get_chunk(dx + chunk_x, dy + chunk_y, dz + chunk_z)
                        .is_dirty = true;
                }
            }
        }
    }

    pub fn break_block(&mut self, pos: IVec3) {
        let block = self.get_block(pos.x, pos.y, pos.z);
        if block == Block::Air || block == Block::Bedrock {
            return;
        }

        self.set_block(pos.x, pos.y, pos.z, Block::Air);

        if block == Block::Glungus {
            for dx in -2i32..=2 {
                for dy in -2i32..=2 {
                    for dz in -2i32..=2 {
                        if vec3(dx as f32, dy as f32, dz as f32).length_squared() > 4.0 {
                            continue;
                        }
                        let neighbor_pos = pos + ivec3(dx, dy, dz);
                        self.break_block(neighbor_pos);
                    }
                }
            }
            let size = rand::random_range(1.0..2.0);
            self.add_entity(
                Billboard::new(
                    pos.as_vec3() + vec3(0.5, 0.5, 0.5),
                    size,
                    size as u32 * 25,
                    BillboardType::Explosion,
                    "billboard_shader",
                    "billboard_atlas",
                ),
            );
        }
    }

    pub fn is_player_colliding(
        &mut self,
        player_pos: Vec3,
        player_width: f32,
        player_height: f32,
    ) -> bool {
        let half_w = player_width / 2.0;

        let x_min = (player_pos.x - half_w).floor() as i32;
        let y_min = player_pos.y.floor() as i32;
        let z_min = (player_pos.z - half_w).floor() as i32;
        let x_max = (player_pos.x + half_w).floor() as i32;
        let y_max = (player_pos.y + player_height).floor() as i32;
        let z_max = (player_pos.z + half_w).floor() as i32;

        // let model_defs = self.model_defs.clone();
        let model_defs = self.resource_mgr.get::<ModelDefs>("model_defs").unwrap().clone();

        for x in x_min..=x_max {
            for y in y_min..=y_max {
                for z in z_min..=z_max {
                    let block = self.get_block(x, y, z);
                    let solid = {
                        let local_x = player_pos.x - x as f32;
                        let local_y = player_pos.y - y as f32;
                        let local_z = player_pos.z - z as f32;
                        block.is_solid_at(
                            &model_defs,
                            vec3(local_x, local_y, local_z),
                            player_width,
                            player_height,
                        )
                    };
                    if solid {
                        return true; // found a collision
                    }
                }
            }
        }

        false
    }

    pub fn player_collision_mask(
        &mut self,
        old_pos: Vec3,
        new_pos: Vec3,
        player_width: f32,
        player_height: f32,
    ) -> (bool, bool, bool) {
        let mut pos = old_pos;
        let mut collided = (false, false, false);

        // Try X movement
        pos.x = new_pos.x;
        if self.is_player_colliding(pos, player_width, player_height) {
            collided.0 = true;
            pos.x = old_pos.x;
        }

        // Try Y movement
        pos.y = new_pos.y;
        if self.is_player_colliding(pos, player_width, player_height) {
            collided.1 = true;
            pos.y = old_pos.y;
        }

        // Try Z movement
        pos.z = new_pos.z;
        if self.is_player_colliding(pos, player_width, player_height) {
            collided.2 = true;
            pos.z = old_pos.z;
        }

        collided
    }

    pub fn draw_entities(&self) {
        for entity in &self.entities {
            entity.borrow().draw(self, &self.resource_mgr);
        }
    }

    pub fn generate_meshes(&mut self, vp: Mat4) {
        struct ChunkMeshData {
            pos: IVec3,
            verts: Vec<BlockVertex>,
            idxs: Vec<u32>,
        }

        let frustum = extract_frustum_planes(vp);
        let results: Vec<_> = self
            .chunks
            .par_iter()
            .map(|(chunk_pos, chunk)| {
                let neighbour_chunks = NeighbourChunks {
                    n: self.chunks.get(&(chunk_pos + ivec3(0, 0, -1))),
                    s: self.chunks.get(&(chunk_pos + ivec3(0, 0, 1))),
                    e: self.chunks.get(&(chunk_pos + ivec3(1, 0, 0))),
                    w: self.chunks.get(&(chunk_pos + ivec3(-1, 0, 0))),
                    u: self.chunks.get(&(chunk_pos + ivec3(0, 1, 0))),
                    d: self.chunks.get(&(chunk_pos + ivec3(0, -1, 0))),
                };
                let pos = *chunk_pos;
                let (verts, idxs) = chunk.generate_chunk_mesh(&neighbour_chunks, &self.resource_mgr.get::<ModelDefs>("model_defs").unwrap());
                ChunkMeshData { pos, verts, idxs }
            })
            .collect();
        let results: HashMap<_, _> = results
            .into_iter()
            .filter_map(|data| {
                let pos = data.pos;
                let verts = data.verts;
                let idxs = data.idxs;
                let chunk = self.get_chunk(pos.x, pos.y, pos.z);
                let mesh_arc = Arc::new((verts, idxs));
                if chunk.is_dirty {
                    chunk.cached_mesh = Some(mesh_arc.clone());
                }
                chunk.is_dirty = false;
                let min = pos.as_vec3() * CHUNK_SIZE as f32;
                let max = min + vec3(CHUNK_SIZE as f32, CHUNK_SIZE as f32, CHUNK_SIZE as f32);
                if mesh_arc.0.is_empty()
                    || mesh_arc.1.is_empty()
                    || !aabb_in_frustum(min, max, &frustum)
                {
                    None
                } else {
                    Some((
                        pos,
                        Mesh::new(&mesh_arc.0, &mesh_arc.1, DrawMode::Triangles),
                    ))
                }
            })
            .collect();

        self.meshes = results;
    }
}

#[derive(Debug, Clone)]
pub struct RayHit {
    pub block_pos: IVec3,
    pub face_normal: IVec3,
}

pub fn cast_ray(
    world: &mut World,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<RayHit> {
    let mut pos = origin;
    let step = 0.01;

    for _ in 0..(max_distance / step) as usize {
        let block_pos = pos.floor().as_ivec3();
        let block = world.get_block(block_pos.x, block_pos.y, block_pos.z);

        if block != Block::Air {
            let normal = calc_face_normal(pos, block_pos.as_vec3());
            return Some(RayHit {
                block_pos,
                face_normal: normal,
            });
        }

        pos += direction * step;
    }

    None
}

fn calc_face_normal(hit: Vec3, block: Vec3) -> IVec3 {
    let rel = hit - block;

    // Distances to faces
    let dx = rel.x.min(1.0 - rel.x).abs();
    let dy = rel.y.min(1.0 - rel.y).abs();
    let dz = rel.z.min(1.0 - rel.z).abs();

    let min = dx.min(dy.min(dz));

    if min == dx {
        if rel.x < 0.5 {
            ivec3(-1, 0, 0)
        } else {
            ivec3(1, 0, 0)
        }
    } else if min == dy {
        if rel.y < 0.5 {
            ivec3(0, -1, 0)
        } else {
            ivec3(0, 1, 0)
        }
    } else if rel.z < 0.5 {
        ivec3(0, 0, -1)
    } else {
        ivec3(0, 0, 1)
    }
}

pub fn cloud_texture_gen(texture_size: UVec2, seed: u32) -> Texture {
    let noise = OpenSimplex::new(seed);
    let width = texture_size.x;
    let height = texture_size.y;
    let mut image_data = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let nx = x as f64 / width as f64 - 0.5;
            let ny = y as f64 / height as f64 - 0.5;

            fn fractal_noise(
                noise: &OpenSimplex,
                x: f64,
                y: f64,
                octaves: i32,
                persistence: f64,
                lacunarity: f64,
            ) -> f64 {
                let mut amplitude = 1.0;
                let mut frequency = 1.0;
                let mut value = 0.0;
                let mut max_value = 0.0;

                for _ in 0..octaves {
                    value += noise.get([x * frequency, y * frequency]) * amplitude;
                    max_value += amplitude;
                    amplitude *= persistence;
                    frequency *= lacunarity;
                }

                value / max_value
            }

            let noise_value = fractal_noise(&noise, nx * 30.0, ny * 15.0, 5, 0.5, 2.0);
            let alpha = if noise_value > 0.0 { 1.0 } else { 0.0 };

            let idx = ((y * width + x) * 4) as usize;
            image_data[idx] = 255;
            image_data[idx + 1] = 255;
            image_data[idx + 2] = 255;
            image_data[idx + 3] = (alpha * 255.0) as u8;
        }
    }

    Texture::new(width, height, &image_data)
}

pub fn make_cloud_plane() -> Mesh<CloudPlaneVertex> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let positions = [
        vec2(-1.0, -1.0),
        vec2(1.0, -1.0),
        vec2(1.0, 1.0),
        vec2(-1.0, 1.0),
    ];
    let uvs = [
        vec2(0.0, 0.0),
        vec2(1.0, 0.0),
        vec2(1.0, 1.0),
        vec2(0.0, 1.0),
    ];

    for i in 0..4 {
        vertices.push(CloudPlaneVertex {
            position: positions[i],
            uv: uvs[i],
        });
    }

    indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);

    Mesh::new(&vertices, &indices, DrawMode::Triangles)
}
