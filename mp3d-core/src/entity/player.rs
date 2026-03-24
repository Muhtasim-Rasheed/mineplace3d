//! The player module provides the `PlayerEntity` entity for Mineplace3D.

use glam::Vec3;

use crate::{
    entity::*,
    item::Inventory,
    saving::{Saveable, WorldLoadError, io::*},
    world::World,
};

pub const GRAVITY: f32 = 60.0;
pub const GROUND_EPSILON: f32 = 0.0005;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MoveInput {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub sneak: bool,
}

impl From<crate::protocol::MoveInstructions> for MoveInput {
    fn from(instr: crate::protocol::MoveInstructions) -> Self {
        Self {
            forward: match instr.forward {
                -1 => -1.0,
                0 => 0.0,
                1 => 1.0,
                2 => 1.5,
                _ => 0.0,
            },
            strafe: match instr.strafe {
                -1 => -1.0,
                0 => 0.0,
                1 => 1.0,
                _ => 0.0,
            },
            jump: instr.jump,
            sneak: instr.sneak,
        }
    }
}

pub struct PlayerEntity {
    pub entity_id: u64,
    pub username: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub(crate) input: MoveInput,
    pub yaw: f32,
    pub pitch: f32,
    pub inventory: Inventory,
    pub hotbar_index: usize,
    pub flying: bool,
    pub cooldown: u8,
    pub on_ground: bool,
}

impl PlayerEntity {
    pub fn new(username: String, position: Vec3) -> Self {
        Self {
            entity_id: 0,
            username,
            position,
            velocity: Vec3::ZERO,
            input: MoveInput::default(),
            yaw: 0.0,
            pitch: 0.0,
            inventory: Inventory::new(),
            hotbar_index: 0,
            flying: false,
            cooldown: 0,
            on_ground: false,
        }
    }
}

impl Saveable for PlayerEntity {
    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.username.len() as u8).to_le_bytes());
        data.extend_from_slice(self.username.as_bytes());
        data.extend_from_slice(&self.position.x.to_le_bytes());
        data.extend_from_slice(&self.position.y.to_le_bytes());
        data.extend_from_slice(&self.position.z.to_le_bytes());
        data.extend_from_slice(&self.velocity.x.to_le_bytes());
        data.extend_from_slice(&self.velocity.y.to_le_bytes());
        data.extend_from_slice(&self.velocity.z.to_le_bytes());
        data.extend_from_slice(&self.yaw.to_le_bytes());
        data.extend_from_slice(&self.pitch.to_le_bytes());
        data.extend_from_slice(&self.inventory.save());
        data.extend_from_slice(&[self.flying as u8]);
        data
    }

    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError> {
        let username_len = read_u8(data, "Player username length")? as usize;
        let username = read_string(data, username_len, "Player username")?;
        let position = read_vec3(data, "Player position")?;
        let velocity = read_vec3(data, "Player velocity")?;
        let yaw = read_f32(data, "Player yaw")?;
        let pitch = read_f32(data, "Player pitch")?;
        let inventory = if version < 2 {
            Inventory::new()
        } else {
            Inventory::load(data, version)?
        };
        let flying = read_u8(data, "Player flying state")? != 0;
        Ok(Self {
            entity_id: 0,
            username,
            position,
            velocity,
            input: MoveInput::default(),
            yaw,
            pitch,
            inventory,
            hotbar_index: 0,
            flying,
            cooldown: 0,
            on_ground: false,
        })
    }
}

impl Entity for PlayerEntity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Player
    }

    fn set_id(&mut self, id: u64) {
        self.entity_id = id;
    }

    fn id(&self) -> u64 {
        self.entity_id
    }

    fn snapshot(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.entity_id.to_le_bytes());
        data.extend_from_slice(&self.position.x.to_le_bytes());
        data.extend_from_slice(&self.position.y.to_le_bytes());
        data.extend_from_slice(&self.position.z.to_le_bytes());
        data.extend_from_slice(&self.yaw.to_le_bytes());
        data.extend_from_slice(&self.pitch.to_le_bytes());
        data.extend_from_slice(&self.inventory.save());
        data.extend_from_slice(&self.hotbar_index.to_le_bytes());
        data.extend_from_slice(&[self.flying as u8]);
        data
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn apply_velocity(&mut self, velocity: Vec3) {
        self.velocity += velocity;
    }

    fn width() -> f32 {
        0.8
    }

    fn height() -> f32 {
        1.8
    }

    fn requests_removal(&self) -> bool {
        false
    }

    fn tick(&mut self, world: &mut World, tps: u8) {
        let delta_time = 1.0 / tps as f32;

        let forward_vec = Vec3::new(
            self.yaw.to_radians().sin(),
            0.0,
            self.yaw.to_radians().cos(),
        );
        let right_vec = Vec3::new(
            self.yaw.to_radians().cos(),
            0.0,
            -self.yaw.to_radians().sin(),
        );
        let mut movement = Vec3::ZERO;
        movement += forward_vec * self.input.forward;
        movement += right_vec * self.input.strafe;
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
        self.velocity += movement * delta_time * 50.0;

        self.pitch = self.pitch.clamp(-89.9, 89.9);
        self.yaw = self.yaw.rem_euclid(360.0);

        if !self.flying {
            if self.on_ground {
                self.velocity.y = 0.0;
            } else {
                self.velocity.y -= GRAVITY * delta_time;
            }
        }

        if self.velocity.length_squared() > 10000.0 {
            log::warn!("High velocity: {}", self.velocity);
        }
        self.velocity.y = self.velocity.y.clamp(-100.0, 100.0);

        let new_pos_x = self
            .position
            .with_x(self.position.x + self.velocity.x * delta_time);
        if !world.collides(new_pos_x, Self::width(), Self::height()) {
            self.position.x = new_pos_x.x;
        } else {
            self.velocity.x = 0.0;
        }

        let new_pos_y = self
            .position
            .with_y(self.position.y + self.velocity.y * delta_time);
        if !world.collides(new_pos_y, Self::width(), Self::height()) {
            self.position.y = new_pos_y.y;
            self.on_ground = world.collides(
                Vec3::new(
                    self.position.x,
                    self.position.y - GROUND_EPSILON,
                    self.position.z,
                ),
                Self::width(),
                Self::height(),
            ) && self.velocity.y <= 0.0;
        } else {
            if self.velocity.y <= 0.0 {
                self.on_ground = true;
            }
            self.velocity.y = 0.0;
        }

        let new_pos_z = self
            .position
            .with_z(self.position.z + self.velocity.z * delta_time);
        if !world.collides(new_pos_z, Self::width(), Self::height()) {
            self.position.z = new_pos_z.z;
        } else {
            self.velocity.z = 0.0;
        }

        let d = 0.75_f32.powf(delta_time * 50.0);
        self.velocity.x *= d;
        self.velocity.z *= d;
    }
}
