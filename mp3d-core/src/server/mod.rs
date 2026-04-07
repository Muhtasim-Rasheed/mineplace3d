//! Server code for handling client connections and requests.
//!
//! Note that this does not include networking, for that please check mp3d-server (doesn't exist
//! yet) and instead focuses on the server-side logic.

use std::{collections::HashMap, path::PathBuf};

use glam::Vec3;

use crate::{
    TextComponent,
    entity::{Entity, PlayerEntity},
    protocol::*,
    world::{World, chunk::CHUNK_SIZE},
};

pub mod user;

/// The maximum distance (in chunks) that the server will keep loaded around players.
pub const MAX_RENDER_DIST: i32 = 12;

/// [`MAX_RENDER_DIST`] squared, used for distance checks without needing to calculate square
/// roots.
pub const MAX_RENDER_DIST_SQ: i32 = MAX_RENDER_DIST * MAX_RENDER_DIST;

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

/// Represents a connected client on the server.
pub struct PlayerSession {
    pub user_id: u64,
    pub entity_id: u64,
    pub username: String,
    pub pending_messages: Vec<S2CMessage>,
}

/// The main server struct that manages player sessions and world state.
pub struct Server {
    pub sessions: HashMap<u64, PlayerSession>,
    pub connections: HashMap<u64, u64>,
    pub world: World,
    pub singleplayer: bool,
    pub save_path: PathBuf,
    pub user_db: user::UserDatabase,
    pub tps: u8,
}

impl Server {
    /// Creates a new server instance. If the server is in singleplayer mode, it will not check
    /// credentials on connection and will allow only one player to connect at a time.
    pub fn new(singleplayer: bool, seed: i32, save_path: PathBuf) -> Server {
        Self {
            sessions: HashMap::new(),
            connections: HashMap::new(),
            world: World::new(seed),
            singleplayer,
            save_path: save_path.clone(),
            user_db: user::UserDatabase::load(save_path.join("users.json")),
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
    pub fn handle_message(
        &mut self,
        connection_id: u64,
        message: C2SMessage,
    ) -> Option<S2CMessage> {
        match message {
            C2SMessage::Connect { username, password } => {
                log::info!(
                    "Connection attempt from {} with username '{}'",
                    connection_id,
                    username
                );

                if self.singleplayer && !self.sessions.is_empty() {
                    log::warn!(
                        "Connection from {} rejected: singleplayer mode only allows one player",
                        connection_id
                    );

                    return Some(S2CMessage::ConnectionFailed {
                        reason: "Singleplayer mode only allows one player".to_string(),
                    });
                }

                let auth_result = self.user_db.login_or_register(username.clone(), password);

                match auth_result {
                    Ok(_) => {
                        log::info!(
                            "Connection from {} accepted for username '{}'",
                            connection_id,
                            username
                        );
                        let user_id = self.next_user_id();
                        let entity = if let Some(entity) = self.world.player_cache.remove(&username)
                        {
                            entity
                        } else {
                            PlayerEntity::new(username.clone(), Vec3::new(0.0, 25.0, 0.0))
                        };
                        let inventory = entity.inventory.clone();
                        let entity_id = self.world.add_entity(Box::new(entity));
                        self.sessions.insert(
                            user_id,
                            PlayerSession {
                                user_id,
                                entity_id,
                                username: username.clone(),
                                pending_messages: vec![S2CMessage::Connected {
                                    user_id,
                                    entity_id,
                                    inventory,
                                }],
                            },
                        );
                        self.connections.insert(connection_id, user_id);
                        broadcast_message(
                            &mut self.sessions,
                            None,
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
                        log::info!(
                            "User '{}' connected with user ID {} and entity ID {}",
                            username,
                            user_id,
                            entity_id
                        );
                    }
                    Err(reason) => {
                        log::warn!("Connection from {} rejected: {}", connection_id, reason);
                        return Some(S2CMessage::ConnectionFailed { reason });
                    }
                }
            }
            C2SMessage::Disconnect => {
                let user_id = self.connections.remove(&connection_id)?;

                if let Some(session) = self.sessions.remove(&user_id) {
                    if let Some(entity) = self.world.remove_entity(session.entity_id)
                        && let Ok(player_entity) = entity.into_any().downcast::<PlayerEntity>()
                    {
                        self.world
                            .player_cache
                            .insert(player_entity.username.clone(), *player_entity);
                    }

                    broadcast_message(
                        &mut self.sessions,
                        None,
                        S2CMessage::Disconnected { user_id },
                    );
                    log::info!(
                        "User '{}' with user ID {} disconnected",
                        session.username,
                        user_id
                    );
                }
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
                    entity.input = MoveInstructions {
                        forward,
                        strafe,
                        jump,
                        sneak,
                        yaw,
                        pitch,
                    }
                    .into();
                }
            }
            C2SMessage::RequestChunks { chunk_positions } => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(pos) = self
                        .world
                        .get_entity::<PlayerEntity>(session.entity_id)
                        .map(|e| (e.position / CHUNK_SIZE as f32).floor().as_ivec3())
                {
                    for chunk_position in chunk_positions {
                        if chunk_position.distance_squared(pos) > MAX_RENDER_DIST_SQ {
                            continue;
                        }
                        let chunk = self.world.get_chunk_or_new(chunk_position);
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
                    None => return None,
                };
                let status = self.execute_command(&message, connection_id);
                match status {
                    Ok(Some((other_msgs, success))) => {
                        if let Some(session) = self.sessions.get_mut(&user_id) {
                            log::info!("{} issued server command: {}", session.username, message);
                            for msg in other_msgs {
                                session.pending_messages.push(msg);
                            }
                            session
                                .pending_messages
                                .push(S2CMessage::ChatMessage { message: success });
                        }
                    }
                    Ok(None) => {
                        if let Some(session) = self.sessions.get_mut(&user_id) {
                            let username = session.username.clone();
                            if let Ok(c) = format!("{}%r: {}", username, message).parse() {
                                broadcast_message(
                                    &mut self.sessions,
                                    None,
                                    S2CMessage::ChatMessage { message: c },
                                );
                                log::info!("{}: {}", username, message);
                            } else {
                                session.pending_messages.push(S2CMessage::ChatMessage {
                                    message: "%bC3Error: Make sure your message doesn't contain invalid formatting codes.%r".parse().unwrap(),
                                });
                                log::warn!(
                                    "{} attempted to send a message with invalid formatting codes: {}",
                                    username,
                                    message
                                );
                            }
                        }
                    }
                    Err(err) => {
                        if let Some(session) = self.sessions.get_mut(&user_id) {
                            log::warn!(
                                "{} issued invalid server command: {}. Error: {}",
                                session.username,
                                message,
                                err
                            );
                            session.pending_messages.push(S2CMessage::ChatMessage {
                                message: format!("%bC3Error executing command: %bD3{}%r", err)
                                    .parse()
                                    .unwrap(),
                            });
                        }
                    }
                }
            }
            C2SMessage::BlockClick {
                position,
                face,
                right,
            } => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(player_pos) = self
                        .world
                        .get_entity::<PlayerEntity>(session.entity_id)
                        .map(|e| e.position)
                {
                    if position.as_vec3().distance_squared(player_pos) > 25.0 {
                        return None;
                    }
                    if right {
                        self.world
                            .block_interaction(session.entity_id, position, face);
                    } else {
                        self.world.urgent_set_block_at(
                            position,
                            crate::block::Block::AIR,
                            crate::block::BlockState::none(),
                            crate::protocol::BlockUpdateKind::Removed,
                        );
                    }
                }
            }
            C2SMessage::InventoryClick { idx, right } => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(player_entity) =
                        self.world.get_entity_mut::<PlayerEntity>(session.entity_id)
                {
                    player_entity.inventory.click(idx, right);
                    session.pending_messages.push(S2CMessage::InventoryUpdated {
                        inventory: player_entity.inventory.clone(),
                    });
                }
            }
            C2SMessage::HotbarChange { idx } => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(player_entity) =
                        self.world.get_entity_mut::<PlayerEntity>(session.entity_id)
                {
                    player_entity.hotbar_index = idx;
                }
            }
        }
        None
    }

    /// Executes a server command, which may modify the world or player sessions.
    pub fn execute_command(
        &mut self,
        command: &str,
        connection_id: u64,
    ) -> Result<Option<(Vec<S2CMessage>, TextComponent)>, String> {
        if !command.starts_with('/') {
            return Ok(None);
        }
        let mut parts = command.split_whitespace();
        let cmd = parts.next().ok_or("No command provided")?;
        match cmd {
            "/give" => {
                let item_name = parts.next().ok_or("No item specified")?;
                let count_str = parts.next().ok_or("No count specified")?;
                let count: u16 = count_str.parse().map_err(|_| "Invalid count")?;
                let item =
                    crate::item::Item::from_ident(item_name).ok_or("Unknown item identifier")?;
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(player_entity) =
                        self.world.get_entity_mut::<PlayerEntity>(session.entity_id)
                {
                    player_entity.inventory.add_stack(*item, count);
                    Ok(Some((
                        vec![S2CMessage::InventoryUpdated {
                            inventory: player_entity.inventory.clone(),
                        }],
                        format!("%b7FGave you {} x {}%r", count, item.ident)
                            .parse()
                            .unwrap(),
                    )))
                } else {
                    Err("You must be connected to use this command".to_string())
                }
            }
            "/tps" => Ok(Some((
                vec![],
                format!("Current TPS: {}%r", self.tps).parse().unwrap(),
            ))),
            _ => Err("Unknown command".to_string()),
        }
    }

    /// Ticks the server.
    pub fn tick(&mut self, tps: u8) {
        // Unload chunks that have no players nearby
        let player_positions: Vec<_> = self
            .sessions
            .values()
            .filter_map(|session| {
                self.world
                    .get_entity::<PlayerEntity>(session.entity_id)
                    .map(|entity| (entity.position / CHUNK_SIZE as f32).floor().as_ivec3())
            })
            .collect();
        self.world.chunks.retain(|&pos, _| {
            player_positions
                .iter()
                .any(|player_pos| pos.distance_squared(*player_pos) <= MAX_RENDER_DIST_SQ)
        });

        self.tps = tps;
        self.world.tick(tps);

        let pending_changes = std::mem::take(&mut self.world.pending_changes).collect::<Vec<_>>();
        broadcast_message(
            &mut self.sessions,
            None,
            S2CMessage::BlocksUpdated {
                updates: pending_changes,
            },
        );

        for entity in self.world.entities.values() {
            if let Some(entity) = entity.as_any().downcast_ref::<PlayerEntity>()
                && entity.velocity.length_squared() > 0.0
            {
                broadcast_message(
                    &mut self.sessions,
                    Some(entity.id()),
                    S2CMessage::PlayerMoved {
                        entity_id: entity.id(),
                        position: entity.position(),
                        yaw: entity.yaw,
                        pitch: entity.pitch,
                    },
                );
            }
        }
    }
}

impl Server {
    /// Saves the server state to disk, including the world and user database.
    pub fn save(&self) -> std::io::Result<()> {
        self.world.save(&self.save_path)?;
        self.user_db.save()?;
        Ok(())
    }

    /// Loads the server state from disk, including the world and user database.
    pub fn load(singleplayer: bool, save_path: PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            sessions: HashMap::new(),
            connections: HashMap::new(),
            world: World::load(&save_path)?,
            singleplayer,
            save_path: save_path.clone(),
            user_db: user::UserDatabase::load(save_path.join("users.json")),
            tps: 48,
        })
    }
}
