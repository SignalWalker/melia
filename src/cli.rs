use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(
    Debug, Default, Deserialize, Serialize, Copy, Clone, Eq, PartialEq, Hash, clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum DirectoryCreation {
    #[default]
    No,
    NonRecursive,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, clap::ValueEnum)]
pub enum LogFormat {
    Compact,
    Full,
    Pretty,
    Json,
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Compact => f.write_str("compact"),
            LogFormat::Full => f.write_str("full"),
            LogFormat::Pretty => f.write_str("pretty"),
            LogFormat::Json => f.write_str("json"),
        }
    }
}

fn parse_path(path: &str) -> Result<PathBuf, std::io::Error> {
    PathBuf::from(path).canonicalize()
}

#[derive(Parser, Debug)]
#[command(version, author, about)]
pub struct Cli {
    /// Logging output filters; comma-separated
    #[arg(
        short,
        long,
        default_value = "warn,melia=info",
        env = "MELIA_LOG_FILTER"
    )]
    pub log_filter: String,
    /// Logging output format
    #[arg(long, default_value_t = LogFormat::Pretty)]
    pub log_format: LogFormat,
    /// Path to the runtime directory
    #[arg(long, value_parser = parse_path, env = "RUNTIME_DIRECTORY")]
    pub runtime_dir: Option<PathBuf>,
    /// Path to the state directory
    #[arg(long, value_parser = parse_path, env = "STATE_DIRECTORY")]
    pub state_dir: Option<PathBuf>,
    /// Path to the cache directory
    #[arg(long, value_parser = parse_path, env = "CACHE_DIRECTORY")]
    pub cache_dir: Option<PathBuf>,
    /// Path to the logs directory
    #[arg(long, value_parser = parse_path, env = "LOGS_DIRECTORY")]
    pub logs_dir: Option<PathBuf>,
    /// Path to the configuration directory
    #[arg(long, value_parser = parse_path, env = "CONFIGURATION_DIRECTORY")]
    pub config_dir: Option<PathBuf>,
    /// Whether to create missing runtime/state/cache/logs/configuration directories.
    ///
    /// If `--create-dirs=recursive`, then directories will be created recursively (as with `mkdir -p`).
    #[arg(
        long,
        default_value_t = DirectoryCreation::No,
        default_missing_value = "non-recursive",
        value_name = "RECURSIVITY"
    )]
    pub create_dirs: DirectoryCreation,
    /// Path to the configuration file; must exist if specified.
    #[arg(short, long, value_parser = parse_path, env = "MELIA_CONFIG")]
    pub config: Option<PathBuf>,
    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Command>,
}

pub enum AddressInput {
    Url(url::Url),
}

#[derive(Subcommand, Debug)]
#[command()]
pub enum Command {
    /// Run the web server daemon [default]
    #[command()]
    Daemon {
        /// Paths/addresses on which to listen for connections, in the format `${scheme}://${address}:${port}`.
        ///
        /// IPv6 addresses must be surrounded by square brackets, ex. `[::1]:80`. For example, to listen on
        /// standard HTTP/HTTPS ports on both IPv4 and IPv6:
        /// `-a http://0.0.0.0:80 -a https://0.0.0.0:443 -a http://[::0]:80 -a https://[::0]:443`.
        ///
        /// On Unix systems, Unix domain socket paths may also be used, in the format
        /// `unix:[//${root}/]${path}[?[user=${user}][group=${group}][mode=${mode}]]`, where query components are separated
        /// by commas (,). Relative paths are resolved relative to the runtime directory (either as specified with
        /// `runtime_dir` or in the configuration file). Ex. `unix:nginx?user=nginx,group=melia,mode=0660`
        /// would attempt to open a socket at `${runtime_dir}/nginx`, with the owning user `nginx`, group `melia`,
        /// and file permission mode `0660`.
        /// `unix:///run/melia/nginx?user=nginx,group=melia,mode=0660` would open a socket with the
        /// same configuration as above, but at `/run/melia/nginx`.
        #[arg(id = "address", short = 'a', long)]
        addresses: Vec<url::Url>,
    },
    /// Control a running daemon
    #[command()]
    Ctl {
        /// Path to the IPC socket on which the daemon is listening
        #[clap(short, long, env = "MELIA_CTL_SOCKET")]
        socket: PathBuf,
        /// Subcommand
        #[command(subcommand)]
        command: Option<CtlCommand>,
    },
}

impl Default for Command {
    fn default() -> Self {
        Self::Daemon {
            addresses: Default::default(),
        }
    }
}

#[derive(Subcommand, Debug, Default)]
#[command()]
pub enum CtlCommand {
    /// Print the current configuration settings
    #[command()]
    #[default]
    PrintCfg,
}

impl Cli {
    pub fn init_defaults(&mut self) {
        if self.command.is_none() {
            self.command.replace(Command::default());
        }
    }
}
