use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigLoadError {
    #[error("couldn't read the configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("couldn't parse the configuration file: {0}")]
    ParseError(#[from] toml::de::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub max_clients: usize,
    pub port: Option<u16>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_clients: 20,
            port: None,
        }
    }
}

pub fn read_config(path: &std::path::Path) -> Result<ServerConfig, ConfigLoadError> {
    Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
}
