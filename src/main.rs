#![feature(impl_trait_in_assoc_type)]

use clap::Parser;

pub mod cli;
pub mod config;
pub mod ctl;
pub mod daemon;
pub mod io;

fn initialize_tracing(log_filter: &str, log_format: cli::LogFormat) {
    use cli::LogFormat;
    let tsub = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_timer(tracing_subscriber::fmt::time::OffsetTime::new(
            time::UtcOffset::current_local_offset().unwrap_or_else(|e| {
                tracing::warn!("couldn't get local time offset: {:?}", e);
                time::UtcOffset::UTC
            }),
            time::macros::format_description!("[hour]:[minute]:[second]"),
        ))
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_env_filter(log_filter);

    match log_format {
        LogFormat::Compact => tsub.compact().init(),
        LogFormat::Full => tsub.init(),
        LogFormat::Pretty => tsub.pretty().init(),
        LogFormat::Json => tsub.json().init(),
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut args = cli::Cli::parse();
    args.init_defaults();

    initialize_tracing(&args.log_filter, args.log_format);

    tracing::debug!("cli argument values: {:?}", &args);

    let cfg = config::Config::from_args(&args).unwrap();

    tracing::debug!("config values: {:?}", &cfg);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    match args.command.unwrap_or_default() {
        cli::Command::Ctl { socket, command } => {
            todo!()
            //runtime.block_on(ctl::run(cfg, socket, command.unwrap_or_default()))
        }
        cli::Command::Daemon { .. } => runtime.block_on(daemon::run(cfg)),
    }
}
