use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    os::unix,
    path::{Path, PathBuf},
};
use url::Url;

mod consts;
pub use consts::*;

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

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub directories: Directories,
    pub listen: Listen,
}

impl Config {
    pub fn from_path<'path>(
        path: impl AsRef<Path> + Clone + 'path,
    ) -> Result<Self, ConfigError<'path>> {
        toml::from_str::<Self>(
            &std::fs::read_to_string(path.as_ref())
                .map_err(|e| ConfigError::new(path.clone(), e))?,
        )
        .map_err(|e| ConfigError::new(path, e))
    }

    pub fn from_args(args: &crate::cli::Cli) -> Result<Self, ConfigError> {
        let cfg_path = args
            .config
            .clone()
            .or_else(|| args.config_dir.as_ref().map(|d| d.join("config.toml")))
            .unwrap();

        let mut res = Self::from_path(cfg_path)?;
        res.directories.overwrite_with_cli(args);
        if let Some(crate::cli::Command::Daemon { addresses }) = args.command.as_ref() {
            res.listen.extend_from_urls(addresses.iter().cloned());
        }

        Ok(res)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Directories {
    pub runtime: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub configuration: PathBuf,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
struct ListenToml {
    pub addresses: Vec<url::Url>,
    // pub inet: Vec<SocketAddr>,
    // pub unix: Vec<unix::net::SocketAddr>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(try_from = "Url", into = "Url")]
pub struct UnixSocket {
    pub path: PathBuf,
    pub user: Option<String>,
    pub group: Option<String>,
    pub mode: u32,
}

impl TryFrom<Url> for UnixSocket {
    type Error = &'static str;

    fn try_from(addr: Url) -> Result<Self, Self::Error> {
        if addr.scheme() != "unix" {
            return Err("incorrect scheme");
        }
        todo!()
    }
}

impl From<UnixSocket> for Url {
    fn from(sock: UnixSocket) -> Self {
        todo!()
    }
}

impl UnixSocket {
    pub fn open(&self) -> unix::net::SocketAddr {
        todo!()
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(from = "ListenToml", into = "ListenToml")]
pub struct Listen {
    pub http: Vec<SocketAddr>,
    pub https: Vec<SocketAddr>,
    pub unix: Vec<UnixSocket>,
}

impl From<ListenToml> for Listen {
    fn from(value: ListenToml) -> Self {
        let mut res = Self {
            http: Vec::new(),
            https: Vec::new(),
            unix: Vec::new(),
        };
        res.extend_from_urls(value.addresses);
        res
    }
}

impl Listen {
    pub fn extend_from_urls<'url>(&mut self, urls: impl IntoIterator<Item = Url>) {
        use url::Host;
        fn addr_from_url(addr: &Url, default_port: u16) -> SocketAddr {
            let ip = match addr.host() {
                Some(Host::Ipv4(ip)) => IpAddr::V4(ip),
                Some(Host::Ipv6(ip)) => IpAddr::V6(ip),
                Some(Host::Domain(_)) => panic!("http listener host must be an ip address"),
                None => panic!("no address specified for http listener"),
            };
            let port = addr.port().unwrap_or(default_port);
            SocketAddr::new(ip, port)
        }
        for addr in urls {
            match addr.scheme() {
                "http" => self.http.push(addr_from_url(&addr, 80)),
                "https" => self.https.push(addr_from_url(&addr, 443)),
                "unix" => self.unix.push(UnixSocket::try_from(addr).unwrap()),
                _ => panic!("incorrect url scheme"),
            }
        }
    }
}

impl Into<ListenToml> for Listen {
    fn into(self) -> ListenToml {
        let mut res = ListenToml {
            addresses: Vec::with_capacity(self.http.len() + self.https.len() + self.unix.len()),
        };
        for http in self.http {
            todo!()
        }
        for https in self.https {
            todo!()
        }
        for unix in self.unix {
            res.addresses.push(unix.into());
        }
        res
    }
}

impl Default for Directories {
    /// If our user has a home directory, use directories from that. Otherwise, use system
    /// directories.
    fn default() -> Self {
        #[inline]
        fn select(user: Option<impl AsRef<Path>>, system: &'static str) -> PathBuf {
            user.map(|p| p.as_ref().to_owned())
                .unwrap_or_else(|| PathBuf::from(system))
        }
        let user = ProjectDirs::get();
        Self {
            runtime: select(user.runtime, SYSTEM_RUNTIME_DIR),
            state: select(user.state, SYSTEM_STATE_DIR),
            cache: select(user.cache, SYSTEM_CACHE_DIR),
            logs: select(user.logs, SYSTEM_LOGS_DIR),
            configuration: select(user.configuration, SYSTEM_CONFIGURATION_DIR),
        }
    }
}

impl Directories {
    /// Overwrite read values with values from the CLI.
    pub fn overwrite_with_cli(&mut self, cli: &crate::cli::Cli) {
        #[inline]
        fn overwrite(field: &mut PathBuf, arg: &Option<PathBuf>) {
            if let Some(arg) = arg {
                *field = arg.clone();
            }
        }
        overwrite(&mut self.runtime, &cli.runtime_dir);
        overwrite(&mut self.state, &cli.state_dir);
        overwrite(&mut self.cache, &cli.cache_dir);
        overwrite(&mut self.logs, &cli.logs_dir);
        overwrite(&mut self.configuration, &cli.config_dir);
    }

    pub fn create_dirs(&self, recursive: bool) -> std::io::Result<bool> {
        fn create(builder: &std::fs::DirBuilder, path: impl AsRef<Path>) -> std::io::Result<bool> {
            match builder.create(path) {
                Ok(_) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
                Err(e) => Err(e),
            }
        }
        let builder = {
            let mut b = std::fs::DirBuilder::new();
            b.recursive(recursive);
            b
        };
        let mut res = false;
        res |= create(&builder, &self.runtime)?;
        res |= create(&builder, &self.state)?;
        res |= create(&builder, &self.cache)?;
        res |= create(&builder, &self.logs)?;
        res |= create(&builder, &self.configuration)?;
        Ok(res)
    }
}
