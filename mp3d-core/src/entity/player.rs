//! The player module provides the `PlayerEntity` entity for Mineplace3D.

use glam::Vec3;

use crate::{
    entity::{Entity, EntityType},
    world::World,
};

pub const GRAVITY: f32 = 60.0;
pub const GROUND_EPSILON: f32 = 0.07;

pub struct PlayerEntity {
    pub entity_id: u64,
    pub username: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
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
            yaw: 0.0,
            pitch: 0.0,
            flying: false,
            cooldown: 0,
            on_ground: false,
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

    fn save(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(&(self.username.len() as u8).to_le_bytes());
        data.extend(self.username.as_bytes());
        data.extend(&self.position.x.to_le_bytes());
        data.extend(&self.position.y.to_le_bytes());
        data.extend(&self.position.z.to_le_bytes());
        data.extend(&self.velocity.x.to_le_bytes());
        data.extend(&self.velocity.y.to_le_bytes());
        data.extend(&self.velocity.z.to_le_bytes());
        data.extend(&self.yaw.to_le_bytes());
        data.extend(&self.pitch.to_le_bytes());
        data.extend(&[self.flying as u8]);
        data
    }

// <<<<<<< HEAD
//     fn load(data: &[u8], version: u8) -> Result<Self, String> {
//         fn read_string(data: &[u8], offset: &mut usize) -> Result<String, String> {
//             if *offset >= data.len() {
// =======
//     fn load(data: &[u8], _version: u8) -> Result<Self, String> {
//         fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, String> {
//             if *offset + 1 > data.len() {
//                 return Err("Unexpected end of data".to_string());
//             }

//             let byte = data[*offset];
//             *offset += 1;
//             Ok(byte)
//         }

//         fn read_u64(data: &[u8], offset: &mut usize) -> Result<u64, String> {
//             if *offset + 8 > data.len() {
// >>>>>>> main
//                 return Err("Unexpected end of data".to_string());
//             }

//             let len = data[*offset] as usize;
//             *offset += 1;

//             if *offset + len > data.len() {
//                 return Err("Unexpected end of data".to_string());
//             }

//             let string_data = &data[*offset..*offset + len];
//             *offset += len;

//             String::from_utf8(string_data.to_vec()).map_err(|_| "Failed to read string".to_string())
//         }
    fn load(data: &[u8], version: u8) -> Result<Self, String> {
        fn read_string(data: &[u8], offset: &mut usize) -> Result<String, String> {
            if *offset >= data.len() {
                return Err("Unexpected end of data".to_string());
            }

            let len = data[*offset] as usize;
            *offset += 1;

            if *offset + len > data.len() {
                return Err("Unexpected end of data".to_string());
            }

            let string_data = &data[*offset..*offset + len];
            *offset += len;

            String::from_utf8(string_data.to_vec()).map_err(|_| "Failed to read string".to_string())
        }

        fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, String> {
            if *offset + 1 > data.len() {
                return Err("Unexpected end of data".to_string());
            }

            let byte = data[*offset];
            *offset += 1;
            Ok(byte)
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

        match version {
            0 => {
                let mut offset = 0;

                let username = read_string(data, &mut offset)?;
                let pos_x = read_f32(data, &mut offset)?;
                let pos_y = read_f32(data, &mut offset)?;
                let pos_z = read_f32(data, &mut offset)?;
                let vel_x = read_f32(data, &mut offset)?;
                let vel_y = read_f32(data, &mut offset)?;
                let vel_z = read_f32(data, &mut offset)?;
                let yaw = read_f32(data, &mut offset)?;
                let pitch = read_f32(data, &mut offset)?;
                let flying = read_u8(data, &mut offset)? != 0;

                Ok(Self {
                    entity_id: 0,
                    username,
                    position: Vec3::new(pos_x, pos_y, pos_z),
                    velocity: Vec3::new(vel_x, vel_y, vel_z),
                    yaw,
                    pitch,
                    flying,
                    cooldown: 0,
                    on_ground: false,
                })
            }
            _ => Err(format!("Unsupported player entity version: {}", version)),
        }
    }

    fn snapshot(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.entity_id.to_le_bytes());
        data.extend_from_slice(&self.position.x.to_le_bytes());
        data.extend_from_slice(&self.position.y.to_le_bytes());
        data.extend_from_slice(&self.position.z.to_le_bytes());
        data.extend_from_slice(&self.yaw.to_le_bytes());
        data.extend_from_slice(&self.pitch.to_le_bytes());
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

        self.pitch = self.pitch.clamp(-89.9, 89.9);
        self.yaw = self.yaw.rem_euclid(360.0);

        if !self.flying {
            if self.on_ground {
                self.velocity.y = 0.0;
            } else {
                self.velocity.y -= GRAVITY * delta_time;
            }
        }

        self.velocity.y = self.velocity.y.clamp(-100.0, 100.0);

        self.position.x += self.velocity.x * delta_time;
        let collide_x = world.collides(self.position, Self::width(), Self::height());
        if collide_x {
            self.position.x -= self.velocity.x * delta_time;
            self.velocity.x = 0.0;
        }
        self.position.y += self.velocity.y * delta_time;
        self.on_ground = world.collides(
            Vec3::new(self.position.x, self.position.y - GROUND_EPSILON, self.position.z),
            Self::width(),
            Self::height(),
        ) && self.velocity.y <= 0.0;
        let collide_y = world.collides(self.position, Self::width(), Self::height());
        if collide_y {
            self.position.y -= self.velocity.y * delta_time;
            self.velocity.y = 0.0;
        }
        self.position.z += self.velocity.z * delta_time;
        let collide_z = world.collides(self.position, Self::width(), Self::height());
        if collide_z {
            self.position.z -= self.velocity.z * delta_time;
            self.velocity.z = 0.0;
        }

        let d = 0.75_f32.powf(delta_time * 50.0);
        self.velocity.x *= d;
        self.velocity.z *= d;
    }
}
