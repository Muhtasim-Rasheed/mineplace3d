use std::{cell::RefCell, rc::Rc};

use glam::{Mat4, Vec3, Vec4};
use mp3d_core::{
    block::block_registry,
    entity::{Entity, PlayerEntity},
    item::Inventory,
    physics::{self, PhysicsState},
    protocol::MoveInstructions,
    world::chunk::CHUNK_SIZE,
};

use crate::client::world::ClientWorld;

pub struct ClientInventory {
    pub inner: Inventory,
    pub clicks: Vec<(usize, bool)>,
    pub slot: usize,
}

impl ClientInventory {
    pub fn new() -> Self {
        Self {
            inner: Inventory::new(),
            clicks: Vec::new(),
            slot: 0,
        }
    }

    pub fn click(&mut self, index: usize, right: bool) {
        self.inner.click(index, right);
        self.clicks.push((index, right));
    }

    pub fn update_from_inventory(&mut self, inventory: Inventory) {
        self.inner = inventory;
        self.clicks.clear();
    }
}

pub struct ClientPlayer {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub delta_yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub flying: bool,
    pub on_ground: bool,
    pub input: MoveInstructions,
    pub inventory: Rc<RefCell<ClientInventory>>,
    pub third_person: bool,
}

impl ClientPlayer {
    pub fn first_person_eye(&self) -> Vec3 {
        self.position + Vec3::new(0.0, 1.62, 0.0)
    }

    pub fn third_person_eye(&self, world: &ClientWorld) -> Vec3 {
        let pivot = self.first_person_eye();

        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();

        let forward = Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
        .normalize();

        let backward = -forward;
        let desired_distance = 3.0;
        let step = 0.03;
        let padding = 0.22;

        let mut pos = pivot;
        let mut traveled = 0.0;

        while traveled <= desired_distance {
            let block_pos = pos.floor().as_ivec3();

            if let Some((block, state)) = world.get_block_at(block_pos) {
                let local = pos - block_pos.as_vec3();

                let block_def = block_registry().get(block).unwrap();
                if block_def.visible
                    && let Some(normal) = block_def.ray_intersect(local, backward, *state)
                {
                    let hit_normal = normal.as_vec3();
                    return pos + hit_normal * padding;
                }
            }

            pos += backward * step;
            traveled += step;
        }

        pivot + backward * desired_distance
    }

    pub fn first_person_view(&self) -> Mat4 {
        let eye = self.first_person_eye();

        let pitch_rad = self.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();

        let forward = Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
        .normalize();

        Mat4::look_at_rh(eye, eye + forward, Vec3::Y)
    }

    pub fn third_person_view(&self, world: &ClientWorld) -> Mat4 {
        let eye = self.third_person_eye(world);

        let pitch_rad = self.pitch.to_radians();
        let yaw_rad = self.yaw.to_radians();

        let forward = Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
        .normalize();

        Mat4::look_at_rh(eye, eye + forward, Vec3::Y)
    }

    pub fn model(&self) -> Mat4 {
        Mat4::from_rotation_translation(
            glam::Quat::from_rotation_y((self.yaw - self.delta_yaw * 2.0).to_radians()),
            self.position,
        )
    }

    pub fn view(&self, world: &ClientWorld) -> Mat4 {
        if self.third_person {
            self.third_person_view(world)
        } else {
            self.first_person_view()
        }
    }

    pub fn projection(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), aspect_ratio, 0.1, 1000.0)
    }

    /// Returns the frustum planes, which can be used for frustum culling of chunks.
    pub fn frustum_planes(&self, aspect_ratio: f32, world: &ClientWorld) -> [Vec4; 6] {
        let vp = self.projection(aspect_ratio) * self.view(world);
        let m = vp.to_cols_array_2d();

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

    pub fn update_from_snapshot(&mut self, snapshot: &[u8]) {
        use mp3d_core::saving::{Saveable, io::*};
        let mut snapshot = snapshot.iter().cloned();
        let _entity_id = read_u64(&mut snapshot, "ClientPlayer reading entity_id").unwrap();
        self.position = read_vec3(&mut snapshot, "ClientPlayer reading position").unwrap();
        let previous_yaw = self.yaw;
        self.yaw = read_f32(&mut snapshot, "ClientPlayer reading yaw").unwrap();
        self.delta_yaw = self.yaw - previous_yaw;
        self.pitch = read_f32(&mut snapshot, "ClientPlayer reading pitch").unwrap();
        self.inventory.borrow_mut().update_from_inventory(
            Inventory::load(&mut snapshot, mp3d_core::saving::SAVE_VERSION).unwrap(),
        );
        self.inventory.borrow_mut().slot =
            read_u8(&mut snapshot, "ClientPlayer reading inventory slot").unwrap() as usize;
        self.flying = read_u8(&mut snapshot, "ClientPlayer reading flying").unwrap() != 0;
    }

    pub fn optimistic(&mut self, dt: f32, world: &ClientWorld) {
        if !world
            .chunks
            .contains_key(&(self.position.as_ivec3() / CHUNK_SIZE as i32))
        {
            return;
        }

        self.pitch = self.pitch.clamp(-89.9, 89.9);
        self.yaw = self.yaw.rem_euclid(360.0);

        let state = PhysicsState {
            position: self.position,
            velocity: self.velocity,
            on_ground: self.on_ground,
            flying: self.flying,
        };

        let new_state = physics::step(
            state,
            self.input.into(),
            self.yaw,
            PlayerEntity::width(),
            PlayerEntity::height(),
            world,
            dt,
        );

        self.position = new_state.position;
        self.velocity = new_state.velocity;
        self.on_ground = new_state.on_ground;
    }
}
