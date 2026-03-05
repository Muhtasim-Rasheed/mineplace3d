//! This module defines the [`WorldLoadError`] type, which represents errors that can occur when loading
//! a world from a save file.

/// Errors that can occur when loading a world from a save file.
pub enum WorldLoadError {
    MissingSaveFile(std::path::PathBuf),
    InvalidSaveFormat(String),
}

impl std::fmt::Display for WorldLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorldLoadError::MissingSaveFile(path) => {
                write!(f, "Save file not found: {}", path.display())
            }
            WorldLoadError::InvalidSaveFormat(msg) => write!(f, "Invalid save format: {}", msg),
        }
    }
}

impl std::fmt::Debug for WorldLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorldLoadError: {}", self)
    }
}

impl std::error::Error for WorldLoadError {}

impl From<WorldLoadError> for std::io::Error {
    fn from(err: WorldLoadError) -> Self {
        match err {
            WorldLoadError::MissingSaveFile(path) => std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Save file not found: {}", path.display()),
            ),
            WorldLoadError::InvalidSaveFormat(msg) => std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid save format: {}", msg),
            ),
        }
    }
}
