use directories::ProjectDirs;

lazy_static::lazy_static! {
    pub static ref PROJECT_DIRS: Option<ProjectDirs> = directories::ProjectDirs::from("net", "Signal Garden", env!("CARGO_BIN_NAME"));
}

#[cfg(target_family = "unix")]
/// From `man 5 systemd.exec` (RuntimeDirectory)
pub const SYSTEM_RUNTIME_DIR: &'static str = concat!("/run/", env!("CARGO_BIN_NAME"));
#[cfg(target_family = "unix")]
/// From `man 5 systemd.exec` (StateDirectory)
pub const SYSTEM_STATE_DIR: &'static str = concat!("/var/lib/", env!("CARGO_BIN_NAME"));
#[cfg(target_family = "unix")]
/// From `man 5 systemd.exec` (CacheDirectory)
pub const SYSTEM_CACHE_DIR: &'static str = concat!("/var/cache/", env!("CARGO_BIN_NAME"));
#[cfg(target_family = "unix")]
/// From `man 5 systemd.exec` (LogsDirectory)
pub const SYSTEM_LOGS_DIR: &'static str = concat!("/var/log/", env!("CARGO_BIN_NAME"));
#[cfg(target_family = "unix")]
/// From `man 5 systemd.exec` (ConfigurationDirectory)
pub const SYSTEM_CONFIGURATION_DIR: &'static str = concat!("/etc/", env!("CARGO_BIN_NAME"));
