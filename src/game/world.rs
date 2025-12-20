use fastnoise_lite::{FastNoiseLite, NoiseType};
use fxhash::FxHashMap;
use glam::*;
use rayon::prelude::*;
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate::{
    asset::{ModelDefs, ResourceManager},
    game::{
        aabb_in_frustum, extract_frustum_planes, Billboard, BillboardType, Block, BlockVertex, Chunk, Entity, EntityId, NeighbourChunks, Player, CHUNK_SIZE
    },
    mesh::Mesh,
};

pub const RENDER_DISTANCE: u32 = 8;

pub struct World {
    chunks: FxHashMap<IVec3, Chunk>,
    changes: FxHashMap<(IVec3, IVec3), Block>,
    chunk_outside_blocks: FxHashMap<(IVec3, IVec3), Block>,
    pub entities: HashMap<EntityId, Rc<RefCell<dyn Entity>>>,
    pub meshes: HashMap<IVec3, Mesh>,
    pub mesh_visible: HashSet<IVec3>,
    // When a chunk is unloaded, its corresponding mesh is stored here for reuse
    unused_meshes: Vec<Mesh>,
    previous_vp: Option<Mat4>,
    noise: Arc<FastNoiseLite>,
    cave_noise: Arc<FastNoiseLite>,
    biome_noise: Arc<FastNoiseLite>,
    pub resource_mgr: ResourceManager,
}

impl World {
    pub fn new(seed: i32, resource_mgr: ResourceManager, window: &sdl2::video::Window) -> Self {
        const TWO_THIRDS_I32: i32 = (i32::MAX as f32 * (2.0 / 3.0)) as i32;

        let mut noise = FastNoiseLite::new();
        noise.set_seed(Some(seed));
        noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        noise.set_frequency(Some(0.1));

        let mut cave_noise = FastNoiseLite::new();
        cave_noise.set_seed(Some(seed.wrapping_add(i32::MAX / 3)));
        cave_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        cave_noise.set_frequency(Some(0.5));

        let mut biome_noise = FastNoiseLite::new();
        biome_noise.set_seed(Some(seed.wrapping_add(TWO_THIRDS_I32)));
        biome_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        biome_noise.set_frequency(Some(0.1));

        let mut chunks = HashMap::new();
        let mut chunk_outside_blocks = HashMap::new();
        for x in -3..=3 {
            for y in -1..=3 {
                for z in -3..=3 {
                    let res = Chunk::new(x, y, z, &noise, &cave_noise, &biome_noise);
                    chunks.insert(ivec3(x, y, z), res.0);
                    chunk_outside_blocks.extend(res.1.into_iter());
                }
            }
        }

        let player = Player::new(vec3(0.0, 100.0, 0.0), window);

        let mut world = World {
            chunks: FxHashMap::from_iter(chunks.into_iter()),
            changes: FxHashMap::default(),
            chunk_outside_blocks: FxHashMap::from_iter(chunk_outside_blocks.into_iter()),
            entities: HashMap::new(),
            meshes: HashMap::new(),
            mesh_visible: HashSet::new(),
            unused_meshes: Vec::new(),
            previous_vp: None,
            noise: noise.into(),
            cave_noise: cave_noise.into(),
            biome_noise: biome_noise.into(),
            resource_mgr,
        };
        world.add_entity(player);

        world
    }

    pub fn get_player(&self) -> Ref<'_, Player> {
        for entity in self.entities.values() {
            if entity.borrow().as_any().is::<Player>() {
                return Ref::map(entity.borrow(), |e| {
                    e.as_any().downcast_ref::<Player>().unwrap()
                });
            }
        }
        panic!("No player found");
    }

    pub fn get_player_mut(&mut self) -> RefMut<'_, Player> {
        for entity in self.entities.values() {
            if entity.borrow().as_any().is::<Player>() {
                return RefMut::map(entity.borrow_mut(), |e| {
                    e.as_any_mut().downcast_mut::<Player>().unwrap()
                });
            }
        }
        panic!("No player found");
    }

    pub fn seed(&self) -> i32 {
        self.noise.seed
    }

    pub fn noise(&self) -> Arc<FastNoiseLite> {
        Arc::clone(&self.noise)
    }

    pub fn cave_noise(&self) -> Arc<FastNoiseLite> {
        Arc::clone(&self.cave_noise)
    }

    pub fn biome_noise(&self) -> Arc<FastNoiseLite> {
        Arc::clone(&self.biome_noise)
    }

    pub fn update(&mut self, events: &[sdl2::event::Event], dt: f64) {
        let player_pos = self.get_player().position();
        self.chunks.retain(|pos, _| {
            let distance_squared = pos
                .as_vec3()
                .distance_squared(player_pos / CHUNK_SIZE as f32);
            distance_squared <= RENDER_DISTANCE as f32 * RENDER_DISTANCE as f32
        });
        for pos in self.meshes.keys().cloned().collect::<Vec<_>>() {
            if !self.chunks.contains_key(&pos) {
                if let Some(mesh) = self.meshes.remove(&pos) {
                    self.unused_meshes.push(mesh);
                }
            }
        }
        self.entities.retain(|_, e| !e.borrow().requests_removal());
        for (id, entity) in self.entities.clone() {
            entity.borrow_mut().update(id, self, events, dt);
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
                    if let Some(block) = self.chunk_outside_blocks.get(&(
                        ivec3(x, y, z),
                        ivec3(local_x as i32, local_y as i32, local_z as i32),
                    )) {
                        chunk.set_block(local_x, local_y, local_z, *block);
                        self.chunk_outside_blocks.remove(&(
                            ivec3(x, y, z),
                            ivec3(local_x as i32, local_y as i32, local_z as i32),
                        ));
                    }
                    if let Some(block) = self.changes.get(&(
                        ivec3(x, y, z),
                        ivec3(local_x as i32, local_y as i32, local_z as i32),
                    )) {
                        chunk.set_block(local_x, local_y, local_z, *block);
                    }
                }
            }
        }
        self.chunk_outside_blocks.extend(outside_blocks.into_iter());
        self.chunks.insert(ivec3(x, y, z), chunk);
    }

    pub fn add_entity(&mut self, entity: impl Entity) {
        self.entities
            .insert(entity.id(), Rc::new(RefCell::new(entity)));
    }

    pub fn get_chunk(&mut self, x: i32, y: i32, z: i32) -> &mut Chunk {
        self.chunks.entry(ivec3(x, y, z)).or_insert_with(|| {
            let res = Chunk::new(x, y, z, &self.noise, &self.cave_noise, &self.biome_noise);
            self.chunk_outside_blocks.extend(res.1.into_iter());
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

        if local_z == 0 {
            self.get_chunk(chunk_x, chunk_y, chunk_z - 1).is_dirty = true;
        }
        if local_z == CHUNK_SIZE - 1 {
            self.get_chunk(chunk_x, chunk_y, chunk_z + 1).is_dirty = true;
        }
        if local_x == 0 {
            self.get_chunk(chunk_x - 1, chunk_y, chunk_z).is_dirty = true;
        }
        if local_x == CHUNK_SIZE - 1 {
            self.get_chunk(chunk_x + 1, chunk_y, chunk_z).is_dirty = true;
        }
        if local_y == 0 {
            self.get_chunk(chunk_x, chunk_y - 1, chunk_z).is_dirty = true;
        }
        if local_y == CHUNK_SIZE - 1 {
            self.get_chunk(chunk_x, chunk_y + 1, chunk_z).is_dirty = true;
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
            self.add_entity(Billboard::new(
                pos.as_vec3() + vec3(0.5, 0.5, 0.5),
                size,
                size as u32 * 25,
                BillboardType::Explosion,
                "billboard_shader",
                "billboard_atlas",
            ));
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
        let model_defs = self
            .resource_mgr
            .get::<ModelDefs>("model_defs")
            .unwrap()
            .clone();

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

    pub fn draw_entities(&self, gl: &Arc<glow::Context>) {
        for entity in self.entities.values() {
            entity.borrow().draw(gl, self, &self.resource_mgr);
        }
    }

    pub fn update_mesh_visibility(&mut self, vp: Mat4) {
        if let Some(previous_vp) = self.previous_vp {
            if previous_vp == vp {
                return;
            }
        }
        self.previous_vp = Some(vp);

        self.mesh_visible.clear();

        let frustum = extract_frustum_planes(vp);
        for (chunk_pos, _) in &self.chunks {
            let min = chunk_pos.as_vec3() * CHUNK_SIZE as f32;
            let max = min + vec3(CHUNK_SIZE as f32, CHUNK_SIZE as f32, CHUNK_SIZE as f32);
            if !aabb_in_frustum(min, max, &frustum) {
                continue;
            }
            let neighbours = NeighbourChunks {
                n: self.chunks.get(&(chunk_pos + ivec3(0, 0, -1))),
                s: self.chunks.get(&(chunk_pos + ivec3(0, 0, 1))),
                e: self.chunks.get(&(chunk_pos + ivec3(1, 0, 0))),
                w: self.chunks.get(&(chunk_pos + ivec3(-1, 0, 0))),
                u: self.chunks.get(&(chunk_pos + ivec3(0, 1, 0))),
                d: self.chunks.get(&(chunk_pos + ivec3(0, -1, 0))),
            };
            if neighbours.all(|i, c| c.is_side_full(i as u8 ^ 1)) {
                continue;
            }
            self.mesh_visible.insert(*chunk_pos);
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.chunks.values().any(|chunk| chunk.is_dirty)
    }

    pub fn generate_meshes(&mut self, gl: &Arc<glow::Context>) {
        if !self.is_dirty() {
            return;
        }

        struct ChunkMeshData {
            pos: IVec3,
            verts: Vec<BlockVertex>,
            idxs: Vec<u32>,
        }

        let dirty_results: Vec<_> = self
            .chunks
            .par_iter()
            .filter_map(|(chunk_pos, chunk)| {
                if chunk.is_empty() || !chunk.is_dirty {
                    return None;
                }
                let neighbour_chunks = NeighbourChunks {
                    n: self.chunks.get(&(chunk_pos + ivec3(0, 0, -1))),
                    s: self.chunks.get(&(chunk_pos + ivec3(0, 0, 1))),
                    e: self.chunks.get(&(chunk_pos + ivec3(1, 0, 0))),
                    w: self.chunks.get(&(chunk_pos + ivec3(-1, 0, 0))),
                    u: self.chunks.get(&(chunk_pos + ivec3(0, 1, 0))),
                    d: self.chunks.get(&(chunk_pos + ivec3(0, -1, 0))),
                };
                if neighbour_chunks.all(|i, c| c.is_side_full(i as u8 ^ 1)) {
                    return None;
                }
                let pos = *chunk_pos;
                let (verts, idxs) = chunk.generate_chunk_mesh(
                    &neighbour_chunks,
                    self.resource_mgr.get::<ModelDefs>("model_defs").unwrap(),
                );
                Some(ChunkMeshData { pos, verts, idxs })
            })
            .collect();

        for data in dirty_results {
            let pos = data.pos;
            let verts = data.verts;
            let idxs = data.idxs;
            // We can unwrap because we are sure it exists because how else would we get the mesh
            let chunk = self.chunks.get_mut(&pos).unwrap();
            chunk.is_dirty = false;
            if let Some(existing_mesh) = self.meshes.get_mut(&pos) {
                // The mesh was edited
                existing_mesh.update(&verts, &idxs);
            } else {
                // Reuse an old mesh if possible
                if let Some(mut mesh) = self.unused_meshes.pop() {
                    mesh.update(&verts, &idxs);
                    self.meshes.insert(pos, mesh);
                } else {
                    self.meshes
                        .insert(pos, Mesh::new(gl, &verts, &idxs, glow::TRIANGLES));
                }
            }
        }
    }
}
