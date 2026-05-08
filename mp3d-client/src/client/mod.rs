//! Client to interact with a local server.
//!
//! This module provides functionality to connect to a server, where if the client is using a local
//! connection, it directly calls the server's message handling functions. Remote connections are
//! not implemented yet.
//!
//! The module also provides a `Connection` trait and a `LocalConnection` struct that implements
//! this trait for local server interactions.

pub mod chunk;
mod emoji;
pub mod player;
pub mod world;

use std::{cell::RefCell, rc::Rc};

use glam::{IVec3, Vec3};
use mp3d_core::{
    TextComponent,
    protocol::{C2SMessage, MoveInstructions, S2CMessage},
    server::Server,
};
use sdl2::keyboard::Keycode;

use crate::{
    client::{player::ClientInventory, world::ClientWorld},
    other::UpdateContext,
    render::particles::ParticleSystem,
};

/// The [`Connection`] trait defines the interface for client-server communication.
pub trait Connection {
    /// Sends a message to the server.
    fn send(&mut self, message: C2SMessage);

    /// Ticks the connection to update its state.
    fn tick(&mut self, _tps: u8) {}

    // Receives messages from the server.
    fn receive(&mut self) -> Vec<S2CMessage>;
}

/// A local connection that directly interacts with a server instance.
///
/// The [`LocalConnection`] owns the server instance instead of borrowing it. The local connection
/// will use a connection ID of `0` for all interactions since it is the only connection, and the
/// server does not need to differentiate between multiple clients.
pub struct LocalConnection {
    pub server: Server,
    pub message: Option<S2CMessage>,
}

impl LocalConnection {
    /// Creates a new `LocalConnection` with the given server and user ID.
    pub fn new(server: Server) -> Self {
        log::info!("Creating local connection");

        Self {
            server,
            message: None,
        }
    }
}

impl Connection for LocalConnection {
    fn send(&mut self, message: C2SMessage) {
        self.message = self.server.handle_message(0, message);
    }

    fn tick(&mut self, tps: u8) {
        self.server.tick(tps);
    }

    fn receive(&mut self) -> Vec<S2CMessage> {
        if let Some(user_id) = self.server.connections.get(&0)
            && let Some(session) = self.server.sessions.get_mut(user_id)
        {
            std::mem::take(&mut session.pending_messages)
        } else if let Some(message) = self.message.take() {
            vec![message]
        } else {
            vec![]
        }
    }
}

/// An enum representing the different GUIs that can be opened on the client.
pub enum CurrentGUI {
    None,
    Chat(String),
    Inventory,
    PauseMenu,
}

impl CurrentGUI {
    pub fn none(&self) -> bool {
        matches!(self, CurrentGUI::None)
    }

    pub fn chat(&self) -> Option<&String> {
        if let CurrentGUI::Chat(message) = self {
            Some(message)
        } else {
            None
        }
    }

    pub fn inventory(&self) -> bool {
        matches!(self, CurrentGUI::Inventory)
    }

    pub fn pause_menu(&self) -> bool {
        matches!(self, CurrentGUI::PauseMenu)
    }
}

/// The client struct that uses a connection to communicate with the server.
pub struct Client<C: Connection> {
    pub connection: C,
    pub player: player::ClientPlayer,
    pub user_id: Option<u64>,
    pub entity_id: Option<u64>,
    pub gui: CurrentGUI,
    pub messages: Vec<TextComponent>,
    pub world: ClientWorld,
}

impl<C: Connection> Client<C> {
    /// Creates a new `Client` with the given connection and credentials. If password is `None`, it
    /// will use default password "SINGLEPLAYER". The client will send a `Connect` message to the
    /// server with the provided credentials upon initialization.
    pub fn new(mut connection: C, username: String, password: Option<String>) -> Self {
        log::info!("Creating client with username '{}'", username);

        if let Some(password) = password {
            connection.send(C2SMessage::Connect { username, password });
        } else {
            connection.send(C2SMessage::Connect {
                username,
                password: "SINGLEPLAYER".to_string(),
            });
        }

        Self {
            connection,
            player: player::ClientPlayer {
                position: Vec3::ZERO,
                velocity: Vec3::ZERO,
                yaw: 0.0,
                delta_yaw: 0.0,
                pitch: 0.0,
                fov: 90.0,
                flying: false,
                on_ground: false,
                input: MoveInstructions::default(),
                inventory: Rc::new(RefCell::new(ClientInventory::new())),
                third_person: false,
            },
            user_id: None,
            entity_id: None,
            gui: CurrentGUI::None,
            messages: vec![],
            world: ClientWorld::new(),
        }
    }

    /// Takes in player input and sends it to the server through the connection.
    pub fn send_input(&mut self, update_context: &UpdateContext, dt: f32, sensitivity: f32) {
        if update_context.keyboard.pressed.contains(&Keycode::Escape) {
            self.gui = match self.gui {
                CurrentGUI::None => CurrentGUI::PauseMenu,
                CurrentGUI::PauseMenu => CurrentGUI::None,
                CurrentGUI::Chat(_) => CurrentGUI::None,
                CurrentGUI::Inventory => CurrentGUI::None,
            };
        }

        if !self.gui.none() {
            self.player.input = MoveInstructions::default();
        }

        // woah is that a state machine
        match &mut self.gui {
            CurrentGUI::None => {
                let mouse_delta = update_context.mouse.delta;
                let previous_yaw = self.player.yaw;
                self.player.yaw -= mouse_delta.x * 0.1 * sensitivity;
                self.player.pitch += mouse_delta.y * 0.1 * sensitivity;
                self.player.pitch = self.player.pitch.clamp(-89.0, 89.0);
                self.player.yaw = self.player.yaw.rem_euclid(360.0);
                self.player.delta_yaw = self.player.yaw - previous_yaw;

                self.player.input.yaw = self.player.yaw;
                self.player.input.pitch = self.player.pitch;

                let kb = &update_context.keyboard;

                self.player.input.forward = if kb.down.contains(&Keycode::W) {
                    if kb.down.contains(&Keycode::LCtrl) {
                        2
                    } else {
                        1
                    }
                } else if kb.down.contains(&Keycode::S) {
                    -1
                } else {
                    0
                };

                self.player.input.strafe = if kb.down.contains(&Keycode::A) {
                    1
                } else if kb.down.contains(&Keycode::D) {
                    -1
                } else {
                    0
                };

                self.player.input.jump = kb.down.contains(&Keycode::Space);
                self.player.input.sneak = kb.down.contains(&Keycode::LShift);

                if kb.pressed.contains(&Keycode::F5) {
                    self.player.third_person = !self.player.third_person;
                }

                if update_context
                    .mouse
                    .pressed
                    .contains(&sdl2::mouse::MouseButton::Left)
                    && let Some((position, face)) = cast_ray(&self.world, &self.player, 5.0) {
                        self.connection.send(C2SMessage::BlockClick {
                            position,
                            face: match face {
                                IVec3 { z: -1, .. } => 0,
                                IVec3 { z: 1, .. } => 1,
                                IVec3 { x: 1, .. } => 2,
                                IVec3 { x: -1, .. } => 3,
                                IVec3 { y: 1, .. } => 4,
                                IVec3 { y: -1, .. } => 5,
                                _ => unreachable!(),
                            },
                            right: false,
                        });
                    }

                if update_context
                    .mouse
                    .pressed
                    .contains(&sdl2::mouse::MouseButton::Right)
                    && let Some((block_pos, normal)) = cast_ray(&self.world, &self.player, 5.0) {
                        let face_idx = match normal {
                            IVec3 { z: -1, .. } => 0,
                            IVec3 { z: 1, .. } => 1,
                            IVec3 { x: 1, .. } => 2,
                            IVec3 { x: -1, .. } => 3,
                            IVec3 { y: 1, .. } => 4,
                            IVec3 { y: -1, .. } => 5,
                            _ => unreachable!(),
                        };

                        self.connection.send(C2SMessage::BlockClick {
                            position: block_pos,
                            face: face_idx,
                            right: true,
                        });
                    }

                if kb.pressed.contains(&Keycode::T) {
                    self.gui = CurrentGUI::Chat(String::new());
                }

                if kb.pressed.contains(&Keycode::Slash) {
                    self.gui = CurrentGUI::Chat("/".to_string());
                }

                if kb.pressed.contains(&Keycode::E) {
                    self.gui = CurrentGUI::Inventory;
                }

                for (i, key) in [
                    Keycode::Num1,
                    Keycode::Num2,
                    Keycode::Num3,
                    Keycode::Num4,
                    Keycode::Num5,
                    Keycode::Num6,
                    Keycode::Num7,
                    Keycode::Num8,
                    Keycode::Num9,
                ]
                .iter()
                .enumerate()
                {
                    if kb.pressed.contains(key) {
                        self.connection.send(C2SMessage::HotbarChange { idx: i });
                        self.player.inventory.borrow_mut().slot = i;
                        break;
                    }
                }
            }

            CurrentGUI::Chat(message) => {
                let mut msg = message.clone();
                msg.push_str(&update_context.keyboard.text_input);

                let kb = &update_context.keyboard;

                if kb.pressed.contains(&Keycode::Return) && !msg.trim().is_empty() {
                    self.connection.send(C2SMessage::SendMessage {
                        message: msg.trim().to_string(),
                    });
                    self.gui = CurrentGUI::None;
                } else if kb.pressed.contains(&Keycode::Backspace) {
                    msg.pop();
                    self.gui = CurrentGUI::Chat(msg);
                } else {
                    let replaced = emoji::replace_emojis(&msg);
                    if replaced != msg {
                        self.gui = CurrentGUI::Chat(replaced);
                    } else {
                        *message = msg;
                    }
                }
            }

            CurrentGUI::Inventory => {
                // Handled elsewhere
            }

            CurrentGUI::PauseMenu => {}
        }

        self.player.optimistic(dt, &self.world);

        self.connection.send(C2SMessage::Move(self.player.input));

        let needed_chunks = self.world.needs_chunks(self.player.position.as_ivec3());
        self.connection.send(C2SMessage::RequestChunks {
            chunk_positions: needed_chunks,
        });

        let inventory_changes = std::mem::take(&mut self.player.inventory.borrow_mut().clicks);
        for (idx, right) in inventory_changes {
            self.connection
                .send(C2SMessage::InventoryClick { idx, right });
        }
    }

    /// Updates any state on the client side from all received messages from the server.
    pub fn receive_state(&mut self, particle_system: &mut ParticleSystem) -> Result<(), String> {
        let messages = self.connection.receive();
        for message in messages {
            match message {
                S2CMessage::Connected {
                    user_id,
                    entity_id,
                    inventory,
                } => {
                    log::info!(
                        "Connected to server with user ID {} and entity ID {}",
                        user_id,
                        entity_id
                    );
                    self.user_id = Some(user_id);
                    self.entity_id = Some(entity_id);
                    self.player
                        .inventory
                        .borrow_mut()
                        .update_from_inventory(inventory);
                }
                S2CMessage::ConnectionFailed { reason } => {
                    log::error!("Connection failed!");
                    return Err(reason);
                }
                S2CMessage::EntitySpawned {
                    entity_id: _,
                    entity_type,
                    entity_snapshot,
                } => {
                    if entity_type == mp3d_core::entity::EntityType::Player as u8 {
                        log::info!("Player snapshot received, {} bytes", entity_snapshot.len());
                        if u64::from_le_bytes(entity_snapshot[0..8].try_into().unwrap())
                            == self.entity_id.unwrap()
                        {
                            self.player.update_from_snapshot(&entity_snapshot);
                        }
                    }
                }
                S2CMessage::PlayerMoved {
                    entity_id,
                    position,
                    ..
                } => {
                    if Some(entity_id) != self.entity_id {
                        continue;
                    }
                    let delta = position - self.player.position;
                    if delta.length_squared() > 3.0 * 3.0 {
                        self.player.position = position;
                    } else {
                        self.player.position += delta * 0.15;
                    }
                }
                S2CMessage::InventoryUpdated { inventory } => {
                    self.player
                        .inventory
                        .borrow_mut()
                        .update_from_inventory(inventory);
                }
                S2CMessage::ChunkData {
                    chunk_position,
                    chunk,
                } => {
                    self.world.chunks.insert(chunk_position, (*chunk).into());
                    self.world.remesh_queue.push(chunk_position, true);
                    // also push the other neighbor chunks to the remesh queue
                    for neighbor in [
                        chunk_position + IVec3::new(0, 0, -1),
                        chunk_position + IVec3::new(0, 0, 1),
                        chunk_position + IVec3::new(1, 0, 0),
                        chunk_position + IVec3::new(-1, 0, 0),
                        chunk_position + IVec3::new(0, 1, 0),
                        chunk_position + IVec3::new(0, -1, 0),
                    ] {
                        self.world.remesh_queue.push(neighbor, false);
                    }
                }
                S2CMessage::ChatMessage { message } => {
                    self.messages.push(message);
                }
                S2CMessage::BlocksUpdated { updates } => {
                    for update in updates {
                        if update.kind == mp3d_core::protocol::BlockUpdateKind::Removed {
                            let Some((old_block, old_state)) =
                                self.world.get_block_at(update.position)
                            else {
                                continue;
                            };
                            particle_system.block_break(update.position, old_block, old_state);
                        }
                        self.world.set_block_at(
                            update.position,
                            update.block,
                            update.block_state,
                            update.urgent,
                        );
                    }
                }
                S2CMessage::HotbarChanged { idx } => {
                    self.player.inventory.borrow_mut().slot = idx;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Performs a raycast from the player's position in the direction they are looking, returning the
/// position and normal of the first block hit within the specified range, or `None` if no block is
/// hit.
pub fn cast_ray(
    world: &ClientWorld,
    player: &player::ClientPlayer,
    max_distance: f32,
) -> Option<(IVec3, IVec3)> {
    let mut pos = player.first_person_eye();
    let yaw_rad = player.yaw.to_radians();
    let pitch_rad = player.pitch.to_radians();
    let direction = Vec3::new(
        yaw_rad.sin() * pitch_rad.cos(),
        -pitch_rad.sin(),
        yaw_rad.cos() * pitch_rad.cos(),
    )
    .normalize();
    let step = 0.003;

    for _ in 0..(max_distance / step) as usize {
        let block_pos = pos.floor().as_ivec3();

        let (block, state) = world.get_block_at(block_pos)?;

        let local = pos - block_pos.as_vec3();

        if block.visible {
            let ray_intersection = block.ray_intersect(local, direction, *state);
            if let Some(normal) = ray_intersection {
                return Some((block_pos, normal));
            }
        }

        pos += direction * step;
    }

    None
}
