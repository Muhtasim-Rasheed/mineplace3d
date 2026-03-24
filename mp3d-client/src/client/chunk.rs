//! Client-side chunk representation.

use glam::IVec3;
use mp3d_core::{
    block::{Block, BlockState},
    world::chunk::Chunk,
};

/// Client-side chunk representation.
///
/// This struct wraps the core [`Chunk`] data structure to be used on the client side. It also
/// contains additional client-specific data like [`ClientChunk::dirty`], which indicates whether
/// the chunk needs to be re-meshed.
pub struct ClientChunk {
    /// The inner chunk data.
    pub chunk: Chunk,
    /// Indicates whether the chunk needs to be re-rendered.
    pub dirty: bool,
}

impl ClientChunk {
    /// Creates a new [`ClientChunk`] with the given core [`Chunk`].
    pub fn new(chunk: Chunk) -> Self {
        Self { chunk, dirty: true }
    }

    /// Gets a block at the given local position within the chunk.
    pub fn get_block(&self, local_pos: IVec3) -> Option<(&Block, &BlockState)> {
        self.chunk.get_block(local_pos)
    }

    /// Sets a block at the given local position within the chunk.
    pub fn set_block(&mut self, local_pos: IVec3, block: Block, state: BlockState) {
        self.chunk.set_block(local_pos, block, state);
        self.dirty = true;
    }
}

impl From<Chunk> for ClientChunk {
    fn from(chunk: Chunk) -> Self {
        Self::new(chunk)
    }
}
