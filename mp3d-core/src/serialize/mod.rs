//! This module contains the [`Saveable`] trait, which defines how to save and load data in a
//! versioned format. It also contains a byte reader and writer.

use crate::serialize::{
    read::{ByteReader, ReadError},
    write::ByteWriter,
};

/// The current version of the world save format (in beta).
pub const SAVE_VERSION: u8 = 0x08;

/// The current generator version. 0x00 is used for alpha generators and 0x01 and onwards are used
/// for beta generators.
pub const GENERATOR_VERSION: u8 = 0x02;

/// A trait for types that can be saved and loaded in a versioned format.
pub trait Saveable {
    fn save(&self, writer: ByteWriter) -> ByteWriter;
    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized;
}

impl<T: Saveable> Saveable for std::sync::Arc<T> {
    fn save(&self, writer: ByteWriter) -> ByteWriter {
        self.as_ref().save(writer)
    }

    fn load(reader: &mut ByteReader, version: u8) -> Result<Self, ReadError>
    where
        Self: Sized,
    {
        T::load(reader, version).map(std::sync::Arc::new)
    }
}

pub mod read;
pub mod write;
