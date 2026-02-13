//! Network address types and low-level socket utilities.

use std::net::SocketAddr;

#[cfg(target_os = "linux")]
use smallvec::SmallVec;

use socket2::{Domain, Socket, Type};
use tokio::{
    net::{TcpStream, UdpSocket},
    time::timeout,
};

use crate::error::DnsError;

/// Local bind configuration: address, optional fwmark, and interface binding.
///
/// Sockets created from this config have `SO_REUSEPORT` and `SO_REUSEADDR` set.
///
/// # Platform support
///
/// `with_fmark` and `with_iface` are Linux-only.
pub struct LocalAddr {
    local_addr: SocketAddr,
    #[cfg(target_os = "linux")]
    fmark: Option<u32>,
    #[cfg(target_os = "linux")]
    iface: Option<SmallVec<[u8; 16]>>,
}

impl LocalAddr {
    /// Create a new local bind config for the given address.
    pub fn new(local_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            #[cfg(target_os = "linux")]
            fmark: None,
            #[cfg(target_os = "linux")]
            iface: None,
        }
    }

    /// Set `SO_MARK` (Linux fwmark) for policy routing.
    #[cfg(target_os = "linux")]
    pub fn with_fmark(mut self, fmark: u32) -> Self {
        self.fmark = Some(fmark);
        self
    }

    /// Bind to a specific network interface (e.g. `b"eth0"`).
    #[cfg(target_os = "linux")]
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
                    #[cfg(target_os = "linux")]
                    Tcp => Type::STREAM.nonblocking(),
                    #[cfg(not(target_os = "linux"))]
                    Tcp => Type::STREAM,
                    #[cfg(target_os = "linux")]
                    Udp => Type::DGRAM.nonblocking(),
                    #[cfg(not(target_os = "linux"))]
                    Udp => Type::DGRAM,
                }
            },
            None,
        )?;

        #[cfg(not(target_os = "linux"))]
        socket.set_nonblocking(true)?;
        #[cfg(unix)]
        socket.set_reuse_port(true)?;
        socket.set_reuse_address(true)?;

        #[cfg(target_os = "linux")]
        {
            if let Some(fmark) = self.fmark {
                socket.set_mark(fmark)?;
            }
            if let Some(iface) = &self.iface {
                socket.bind_device(Some(iface))?;
            }
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

/// Remote endpoint address, either a resolved IP or a domain requiring DNS lookup.
///
/// Construct via [`RemoteAddr::from_addr`], [`RemoteAddr::from_host`],
/// or `From<SocketAddr>`.
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
    /// Create from a resolved `SocketAddr` (no DNS needed).
    pub fn from_addr(addr: SocketAddr) -> Self {
        Self {
            kind: RemoteAddrKind::Resolved(addr),
        }
    }

    /// Create from a domain name and port. DNS resolution happens at connection time.
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

    pub(crate) fn host_str(&self) -> String {
        match &self.kind {
            RemoteAddrKind::Resolved(addr) => addr.ip().to_string(),
            RemoteAddrKind::Host { domain, .. } => domain.clone(),
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

/// IP version preference for DNS resolution.
#[derive(Clone, Copy)]
pub enum IpVer {
    /// Prefer IPv6 addresses.
    V6,
    /// Prefer IPv4 addresses.
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
    let mut addrs = timeout(crate::TIMEOUT_DURATION, tokio::net::lookup_host(host))
        .await
        .map_err(std::io::Error::from)??;

    if let Some(ver) = ver_preference {
        addrs.find(|s| match ver {
            IpVer::V6 => s.is_ipv6(),
            IpVer::V4 => s.is_ipv4(),
        })
    } else {
        addrs.next()
    }
    .ok_or(DnsError::AddrNotFound)
}

/// create tcp stream
pub(crate) async fn connect_remote(
    socket: Socket,
    remote_addr: SocketAddr,
) -> Result<TcpStream, std::io::Error> {
    match socket.connect(&remote_addr.into()) {
        Ok(_) => {}
        #[cfg(unix)]
        Err(ref e) if e.raw_os_error() == Some(libc::EINPROGRESS) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(e) => return Err(e),
    };

    let stream = TcpStream::from_std(socket.into())?;
    timeout(crate::TIMEOUT_DURATION, stream.writable()).await??;

    if let Some(e) = stream.take_error()? {
        return Err(e);
    }

    Ok(stream)
}
