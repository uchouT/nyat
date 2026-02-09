use std::net::SocketAddr;

use crate::net::{DnsError, Protocol, resolve_dns};
use smallvec::SmallVec;
use socket2::{Domain, Socket, Type};
use tokio::net::UdpSocket;

pub struct LocalAddr {
    local_addr: SocketAddr,
    fmark: Option<u32>,
    iface: Option<SmallVec<[u8; 16]>>,
}

impl LocalAddr {
    /// Create non-blocking & reuse port & reuse address, with no-exec flag
    /// and bind the local address
    /// TODO: cross platform support
    pub(crate) fn socket(&self, p: Protocol) -> Result<Socket, std::io::Error> {
        let socket = Socket::new(
            Domain::for_address(self.local_addr),
            {
                use Protocol::*;
                match p {
                    Tcp => Type::STREAM.nonblocking(),
                    Udp => Type::DGRAM.nonblocking(),
                }
            },
            None,
        )?;

        socket.set_reuse_port(true)?;
        socket.set_reuse_address(true)?;

        if let Some(fmark) = self.fmark {
            socket.set_mark(fmark)?;
        }
        if let Some(iface) = &self.iface {
            socket.bind_device(Some(iface))?;
        }
        // TODO: getpid_fd force to set reuse port
        socket.bind(&self.local_addr.into())?;
        Ok(socket)
    }

    pub(crate) fn udp_socket(&self) -> std::io::Result<tokio::net::UdpSocket> {
        let socket = self.socket(Protocol::Udp)?;
        UdpSocket::from_std(socket.into())
    }
}

pub(crate) enum RemoteAddr {
    /// bare socket address
    SocketAddr(SocketAddr),
    /// domain, requires DNS
    Host { domain: String, port: u16 },
}

impl RemoteAddr {
    /// get socket addr from remote addr
    pub(crate) async fn socket_addr(&self) -> Result<SocketAddr, DnsError> {
        match self {
            Self::SocketAddr(addr) => Ok(*addr),
            Self::Host { domain, port } => resolve_dns((domain.as_ref(), *port), None).await,
        }
    }
}
