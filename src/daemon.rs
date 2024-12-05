use crate::{
    config::Config,
    io::{SocketFormat, SystemdSocket, SystemdSocketType},
};
use crossbeam::sync::ShardedLock;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use std::{net::SocketAddr, os::fd::FromRawFd};
use std::{os::unix, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    task::JoinSet,
};
use tokio_util::net::Listener;

use self::service::ServiceConfig;

pub mod service;

#[tracing::instrument(skip(cfg))]
pub async fn run(cfg: Config) -> std::io::Result<()> {
    tracing::debug!("initializing daemon...");
    let cfg = Arc::new(ShardedLock::new(cfg));

    let systemd_sockets = crate::io::collect_systemd_fds().unwrap();
    // let mut servers = JoinSet::new();

    let mut added_http = Vec::<SocketAddr>::new();
    let mut added_https = Vec::<SocketAddr>::new();
    let mut added_unix = Vec::<unix::net::SocketAddr>::new();

    let mut tasks: JoinSet<Result<(), std::io::Error>> = JoinSet::new();

    for sock in systemd_sockets {
        match sock {
            SystemdSocket {
                fd,
                ty: SystemdSocketType::Unix,
                format: SocketFormat::Stream { listening: true },
                name,
            } => {
                let listener = tokio::net::UnixListener::from_std(unsafe {
                    let unix = unix::net::UnixListener::from_raw_fd(fd);
                    unix.set_nonblocking(true)?;
                    added_unix.push(unix.local_addr().unwrap());
                    unix
                })?;
                tasks.spawn(accept(cfg.clone(), ServiceConfig::UNIX, listener));
                // let task = tokio::task::spawn(accept(stream));
                // tasks.push(task);
            }
            SystemdSocket {
                fd,
                ty: SystemdSocketType::INet,
                format: SocketFormat::Stream { listening: true },
                name,
            } => {
                let https = matches!(name.as_str(), "https");
                let listener = tokio::net::TcpListener::from_std(unsafe {
                    let tcp = std::net::TcpListener::from_raw_fd(fd);
                    tcp.set_nonblocking(true)?;
                    if https {
                        &mut added_https
                    } else {
                        &mut added_http
                    }
                    .push(tcp.local_addr().unwrap());
                    tcp
                })?;
                tasks.spawn(accept(
                    cfg.clone(),
                    if https {
                        ServiceConfig::HTTPS
                    } else {
                        ServiceConfig::HTTP
                    },
                    listener,
                ));
                // tasks.join_next().await;

                // let _ = accept(stream).await;
                // todo!();

                // let task = tokio::task::spawn(accept(stream));
                // tasks.push(task);
            }
            sock => todo!("systemd socket configuration: {sock:?}"),
        }
    }

    for addr in &cfg.read().unwrap().listen.http {
        tasks.spawn(accept(
            cfg.clone(),
            ServiceConfig::HTTP,
            tokio::net::TcpListener::bind(addr).await?,
        ));
    }
    for addr in &cfg.read().unwrap().listen.https {
        tasks.spawn(accept(
            cfg.clone(),
            ServiceConfig::HTTPS,
            tokio::net::TcpListener::bind(addr).await?,
        ));
    }
    for addr in &cfg.read().unwrap().listen.unix {
        todo!()
        // tasks.spawn(accept(
        //     cfg.clone(),
        //     tokio::net::UnixListener::bind(todo!())?,
        // ));
    }

    {
        let mut cfg = cfg.write().unwrap();
        cfg.listen.http.append(&mut added_http);
        cfg.listen.https.append(&mut added_https);
        // cfg.listen.unix.append(&mut added_unix);
    }

    tracing::trace!("awaiting server results");

    while let Some(task) = tasks.join_next().await {
        // tracing::trace!(?task, "awaiting task");
        match task {
            Ok(Ok(())) => tracing::trace!("server exited successfully"),
            Ok(Err(e)) => {
                tracing::error!(error = ?e, "server error");
            }
            Err(e) => {
                tracing::error!(error = ?e, "task error");
            }
        }
    }

    tracing::trace!("complete");

    Ok(())
}

#[allow(unreachable_code)]
#[tracing::instrument(level = "info")]
async fn accept<
    Conn: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug + 'static,
    Addr: std::fmt::Debug,
>(
    cfg: Arc<ShardedLock<Config>>,
    svc_cfg: ServiceConfig,
    mut listener: impl Listener<Io = Conn, Addr = Addr> + std::fmt::Debug,
) -> Result<(), std::io::Error> {
    tracing::debug!(?listener, "accepting connections");
    // TODO :: Axum
    // let router = Router::<()>::new().route("/", routing::get(|| async { "Hello, world!" }));

    let svc = tower::ServiceBuilder::new()
        // .layer(TraceLayer::new_for_http())
        .service(service::Service::new(cfg, &svc_cfg));

    let conn_builder = Arc::new(auto::Builder::new(TokioExecutor::new()));

    loop {
        let (conn, addr) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = ?e, "failed to initialize stream");
                continue;
            }
        };
        tracing::debug!(connection = ?conn, address = ?addr, "new connection");
        let conn_builder = conn_builder.clone();
        let svc = svc.clone();
        tokio::task::spawn(async move {
            let conn = conn_builder.serve_connection_with_upgrades(TokioIo::new(conn), svc);
            if let Err(err) = conn.await {
                tracing::error!(error = err);
            }
        });
    }
    Ok(())
}
