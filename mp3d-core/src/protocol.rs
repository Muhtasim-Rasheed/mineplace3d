//! Contains structs and enums for representing messsages between the client and the server.
//!
//! This module defines the protocol used for communication in both the singleplayer and
//! multiplayer modes of the game.

use glam::{IVec3, Vec3};

use crate::{
    block::{Block, BlockState},
    world::chunk::Chunk,
};

/// Move instructions for the player.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveInstructions {
    /// Forward movement: -1 (backward), 0 (none), 1 (forward), 2 (sprint).
    pub forward: i8,
    /// Strafe movement: -1 (left), 0 (none), 1 (right).
    pub strafe: i8,
    /// Whether the player is jumping.
    pub jump: bool,
    /// Whether the player is sneaking or going down if flying.
    pub sneak: bool,
    /// Yaw angle in degrees.
    pub yaw: f32,
    /// Pitch angle in degrees.
    pub pitch: f32,
}

/// The type of block update, which can be used to determine how the client should animate the
/// update.
#[derive(Clone, Copy, Debug)]
pub enum BlockUpdateKind {
    /// A block was placed by a player.
    Placed,
    /// A block was removed by a player.
    Removed,
    /// A block was updated.
    RandomTick,
    /// A block was affected by an interaction result.
    Interaction,
}

/// Represents an update to a block at a specified position with a given block and block state.
#[derive(Clone, Debug)]
pub struct BlockUpdate {
    pub position: IVec3,
    pub block: Block,
    pub block_state: BlockState,
    pub urgent: bool,
    pub kind: BlockUpdateKind,
}

/// Messages sent from the client to the server.
pub enum C2SMessage {
    /// Request to join a world. This contains credentials to register the player or log in if the
    /// player already has an account.
    Connect { username: String, password: String },
    /// Request to leave a world.
    Disconnect,
    /// Request to move the player.
    Move(MoveInstructions),
    /// Request for chunk data.
    RequestChunks { chunk_positions: Vec<IVec3> },
    /// Request to send a chat message or execute a command.
    SendMessage { message: String },
    /// Request for interaction with / placement of / removal of a block. The face is a number
    /// from 0 to 5 in the order of NSEWUD. No block data is sent with this message, so the server
    /// will determine the block being placed (if the targetted block is not interactable)
    BlockClick {
        position: IVec3,
        face: u8,
        right: bool,
    },
    /// Request to click on an inventory slot.
    InventoryClick { idx: usize, right: bool },
    /// Request to change the hotbar slot.
    HotbarChange { idx: usize },
}

/// Messages sent from the server to the client.
#[derive(Clone, Debug)]
pub enum S2CMessage {
    /// Confirmation of connection to a world.
    Connected {
        user_id: u64,
        entity_id: u64,
        inventory: crate::item::Inventory,
    },
    /// Notification of connection failure with a reason.
    ConnectionFailed { reason: String },
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
        entity_id: u64,
        position: Vec3,
        yaw: f32,
        pitch: f32,
    },
    /// Update of a player's inventory.
    InventoryUpdated { inventory: crate::item::Inventory },
    /// Update of multiple blocks changed in one tick.
    BlocksUpdated { updates: Vec<BlockUpdate> },
    /// Delivery of chunk data.
    ChunkData {
        chunk_position: IVec3,
        chunk: Box<Chunk>,
    },
    /// Delivery of a chat message or command output.
    ChatMessage { message: crate::TextComponent },
    /// Notification of change of selected hotbar slot.
    HotbarChanged { idx: usize },
}
