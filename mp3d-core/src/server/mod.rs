//! Server code for handling client connections and requests.
//!
//! Note that this does not include networking, for that please check mp3d-server (doesn't exist
//! yet) and instead focuses on the server-side logic.

use std::{collections::HashMap, path::PathBuf};

use fxhash::FxHashMap;
use glam::{IVec3, Vec3};

use crate::{
    command::{CommandContext, CommandManager, commands},
    entity::{EntityDetails, EntityId, MoveInput, components::*, ecs::Scheduler, systems},
    item::Inventory,
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
    sessions: &mut FxHashMap<u64, PlayerSession>,
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
    pub entity_id: EntityId,
    pub username: String,
    pub pending_messages: Vec<S2CMessage>,
}

impl PlayerSession {
    pub fn send_chat_message(
        self_id: u64,
        sessions: &mut FxHashMap<u64, PlayerSession>,
        message: &str,
    ) {
        if let Some(session) = sessions.get_mut(&self_id) {
            let username = session.username.clone();
            if let Ok(c) = format!("{}%r: {}", username, message).parse() {
                broadcast_message(sessions, None, S2CMessage::ChatMessage { message: c });
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
}

/// The main server struct that manages player sessions and world state.
pub struct Server {
    pub sessions: FxHashMap<u64, PlayerSession>,
    pub connections: FxHashMap<u64, u64>,
    pub entity_to_user: FxHashMap<EntityId, u64>,
    pub world: World,
    pub scheduler: Scheduler,
    pub singleplayer: bool,
    pub save_path: PathBuf,
    pub user_db: user::UserDatabase,
    pub command_manager: CommandManager,
    pub tps: u8,
}

impl Server {
    /// Creates a new server instance. If the server is in singleplayer mode, it will not check
    /// credentials on connection and will allow only one player to connect at a time.
    pub fn new(singleplayer: bool, seed: i32, save_path: PathBuf) -> Server {
        let mut command_manager = CommandManager::new();
        commands::init_command_mgr(&mut command_manager);
        let scheduler = Scheduler::new().add_system(systems::movement_system);
        Self {
            sessions: FxHashMap::default(),
            connections: FxHashMap::default(),
            entity_to_user: FxHashMap::default(),
            world: World::new(seed),
            scheduler,
            singleplayer,
            save_path: save_path.clone(),
            user_db: user::UserDatabase::load(save_path.join("users.json")),
            command_manager,
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

    /// Gets a session by entity ID, if it exists.
    pub fn get_session_by_entity<'a>(
        entity_to_user: &HashMap<EntityId, u64>,
        sessions: &'a HashMap<u64, PlayerSession>,
        entity_id: EntityId,
    ) -> Option<&'a PlayerSession> {
        let user_id = entity_to_user.get(&entity_id)?;
        sessions.get(user_id)
    }

    /// Gets a mutable reference to a session by entity ID, if it exists.
    pub fn get_session_by_entity_mut<'a>(
        entity_to_user: &FxHashMap<EntityId, u64>,
        sessions: &'a mut FxHashMap<u64, PlayerSession>,
        entity_id: EntityId,
    ) -> Option<&'a mut PlayerSession> {
        let user_id = entity_to_user.get(&entity_id)?;
        sessions.get_mut(user_id)
    }

    /// Default entity details for a player who has logged in for the first time.
    fn default_player_details(username: &str, pos: Vec3) -> EntityDetails {
        EntityDetails::builder()
            .with(Position(pos))
            .with(Velocity(Vec3::ZERO))
            .with(Rotation {
                yaw: 0.0,
                pitch: 0.0,
            })
            .with(OnGround(false))
            .with(Username(username.to_string()))
            .with(Inventory::new())
            .with(SelectedHotbarSlot(0))
            .with(Flying(false))
            .with(Hitbox {
                width: 0.8,
                height: 1.8,
            })
            .with(MoveInput::default())
            .build()
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
                        let entitydet =
                            if let Some(entitydet) = self.world.player_cache.remove(&username) {
                                entitydet
                            } else {
                                Self::default_player_details(&username, Vec3::new(0.0, 25.0, 0.0))
                            };
                        self.world.load_around(IVec3::new(0, 25, 0));
                        let entity_id = self.world.ecs.spawn_from_details(&entitydet);
                        let mut session = PlayerSession {
                            user_id,
                            entity_id,
                            username: username.clone(),
                            pending_messages: vec![S2CMessage::Connected { user_id, entity_id }],
                        };
                        self.connections.insert(connection_id, user_id);
                        self.entity_to_user.insert(entity_id, user_id);
                        let render_distance = 8.0 * MAX_RENDER_DIST as f32;

                        for other_entity in self.world.entities_in_range(
                            entitydet.get::<Position>().unwrap().0,
                            render_distance,
                        ) {
                            if other_entity == entity_id {
                                continue; // don't send the joining player their own entity twice
                            }

                            let details = self.world.ecs.entity_details(other_entity);
                            session.pending_messages.push(S2CMessage::EntitySpawned {
                                entity_id: other_entity,
                                entity_details: details.to_bytes(),
                            });
                        }
                        self.sessions.insert(user_id, session);
                        broadcast_message(
                            &mut self.sessions,
                            None,
                            S2CMessage::EntitySpawned {
                                entity_id,
                                entity_details: entitydet.to_bytes(),
                            },
                        );
                        log::info!(
                            "User '{username}' connected with user ID {user_id} and entity ID {entity_id}"
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
                    let details = self.world.ecs.entity_details(session.entity_id);
                    self.world.ecs.despawn(session.entity_id);

                    self.world
                        .player_cache
                        .insert(session.username.clone(), details);

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
            C2SMessage::Move(inst) => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get(user_id)
                {
                    if let Some(rot) = self
                        .world
                        .ecs
                        .get_component_mut::<Rotation>(session.entity_id)
                    {
                        rot.yaw = inst.yaw;
                        rot.pitch = inst.pitch;
                    }
                    if let Some(input) = self
                        .world
                        .ecs
                        .get_component_mut::<MoveInput>(session.entity_id)
                    {
                        *input = inst.into();
                    }
                }
            }
            C2SMessage::RequestChunks { chunk_positions } => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(pos) = self
                        .world
                        .ecs
                        .get_component::<Position>(session.entity_id)
                        .map(|v| v.0 / CHUNK_SIZE as f32)
                {
                    session.pending_messages.push(S2CMessage::ChunkData {
                        chunks: chunk_positions
                            .into_iter()
                            .filter_map(|chunk_position| {
                                let cp_float = chunk_position.as_vec3() + Vec3::splat(0.5);
                                if cp_float.distance_squared(pos) > MAX_RENDER_DIST_SQ as f32 {
                                    return None;
                                }
                                let chunk = self.world.get_chunk_or_new(chunk_position);
                                Some((chunk_position, chunk.clone()))
                            })
                            .collect::<Vec<_>>(),
                    });
                }
            }
            C2SMessage::SendMessage { message } => {
                let user_id = match self.connections.get(&connection_id) {
                    Some(uid) => *uid,
                    None => return None,
                };
                let mut ctx = CommandContext {
                    connections: &self.connections,
                    sessions: &mut self.sessions,
                    world: &mut self.world,
                    command_manager: &self.command_manager,
                    connection_id,
                    tps: self.tps,
                };
                let args = message.split_whitespace().collect::<Vec<_>>();
                let status = self.command_manager.execute(&mut ctx, &args);
                match status {
                    Ok(Some(success)) => {
                        if let Some(session) = self.sessions.get_mut(&user_id) {
                            log::info!("{} issued server command: {}", session.username, message);
                            session
                                .pending_messages
                                .push(S2CMessage::ChatMessage { message: success });
                        }
                    }
                    Ok(None) => {
                        PlayerSession::send_chat_message(user_id, &mut self.sessions, &message);
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
                    && let Some(pos) = self
                        .world
                        .ecs
                        .get_component_copied::<Position>(session.entity_id)
                {
                    if position.as_vec3().distance_squared(pos.0) > 25.0 {
                        return None;
                    }
                    if right {
                        self.world
                            .block_interaction(session.entity_id, position, face);
                    } else {
                        self.world.break_block(session.entity_id, position);
                    }
                }
            }
            C2SMessage::InventoryClick { idx, right } => {
                if let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(inv) = self
                        .world
                        .ecs
                        .get_component_mut::<Inventory>(session.entity_id)
                {
                    inv.click(idx, right);
                }
            }
            C2SMessage::HotbarChange { idx } => {
                if idx < 9
                    && let Some(user_id) = self.connections.get(&connection_id)
                    && let Some(session) = self.sessions.get_mut(user_id)
                    && let Some(selected_hotbar) = self
                        .world
                        .ecs
                        .get_component_mut::<SelectedHotbarSlot>(session.entity_id)
                {
                    selected_hotbar.0 = idx;
                }
            }
        }
        None
    }

    fn replication_tick(&mut self) {
        for entity in self.world.ecs.e_alloc.iter() {
            let current = self.world.ecs.entity_details(entity);
            let previous = self.world.replicated_snapshots.get(&entity);

            let diff = match previous {
                Some(prev) => prev.diff(&current),
                None => current.clone(),
            };

            if !diff.is_empty() {
                broadcast_message(
                    &mut self.sessions,
                    None,
                    S2CMessage::EntityUpdated {
                        entity_id: entity,
                        entity_details: diff.to_bytes(),
                    },
                );
            }

            self.world.replicated_snapshots.insert(entity, current);
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
                    .ecs
                    .get_component::<Position>(session.entity_id)
                    .map(|v| v.0 / CHUNK_SIZE as f32)
            })
            .collect();
        self.world.chunks.retain(|&pos, _| {
            let pos = pos.as_vec3() + Vec3::splat(0.5);
            player_positions
                .iter()
                .any(|player_pos| pos.distance_squared(*player_pos) as i32 <= MAX_RENDER_DIST_SQ)
        });

        self.tps = tps;
        self.world.tick(&mut self.scheduler, tps);

        let pending_changes = std::mem::take(&mut self.world.pending_changes).collect::<Vec<_>>();
        broadcast_message(
            &mut self.sessions,
            None,
            S2CMessage::BlocksUpdated {
                updates: pending_changes,
            },
        );

        self.replication_tick();
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
        let mut command_manager = CommandManager::new();
        commands::init_command_mgr(&mut command_manager);
        let scheduler = Scheduler::new().add_system(systems::movement_system);
        Ok(Self {
            sessions: FxHashMap::default(),
            connections: FxHashMap::default(),
            entity_to_user: FxHashMap::default(),
            world: World::load(&save_path)?,
            scheduler,
            singleplayer,
            save_path: save_path.clone(),
            user_db: user::UserDatabase::load(save_path.join("users.json")),
            command_manager,
            tps: 48,
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        log::info!("Closing server!");
    }
}
