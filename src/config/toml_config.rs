use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{ConfigError, DirectoryCreation};

#[derive(Debug, Deserialize, Default)]
pub struct TomlConfig {
    pub directories: Option<TomlDirectories>,
    pub listen: Option<TomlListen>,
}

impl TomlConfig {
    pub fn from_path<'path>(path: &'path Path) -> Result<Self, ConfigError<'path>> {
        toml::from_slice::<Self>(&std::fs::read(path).map_err(|e| ConfigError::new(path, e))?)
            .map_err(|e| ConfigError::new(path, e))
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct TomlDirectories {
    pub create_directories: Option<DirectoryCreation>,
    pub runtime: Option<PathBuf>,
    pub state: Option<PathBuf>,
    pub cache: Option<PathBuf>,
    pub logs: Option<PathBuf>,
    pub configuration: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TomlListen {
    pub addresses: Vec<url::Url>,
}
