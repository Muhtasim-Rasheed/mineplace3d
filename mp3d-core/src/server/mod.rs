//! Server code for handling client connections and requests.
//!
//! Note that this does not include networking, for that please check mp3d-server (doesn't exist
//! yet) and instead focuses on the server-side logic.

use std::collections::HashMap;

use glam::Vec3;

use crate::{
    TextComponent,
    entity::{Entity, PlayerEntity},
    protocol::*,
    world::{World, chunk::Chunk},
};

/// Represents a connected client on the server.
pub struct PlayerSession {
    pub user_id: u64,
    pub entity_id: u64,
    pub nickname: Option<String>,
    pub pending_messages: Vec<S2CMessage>,
}

/// The main server struct that manages player sessions and world state.
pub struct Server {
    pub sessions: HashMap<u64, PlayerSession>,
    pub connections: HashMap<u64, u64>,
    pub world: World,
    pub tps: u8,
}

impl Server {
    /// Creates a new server instance.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            connections: HashMap::new(),
            world: World::new(),
            tps: 48,
        }
    }

    /// Returns the next available user ID.
    fn next_user_id(&self) -> u64 {
        let mut user_id = 1;
        while self.sessions.contains_key(&user_id) {
            user_id += 1;
        }
        user_id
    }

    /// Handles messages received from clients, and prepares responses. Note that this does not
    /// tick the server, that must be done separately.
    pub fn handle_message(&mut self, connection_id: u64, message: C2SMessage) {
        fn broadcast_message(
            sessions: &mut HashMap<u64, PlayerSession>,
            sender_id: Option<u64>,
            message: S2CMessage,
        ) {
            for (uid, session) in sessions.iter_mut() {
                if sender_id != Some(*uid) {
                    session.pending_messages.push(message.clone());
                }
            }
        }

        match message {
            C2SMessage::Connect => {
                let user_id = self.next_user_id();
                let entity_id = self
                    .world
                    .add_entity(Box::new(PlayerEntity::new(user_id, Vec3::ZERO)));
                self.sessions.insert(
                    user_id,
                    PlayerSession {
                        user_id,
                        entity_id,
                        nickname: None,
                        pending_messages: vec![S2CMessage::Connected { user_id }],
                    },
                );
                self.connections.insert(connection_id, user_id);
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
                let user_id = match self.connections.remove(&connection_id) {
                    Some(uid) => uid,
                    None => return,
                };
                let session = self.sessions.remove(&user_id);
                self.world.remove_entity(session.unwrap().entity_id);
                broadcast_message(
                    &mut self.sessions,
                    None,
                    S2CMessage::Disconnected { user_id },
                );
            }
            C2SMessage::Move(MoveInstructions {
                forward,
                strafe,
                jump,
                sneak,
                yaw,
                pitch,
            }) => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get(user_id)
                    && let Some(entity) =
                        self.world.get_entity_mut::<PlayerEntity>(session.entity_id)
                {
                    entity.yaw = yaw;
                    entity.pitch = pitch;
                    let forward_vec =
                        Vec3::new(yaw.to_radians().sin(), 0.0, yaw.to_radians().cos());
                    let right_vec = Vec3::new(yaw.to_radians().cos(), 0.0, -yaw.to_radians().sin());
                    let mut movement = Vec3::ZERO;
                    movement += forward_vec * (forward.clamp(-1, 2) as f32) * 7.5;
                    movement += right_vec * (strafe.clamp(-1, 1) as f32) * 7.5;
                    if jump {
                        movement.y += 6.0;
                    }
                    if sneak {
                        movement.y -= 6.0;
                    }
                    let dt = 1.0 / (self.tps as f32);
                    entity.apply_velocity(movement * dt * 5.0);
                    broadcast_message(
                        &mut self.sessions,
                        None,
                        S2CMessage::PlayerMoved {
                            user_id: *user_id,
                            position: entity.position,
                            yaw: entity.yaw,
                            pitch: entity.pitch,
                        },
                    );
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
                        .or_insert_with(|| Chunk::new(chunk_position, &self.world.noise));
                    if let Some(user_id) = self.connections.get(&connection_id)
                        && let Some(session) = self.sessions.get_mut(user_id)
                    {
                        session.pending_messages.push(S2CMessage::ChunkData {
                            chunk_position,
                            chunk: Box::new(chunk.clone()),
                        });
                    }
                }
            }
            C2SMessage::SendMessage { message } => {
                let user_id = match self.connections.get(&connection_id) {
                    Some(uid) => *uid,
                    None => return,
                };
                let status = self.execute_command(&message, connection_id);
                if let Err(err) = status {
                    if let Some(user_id) = self.connections.get(&connection_id)
                        && let Some(session) = self.sessions.get_mut(user_id)
                    {
                        session.pending_messages.push(S2CMessage::ChatMessage {
                            message: format!("%bC3Error executing command: %bD3{}%r", err)
                                .parse()
                                .unwrap(),
                        });
                    }
                    return;
                } else if let Some(success) = status.unwrap() {
                    if let Some(user_id) = self.connections.get(&connection_id)
                        && let Some(session) = self.sessions.get_mut(user_id)
                    {
                        session
                            .pending_messages
                            .push(S2CMessage::ChatMessage { message: success });
                    }
                    return;
                }
                let nickname = self.connections.get(&connection_id).and_then(|user_id| {
                    self.sessions
                        .get(user_id)
                        .and_then(|session| session.nickname.clone())
                });
                if let Some(nickname) = nickname {
                    broadcast_message(
                        &mut self.sessions,
                        None,
                        S2CMessage::ChatMessage {
                            message: format!("{}: {}", nickname, message).parse().unwrap(),
                        },
                    );
                } else {
                    if let Some(session) = self.sessions.get_mut(&user_id) {
                        session.pending_messages.push(S2CMessage::ChatMessage {
                            message: "%bC3Please set a nickname before chatting! To do so, use the command %bD3/nick <nickname>%r.".parse().unwrap(),
                        });
                    }
                }
            }
        }
    }

    /// Executes a server command, which may modify the world or player sessions.
    pub fn execute_command(
        &mut self,
        command: &str,
        connection_id: u64,
    ) -> Result<Option<TextComponent>, String> {
        if !command.starts_with('/') {
            return Ok(None);
        }
        let mut parts = command.split_whitespace();
        let cmd = parts.next().ok_or("No command provided")?;
        match cmd {
            "/nick" => {
                let nickname = parts.next().ok_or("No nickname provided")?;
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                {
                    session.nickname = Some(nickname.to_string());
                    Ok(Some(
                        format!("Nickname set to '{}'", nickname).parse().unwrap(),
                    ))
                } else {
                    Err("You must be connected to set a nickname".to_string())
                }
            }
            _ => Err("Unknown command".to_string()),
        }
    }

    /// Ticks the server.
    pub fn tick(&mut self, tps: u8) {
        self.tps = tps;
        self.world.tick(tps);
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
