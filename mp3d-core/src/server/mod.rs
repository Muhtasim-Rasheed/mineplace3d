//! Server code for handling client connections and requests.
//!
//! Note that this does not include networking, for that please check mp3d-server (doesn't exist
//! yet) and instead focuses on the server-side logic.

use std::collections::HashMap;

use glam::Vec3;

use crate::{
    entity::{Entity, PlayerEntity},
    protocol::*,
    world::{chunk::Chunk, World},
};

/// Represents a connected client on the server.
pub struct PlayerSession {
    pub user_id: u64,
    pub entity_id: u64,
    pub pending_messages: Vec<S2CMessage>,
}

/// The main server struct that manages player sessions and world state.
pub struct Server {
    pub sessions: HashMap<u64, PlayerSession>,
    pub world: World,
    pub tps: u8,
}

impl Server {
    /// Handles messages received from clients, updates the world state, and prepares responses.
    pub fn handle_message(&mut self, user_id: u64, message: C2SMessage) {
        fn broadcast_message(
            sessions: &mut HashMap<u64, PlayerSession>,
            sender_id: Option<u64>,
            message: S2CMessage,
        ) {
            for (uid, session) in sessions.iter_mut() {
                if sender_id.map_or(true, |sid| sid != *uid) {
                    session.pending_messages.push(message.clone());
                }
            }
        }

        match message {
            C2SMessage::Connect => {
                let entity_id = self.world.add_entity(Box::new(PlayerEntity::new(
                    user_id,
                    Vec3::new(0.0, 0.0, 0.0),
                )));
                self.sessions.insert(
                    user_id,
                    PlayerSession {
                        user_id,
                        entity_id,
                        pending_messages: vec![
                            S2CMessage::Connected { user_id },
                        ],
                    },
                );
                broadcast_message(
                    &mut self.sessions,
                    Some(user_id),
                    S2CMessage::EntitySpawned {
                        entity_id,
                        entity_type: crate::entity::EntityType::Player as u8,
                        entity_snapshot: self
                            .world
                            .get_entity::<PlayerEntity>(entity_id)
                            .unwrap()
                            .snapshot(),
                    },
                );
            }
            C2SMessage::Disconnect => {
                let session = self.sessions.remove(&user_id);
                self.world.remove_entity(session.unwrap().entity_id);
                broadcast_message(
                    &mut self.sessions,
                    None,
                    S2CMessage::Disconnected { user_id },
                );
            }
            C2SMessage::Move {
                forward,
                strafe,
                jump,
                yaw,
                pitch,
            } => {
                if let Some(session) = self.sessions.get_mut(&user_id) {
                    if let Some(entity) = self.world.get_entity_mut::<PlayerEntity>(session.entity_id) {
                        entity.yaw = yaw;
                        entity.pitch = pitch;
                        let forward_vec = Vec3::new(
                            yaw.to_radians().sin(),
                            0.0,
                            yaw.to_radians().cos(),
                        );
                        let right_vec = Vec3::new(
                            yaw.to_radians().cos(),
                            0.0,
                            -yaw.to_radians().sin(),
                        );
                        let mut movement = Vec3::ZERO;
                        movement += forward_vec * (forward as f32);
                        movement += right_vec * (strafe as f32);
                        if jump {
                            movement.y += 1.0;
                        }
                        let dt = 1.0 / (self.tps as f32);
                        entity.apply_velocity(movement * dt * 5.0);
                        broadcast_message(
                            &mut self.sessions,
                            None,
                            S2CMessage::PlayerMoved {
                                user_id,
                                position: entity.position,
                                yaw: entity.yaw,
                                pitch: entity.pitch,
                            },
                        );
                    }
                }
            }
            C2SMessage::SetBlock { position, block } => {
                self.world.set_block_at(position, block);
                broadcast_message(
                    &mut self.sessions,
                    None,
                    S2CMessage::BlockUpdated { position, block },
                );
            }
            C2SMessage::RequestChunks { chunk_positions } => {
                for chunk_position in chunk_positions {
                    let chunk = self
                        .world
                        .chunks
                        .entry(chunk_position)
                        .or_insert_with(|| Chunk::new(chunk_position));
                    if let Some(session) = self.sessions.get_mut(&user_id) {
                        session.pending_messages.push(S2CMessage::ChunkData {
                            chunk_position,
                            chunk: chunk.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Ticks the server.
    pub fn tick(&mut self, tps: u8) {
        self.tps = tps;
        self.world.tick(tps);
    }
}
