use std::{env, os::fd::RawFd};
use systemd::daemon;

// the api for systemd sockets is *so* bad, oh my god

pub(crate) fn collect_systemd_fds() -> Result<Vec<SystemdSocket>, SystemdError> {
    #[cfg(debug_assertions)]
    if env::var("LISTEN_FDS").is_ok()
        && matches!(env::var("LISTEN_PID"), Err(env::VarError::NotPresent))
    {
        // if LISTEN_FDS is set and LISTEN_PID isn't, that means we're probably running under
        // systemfd & cargo-watch, so it should be fine to set LISTEN_PID here (so that
        // `listen_fds()` works as expected)
        env::set_var("LISTEN_PID", unsafe { libc::getpid() }.to_string());
    }
    let mut res = vec![];
    for fd in daemon::listen_fds(true)?.iter() {
        res.push(SystemdSocket::from_fd(fd)?);
    }
    Ok(res)
}

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
    #[error("unsupported socket format")]
    UnsupportedSocketFormat,
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

#[derive(Debug)]
pub enum SocketFormat {
    Datagram,
    Stream { listening: bool },
}

// oooooh my gooooooood why do i have to write so many if-elses for this; am i using this wrong???
impl SocketFormat {
    fn from_fd_inet(fd: RawFd) -> Result<Self, SystemdError> {
        // check if UDP
        if daemon::is_socket_inet(
            fd,
            None,
            Some(daemon::SocketType::Datagram),
            daemon::Listening::NoListeningCheck,
            None,
        )? {
            Ok(Self::Datagram)
        }
        // check if TCP (listening)
        else if daemon::is_socket_inet(
            fd,
            None,
            Some(daemon::SocketType::Stream),
            daemon::Listening::IsListening,
            None,
        )? {
            Ok(Self::Stream { listening: true })
        }
        // check if TCP (not listening)
        else if daemon::is_socket_inet(
            fd,
            None,
            Some(daemon::SocketType::Stream),
            daemon::Listening::IsNotListening,
            None,
        )? {
            Ok(Self::Stream { listening: false })
        }
        // not gonna bother with anything else
        else {
            Err(SystemdError::UnsupportedSocketFormat)
        }
    }
    fn from_fd_unix(fd: RawFd) -> Result<Self, SystemdError> {
        // the unix function has a different interface so i can't even make this into a macro ;-;
        // check if UDP
        if daemon::is_socket_unix(
            fd,
            Some(daemon::SocketType::Datagram),
            daemon::Listening::NoListeningCheck,
            None::<&str>,
        )? {
            Ok(Self::Datagram)
        }
        // check if TCP (listening)
        else if daemon::is_socket_unix(
            fd,
            Some(daemon::SocketType::Stream),
            daemon::Listening::IsListening,
            None::<&str>,
        )? {
            Ok(Self::Stream { listening: true })
        }
        // check if TCP (not listening)
        else if daemon::is_socket_unix(
            fd,
            Some(daemon::SocketType::Stream),
            daemon::Listening::IsNotListening,
            None::<&str>,
        )? {
            Ok(Self::Stream { listening: false })
        }
        // not gonna bother with anything else
        else {
            Err(SystemdError::UnsupportedSocketFormat)
        }
    }
}

#[derive(Debug)]
pub struct SystemdSocket {
    pub fd: RawFd,
    pub ty: SystemdSocketType,
    pub format: SocketFormat,
}

impl SystemdSocket {
    fn from_fd(fd: RawFd) -> Result<Self, SystemdError> {
        if daemon::is_socket_inet(fd, None, None, daemon::Listening::NoListeningCheck, None)? {
            Ok(SystemdSocket {
                fd,
                ty: SystemdSocketType::INet,
                format: SocketFormat::from_fd_inet(fd)?,
            })
        } else if daemon::is_socket_unix(
            fd,
            None,
            daemon::Listening::NoListeningCheck,
            None::<&str>,
        )? {
            Ok(SystemdSocket {
                fd,
                ty: SystemdSocketType::Unix,
                format: SocketFormat::from_fd_unix(fd)?,
            })
        } else if daemon::is_fifo(fd, None::<&str>)? {
            Err(SystemdError::UnsupportedSocketType(SystemdSocketType::Fifo))
        } else if daemon::is_mq(fd, None::<&str>)? {
            Err(SystemdError::UnsupportedSocketType(
                SystemdSocketType::Queue,
            ))
        } else if daemon::is_special(fd, None::<&str>)? {
            Err(SystemdError::UnsupportedSocketType(
                SystemdSocketType::Special,
            ))
        } else {
            Err(SystemdError::UnsupportedSocketType(
                SystemdSocketType::Unrecognized,
            ))
        }
    }

    // pub fn to_incoming<Conn, Error>(self) -> Box<dyn Accept<Conn = Conn, Error = Error>> {
    //     match self {
    //         SystemdSocket {
    //             fd,
    //             ty: SystemdSocketType::Unix,
    //             format: SocketFormat::Stream { listening: true },
    //         } => {
    //             let listener = tokio::net::UnixListener::from_std(unsafe {
    //                 std::os::unix::net::UnixListener::from_raw_fd(fd)
    //             })
    //             .unwrap();
    //             Box::new(hyper::server::accept::from_stream(UnixListenerStream::new(
    //                 listener,
    //             )))
    //         }
    //         SystemdSocket {
    //             fd,
    //             ty: SystemdSocketType::INet,
    //             format: SocketFormat::Stream { listening: true },
    //         } => {
    //             let listener = tokio::net::TcpListener::from_std(unsafe {
    //                 std::net::TcpListener::from_raw_fd(fd)
    //             })
    //             .unwrap();
    //             Box::new(AddrIncoming::from_listener(listener).unwrap())
    //         }
    //         sock => todo!("systemd socket configuration: {sock:?}"),
    //     }
    // }
}

#[macro_export]
macro_rules! serve_on_socket {
    ($socket:expr => $svc:expr) => {
        match $socket {
            SystemdSocket {
                fd,
                ty: SystemdSocketType::Unix,
                format: SocketFormat::Stream { listening: true },
            } => {
                let listener = tokio::net::UnixListener::from_std(unsafe {
                    std::os::unix::net::UnixListener::from_raw_fd(fd)
                })
                .unwrap();
                hyper::server::Server::builder(hyper::server::accept::from_stream(
                    UnixListenerStream::new(listener),
                ))
                .serve($svc)
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
                hyper::server::Server::builder(AddrIncoming::from_listener(listener).unwrap())
                    .serve($svc)
            }
            sock => todo!("systemd socket configuration: {sock:?}"),
        }
    };
}
