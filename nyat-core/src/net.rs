use std::net::SocketAddr;

use smallvec::SmallVec;
use socket2::{Domain, Socket, Type};
use tokio::net::{TcpStream, UdpSocket};

use crate::error::DnsError;

pub struct LocalAddr {
    local_addr: SocketAddr,
    fmark: Option<u32>,
    iface: Option<SmallVec<[u8; 16]>>,
}

impl LocalAddr {
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

pub struct RemoteAddr {
    kind: RemoteAddrKind,
}

enum RemoteAddrKind {
    /// bare socket address
    Resolved(SocketAddr),
    /// domain, requires DNS
    Host {
        domain: String,
        port: u16,
        ver_preference: Option<IpVer>,
    },
}

impl RemoteAddr {
    #[inline]
    pub(crate) const fn is_resolved(&self) -> bool {
        matches!(self.kind, RemoteAddrKind::Resolved(_))
    }

    pub fn from_addr(addr: SocketAddr) -> Self {
        Self {
            kind: RemoteAddrKind::Resolved(addr),
        }
    }

    pub fn from_host(domain: impl Into<String>, port: u16, ver_preference: Option<IpVer>) -> Self {
        Self {
            kind: RemoteAddrKind::Host {
                domain: domain.into(),
                port,
                ver_preference,
            },
        }
    }

    /// get socket addr from remote addr
    pub(crate) async fn socket_addr(&self) -> Result<SocketAddr, DnsError> {
        use RemoteAddrKind::*;
        match &self.kind {
            Host {
                domain,
                port,
                ver_preference,
            } => resolve_dns((domain.as_ref(), *port), *ver_preference).await,
            Resolved(addr) => Ok(*addr),
        }
    }

    #[inline]
    pub(crate) const fn socket_addr_resolved(&self) -> SocketAddr {
        match self.kind {
            RemoteAddrKind::Resolved(socket_addr) => socket_addr,
            _ => panic!("RemoteAddr is not resolved"),
        }
    }
}

impl From<SocketAddr> for RemoteAddr {
    fn from(addr: SocketAddr) -> Self {
        Self {
            kind: RemoteAddrKind::Resolved(addr),
        }
    }
}

#[derive(Clone, Copy)]
pub enum IpVer {
    V6,
    V4,
}

#[derive(Clone, Copy)]
pub(crate) enum Protocol {
    Tcp,
    Udp,
}

pub(crate) async fn resolve_dns<T: tokio::net::ToSocketAddrs>(
    host: T,
    ver_preference: Option<IpVer>,
) -> Result<SocketAddr, DnsError> {
    let mut addrs = tokio::net::lookup_host(host).await?;

    if let Some(ver) = ver_preference {
        addrs.find(|s| match ver {
            IpVer::V6 => s.is_ipv6(),
            IpVer::V4 => s.is_ipv4(),
        })
    } else {
        addrs.next()
    }
    .ok_or(DnsError::NotFound)
}

/// create tcp stream
pub(crate) async fn connect_remote(
    socket: Socket,
    remote_addr: SocketAddr,
) -> Result<TcpStream, std::io::Error> {
    match socket.connect(&remote_addr.into()) {
        Ok(_) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(e) => return Err(e),
    };

    let stream = TcpStream::from_std(socket.into())?;
    stream.writable().await?;

    if let Some(e) = stream.take_error()? {
        return Err(e);
    }

    Ok(stream)
}
