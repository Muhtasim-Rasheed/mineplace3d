use std::sync::Arc;

use glam::*;
use glow::HasContext;

use crate::{
    abs::{Mesh, UIVertex, Vertex},
    game::{collision_aabb, mask_partial, pack_color_rgb677, pack_uv, Key, KeyPart, ModelDefs},
};

const FULL_BLOCK: u32 = 0x00000000;
const PARTIAL_SLAB_TOP: u32 = 0x00010000;
const PARTIAL_SLAB_BOTTOM: u32 = 0x00020000;
const PARTIAL_STAIRS_N: u32 = 0x00030000;
const PARTIAL_STAIRS_S: u32 = 0x00040000;
const PARTIAL_STAIRS_E: u32 = 0x00050000;
const PARTIAL_STAIRS_W: u32 = 0x00060000;
const BLOCK_MASK: u32 = 0x0000FFFF;

#[derive(Copy, Clone)]
pub struct Face {
    pub vertices: [Vec3; 4],
    pub uvs: [UVec2; 4],
}

#[derive(Copy, Clone)]
pub struct FaceTemplate {
    pub normal: IVec3,
    pub vertices: [IVec3; 4],
}

impl Face {
    pub fn use_template(template: FaceTemplate, from: Vec3, to: Vec3, uvs: [UVec2; 4]) -> Self {
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

pub const FACE_TEMPLATES: [FaceTemplate; 6] = [
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

#[inline(always)]
pub fn should_occlude(self_block: BlockType, neighbour: BlockType) -> bool {
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

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct BlockVertex {
    pub hi: u32,
    pub lo: u32,
    pub position: Vec3,
}

impl BlockVertex {
    pub fn new(position: Vec3, normal: u8, uv: UVec2, block_type: u16, foliage: Vec3) -> Self {
        let uv = pack_uv(uv);
        let foliage = pack_color_rgb677(foliage);
        let normal = normal as u64;
        let block_type = block_type as u64;
        // space for lighting stuff or anything really that fits in 15 bits
        let serialized = (normal << 15) | (uv << 18) | (block_type << 28) | (foliage << 44);
        BlockVertex {
            hi: (serialized >> 32) as u32,
            lo: (serialized & 0xFFFFFFFF) as u32,
            position,
        }
    }
}

impl Vertex for BlockVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<BlockVertex>() as i32;

            gl.vertex_attrib_pointer_i32(
                0,
                1,
                glow::UNSIGNED_INT,
                stride,
                0,
            );
            gl.enable_vertex_attrib_array(0);

            gl.vertex_attrib_pointer_i32(
                1,
                1,
                glow::UNSIGNED_INT,
                stride,
                std::mem::size_of::<u32>() as i32,
            );
            gl.enable_vertex_attrib_array(1);

            gl.vertex_attrib_pointer_f32(
                2,
                3,
                glow::FLOAT,
                false,
                stride,
                2 * std::mem::size_of::<u32>() as i32,
            );
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct OutlineVertex {
    pub position: Vec3,
}

impl Vertex for OutlineVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<OutlineVertex>() as i32;

            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                stride,
                0,
            );
            gl.enable_vertex_attrib_array(0);
        }
    }
}

pub fn outline_mesh(gl: &Arc<glow::Context>) -> Mesh {
    let vertices: [OutlineVertex; 8] = [
        OutlineVertex {
            position: vec3(0.0, 0.0, 0.0),
        },
        OutlineVertex {
            position: vec3(1.0, 0.0, 0.0),
        },
        OutlineVertex {
            position: vec3(1.0, 1.0, 0.0),
        },
        OutlineVertex {
            position: vec3(0.0, 1.0, 0.0),
        },
        OutlineVertex {
            position: vec3(0.0, 0.0, 1.0),
        },
        OutlineVertex {
            position: vec3(1.0, 0.0, 1.0),
        },
        OutlineVertex {
            position: vec3(1.0, 1.0, 1.0),
        },
        OutlineVertex {
            position: vec3(0.0, 1.0, 1.0),
        },
    ];

    let indices: [u32; 24] = [
        0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
    ];

    Mesh::new(gl, &vertices, &indices, glow::LINES)
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

    pub fn ui_mesh(&self, gl: &Arc<glow::Context>, from: Vec2, to: Vec2, m: Mat4, model_defs: &ModelDefs) -> Mesh {
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

        Mesh::new(gl, &vertices, &indices, glow::TRIANGLES)
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
            ($name:literal) => {
                model_defs
                    .get($name)
                    .unwrap()
                    .uvs
                    .iter()
                    .map(|face_uvs| faces_uvs(face_uvs))
                    .collect::<Vec<_>>()
            };
        }

        match partial_bits {
            0 => get_uvs!("full"),
            1 => get_uvs!("slab_top"),
            2 => get_uvs!("slab_bottom"),
            3 => get_uvs!("stairs_n"),
            4 => get_uvs!("stairs_s"),
            5 => get_uvs!("stairs_e"),
            6 => get_uvs!("stairs_w"),
            _ => unreachable!(),
        }
    }

    pub fn cubes(&self, model_defs: &ModelDefs) -> Vec<[Vec3; 2]> {
        macro_rules! get_cubes {
            ($name:literal) => {
                model_defs.get($name).unwrap().cubes.clone()
            };
        }

        if *self == Block::Air {
            return vec![];
        }

        let partial_bits = mask_partial(*self as u32);
        match partial_bits {
            0 => get_cubes!("full"),
            1 => get_cubes!("slab_top"),
            2 => get_cubes!("slab_bottom"),
            3 => get_cubes!("stairs_n"),
            4 => get_cubes!("stairs_s"),
            5 => get_cubes!("stairs_e"),
            6 => get_cubes!("stairs_w"),
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
