//! All utilities related to meshing worlds and chunks.

use std::{collections::HashMap, sync::Arc};

use glam::{IVec3, Vec2, Vec3};
use glow::HasContext;
use mp3d_core::{
    block::{BlockState, block_registry},
    direction::Direction,
    world::chunk::CHUNK_SIZE,
};

use crate::{
    abs::{Mesh, Vertex},
    client::{chunk::ClientChunk, world::ClientWorld},
    resource::block::{BlockFace, modelstore::BlockModelLoader},
};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ChunkVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub ao: u8,
}

impl Vertex for ChunkVertex {
    fn vertex_attribs(gl: &glow::Context) {
        unsafe {
            let stride = std::mem::size_of::<ChunkVertex>() as i32;
            let mut offset = 0;

            // Position attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, offset);
            offset += std::mem::size_of::<Vec3>() as i32;

            // Normal attribute
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, offset);
            offset += std::mem::size_of::<IVec3>() as i32;

            // UV attribute
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, offset);
            offset += std::mem::size_of::<Vec2>() as i32;

            // AO attribute
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_i32(3, 1, glow::UNSIGNED_BYTE, stride, offset);
        }
    }
}

/// True if rect `b` completely covers rect `a`.
#[inline]
fn covers(a_min: Vec2, a_max: Vec2, b_min: Vec2, b_max: Vec2) -> bool {
    b_min.cmple(a_min).all() && b_max.cmpge(a_max).all()
}

/// True if `a_face` (which points in `dir`) hidden by anything in the neighboring model
fn face_occluded(
    a_face: &BlockFace,
    dir: Direction,
    b_model: &crate::resource::block::BlockModel,
) -> bool {
    let Some(a_occ) = &a_face.occlusion_face else {
        return false;
    };
    let opp = dir.opposite();

    b_model
        .elements
        .iter()
        .flat_map(|e| &e.faces)
        .any(|b_face| {
            b_face.cull_dir == Some(opp)
                && b_face.occludes
                && b_face.occlusion_face.as_ref().is_some_and(|b_occ| {
                    covers(a_occ.rect[0], a_occ.rect[1], b_occ.rect[0], b_occ.rect[1])
                })
        })
}

#[inline]
fn block_is_full_cube(block: Option<BlockState>, block_model_loader: &BlockModelLoader) -> bool {
    let Some(block) = block else {
        return false;
    };

    let block_def = block_registry().get(block.block).unwrap();

    if !block_def.visible {
        return false;
    }

    block_model_loader
        .get(block)
        .is_some_and(|model| model.is_full_cube())
}

fn ao_for_vertex(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - (side1 as u8 + side2 as u8 + corner as u8)
    }
}

// For AO, precompute the 3 neighbor offsets for each vertex of each face.
// The first two coordinates are the side neighbors and the last coordinate is the corner neighbor.
const AO_NEIGHBORS: [[[IVec3; 3]; 4]; 6] = [
    // North face (-Z)
    [
        [
            IVec3::new(-1, 0, -1),
            IVec3::new(0, 1, -1),
            IVec3::new(-1, 1, -1),
        ],
        [
            IVec3::new(1, 0, -1),
            IVec3::new(0, 1, -1),
            IVec3::new(1, 1, -1),
        ],
        [
            IVec3::new(1, 0, -1),
            IVec3::new(0, -1, -1),
            IVec3::new(1, -1, -1),
        ],
        [
            IVec3::new(-1, 0, -1),
            IVec3::new(0, -1, -1),
            IVec3::new(-1, -1, -1),
        ],
    ],
    // South face (+Z)
    [
        [
            IVec3::new(1, 0, 1),
            IVec3::new(0, 1, 1),
            IVec3::new(1, 1, 1),
        ],
        [
            IVec3::new(-1, 0, 1),
            IVec3::new(0, 1, 1),
            IVec3::new(-1, 1, 1),
        ],
        [
            IVec3::new(-1, 0, 1),
            IVec3::new(0, -1, 1),
            IVec3::new(-1, -1, 1),
        ],
        [
            IVec3::new(1, 0, 1),
            IVec3::new(0, -1, 1),
            IVec3::new(1, -1, 1),
        ],
    ],
    // East face (+X)
    [
        [
            IVec3::new(1, 0, -1),
            IVec3::new(1, 1, 0),
            IVec3::new(1, 1, -1),
        ],
        [
            IVec3::new(1, 0, 1),
            IVec3::new(1, 1, 0),
            IVec3::new(1, 1, 1),
        ],
        [
            IVec3::new(1, 0, 1),
            IVec3::new(1, -1, 0),
            IVec3::new(1, -1, 1),
        ],
        [
            IVec3::new(1, 0, -1),
            IVec3::new(1, -1, 0),
            IVec3::new(1, -1, -1),
        ],
    ],
    // West face (-X)
    [
        [
            IVec3::new(-1, 0, 1),
            IVec3::new(-1, 1, 0),
            IVec3::new(-1, 1, 1),
        ],
        [
            IVec3::new(-1, 0, -1),
            IVec3::new(-1, 1, 0),
            IVec3::new(-1, 1, -1),
        ],
        [
            IVec3::new(-1, 0, -1),
            IVec3::new(-1, -1, 0),
            IVec3::new(-1, -1, -1),
        ],
        [
            IVec3::new(-1, 0, 1),
            IVec3::new(-1, -1, 0),
            IVec3::new(-1, -1, 1),
        ],
    ],
    // Up face (+Y)
    [
        [
            IVec3::new(1, 1, 0),
            IVec3::new(0, 1, -1),
            IVec3::new(1, 1, -1),
        ],
        [
            IVec3::new(-1, 1, 0),
            IVec3::new(0, 1, -1),
            IVec3::new(-1, 1, -1),
        ],
        [
            IVec3::new(-1, 1, 0),
            IVec3::new(0, 1, 1),
            IVec3::new(-1, 1, 1),
        ],
        [
            IVec3::new(1, 1, 0),
            IVec3::new(0, 1, 1),
            IVec3::new(1, 1, 1),
        ],
    ],
    // Down face (-Y)
    [
        [
            IVec3::new(1, -1, 0),
            IVec3::new(0, -1, 1),
            IVec3::new(1, -1, 1),
        ],
        [
            IVec3::new(-1, -1, 0),
            IVec3::new(0, -1, 1),
            IVec3::new(-1, -1, 1),
        ],
        [
            IVec3::new(-1, -1, 0),
            IVec3::new(0, -1, -1),
            IVec3::new(-1, -1, -1),
        ],
        [
            IVec3::new(1, -1, 0),
            IVec3::new(0, -1, -1),
            IVec3::new(1, -1, -1),
        ],
    ],
];

/// Generates meshes for all chunks that require being meshed again.
pub fn mesh_world(
    gl: &Arc<glow::Context>,
    world: &mut ClientWorld,
    chunk_meshes: &mut HashMap<IVec3, Mesh>,
    chunk_mesh_pool: &mut Vec<Mesh>,
    block_textures: &crate::resource::block::TextureAtlas,
    block_model_loader: &BlockModelLoader,
) {
    use rayon::prelude::*;

    const MAX_MESHES_PER_FRAME: usize = 12;

    if world.remesh_queue.is_empty() {
        return;
    }

    let batch_size = world.remesh_queue.len().min(MAX_MESHES_PER_FRAME);

    let batch: Vec<IVec3> = world.remesh_queue.drain(batch_size);

    let world_ref = &*world;

    let new_meshes: Vec<(IVec3, Vec<ChunkVertex>, Vec<u32>)> = batch
        .par_iter()
        .filter_map(|chunk_pos| {
            if let Some(chunk) = world_ref.chunks.get(chunk_pos) {
                let (chunk_vertices, chunk_indices) = mesh_chunk(
                    chunk,
                    *chunk_pos,
                    world_ref,
                    block_textures,
                    block_model_loader,
                );
                Some((*chunk_pos, chunk_vertices, chunk_indices))
            } else {
                None
            }
        })
        .collect();

    for (chunk_pos, chunk_vertices, chunk_indices) in new_meshes {
        world.chunks.get_mut(&chunk_pos).unwrap().dirty = false;

        if let Some(mut mesh) = chunk_mesh_pool.pop() {
            mesh.update(&chunk_vertices, &chunk_indices);
            chunk_meshes.insert(chunk_pos, mesh);
        } else {
            let mesh = Mesh::new(gl, &chunk_vertices, &chunk_indices, glow::TRIANGLES);
            chunk_meshes.insert(chunk_pos, mesh);
        }
    }
}

/// Generates the mesh for a single chunk at the given position in the world.
/// Returns a tuple containing the list of vertices and the list of indices.
fn mesh_chunk(
    chunk: &ClientChunk,
    chunk_pos: glam::IVec3,
    world: &ClientWorld,
    block_textures: &crate::resource::block::TextureAtlas,
    block_model_loader: &BlockModelLoader,
) -> (Vec<ChunkVertex>, Vec<u32>) {
    let chunk_origin = chunk_pos * (CHUNK_SIZE as i32);

    let mut vertices = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 24);
    let mut indices = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 36);

    let mut neighbors = [[[None; 3]; 3]; 3];

    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    neighbors[1][1][1] = Some(chunk);
                    continue;
                }
                let idx = ((dx + 1) as usize, (dy + 1) as usize, (dz + 1) as usize);
                neighbors[idx.0][idx.1][idx.2] =
                    world.chunks.get(&(chunk_pos + IVec3::new(dx, dy, dz)));
            }
        }
    }

    #[inline(always)]
    fn get_block(
        chunk_origin: IVec3,
        world_pos: IVec3,
        neighbors: [[[Option<&ClientChunk>; 3]; 3]; 3],
    ) -> Option<BlockState> {
        let local = world_pos - chunk_origin;

        let chunk_size = CHUNK_SIZE as i32;

        let cx = local.x.div_euclid(chunk_size);
        let cy = local.y.div_euclid(chunk_size);
        let cz = local.z.div_euclid(chunk_size);

        debug_assert!((-1..=1).contains(&cx), "cx: {}, cy: {}, cz: {}", cx, cy, cz);
        debug_assert!((-1..=1).contains(&cy), "cx: {}, cy: {}, cz: {}", cx, cy, cz);
        debug_assert!((-1..=1).contains(&cz), "cx: {}, cy: {}, cz: {}", cx, cy, cz);

        let lx = local.x.rem_euclid(chunk_size);
        let ly = local.y.rem_euclid(chunk_size);
        let lz = local.z.rem_euclid(chunk_size);

        let chunk_ref = neighbors[(cx + 1) as usize][(cy + 1) as usize][(cz + 1) as usize]?;

        chunk_ref.get_block(IVec3::new(lx, ly, lz))
    }

    for x in 0..(CHUNK_SIZE as i32) {
        let world_x = chunk_pos.x * (CHUNK_SIZE as i32) + x;
        for y in 0..(CHUNK_SIZE as i32) {
            let world_y = chunk_pos.y * (CHUNK_SIZE as i32) + y;
            for z in 0..(CHUNK_SIZE as i32) {
                // Check if the block is visible
                let block_local_pos = glam::IVec3::new(x, y, z);
                let block = chunk.get_block(block_local_pos).unwrap();
                let block_def = block_registry().get(block.block).unwrap();
                if !block_def.visible {
                    continue;
                }

                let world_z = chunk_pos.z * (CHUNK_SIZE as i32) + z;
                let world_pos = glam::IVec3::new(world_x, world_y, world_z);

                let model = block_model_loader
                    .get(block)
                    .unwrap_or_else(|| panic!("No model found for {block:?}"));

                for el in &model.elements {
                    for face in &el.faces {
                        if let (Some(dir), true) = (face.cull_dir, face.cullable) {
                            let neighbor_pos = world_pos + dir;
                            let Some(neighbor_block) =
                                get_block(chunk_origin, neighbor_pos, neighbors)
                            else {
                                continue;
                            };
                            let n_def = block_registry().get(neighbor_block.block).unwrap();
                            if n_def.visible {
                                if let Some(n_model) = block_model_loader.get(neighbor_block) {
                                    if face_occluded(face, dir, n_model) {
                                        continue;
                                    }
                                }
                            }
                        }

                        let mut aos = [3u8; 4];
                        if let (true, Some(dir)) = (model.is_full_cube(), face.cull_dir) {
                            for vert_idx in 0..4 {
                                let [side1_off, side2_off, corner_off] =
                                    AO_NEIGHBORS[dir as usize][vert_idx];

                                let side1 =
                                    get_block(chunk_origin, world_pos + side1_off, neighbors);
                                let side2 =
                                    get_block(chunk_origin, world_pos + side2_off, neighbors);
                                let corner =
                                    get_block(chunk_origin, world_pos + corner_off, neighbors);

                                let side1_full = block_is_full_cube(side1, block_model_loader);
                                let side2_full = block_is_full_cube(side2, block_model_loader);
                                let corner_full = block_is_full_cube(corner, block_model_loader);

                                aos[vert_idx] = ao_for_vertex(side1_full, side2_full, corner_full);
                            }
                        }

                        let model_uv = face.uv;
                        let [uv_min, uv_max] =
                            block_textures.get_uv(&face.texture_name, model_uv).unwrap();

                        let base_index = vertices.len() as u32;
                        let uvs = [
                            Vec2::new(uv_max.x, uv_max.y),
                            Vec2::new(uv_min.x, uv_max.y),
                            Vec2::new(uv_min.x, uv_min.y),
                            Vec2::new(uv_max.x, uv_min.y),
                        ];
                        let normal = face.normal;
                        for (i, vert) in face.vertices.iter().enumerate() {
                            vertices.push(ChunkVertex {
                                position: *vert + world_pos.as_vec3(),
                                normal,
                                uv: uvs[i],
                                ao: aos[i],
                            });
                        }

                        if aos[0] + aos[2] < aos[1] + aos[3] {
                            indices.extend_from_slice(&[
                                base_index,
                                base_index + 1,
                                base_index + 3,
                                base_index + 1,
                                base_index + 2,
                                base_index + 3,
                            ]);
                        } else {
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
