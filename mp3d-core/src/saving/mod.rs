//! This module contains the [`Saveable`] trait, which defines how to save and load data in a
//! versioned format.

/// The current version of the world save format (in beta).
pub const SAVE_VERSION: u8 = 2;

/// A trait for types that can be saved and loaded in a versioned format.
pub trait Saveable {
    fn save(&self) -> Vec<u8>;
    fn load<I: Iterator<Item = u8>>(data: &mut I, version: u8) -> Result<Self, WorldLoadError>
    where
        Self: Sized;
}

pub mod error;
pub mod io;

pub use error::WorldLoadError;
