//! All utilities related to meshing worlds and chunks.

use std::{collections::HashMap, sync::Arc};

use glam::{IVec3, Vec3};
use glow::HasContext;
use mp3d_core::{
    block::Block,
    world::{
        World,
        chunk::{CHUNK_SIZE, Chunk},
    },
};

use crate::abs::{Mesh, Vertex};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ChunkVertex {
    pub position: Vec3,
    pub normal: IVec3,
    pub color: Vec3,
}

impl Vertex for ChunkVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<ChunkVertex>() as i32;

            // Position attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);

            // Normal attribute
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_i32(1, 3, glow::INT, stride, size_of::<Vec3>() as i32);

            // Color attribute
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                3,
                glow::FLOAT,
                false,
                stride,
                (size_of::<Vec3>() + size_of::<IVec3>()) as i32,
            );
        }
    }
}

/// Determines if a certain face of block `a` should be occluded by block `b`.
#[inline]
fn should_occlude(a: &Block, b: &Block) -> bool {
    a.full && b.full
}

/// Generates meshes for all chunks in the given world.
/// Returns a hashmap mapping chunk positions to their corresponding meshes.
pub fn mesh_world(gl: &Arc<glow::Context>, world: &World) -> HashMap<IVec3, Mesh> {
    let start = std::time::Instant::now();

    let mut meshes = HashMap::with_capacity(world.chunks.len());

    for (chunk_pos, chunk) in &world.chunks {
        let (chunk_vertices, chunk_indices) = mesh_chunk(chunk, *chunk_pos, world);

        let mesh = Mesh::new(gl, &chunk_vertices, &chunk_indices, glow::TRIANGLES);
        meshes.insert(*chunk_pos, mesh);
    }

    println!("Generated world mesh in {:?}", start.elapsed());

    meshes
}

/// Generates the mesh for a single chunk at the given position in the world.
/// Returns a tuple containing the list of vertices and the list of indices.
fn mesh_chunk(
    chunk: &Chunk,
    chunk_pos: glam::IVec3,
    world: &World,
) -> (Vec<ChunkVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    fn get_block<'a>(
        chunk: &'a Chunk,
        world: &'a World,
        chunk_pos: IVec3,
        world_pos: IVec3,
    ) -> Option<&'a Block> {
        let local_x = world_pos.x - chunk_pos.x * (CHUNK_SIZE as i32);
        let local_y = world_pos.y - chunk_pos.y * (CHUNK_SIZE as i32);
        let local_z = world_pos.z - chunk_pos.z * (CHUNK_SIZE as i32);

        if local_x >= 0
            && local_x < CHUNK_SIZE as i32
            && local_y >= 0
            && local_y < CHUNK_SIZE as i32
            && local_z >= 0
            && local_z < CHUNK_SIZE as i32
        {
            let local_pos = IVec3::new(local_x, local_y, local_z);
            Some(chunk.get_block(local_pos))
        } else {
            world.get_block_at(world_pos)
        }
    }

    for x in 0..(CHUNK_SIZE as i32) {
        for y in 0..(CHUNK_SIZE as i32) {
            for z in 0..(CHUNK_SIZE as i32) {
                // Check if the block is full
                let block_local_pos = glam::IVec3::new(x, y, z);
                let block = chunk.get_block(block_local_pos);
                if !block.full {
                    continue;
                }

                // Calculate world position of the block
                let world_x = chunk_pos.x * (CHUNK_SIZE as i32) + x;
                let world_y = chunk_pos.y * (CHUNK_SIZE as i32) + y;
                let world_z = chunk_pos.z * (CHUNK_SIZE as i32) + z;

                // Create faces for each non-occluded side
                for dx in -1_i32..=1 {
                    for dy in -1_i32..=1 {
                        for dz in -1_i32..=1 {
                            if (dx.abs() + dy.abs() + dz.abs()) != 1 {
                                continue;
                            }

                            let neighbor_pos =
                                glam::IVec3::new(world_x + dx, world_y + dy, world_z + dz);

                            // Create face if neighbor block is non-full or out of bounds
                            // let neighbor_block = world.get_block_at(neighbor_pos);
                            let neighbor_block = get_block(chunk, world, chunk_pos, neighbor_pos);
                            if neighbor_block.is_none()
                                || !should_occlude(block, neighbor_block.unwrap())
                            {
                                // Add face
                                let face_vertices = match (dx, dy, dz) {
                                    (1, 0, 0) => vec![
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(1, 0, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32 + 1.0,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(1, 0, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32 + 1.0,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(1, 0, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(1, 0, 0),
                                            color: block.color,
                                        },
                                    ],
                                    (-1, 0, 0) => vec![
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(-1, 0, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32 + 1.0,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(-1, 0, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32 + 1.0,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(-1, 0, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(-1, 0, 0),
                                            color: block.color,
                                        },
                                    ],
                                    (0, 1, 0) => vec![
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32 + 1.0,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, 1, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32 + 1.0,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, 1, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32 + 1.0,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, 1, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32 + 1.0,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, 1, 0),
                                            color: block.color,
                                        },
                                    ],
                                    (0, -1, 0) => vec![
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, -1, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, -1, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, -1, 0),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, -1, 0),
                                            color: block.color,
                                        },
                                    ],
                                    (0, 0, 1) => vec![
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, 0, 1),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32 + 1.0,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, 0, 1),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32 + 1.0,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, 0, 1),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32,
                                                world_z as f32 + 1.0,
                                            ),
                                            normal: IVec3::new(0, 0, 1),
                                            color: block.color,
                                        },
                                    ],
                                    (0, 0, -1) => vec![
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, 0, -1),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32,
                                                world_y as f32 + 1.0,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, 0, -1),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32 + 1.0,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, 0, -1),
                                            color: block.color,
                                        },
                                        ChunkVertex {
                                            position: Vec3::new(
                                                world_x as f32 + 1.0,
                                                world_y as f32,
                                                world_z as f32,
                                            ),
                                            normal: IVec3::new(0, 0, -1),
                                            color: block.color,
                                        },
                                    ],
                                    _ => vec![],
                                };

                                let base_index = vertices.len() as u32;
                                vertices.extend(face_vertices);

                                indices.extend_from_slice(&[
                                    base_index,
                                    base_index + 1,
                                    base_index + 2,
                                    base_index,
                                    base_index + 2,
                                    base_index + 3,
                                ]);
                            }
                        }
                    }
                }
            }
        }
    }

    (vertices, indices)
}
