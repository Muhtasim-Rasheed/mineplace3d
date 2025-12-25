use std::{
    collections::HashSet,
    sync::{Arc, mpsc},
};

use fastnoise_lite::{FastNoiseLite, NoiseType};
use glam::*;
use glow::HasContext;

use crate::{
    game::{BlockType, CHUNK_SIZE, ChunkTask, RENDER_DISTANCE, World},
    mesh::{Mesh, Vertex},
    texture::Texture,
};

const CHUNK_RADIUS: i32 = RENDER_DISTANCE as i32 - 1;

#[inline]
pub fn pack_uv(uv: UVec2) -> u64 {
    ((uv.x << 5) | uv.y) as u64
}

#[inline]
pub fn pack_block_pos(pos: UVec3) -> u64 {
    ((pos.x << 8) | (pos.y << 4) | pos.z) as u64
}

#[inline]
pub fn pack_color_rgb677(color: Vec3) -> u64 {
    let r = (color.x * 63.0).round() as u64; // 6 bits
    let g = (color.y * 127.0).round() as u64; // 7 bits
    let b = (color.z * 127.0).round() as u64; // 7 bits
    (r << 14) | (g << 7) | b
}

#[inline]
pub fn mask_partial(bits: u32) -> u32 {
    (bits >> 16) & 0x000F
}

#[inline]
pub fn collision_aabb(min_a: Vec3, max_a: Vec3, min_b: Vec3, max_b: Vec3) -> bool {
    (min_a.x <= max_b.x && max_a.x >= min_b.x)
        && (min_a.y <= max_b.y && max_a.y >= min_b.y)
        && (min_a.z <= max_b.z && max_a.z >= min_b.z)
}

#[inline]
pub fn extract_frustum_planes(pv: Mat4) -> [Vec4; 6] {
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
pub fn aabb_in_frustum(min: Vec3, max: Vec3, planes: &[Vec4; 6]) -> bool {
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

        if block.block_type() != BlockType::Air {
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

pub fn request_chunks_around_player(
    player_pos: Vec3,
    world: &mut World,
    task_sender: &mpsc::Sender<ChunkTask>,
    queued_chunks: &mut HashSet<IVec3>,
) {
    let player_chunk = (player_pos / CHUNK_SIZE as f32).floor().as_ivec3();

    for x in -CHUNK_RADIUS..=CHUNK_RADIUS {
        for y in -CHUNK_RADIUS..=CHUNK_RADIUS {
            for z in -CHUNK_RADIUS..=CHUNK_RADIUS {
                let offset = ivec3(x, y, z);
                let chunk_pos = player_chunk + offset;

                if offset.length_squared() > (CHUNK_RADIUS * CHUNK_RADIUS) {
                    continue;
                }

                if !world.chunk_exists(chunk_pos.x, chunk_pos.y, chunk_pos.z)
                    && !queued_chunks.contains(&chunk_pos)
                {
                    task_sender
                        .send(ChunkTask::Generate {
                            cx: chunk_pos.x,
                            cy: chunk_pos.y,
                            cz: chunk_pos.z,
                            noise: world.noise(),
                            cave_noise: world.cave_noise(),
                            biome_noise: world.biome_noise(),
                        })
                        .unwrap();
                    queued_chunks.insert(chunk_pos);
                }
            }
        }
    }
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

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct CloudPlaneVertex {
    pub position: Vec2,
    pub uv: Vec2,
}

impl Vertex for CloudPlaneVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<CloudPlaneVertex>() as i32;

            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(0);

            gl.vertex_attrib_pointer_f32(
                1,
                2,
                glow::FLOAT,
                false,
                stride,
                std::mem::size_of::<Vec2>() as i32,
            );
            gl.enable_vertex_attrib_array(1);
        }
    }
}

pub fn cloud_texture_gen(gl: &Arc<glow::Context>, texture_size: UVec2, seed: i32) -> Texture {
    let mut noise = FastNoiseLite::new();
    noise.set_seed(Some(seed));
    noise.set_noise_type(Some(NoiseType::OpenSimplex2));
    noise.set_frequency(Some(0.4));
    let width = texture_size.x;
    let height = texture_size.y;
    let mut image_data = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let nx = x as f32 / width as f32 - 0.5;
            let ny = y as f32 / height as f32 - 0.5;

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

            let noise_value = fractal_noise(&noise, nx * 30.0, ny * 15.0, 5, 0.5, 2.0);
            let alpha = if noise_value > 0.0 { 1.0 } else { 0.0 };

            let idx = ((y * width + x) * 4) as usize;
            image_data[idx] = 255;
            image_data[idx + 1] = 255;
            image_data[idx + 2] = 255;
            image_data[idx + 3] = (alpha * 255.0) as u8;
        }
    }

    Texture::new(
        gl,
        &image::DynamicImage::ImageRgba8(
            image::ImageBuffer::from_raw(width, height, image_data).unwrap(),
        ),
    )
}

pub fn make_cloud_plane(gl: &Arc<glow::Context>) -> Mesh {
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

    Mesh::new(gl, &vertices, &indices, glow::TRIANGLES)
}
