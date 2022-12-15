mod consts;
pub use consts::*;
mod toml_config;
use serde::Deserialize;
use toml_config::*;

use std::{
    ffi::OsStr,
    net::SocketAddr,
    os::unix,
    path::{Path, PathBuf},
};

use crate::cli;

#[derive(Debug, thiserror::Error)]
pub enum ConfigErrorVariant {
    #[error("could not access file/directory : {0:?}")]
    AccessFailed(#[from] std::io::Error),
    #[error("TOML error in config file : {0:?}")]
    Toml(#[from] toml::de::Error),
    #[error("failed to parse URL: {0:?}")]
    Url(#[from] url::ParseError),
    #[error("URL host is invalid")]
    InvalidUrlHost,
}

#[derive(thiserror::Error)]
#[error("{variant}{}", if let Some(ref p) = .path { format!(" in {:?}", p.as_ref().as_ref()) } else { "".to_owned() })]
pub struct ConfigError<'data> {
    path: Option<Box<dyn AsRef<Path> + 'data>>,
    #[source]
    variant: ConfigErrorVariant,
}

impl<'data> std::fmt::Debug for ConfigError<'data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigError")
            .field("path", &self.path.as_ref().map(|p| p.as_ref().as_ref()))
            .field("variant", &self.variant)
            .finish()
    }
}

impl<'data> From<ConfigErrorVariant> for ConfigError<'data> {
    fn from(variant: ConfigErrorVariant) -> Self {
        Self {
            path: None,
            variant,
        }
    }
}

impl<'data> ConfigError<'data> {
    pub fn new(path: impl AsRef<Path> + 'data, variant: impl Into<ConfigErrorVariant>) -> Self {
        Self {
            path: Some(Box::new(path)),
            variant: variant.into(),
        }
    }

    pub fn into_owned(self) -> ConfigError<'static> {
        ConfigError {
            path: self.path.map(|p| {
                Box::new(p.as_ref().as_ref().to_owned()) as Box<dyn AsRef<Path> + 'static>
            }),
            variant: self.variant,
        }
    }
}

#[derive(Debug, Default, Deserialize, Copy, Clone, Eq, PartialEq, Hash, clap::ValueEnum)]
pub enum DirectoryCreation {
    #[default]
    #[serde(rename = "no")]
    No,
    #[serde(rename = "non-recursive")]
    NonRecursive,
    #[serde(rename = "recursive")]
    Recursive,
}

impl std::fmt::Display for DirectoryCreation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::No => f.write_str("no"),
            Self::NonRecursive => f.write_str("non-recursive"),
            Self::Recursive => f.write_str("recursive"),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub directories: Directories,
    pub listen: Listen,
}

#[derive(Debug)]
pub struct Directories {
    pub create_directories: DirectoryCreation,
    pub runtime: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub configuration: PathBuf,
}

#[derive(Debug)]
pub struct Listen {
    pub sockets: Vec<SocketAddr>,
    pub unix: Vec<unix::net::SocketAddr>,
}

impl Config {
    pub fn from_args(args: &cli::Cli) -> Result<Self, ConfigError> {
        let toml_cfg: Option<TomlConfig> = match args.config.as_ref() {
            Some(cfg_path) => Some(TomlConfig::from_path(cfg_path)?),
            None => None,
        };
        tracing::debug!("toml configuration values: {:?}", &toml_cfg);
        let directories = Directories::from_args_and_toml(args, toml_cfg.as_ref())
            .map_err(ConfigError::into_owned)?;
        let listen = Listen::from_args_and_toml_and_runtime_dir(
            args,
            toml_cfg.as_ref(),
            &directories.runtime,
        )
        .map_err(ConfigError::into_owned)?;
        Ok(Self {
            directories,
            listen,
        })
    }
}

impl Directories {
    fn from_args_and_toml<'data>(
        args: &'data cli::Cli,
        cfg: Option<&'data TomlConfig>,
    ) -> Result<Self, ConfigError<'data>> {
        use std::env;
        fn get_dir<'data>(
            ty: &str,                                 // type of directory; shown in logs
            cli: Option<impl AsRef<OsStr> + 'data>, // path provided by the user to the cli, if extant
            cfg: Option<impl AsRef<OsStr> + 'data>, // path provided by the user in the config file, if extant
            systemd: &'data str, // name of environment var set by systemd when running as a service unit
            user_dir: Option<&'data Path>, // user directory, if found
            system_default: impl AsRef<Path> + 'data, // system default directory
            create_dirs: DirectoryCreation,
        ) -> Result<PathBuf, ConfigError<'data>> {
            let res = cli
                .map_or_else(
                    || {
                        tracing::debug!(
                            "{} directory not specified by user on the CLI; trying config.toml...",
                            ty
                        );
                        cfg.map(|c| c.as_ref().to_owned())
                    },
                    |cli| Some(cli.as_ref().to_owned()),
                )
                .or_else(|| {
                    tracing::debug!(
                        "{} directory not specified in config.toml; trying ${} variable...",
                        ty,
                        systemd
                    );
                    env::var_os(systemd)
                })
                .or_else(|| {
                    tracing::debug!(
                        "systemd {} directory variable not found; trying XDG user directory...",
                        ty
                    );
                    user_dir.map(|p| p.as_os_str().to_owned())
                })
                .map_or_else(
                    || {
                        tracing::debug!(
                            "user {} directory not found; using default system directory",
                            ty
                        );
                        PathBuf::from(system_default.as_ref())
                    },
                    Into::<PathBuf>::into,
                );
            match create_dirs {
                DirectoryCreation::NonRecursive => {
                    match std::fs::DirBuilder::new().recursive(false).create(&res) {
                        Ok(_) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(e) => return Err(ConfigError::new(res, e)),
                    }
                }
                DirectoryCreation::Recursive => {
                    match std::fs::DirBuilder::new().recursive(true).create(&res) {
                        Ok(_) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(e) => return Err(ConfigError::new(res, e)),
                    }
                }
                _ => {}
            }
            res.canonicalize().map_err(|e| ConfigError::new(res, e))
        }
        let (user_runtime, user_state, user_cache, user_logs, user_config) = {
            match &*PROJECT_DIRS {
                Some(p) => (
                    p.runtime_dir(),
                    p.state_dir(),
                    Some(p.cache_dir()),
                    None::<&Path>,
                    Some(p.config_dir()),
                ),
                None => (None, None, None, None, None),
            }
        };
        let (create_directories, cfg_runtime, cfg_state, cfg_cache, cfg_logs, cfg_config) =
            match cfg {
                Some(TomlConfig {
                    directories: Some(dirs),
                    ..
                }) => (
                    args.create_dirs
                        .unwrap_or_else(|| dirs.create_directories.unwrap_or_default()),
                    dirs.runtime.as_ref(),
                    dirs.state.as_ref(),
                    dirs.cache.as_ref(),
                    dirs.logs.as_ref(),
                    dirs.configuration.as_ref(),
                ),
                _ => (
                    args.create_dirs.unwrap_or_default(),
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            };
        tracing::debug!("directory creation mode: {}", create_directories);
        Ok(Self {
            create_directories,
            runtime: get_dir(
                "runtime",
                args.runtime_dir.as_ref(),
                cfg_runtime,
                "RUNTIME_DIRECTORY",
                user_runtime,
                SYSTEM_RUNTIME_DIR,
                create_directories,
            )?,
            state: get_dir(
                "state",
                args.state_dir.as_ref(),
                cfg_state,
                "STATE_DIRECTORY",
                user_state,
                SYSTEM_STATE_DIR,
                create_directories,
            )?,
            cache: get_dir(
                "cache",
                args.cache_dir.as_ref(),
                cfg_cache,
                "CACHE_DIRECTORY",
                user_cache,
                SYSTEM_CACHE_DIR,
                create_directories,
            )?,
            logs: get_dir(
                "logs",
                args.logs_dir.as_ref(),
                cfg_logs,
                "LOGS_DIRECTORY",
                user_logs,
                SYSTEM_LOGS_DIR,
                create_directories,
            )?,
            configuration: get_dir(
                "configuration",
                args.config_dir.as_ref(),
                cfg_config,
                "CONFIGURATION_DIRECTORY",
                user_config,
                SYSTEM_CONFIGURATION_DIR,
                create_directories,
            )?,
        })
    }
}

impl Listen {
    fn from_args_and_toml_and_runtime_dir<'data>(
        args: &'data cli::Cli,
        cfg: Option<&'data TomlConfig>,
        runtime_dir: &Path,
    ) -> Result<Self, ConfigError<'data>> {
        use url::{Host, Url};
        // runtime_dir guaranteed to be absolute by Directories constructor
        let runtime_url = Url::from_directory_path(runtime_dir).unwrap();

        let mut sockets = Vec::new();
        let mut unix = Vec::new();

        for addr in (match args.command {
            Some(cli::Command::Daemon { ref addresses }) => addresses.as_slice(),
            _ => [].as_slice(),
        })
        .iter()
        .chain(match cfg {
            Some(TomlConfig {
                listen: Some(TomlListen { addresses }),
                ..
            }) => addresses.as_slice(),
            _ => [].as_slice(),
        }) {
            match addr.host() {
                Some(Host::Domain(_)) => {
                    return Err(ConfigError::from(ConfigErrorVariant::InvalidUrlHost))
                }
                Some(_) => sockets.push(addr.socket_addrs(|| None).unwrap().pop().unwrap()),
                None => todo!(),
            }
        }
        Ok(Self { sockets, unix })
    }
}
