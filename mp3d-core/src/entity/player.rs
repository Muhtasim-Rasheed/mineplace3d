//! The player module provides the `PlayerEntity` entity for Mineplace3D.

use glam::Vec3;

use crate::{
    entity::*,
    item::Inventory,
    physics::{self, PhysicsState},
    saving::{Saveable, WorldLoadError, io::*},
    world::World,
};

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

    fn position_mut(&mut self) -> &mut Vec3 {
        &mut self.position
    }

    fn forward(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();
        Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
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
            self.input,
            self.yaw,
            Self::width(),
            Self::height(),
            world,
            1.0 / tps as f32,
        );

        self.position = new_state.position;
        self.velocity = new_state.velocity;
        self.on_ground = new_state.on_ground;
    }
}
