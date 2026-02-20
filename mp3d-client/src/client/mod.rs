//! Client to interact with a local server.
//!
//! This module provides functionality to connect to a server, where if the client is using a local
//! connection, it directly calls the server's message handling functions. Remote connections are
//! not implemented yet.
//!
//! The module also provides a `Connection` trait and a `LocalConnection` struct that implements
//! this trait for local server interactions.

pub mod chunk;
pub mod player;
pub mod world;

use glam::{IVec3, Vec3};
use mp3d_core::{
    TextComponent,
    protocol::{C2SMessage, MoveInstructions, S2CMessage},
    server::Server,
};

use crate::{client::world::ClientWorld, other::UpdateContext};

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
}

impl LocalConnection {
    /// Creates a new `LocalConnection` with the given server and user ID.
    pub fn new(server: Server) -> Self {
        Self { server }
    }
}

impl Connection for LocalConnection {
    fn send(&mut self, message: C2SMessage) {
        self.server.handle_message(0, message);
    }

    fn tick(&mut self, tps: u8) {
        self.server.tick(tps);
    }

    fn receive(&mut self) -> Vec<S2CMessage> {
        if let Some(user_id) = self.server.connections.get(&0)
            && let Some(session) = self.server.sessions.get_mut(user_id)
        {
            std::mem::take(&mut session.pending_messages)
        } else {
            vec![]
        }
    }
}

/// The client struct that uses a connection to communicate with the server.
pub struct Client<C: Connection> {
    pub connection: C,
    pub player: player::ClientPlayer,
    pub user_id: Option<u64>,
    pub chat_message: Option<String>,
    pub chat_open: bool,
    pub messages: Vec<TextComponent>,
    pub world: ClientWorld,
}

impl<C: Connection> Client<C> {
    /// Creates a new `Client` with the given connection.
    pub fn new(mut connection: C) -> Self {
        connection.send(C2SMessage::Connect);

        Self {
            connection,
            player: player::ClientPlayer {
                position: Vec3::ZERO,
                velocity: Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.0,
                fov: 90.0,
                input: MoveInstructions::default(),
            },
            user_id: None,
            chat_message: None,
            chat_open: false,
            messages: vec![],
            world: ClientWorld::new(),
        }
    }

    /// Takes in player input and sends it to the server through the connection.
    pub fn send_input(&mut self, update_context: &UpdateContext, tps: u8) {
        if !self.chat_open {
            let mouse_delta = update_context.mouse.delta;
            self.player.yaw -= mouse_delta.x * 0.1;
            self.player.pitch += mouse_delta.y * 0.1;
            self.player.pitch = self.player.pitch.clamp(-89.0, 89.0);
            self.player.yaw = self.player.yaw.rem_euclid(360.0);
            self.player.input.yaw = self.player.yaw;
            self.player.input.pitch = self.player.pitch;

            if update_context
                .keyboard
                .down
                .contains(&sdl2::keyboard::Keycode::W)
            {
                if update_context
                    .keyboard
                    .down
                    .contains(&sdl2::keyboard::Keycode::LCtrl)
                {
                    self.player.input.forward = 2;
                } else {
                    self.player.input.forward = 1;
                }
            } else if update_context
                .keyboard
                .down
                .contains(&sdl2::keyboard::Keycode::S)
            {
                self.player.input.forward = -1;
            } else {
                self.player.input.forward = 0;
            }

            if update_context
                .keyboard
                .down
                .contains(&sdl2::keyboard::Keycode::A)
            {
                self.player.input.strafe = 1;
            } else if update_context
                .keyboard
                .down
                .contains(&sdl2::keyboard::Keycode::D)
            {
                self.player.input.strafe = -1;
            } else {
                self.player.input.strafe = 0;
            }

            self.player.input.jump = update_context
                .keyboard
                .down
                .contains(&sdl2::keyboard::Keycode::Space);

            self.player.input.sneak = update_context
                .keyboard
                .down
                .contains(&sdl2::keyboard::Keycode::LShift);

            if update_context
                .mouse
                .pressed
                .contains(&sdl2::mouse::MouseButton::Left)
            {
                let raycast_result = cast_ray(&self.world, &self.player, 5.0);
                if let Some((block_pos, _)) = raycast_result {
                    self.world
                        .set_block_at(block_pos, mp3d_core::block::Block::AIR);
                }
            }

            if update_context
                .mouse
                .pressed
                .contains(&sdl2::mouse::MouseButton::Right)
            {
                let raycast_result = cast_ray(&self.world, &self.player, 5.0);
                if let Some((block_pos, normal)) = raycast_result {
                    let place_pos = block_pos + normal;
                    self.world
                        .set_block_at(place_pos, mp3d_core::block::Block::STONE);
                }
            }

            if update_context
                .keyboard
                .pressed
                .contains(&sdl2::keyboard::Keycode::T)
            {
                self.chat_open = true;
            }

            if update_context
                .keyboard
                .pressed
                .contains(&sdl2::keyboard::Keycode::Slash)
            {
                self.chat_open = true;
                self.chat_message = Some("/".to_string());
            }
        } else {
            self.chat_message
                .get_or_insert_with(String::new)
                .push_str(&update_context.keyboard.text_input);

            if update_context
                .keyboard
                .pressed
                .contains(&sdl2::keyboard::Keycode::Return)
            {
                if let Some(message) = self.chat_message.take()
                    && !message.trim().is_empty()
                {
                    self.connection.send(C2SMessage::SendMessage {
                        message: message.trim().to_string(),
                    });
                    self.chat_open = false;
                    self.chat_message = None;
                }
            }

            if update_context
                .keyboard
                .pressed
                .contains(&sdl2::keyboard::Keycode::Escape)
            {
                self.chat_open = false;
                self.chat_message = None;
            }

            if update_context
                .keyboard
                .pressed
                .contains(&sdl2::keyboard::Keycode::Backspace)
            {
                if let Some(message) = self.chat_message.as_mut() {
                    message.pop();
                }
            }
        }

        self.player.optimistic(tps);

        self.connection.send(C2SMessage::Move(self.player.input));

        let needed_chunks = self.world.needs_chunks(self.player.position.as_ivec3());
        self.connection.send(C2SMessage::RequestChunks {
            chunk_positions: needed_chunks,
        });

        let block_changes = std::mem::take(&mut self.world.pending_changes);
        for (position, block) in block_changes {
            self.connection
                .send(C2SMessage::SetBlock { position, block });
        }
    }

    /// Updates any state on the client side from all recieved messages from the server
    pub fn recieve_state(&mut self) {
        let messages = self.connection.receive();
        for message in messages {
            match message {
                S2CMessage::Connected { user_id } => {
                    self.user_id = Some(user_id);
                }
                S2CMessage::EntitySpawned {
                    entity_id: _,
                    entity_type,
                    entity_snapshot,
                } => {
                    if entity_type == mp3d_core::entity::EntityType::Player as u8 {
                        println!("Player snapshot {:?}", entity_snapshot);
                    }
                }
                S2CMessage::PlayerMoved {
                    user_id,
                    position,
                    yaw,
                    pitch,
                } => {
                    if Some(user_id) != self.user_id {
                        continue;
                    }
                    let delta = position - self.player.position;
                    if delta.length_squared() > 9.0 {
                        self.player.position = position;
                    } else {
                        self.player.position += delta * 0.15;
                    }
                    self.player.yaw = yaw;
                    self.player.pitch = pitch;
                }
                S2CMessage::ChunkData {
                    chunk_position,
                    chunk,
                } => {
                    self.world.chunks.insert(chunk_position, (*chunk).into());
                }
                S2CMessage::ChatMessage { message } => {
                    self.messages.push(message);
                }
                _ => {}
            }
        }
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
    let mut pos = player.position;
    let yaw_rad = player.yaw.to_radians();
    let pitch_rad = player.pitch.to_radians();
    let direction = Vec3::new(
        yaw_rad.sin() * pitch_rad.cos(),
        -pitch_rad.sin(),
        yaw_rad.cos() * pitch_rad.cos(),
    )
    .normalize();
    let step = 0.01;

    for _ in 0..(max_distance / step) as usize {
        let block_pos = pos.floor().as_ivec3();
        let block = world.get_block_at(block_pos)?;

        if block.full {
            let normal = calc_face_normal(pos, block_pos.as_vec3());
            return Some((block_pos, normal));
        }

        pos += direction * step;
    }

    None
}

fn calc_face_normal(hit: Vec3, block: Vec3) -> IVec3 {
    let rel = hit - block;

    // Distances to faces
    let dx = rel.x.min(1.0 - rel.x).abs();
    let dy = rel.y.min(1.0 - rel.y).abs();
    let dz = rel.z.min(1.0 - rel.z).abs();

    let min = dx.min(dy.min(dz));

    if min == dx {
        if rel.x < 0.5 {
            glam::ivec3(-1, 0, 0)
        } else {
            glam::ivec3(1, 0, 0)
        }
    } else if min == dy {
        if rel.y < 0.5 {
            glam::ivec3(0, -1, 0)
        } else {
            glam::ivec3(0, 1, 0)
        }
    } else if rel.z < 0.5 {
        glam::ivec3(0, 0, -1)
    } else {
        glam::ivec3(0, 0, 1)
    }
}
