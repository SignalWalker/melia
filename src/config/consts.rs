use std::path::{Path, PathBuf};

lazy_static::lazy_static! {
    pub static ref PROJECT_DIRS: Option<directories::ProjectDirs> = directories::ProjectDirs::from("net", "Signal Garden", env!("CARGO_BIN_NAME"));
    pub static ref BASE_DIRS: Option<directories::BaseDirs> = directories::BaseDirs::new();
}

#[cfg(target_family = "unix")]
mod _unix_dirs {
    /// From `man 5 systemd.exec` (RuntimeDirectory)
    pub const SYSTEM_RUNTIME_DIR: &'static str = concat!("/run/", env!("CARGO_BIN_NAME"));
    /// From `man 5 systemd.exec` (StateDirectory)
    pub const SYSTEM_STATE_DIR: &'static str = concat!("/var/lib/", env!("CARGO_BIN_NAME"));
    /// From `man 5 systemd.exec` (CacheDirectory)
    pub const SYSTEM_CACHE_DIR: &'static str = concat!("/var/cache/", env!("CARGO_BIN_NAME"));
    /// From `man 5 systemd.exec` (LogsDirectory)
    pub const SYSTEM_LOGS_DIR: &'static str = concat!("/var/log/", env!("CARGO_BIN_NAME"));
    /// From `man 5 systemd.exec` (ConfigurationDirectory)
    pub const SYSTEM_CONFIGURATION_DIR: &'static str = concat!("/etc/", env!("CARGO_BIN_NAME"));
}
#[cfg(target_family = "unix")]
pub use _unix_dirs::*;

#[derive(Debug, Clone, Default)]
pub struct ProjectDirs<'p> {
    pub runtime: Option<&'p Path>,
    pub state: Option<&'p Path>,
    pub cache: Option<&'p Path>,
    pub logs: Option<PathBuf>,
    pub configuration: Option<&'p Path>,
}

impl<'p> ProjectDirs<'p> {
    #[cfg(target_family = "unix")]
    pub fn get() -> Self {
        match &*PROJECT_DIRS {
            Some(p) => Self {
                runtime: p.runtime_dir(),
                state: p.state_dir(),
                cache: Some(p.cache_dir()),
                logs: (&*BASE_DIRS).as_ref().map(|dirs| {
                    dirs.config_dir()
                        .join(concat!("log/", env!("CARGO_BIN_NAME")))
                }),
                configuration: Some(p.config_dir()),
            },
            None => Self::default(),
        }
    }
}
