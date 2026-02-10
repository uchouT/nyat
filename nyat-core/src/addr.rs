use std::net::SocketAddr;

use crate::net::{DnsError, IpVer, Protocol, resolve_dns};
use smallvec::SmallVec;
use socket2::{Domain, Socket, Type};
use tokio::net::UdpSocket;

pub struct Local {
    local_addr: SocketAddr,
    fmark: Option<u32>,
    iface: Option<SmallVec<[u8; 16]>>,
}

impl Local {
    pub fn new(local_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            fmark: None,
            iface: None,
        }
    }

    pub fn with_fmark(mut self, fmark: u32) -> Self {
        self.fmark = Some(fmark);
        self
    }

    pub fn with_iface(mut self, iface: impl AsRef<[u8]>) -> Self {
        self.iface = Some(iface.as_ref().into());
        self
    }

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

pub enum RemoteAddr {
    /// bare socket address
    Resolved(SocketAddr),
    /// domain, requires DNS
    Host {
        domain: String,
        port: u16,
        ver_prefered: Option<IpVer>,
    },
}

impl RemoteAddr {
    pub fn from_addr(addr: SocketAddr) -> Self {
        Self::Resolved(addr)
    }

    pub fn from_host(domain: impl Into<String>, port: u16, ver_prefered: Option<IpVer>) -> Self {
        Self::Host {
            domain: domain.into(),
            port,
            ver_prefered,
        }
    }

    /// get socket addr from remote addr
    pub(crate) async fn socket_addr(&self) -> Result<SocketAddr, DnsError> {
        match self {
            Self::Host {
                domain,
                port,
                ver_prefered,
            } => resolve_dns((domain.as_ref(), *port), *ver_prefered).await,
            Self::Resolved(addr) => Ok(*addr),
        }
    }

    #[inline]
    pub(crate) const fn socket_addr_resolved(&self) -> SocketAddr {
        match self {
            Self::Resolved(socket_addr) => *socket_addr,
            _ => panic!("RemoteAddr is not resolved"),
        }
    }
}

impl From<SocketAddr> for RemoteAddr {
    fn from(addr: SocketAddr) -> Self {
        Self::Resolved(addr)
    }
}
