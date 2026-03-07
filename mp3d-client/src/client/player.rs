use std::{cell::RefCell, rc::Rc};

use glam::{Mat4, Vec3, Vec4};
use mp3d_core::{
    entity::{Entity, PlayerEntity},
    item::Inventory,
    protocol::MoveInstructions,
};

use crate::client::world::ClientWorld;

pub struct ClientInventory {
    pub inner: Inventory,
    pub clicks: Vec<(usize, bool)>,
}

impl ClientInventory {
    pub fn new() -> Self {
        Self {
            inner: Inventory::new(),
            clicks: Vec::new(),
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
    pub pitch: f32,
    pub fov: f32,
    pub flying: bool,
    pub on_ground: bool,
    pub input: MoveInstructions,
    pub inventory: Rc<RefCell<ClientInventory>>,
}

impl ClientPlayer {
    pub fn eye(&self) -> Vec3 {
        self.position + Vec3::new(0.0, 1.62, 0.0)
    }

    pub fn view(&self) -> Mat4 {
        let eye = self.eye();

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

    pub fn projection(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), aspect_ratio, 0.1, 1000.0)
    }

    /// Returns the frustum planes, which can be used for frustum culling of chunks.
    pub fn frustum_planes(&self, aspect_ratio: f32) -> [Vec4; 6] {
        let vp = self.projection(aspect_ratio) * self.view();
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
        self.yaw = read_f32(&mut snapshot, "ClientPlayer reading yaw").unwrap();
        self.pitch = read_f32(&mut snapshot, "ClientPlayer reading pitch").unwrap();
        self.inventory.borrow_mut().update_from_inventory(
            Inventory::load(&mut snapshot, mp3d_core::saving::SAVE_VERSION).unwrap(),
        );
        self.flying = read_u8(&mut snapshot, "ClientPlayer reading flying").unwrap() != 0;
    }

    pub fn optimistic(&mut self, dt: f32, world: &ClientWorld) {
        let yaw_rad = self.input.yaw.to_radians();
        let forward_vec = Vec3::new(yaw_rad.sin(), 0.0, yaw_rad.cos());
        let right_vec = Vec3::new(yaw_rad.cos(), 0.0, -yaw_rad.sin());
        let mut movement = Vec3::ZERO;
        movement += forward_vec * self.input.forward as f32;
        movement += right_vec * self.input.strafe as f32;
        if self.input.jump {
            if self.flying {
                movement.y += 0.8;
            } else if self.on_ground {
                self.velocity.y += 12.5;
                self.on_ground = false;
            }
        }
        if self.input.sneak && self.flying {
            movement.y -= 0.8;
        }
        self.velocity += movement * dt * 50.0;

        if !self.flying {
            if self.on_ground && self.velocity.y < 0.0 {
                self.velocity.y = 0.0;
            } else {
                self.velocity.y -= mp3d_core::entity::player::GRAVITY * dt;
            }
        }

        self.velocity.y = self.velocity.y.clamp(-100.0, 100.0);

        self.position.x += self.velocity.x * dt;
        if world.collides(self.position, PlayerEntity::width(), PlayerEntity::height()) {
            self.position.x -= self.velocity.x * dt;
            self.velocity.x = 0.0;
        }
        self.position.y += self.velocity.y * dt;
        self.on_ground = world.collides(
            Vec3::new(
                self.position.x,
                self.position.y - mp3d_core::entity::player::GROUND_EPSILON,
                self.position.z,
            ),
            PlayerEntity::width(),
            PlayerEntity::height(),
        ) && self.velocity.y <= 0.0;
        if world.collides(self.position, PlayerEntity::width(), PlayerEntity::height()) {
            self.position.y -= self.velocity.y * dt;
            self.velocity.y = 0.0;
        }
        self.position.z += self.velocity.z * dt;
        if world.collides(self.position, PlayerEntity::width(), PlayerEntity::height()) {
            self.position.z -= self.velocity.z * dt;
            self.velocity.z = 0.0;
        }
        let d = 0.75_f32.powf(dt * 50.0);
        self.velocity.x *= d;
        self.velocity.z *= d;
    }
}
