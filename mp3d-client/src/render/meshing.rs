//! All utilities related to meshing worlds and chunks.

use std::{collections::HashMap, sync::Arc};

use glam::{IVec3, Vec2, Vec3};
use glow::HasContext;
use mp3d_core::{block::{Block, BlockState}, world::chunk::CHUNK_SIZE};

use crate::{
    abs::{Mesh, Vertex},
    client::{chunk::ClientChunk, world::ClientWorld},
};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ChunkVertex {
    pub position: Vec3,
    pub normal: IVec3,
    pub uv: Vec2,
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

            // UV attribute
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                2,
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
fn should_occlude(
    a: &Block,
    b: &Block,
    face_idx: usize,
    a_model: &crate::resource::block::BlockModel,
    b_model: &crate::resource::block::BlockModel,
) -> bool {
    if !a.visible {
        unreachable!("Invisible blocks have no faces");
    }
    if !b.visible {
        return false;
    }

    for a_el in &a_model.elements {
        if !a_el.faces[face_idx].occludes {
            continue;
        }
        for b_el in &b_model.elements {
            if b_el.faces[face_idx ^ 1].cullable {
                return true;
            }
        }
    }

    false
}

/// The vertex positions for each face of a cube, in the order of NSEWUD.
const FACE_VERTS: [[Vec3; 4]; 6] = [
    // North (-Z)
    [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
    ],
    // South (+Z)
    [
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
    ],
    // East (+X)
    [
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 1.0),
    ],
    // West (-X)
    [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
    ],
    // Up (+Y)
    [
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ],
    // Down (-Y)
    [
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
    ],
];

/// The normal vectors for each face of a cube, in the order of NSEWUD.
const NORMALS: [IVec3; 6] = [
    IVec3::new(0, 0, -1), // North
    IVec3::new(0, 0, 1),  // South
    IVec3::new(1, 0, 0),  // East
    IVec3::new(-1, 0, 0), // West
    IVec3::new(0, 1, 0),  // Up
    IVec3::new(0, -1, 0), // Down
];

/// Generates meshes for all chunks that require being meshed again.
pub fn mesh_world(
    gl: &Arc<glow::Context>,
    world: &mut ClientWorld,
    chunk_meshes: &mut HashMap<IVec3, Mesh>,
    block_textures: &crate::resource::block::TextureAtlas,
    block_models: &HashMap<(&'static str, &'static str), crate::resource::block::BlockModel>,
) {
    use rayon::prelude::*;

    let remesh_queue = std::mem::take(&mut world.remesh_queue);

    let world_ref = &*world;

    let new_meshes: Vec<(IVec3, Vec<ChunkVertex>, Vec<u32>)> = remesh_queue
        .par_iter()
        .map(|chunk_pos| {
            let chunk_pos = *chunk_pos;
            let chunk = world_ref.chunks.get(&chunk_pos).unwrap();
            let (chunk_vertices, chunk_indices) =
                mesh_chunk(chunk, chunk_pos, world_ref, block_textures, block_models);
            (chunk_pos, chunk_vertices, chunk_indices)
        })
        .collect();

    for (chunk_pos, chunk_vertices, chunk_indices) in new_meshes {
        world.chunks.get_mut(&chunk_pos).unwrap().dirty = false;
        let mesh = Mesh::new(gl, &chunk_vertices, &chunk_indices, glow::TRIANGLES);
        chunk_meshes.insert(chunk_pos, mesh);
    }
}

/// Generates the mesh for a single chunk at the given position in the world.
/// Returns a tuple containing the list of vertices and the list of indices.
fn mesh_chunk(
    chunk: &ClientChunk,
    chunk_pos: glam::IVec3,
    world: &ClientWorld,
    block_textures: &crate::resource::block::TextureAtlas,
    block_models: &HashMap<(&'static str, &'static str), crate::resource::block::BlockModel>,
) -> (Vec<ChunkVertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(20_000);
    let mut indices = Vec::with_capacity(30_000);

    fn get_block<'a>(
        chunk: &'a ClientChunk,
        world: &'a ClientWorld,
        chunk_pos: IVec3,
        world_pos: IVec3,
    ) -> Option<(&'a Block, &'a BlockState)> {
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

    fn ident(block: &Block, state: &BlockState) -> (&'static str, &'static str) {
        (block.ident, state.to_ident().unwrap_or_else(|| {
            panic!(
                "Block '{}' has an unrecognized block state type: {}",
                block.ident, block.state_type
            )
        }))
    }

    for x in 0..(CHUNK_SIZE as i32) {
        let world_x = chunk_pos.x * (CHUNK_SIZE as i32) + x;
        for y in 0..(CHUNK_SIZE as i32) {
            let world_y = chunk_pos.y * (CHUNK_SIZE as i32) + y;
            for z in 0..(CHUNK_SIZE as i32) {
                // Check if the block is visible
                let block_local_pos = glam::IVec3::new(x, y, z);
                let (block, state) = chunk.get_block(block_local_pos);
                if !block.visible {
                    continue;
                }

                let model = block_models.get(&ident(block, state)).unwrap_or_else(|| {
                    panic!(
                        "No model found for block {} with state {}",
                        block.ident,
                        state.to_ident().unwrap()
                    )
                });

                let world_z = chunk_pos.z * (CHUNK_SIZE as i32) + z;

                // Create faces for each non-occluded side
                for (i, (dx, dy, dz)) in NORMALS.iter().map(|n| (n.x, n.y, n.z)).enumerate() {
                    if (dx.abs() + dy.abs() + dz.abs()) != 1 {
                        continue;
                    }

                    let neighbor_pos = glam::IVec3::new(world_x + dx, world_y + dy, world_z + dz);

                    // Create face the neighboring block is air or doesn't occlude this face.
                    let neighbor_block = get_block(chunk, world, chunk_pos, neighbor_pos);
                    let neighbor_state = neighbor_block.map(|(_, state)| state);
                    let neighbor_block = neighbor_block.map(|(block, _)| block);
                    // let neighbor_model = neighbor_block.and_then(|b| block_models.get(b.ident));
                    let neighbor_model = neighbor_block
                        .and_then(|b| neighbor_state.map(|s| ident(b, s)))
                        .and_then(|ident| block_models.get(&ident));
                    if neighbor_block.is_none()
                        || !should_occlude(
                            block,
                            neighbor_block.unwrap(),
                            i,
                            model,
                            neighbor_model.unwrap(),
                        )
                    {
                        for el in &model.elements {
                            // The elements' faces are ordered as NSEWUD and we are using a
                            // right handed coordinate system with +X = east, +Y = up, +Z =
                            // south.
                            let face = match (dx, dy, dz) {
                                (0, 0, -1) => &el.faces[0], // North
                                (0, 0, 1) => &el.faces[1],  // South
                                (1, 0, 0) => &el.faces[2],  // East
                                (-1, 0, 0) => &el.faces[3], // West
                                (0, 1, 0) => &el.faces[4],  // Up
                                (0, -1, 0) => &el.faces[5], // Down
                                _ => unreachable!(),
                            };

                            let model_uv = face.uv;
                            let [uv_min, uv_max] =
                                block_textures.get_uv(&face.texture_name, model_uv).unwrap();

                            let base_index = vertices.len() as u32;
                            let uvs = [
                                Vec2::new(uv_max.x, uv_min.y),
                                Vec2::new(uv_min.x, uv_min.y),
                                Vec2::new(uv_min.x, uv_max.y),
                                Vec2::new(uv_max.x, uv_max.y),
                            ];
                            for (vert, uv) in
                                FACE_VERTS[i].iter().zip(uvs.iter())
                            {
                                vertices.push(ChunkVertex {
                                    position: *vert * (el.to - el.from)
                                        + el.from
                                        + Vec3::new(world_x as f32, world_y as f32, world_z as f32),
                                    normal: IVec3::new(dx, dy, dz),
                                    uv: *uv,
                                });
                            }

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

    (vertices, indices)
}
