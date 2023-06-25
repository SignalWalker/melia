use crate::{
    config::Config,
    io::{SocketFormat, SystemdSocket, SystemdSocketType},
};
use hyper::{server::conn::http1, service::service_fn};
use std::os::fd::FromRawFd;
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    task::JoinSet,
};
use tokio_util::net::Listener;

pub mod service;

#[tracing::instrument(skip(cfg))]
pub async fn run(cfg: Config) -> Result<(), std::io::Error> {
    tracing::debug!("initializing daemon...");

    let systemd_sockets = crate::io::collect_systemd_fds().unwrap();
    // let mut servers = JoinSet::new();

    // let router = Router::new().route("/", routing::get(|| async { "Hello, world!" }));

    // let mut stream_unix = vec![];
    // let mut stream_tcp = vec![];

    // let mut tasks: Vec<JoinHandle<Result<(), std::io::Error>>> = Vec::new();
    let mut tasks: JoinSet<Result<(), std::io::Error>> = JoinSet::new();

    let cfg = Arc::new(cfg);

    for sock in systemd_sockets {
        match sock {
            SystemdSocket {
                fd,
                ty: SystemdSocketType::Unix,
                format: SocketFormat::Stream { listening: true },
            } => {
                let listener = tokio::net::UnixListener::from_std(unsafe {
                    let unix = std::os::unix::net::UnixListener::from_raw_fd(fd);
                    unix.set_nonblocking(true)?;
                    unix
                })?;
                tasks.spawn(accept(cfg.clone(), listener));
                // let task = tokio::task::spawn(accept(stream));
                // tasks.push(task);
            }
            SystemdSocket {
                fd,
                ty: SystemdSocketType::INet,
                format: SocketFormat::Stream { listening: true },
            } => {
                let listener = tokio::net::TcpListener::from_std(unsafe {
                    let tcp = std::net::TcpListener::from_raw_fd(fd);
                    tcp.set_nonblocking(true)?;
                    tcp
                })?;
                tasks.spawn(accept(cfg.clone(), listener));
                // tasks.join_next().await;

                // let _ = accept(stream).await;
                // todo!();

                // let task = tokio::task::spawn(accept(stream));
                // tasks.push(task);
            }
            sock => todo!("systemd socket configuration: {sock:?}"),
        }
    }

    for sock in &cfg.listen.http {
        todo!()
    }
    for sock in &cfg.listen.https {
        todo!()
    }
    for unix in &cfg.listen.unix {
        todo!()
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
async fn accept<
    Conn: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug + 'static,
    Addr: std::fmt::Debug,
>(
    cfg: Arc<Config>,
    mut listener: impl Listener<Io = Conn, Addr = Addr> + std::fmt::Debug,
) -> Result<(), std::io::Error> {
    tracing::debug!(?listener, "accepting connections");
    loop {
        let (conn, addr) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = ?e, "failed to initialize stream");
                continue;
            }
        };
        tracing::debug!(connection = ?conn, address = ?addr, "new connection");
        tokio::task::spawn(
            http1::Builder::new().serve_connection(conn, service_fn(service::respond)),
        );
    }
    Ok(())
}
