use std::{any::Any, os::fd::RawFd};

use tokio::net::{TcpListener, TcpStream, UdpSocket, UnixDatagram, UnixListener, UnixStream};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SystemdSocketType {
    Unrecognized,
    Fifo,
    Queue,
    INet,
    Unix,
    Special,
}

impl std::fmt::Display for SystemdSocketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemdSocketType::Unrecognized => f.write_str("Unrecognized"),
            SystemdSocketType::Fifo => f.write_str("FIFO"),
            SystemdSocketType::Queue => f.write_str("Message Queue"),
            SystemdSocketType::INet => f.write_str("IP"),
            SystemdSocketType::Unix => f.write_str("Unix"),
            SystemdSocketType::Special => f.write_str("Special"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SystemdError {
    #[error(transparent)]
    Systemd(#[from] systemd::Error),
    #[error("unsupported socket type: {0}")]
    UnsupportedSocketType(SystemdSocketType),
}

// pub enum Socket {
//     Datagram(DatagramSocket),
//     Listener(StreamListener),
//     Stream(StreamSocket),
// }
//
// pub enum DatagramSocket {
//     Unix(UnixDatagram),
//     Net(UdpSocket),
// }
//
// pub enum StreamSocket {
//     Unix(UnixStream),
//     Net(TcpStream),
// }
//
// pub enum StreamListener {
//     Unix(UnixListener),
//     Net(TcpListener),
// }

pub enum SocketFormat {
    Datagram,
    Stream { listener: bool },
}

pub struct SystemdSocket {
    pub fd: RawFd,
    pub ty: SystemdSocketType,
    pub format: SocketFormat,
}

pub(crate) fn collect_systemd_fds() -> Result<Vec<SystemdSocket>, SystemdError> {
    use systemd::daemon;
    let mut res = vec![];
    for fd in daemon::listen_fds(true)?.iter() {
        if daemon::is_socket_inet(fd, None, None, todo!(), None)? {
        } else if daemon::is_socket_unix(fd, None, todo!(), None::<&str>)? {
        } else if daemon::is_fifo(fd, None::<&str>)? {
            return Err(SystemdError::UnsupportedSocketType(SystemdSocketType::Fifo));
        } else if daemon::is_mq(fd, None::<&str>)? {
            return Err(SystemdError::UnsupportedSocketType(
                SystemdSocketType::Queue,
            ));
        } else if daemon::is_special(fd, None::<&str>)? {
            return Err(SystemdError::UnsupportedSocketType(
                SystemdSocketType::Special,
            ));
        } else {
            return Err(SystemdError::UnsupportedSocketType(
                SystemdSocketType::Unrecognized,
            ));
        }
    }
    Ok(res)
}
