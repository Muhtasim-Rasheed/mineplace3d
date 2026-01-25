//! Client to interact with a local server.
//!
//! This module provides functionality to connect to a server, where if the client is using a local
//! connection, it directly calls the server's message handling functions. Remote connections are
//! not implemented yet.
//!
//! The module also provides a `Connection` trait and a `LocalConnection` struct that implements
//! this trait for local server interactions.

use glam::Vec3;
use mp3d_core::{
    protocol::{C2SMessage, MoveInstructions, S2CMessage},
    server::Server,
};

use crate::{clientplayer::ClientPlayer, other::UpdateContext};

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
    server: Server,
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
    pub player: ClientPlayer,
    pub user_id: Option<u64>,
}

impl<C: Connection> Client<C> {
    /// Creates a new `Client` with the given connection.
    pub fn new(mut connection: C) -> Self {
        connection.send(C2SMessage::Connect);

        Self {
            connection,
            player: ClientPlayer {
                position: Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.0,
                fov: 90.0,
                input: MoveInstructions::default(),
            },
            user_id: None,
        }
    }

    /// Takes in player input and sends it to the server through the connection.
    pub fn send_input(&mut self, update_context: &UpdateContext) {
        let mouse_delta = update_context.mouse.delta;
        self.player.yaw -= mouse_delta.x * 0.1;
        self.player.pitch -= mouse_delta.y * 0.1;
        self.player.pitch = self.player.pitch.clamp(-89.0, 89.0);
        self.player.yaw = self.player.yaw.rem_euclid(360.0);

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
            self.player.input.strafe = -1;
        } else if update_context
            .keyboard
            .down
            .contains(&sdl2::keyboard::Keycode::D)
        {
            self.player.input.strafe = 1;
        } else {
            self.player.input.strafe = 0;
        }

        self.connection.send(C2SMessage::Move(self.player.input));
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
                    entity_id,
                    entity_type,
                    entity_snapshot,
                } => {
                    if entity_type == mp3d_core::entity::EntityType::Player as u8 {
                        println!("Player snapshot {:?}", entity_snapshot,);
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
                    self.player.position = position;
                    self.player.yaw = yaw;
                    self.player.pitch = pitch;
                }
                _ => {}
            }
        }
    }
}
