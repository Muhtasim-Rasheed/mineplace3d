//! The player module provides the `PlayerEntity` entity for Mineplace3D.

use glam::Vec3;

use crate::{entity::Entity, world::World};

pub struct PlayerEntitySnapshot {
    pub user_id: u64,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

pub struct PlayerEntity {
    pub entity_id: u64,
    pub user_id: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub cooldown: u8,
}

impl PlayerEntity {
    pub fn new(user_id: u64, position: Vec3) -> Self {
        Self {
            entity_id: 0,
            user_id,
            position,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            cooldown: 0,
        }
    }
}

impl Entity for PlayerEntity {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn set_id(&mut self, id: u64) {
        self.entity_id = id;
    }

    fn id(&self) -> u64 {
        self.entity_id
    }

    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(&self.user_id.to_le_bytes());
        data.extend(&self.position.x.to_le_bytes());
        data.extend(&self.position.y.to_le_bytes());
        data.extend(&self.position.z.to_le_bytes());
        data.extend(&self.velocity.x.to_le_bytes());
        data.extend(&self.velocity.y.to_le_bytes());
        data.extend(&self.velocity.z.to_le_bytes());
        data.extend(&self.yaw.to_le_bytes());
        data.extend(&self.pitch.to_le_bytes());
        data
    }

    fn load(data: &[u8], version: u8) -> Result<Self, String> {
        fn read_u64(data: &[u8], offset: &mut usize) -> Result<u64, String> {
            if *offset + 8 > data.len() {
                return Err("Unexpected end of data".to_string());
            }

            let bytes: [u8; 8] = data[*offset..*offset + 8]
                .try_into()
                .map_err(|_| "Failed to read u64".to_string())?;

            *offset += 8;
            Ok(u64::from_le_bytes(bytes))
        }

        fn read_f32(data: &[u8], offset: &mut usize) -> Result<f32, String> {
            if *offset + 4 > data.len() {
                return Err("Unexpected end of data".to_string());
            }

            let bytes: [u8; 4] = data[*offset..*offset + 4]
                .try_into()
                .map_err(|_| "Failed to read f32".to_string())?;

            *offset += 4;
            Ok(f32::from_le_bytes(bytes))
        }

        let mut offset = 0;

        let user_id = if version >= 1 {
            read_u64(data, &mut offset)?
        } else {
            0
        };
        let pos_x = read_f32(data, &mut offset)?;
        let pos_y = read_f32(data, &mut offset)?;
        let pos_z = read_f32(data, &mut offset)?;
        let vel_x = read_f32(data, &mut offset)?;
        let vel_y = read_f32(data, &mut offset)?;
        let vel_z = read_f32(data, &mut offset)?;
        let yaw = read_f32(data, &mut offset)?;
        let pitch = read_f32(data, &mut offset)?;

        Ok(Self {
            entity_id: 0,
            user_id,
            position: Vec3::new(pos_x, pos_y, pos_z),
            velocity: Vec3::new(vel_x, vel_y, vel_z),
            yaw,
            pitch,
            cooldown: 0,
        })
    }

    fn snapshot(&self) -> Vec<u8> {
        // The client doesn't need to concern itself with velocity.

        let mut data = Vec::new();
        data.extend(&self.user_id.to_le_bytes());
        data.extend(&self.position.x.to_le_bytes());
        data.extend(&self.position.y.to_le_bytes());
        data.extend(&self.position.z.to_le_bytes());
        data.extend(&self.yaw.to_le_bytes());
        data.extend(&self.pitch.to_le_bytes());
        data
    }

    fn position(&self) -> Vec3 {
        self.position
    }

    fn apply_velocity(&mut self, velocity: Vec3) {
        self.velocity += velocity;
    }

    fn width(&self) -> f32 {
        0.6
    }

    fn height(&self) -> f32 {
        1.8
    }

    fn requests_removal(&self) -> bool {
        false
    }

    fn tick(&mut self, _world: &mut World, tps: u8) {
        let delta_time = 1.0 / tps as f32;

        self.pitch = self.pitch.clamp(-89.9, 89.9);
        self.yaw = self.yaw.rem_euclid(360.0);

        self.position += self.velocity * delta_time;
        self.velocity *= 0.9_f32.powf(delta_time * 48.0);

        // nothing much right now
    }
}
