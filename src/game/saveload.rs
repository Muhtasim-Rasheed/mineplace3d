use std::cell::RefCell;
use std::rc::Rc;

use crate::game::entity::*;
use crate::game::{CHUNK_SIZE, World};

pub const LATEST_SAVEFILE_VER: u8 = 0;
pub const MAGIC: &[u8; 4] = b"MP3D";

#[inline(always)]
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

#[inline(always)]
fn read_u32(data: &[u8], offset: &mut usize) -> Result<u32, String> {
    if *offset + 4 > data.len() {
        return Err("Unexpected end of data".to_string());
    }

    let bytes: [u8; 4] = data[*offset..*offset + 4]
        .try_into()
        .map_err(|_| "Failed to read u32".to_string())?;

    *offset += 4;
    Ok(u32::from_le_bytes(bytes))
}

#[inline(always)]
fn read_i32(data: &[u8], offset: &mut usize) -> Result<i32, String> {
    if *offset + 4 > data.len() {
        return Err("Unexpected end of data".to_string());
    }

    let bytes: [u8; 4] = data[*offset..*offset + 4]
        .try_into()
        .map_err(|_| "Failed to read i32".to_string())?;

    *offset += 4;
    Ok(i32::from_le_bytes(bytes))
}

#[inline(always)]
fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, String> {
    if *offset >= data.len() {
        return Err("Unexpected end of data".to_string());
    }
    let v = data[*offset];
    *offset += 1;
    Ok(v)
}

#[inline(always)]
fn read_string_ascii(data: &[u8], offset: &mut usize) -> String {
    let mut string = String::new();
    while data[*offset] != 0 {
        string.push(data[*offset] as char);
        *offset += 1;
    }
    *offset += 1;
    string
}

impl World {
    pub fn load(
        data: &[u8],
        resource_manager: crate::game::ResourceManager,
        window: &sdl2::video::Window,
    ) -> Result<Self, String> {
        let mut offset = 0;
        if &data[offset..offset + 4] != MAGIC {
            return Err("Invalid save file signature".to_string());
        }
        offset += 4;

        let version = read_u8(data, &mut offset)?;

        match version {
            0 => {
                let chunk_size = read_u8(data, &mut offset)? as i32;
                let seed = read_i32(data, &mut offset)?;
                let mut world = World::new(seed, resource_manager, window);
                world.entities.clear();

                let changes_len = read_u64(data, &mut offset)?;
                for _ in 0..changes_len {
                    let chunk_x = read_i32(data, &mut offset)?;
                    let chunk_y = read_i32(data, &mut offset)?;
                    let chunk_z = read_i32(data, &mut offset)?;

                    let local_x = read_u8(data, &mut offset)? as i32;
                    let local_y = read_u8(data, &mut offset)? as i32;
                    let local_z = read_u8(data, &mut offset)? as i32;

                    let block_id = read_u32(data, &mut offset)?;
                    let block = block_id.into();

                    let global_x = chunk_x * chunk_size + local_x;
                    let global_y = chunk_y * chunk_size + local_y;
                    let global_z = chunk_z * chunk_size + local_z;
                    world.set_block(global_x, global_y, global_z, block);
                }

                let entities_len = read_u64(data, &mut offset)?;
                for _ in 0..entities_len {
                    let entity_id = EntityId {
                        id: read_u32(data, &mut offset)?,
                        entity_name: read_string_ascii(data, &mut offset),
                    };

                    let entity_data_len = read_u64(data, &mut offset)? as usize;
                    let entity_data = &data[offset..(offset + entity_data_len)];
                    offset += entity_data_len;
                    let entity: Rc<RefCell<dyn Entity>> = match entity_id.entity_name.as_str() {
                        "Player" => Rc::new(RefCell::new(Player::load(entity_data, window)?)),
                        "Billboard" => Rc::new(RefCell::new(Billboard::load(entity_data, window)?)),
                        _ => return Err(format!("Unsupported entity: {}", entity_id.entity_name)),
                    };
                    world.entities.insert(entity_id, entity);
                }

                Ok(world)
            }
            _ => Err(format!("Unsupported save file version: {}", version)),
        }
    }

    pub fn save(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // Signature & Version
        out.extend(MAGIC);
        out.push(LATEST_SAVEFILE_VER);
        out.push(CHUNK_SIZE as u8);

        // Seed
        out.extend(self.seed().to_le_bytes());

        // Changes
        let changes = &self.changes;
        out.extend((changes.len() as u64).to_le_bytes());
        for ((chunk, local), block) in changes {
            out.extend(chunk.x.to_le_bytes());
            out.extend(chunk.y.to_le_bytes());
            out.extend(chunk.z.to_le_bytes());

            out.push(local.x as u8);
            out.push(local.y as u8);
            out.push(local.z as u8);

            out.extend((*block as u32).to_le_bytes());
        }

        // Entities
        let entities = &self.entities;
        out.extend((entities.len() as u64).to_le_bytes());
        for (id, entity) in entities {
            out.extend(id.id.to_le_bytes());
            out.extend(id.entity_name.as_bytes());
            out.push(0x00);

            let entity_data = entity.borrow().save();
            out.extend((entity_data.len() as u64).to_le_bytes());
            out.extend(entity_data);
        }

        out
    }
}
