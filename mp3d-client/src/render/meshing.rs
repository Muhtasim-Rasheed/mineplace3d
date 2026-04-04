//! All utilities related to meshing worlds and chunks.

use std::{collections::HashMap, sync::Arc};

use glam::{IVec3, Vec2, Vec3, Vec3Swizzles};
use glow::HasContext;
use mp3d_core::{
    block::{Block, BlockState},
    world::chunk::CHUNK_SIZE,
};

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
            gl.vertex_attrib_pointer_i32(1, 3, glow::INT, stride, offset);
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

/// Gets a rect of a face of a block
#[inline]
fn get_face_rect(face_idx: usize, element: &crate::resource::block::BlockElement) -> [Vec2; 2] {
    match face_idx {
        0 | 1 => [element.from.xy(), element.to.xy()],
        2 | 3 => [element.from.zy(), element.to.zy()],
        4 | 5 => [element.from.xz(), element.to.xz()],
        _ => unreachable!(),
    }
}

/// Determines if the face of block `a` is completely covered by block `b` on the given face index.
#[inline]
fn covers(a_min: Vec2, a_max: Vec2, b_min: Vec2, b_max: Vec2) -> bool {
    a_min.cmple(b_min).all() && a_max.cmpge(b_max).all()
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
        let a_face = &a_el.faces[face_idx];
        if !a_face.cullable {
            continue;
        }

        let a_rect = get_face_rect(face_idx, a_el);

        for b_el in &b_model.elements {
            let b_face = &b_el.faces[face_idx ^ 1];
            if !b_face.occludes {
                continue;
            }

            let b_rect = get_face_rect(face_idx ^ 1, b_el);
            if covers(a_rect[0], a_rect[1], b_rect[0], b_rect[1]) {
                return true;
            }
        }
    }

    false
}

#[inline]
fn block_is_full_cube(
    block: Option<(&Block, &BlockState)>,
    block_models: &HashMap<(&'static str, &'static str), crate::resource::block::BlockModel>,
) -> bool {
    let Some((block, state)) = block else {
        return false;
    };

    if !block.visible {
        return false;
    }

    let ident = (
        block.ident,
        state.to_ident().unwrap_or_else(|| {
            panic!(
                "Block '{}' has an unrecognized block state type: {}",
                block.ident, block.state_type
            )
        }),
    );

    block_models
        .get(&ident)
        .is_some_and(|model| model.is_full_cube())
}

/// The vertex positions for each face of a cube, in the order of NSEWUD.
pub const FACE_VERTS: [[Vec3; 4]; 6] = [
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
            IVec3::new(1, 0, -1),
            IVec3::new(0, -1, -1),
            IVec3::new(1, -1, -1),
        ],
        [
            IVec3::new(-1, 0, -1),
            IVec3::new(0, -1, -1),
            IVec3::new(-1, -1, -1),
        ],
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
    ],
    // South face (+Z)
    [
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
    ],
    // East face (+X)
    [
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
    ],
    // West face (-X)
    [
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
    block_models: &HashMap<(&'static str, &'static str), crate::resource::block::BlockModel>,
    player_pos_chunk: IVec3,
) {
    use rayon::prelude::*;

    const MAX_MESHES_PER_FRAME: usize = 12;

    if world.remesh_queue.is_empty() {
        return;
    }

    let batch_size = world.remesh_queue.len().min(MAX_MESHES_PER_FRAME);

    let mut batch: Vec<IVec3> = world.remesh_queue.drain(batch_size);
    batch.sort_unstable_by(|a, b| {
        let da = (*a - player_pos_chunk).length_squared();
        let db = (*b - player_pos_chunk).length_squared();
        da.cmp(&db)
    });

    let world_ref = &*world;

    let new_meshes: Vec<(IVec3, Vec<ChunkVertex>, Vec<u32>)> = batch
        .par_iter()
        .filter_map(|chunk_pos| {
            if let Some(chunk) = world_ref.chunks.get(chunk_pos) {
                let (chunk_vertices, chunk_indices) =
                    mesh_chunk(chunk, *chunk_pos, world_ref, block_textures, block_models);
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
    block_models: &HashMap<(&'static str, &'static str), crate::resource::block::BlockModel>,
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
    fn get_block<'a>(
        chunk_origin: IVec3,
        world_pos: IVec3,
        neighbors: [[[Option<&'a ClientChunk>; 3]; 3]; 3],
    ) -> Option<(&'a Block, &'a BlockState)> {
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

    #[inline(always)]
    fn ident(block: &Block, state: &BlockState) -> (&'static str, &'static str) {
        (
            block.ident,
            state.to_ident().unwrap_or_else(|| {
                panic!(
                    "Block '{}' has an unrecognized block state type: {}",
                    block.ident, block.state_type
                )
            }),
        )
    }

    for x in 0..(CHUNK_SIZE as i32) {
        let world_x = chunk_pos.x * (CHUNK_SIZE as i32) + x;
        for y in 0..(CHUNK_SIZE as i32) {
            let world_y = chunk_pos.y * (CHUNK_SIZE as i32) + y;
            for z in 0..(CHUNK_SIZE as i32) {
                // Check if the block is visible
                let block_local_pos = glam::IVec3::new(x, y, z);
                let (block, state) = chunk.get_block(block_local_pos).unwrap();
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
                    let neighbor_pos = glam::IVec3::new(world_x + dx, world_y + dy, world_z + dz);

                    // Create face the neighboring block is air or doesn't occlude this face.
                    let neighbor_block = get_block(chunk_origin, neighbor_pos, neighbors);
                    let neighbor_state = neighbor_block.map(|(_, state)| state);
                    let neighbor_block = neighbor_block.map(|(block, _)| block);
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

                            let block_world_pos = IVec3::new(world_x, world_y, world_z);

                            // AO for the 4 vertices of this face
                            let mut aos = [3u8; 4];

                            if model.is_full_cube() {
                                for vert_idx in 0..4 {
                                    let [side1_off, side2_off, corner_off] =
                                        AO_NEIGHBORS[i][vert_idx];

                                    let side1 = get_block(
                                        chunk_origin,
                                        block_world_pos + side1_off,
                                        neighbors,
                                    );
                                    let side2 = get_block(
                                        chunk_origin,
                                        block_world_pos + side2_off,
                                        neighbors,
                                    );
                                    let corner = get_block(
                                        chunk_origin,
                                        block_world_pos + corner_off,
                                        neighbors,
                                    );

                                    let side1_full = block_is_full_cube(side1, block_models);
                                    let side2_full = block_is_full_cube(side2, block_models);
                                    let corner_full = block_is_full_cube(corner, block_models);

                                    aos[vert_idx] =
                                        ao_for_vertex(side1_full, side2_full, corner_full);
                                }
                            }

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
                            for (i, (vert, uv)) in FACE_VERTS[i].iter().zip(uvs.iter()).enumerate()
                            {
                                vertices.push(ChunkVertex {
                                    position: *vert * (el.to - el.from)
                                        + el.from
                                        + Vec3::new(world_x as f32, world_y as f32, world_z as f32),
                                    normal: IVec3::new(dx, dy, dz),
                                    uv: *uv,
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
    }

    (vertices, indices)
}
