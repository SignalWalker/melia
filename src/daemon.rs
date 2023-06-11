use crate::{
    config::Config,
    io::{SocketFormat, SystemdSocket, SystemdSocketType},
};
use hyper::{
    server::conn::{http1, http2},
    service::service_fn,
};
use std::os::fd::FromRawFd;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpStream, UnixStream},
    task::{JoinHandle, JoinSet, Unconstrained},
};
use tokio_stream::{
    wrappers::{TcpListenerStream, UnixListenerStream},
    Stream, StreamExt,
};

use std::future::Future;

pub mod service;

#[tracing::instrument(skip(cfg))]
pub async fn run(cfg: Config) {
    tracing::debug!("initializing daemon...");

    let systemd_sockets = crate::io::collect_systemd_fds().unwrap();
    // let mut servers = JoinSet::new();

    // let router = Router::new().route("/", routing::get(|| async { "Hello, world!" }));

    // let mut stream_unix = vec![];
    // let mut stream_tcp = vec![];

    // let mut tasks: Vec<JoinHandle<Result<(), std::io::Error>>> = Vec::new();
    let mut tasks: JoinSet<Result<(), std::io::Error>> = JoinSet::new();

    for sock in systemd_sockets {
        match sock {
            SystemdSocket {
                fd,
                ty: SystemdSocketType::Unix,
                format: SocketFormat::Stream { listening: true },
            } => {
                let listener = tokio::net::UnixListener::from_std(unsafe {
                    std::os::unix::net::UnixListener::from_raw_fd(fd)
                })
                .unwrap();
                let stream = UnixListenerStream::new(listener);
                tasks.spawn(accept(stream));
                // let task = tokio::task::spawn(accept(stream));
                // tasks.push(task);
            }
            SystemdSocket {
                fd,
                ty: SystemdSocketType::INet,
                format: SocketFormat::Stream { listening: true },
            } => {
                let listener = tokio::net::TcpListener::from_std(unsafe {
                    std::net::TcpListener::from_raw_fd(fd)
                })
                .unwrap();
                let stream = TcpListenerStream::new(listener);
                tasks.spawn(accept(stream));
                // let task = tokio::task::spawn(accept(stream));
                // tasks.push(task);
            }
            sock => todo!("systemd socket configuration: {sock:?}"),
        }
    }

    for sock in cfg.listen.sockets {
        todo!()
    }
    for unix in cfg.listen.unix {
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
}

async fn accept<Conn: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug + 'static>(
    mut stream: impl Stream<Item = Result<Conn, std::io::Error>> + std::marker::Unpin + std::fmt::Debug,
) -> Result<(), std::io::Error> {
    tracing::debug!(?stream, "accepting connections");
    while let Some(conn) = stream.next().await {
        let conn = match conn {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = ?e, "failed to initialize stream");
                continue;
            }
        };
        tracing::debug!(connection = ?conn, "new connection");
        tokio::task::spawn(async move {
            tracing::trace!(connection = ?conn, "serving connection");
            http1::Builder::new()
                .serve_connection(conn, service_fn(service::respond))
                .await
        });
    }
    Ok(())
}
