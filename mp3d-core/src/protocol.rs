//! Contains structs and enums for representing messsages between the client and the server.
//!
//! This module defines the protocol used for communication in both the singleplayer and
//! multiplayer modes of the game.

use glam::{IVec3, Vec3};

use crate::{block::Block, world::chunk::Chunk};

/// Messages sent from the client to the server.
pub enum C2SMessage {
    /// Request to join a world.
    Connect,
    /// Request to leave a world.
    Disconnect,
    /// Request to move the player.
    Move {
        forward: i8,
        strafe: i8,
        jump: bool,
        yaw: f32,
        pitch: f32,
    },
    /// Request to set a block at a specified position with a given block.
    SetBlock { position: IVec3, block: Block },
    /// Request for chunk data.
    RequestChunks { chunk_positions: Vec<IVec3> },
}

/// Messages sent from the server to the client.
#[derive(Clone, Debug)]
pub enum S2CMessage {
    /// Confirmation of connection to a world.
    Connected { user_id: u64 },
    /// Notification of disconnection from a world.
    Disconnected { user_id: u64 },
    /// An entity has spawned in the world.
    EntitySpawned {
        entity_id: u64,
        entity_type: u8,
        entity_snapshot: Vec<u8>,
    },
    /// Update of a player's position, yaw, and pitch.
    PlayerMoved {
        user_id: u64,
        position: Vec3,
        yaw: f32,
        pitch: f32,
    },
    /// Update of a block at a specified position with a given block.
    BlockUpdated { position: IVec3, block: Block },
    /// Delivery of chunk data.
    ChunkData { chunk_position: IVec3, chunk: Chunk },
}
