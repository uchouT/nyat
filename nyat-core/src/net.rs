//! Network address types and low-level socket utilities.
#[cfg(all(feature = "reuse_port", target_os = "linux"))]
mod reuse_port;

use socket2::{Domain, Socket, Type};
use std::net::SocketAddr;
#[cfg(feature = "tcp")]
use tokio::net::TcpStream;
#[cfg(feature = "udp")]
use tokio::net::UdpSocket;
use tokio::time::timeout;

use crate::error::DnsError;

const TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

/// Local bind configuration: address, optional fwmark, and interface binding.
///
/// Sockets created from this config have `SO_REUSEPORT` and `SO_REUSEADDR` set.
///
/// # Platform support
///
/// `with_fmark` and `with_iface` are Linux-only.
#[derive(Debug)]
pub struct LocalAddr {
    local_addr: SocketAddr,
    #[cfg(target_os = "linux")]
    fmark: Option<u32>,
    #[cfg(target_os = "linux")]
    iface: Option<([u8; libc::IFNAMSIZ], u8)>,
    #[cfg(all(feature = "reuse_port", target_os = "linux"))]
    reuse_port: bool,
}

impl LocalAddr {
    /// Create a new local bind config for the given address.
    pub const fn new(local_addr: SocketAddr) -> Self {
        Self {
            local_addr,
            #[cfg(target_os = "linux")]
            fmark: None,
            #[cfg(target_os = "linux")]
            iface: None,
            #[cfg(all(feature = "reuse_port", target_os = "linux"))]
            reuse_port: false,
        }
    }

    /// Set `SO_MARK` (Linux fwmark) for policy routing.
    #[cfg(target_os = "linux")]
    pub const fn with_fmark(mut self, fmark: u32) -> Self {
        self.fmark = Some(fmark);
        self
    }

    /// Bind to a specific network interface (e.g. `b"eth0"`).
    ///
    /// # Panics
    ///
    /// Panics if `iface` is longer than 16 bytes (`IFNAMSIZ`).
    #[cfg(target_os = "linux")]
    pub fn with_iface(mut self, iface: impl AsRef<[u8]>) -> Self {
        let src = iface.as_ref();
        assert!(
            src.len() <= libc::IFNAMSIZ,
            "interface name exceeds IFNAMSIZ (16)"
        );
        let mut buf = [0u8; 16];
        buf[..src.len()].copy_from_slice(src);
        self.iface = Some((buf, src.len() as u8));
        self
    }

    /// Force `SO_REUSEPORT` on existing sockets if `bind` fails with `EADDRINUSE`.
    ///
    /// Uses `pidfd_open(2)` + `pidfd_getfd(2)` to duplicate each matching socket
    /// from other processes and set `SO_REUSEPORT`. Requires `CAP_SYS_PTRACE`
    /// (or root) and Linux ≥ 5.6.
    #[cfg(all(feature = "reuse_port", target_os = "linux"))]
    #[must_use]
    pub const fn force_reuse_port(mut self) -> Self {
        self.reuse_port = true;
        self
    }

    /// Create non-blocking & reuse port & reuse address, with no-exec flag
    /// and bind the local address
    pub(crate) fn socket(&self, p: Protocol) -> Result<Socket, std::io::Error> {
        let socket = Socket::new(
            Domain::for_address(self.local_addr),
            {
                use Protocol::*;
                match p {
                    #[cfg(all(target_os = "linux", feature = "tcp"))]
                    Tcp => Type::STREAM.nonblocking(),
                    #[cfg(all(target_os = "linux", feature = "udp"))]
                    Udp => Type::DGRAM.nonblocking(),
                    #[cfg(all(not(target_os = "linux"), feature = "tcp"))]
                    Tcp => Type::STREAM,
                    #[cfg(all(not(target_os = "linux"), feature = "udp"))]
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
            if let Some((buf, len)) = &self.iface {
                socket.bind_device(Some(&buf[..*len as usize]))?;
            }
        }

        let socket_addr = &self.local_addr.into();

        #[cfg(not(all(feature = "reuse_port", target_os = "linux")))]
        socket.bind(socket_addr)?;

        #[cfg(all(feature = "reuse_port", target_os = "linux"))]
        if let Err(e) = socket.bind(socket_addr) {
            if self.reuse_port && e.kind() == std::io::ErrorKind::AddrInUse {
                reuse_port::force_reuse_port(self.local_addr.port())?;
                socket.bind(socket_addr)?;
            } else {
                return Err(e);
            }
        }
        Ok(socket)
    }

    #[cfg(feature = "udp")]
    pub(crate) fn udp_socket(&self) -> std::io::Result<tokio::net::UdpSocket> {
        let socket = self.socket(Protocol::Udp)?;
        UdpSocket::from_std(socket.into())
    }
}

/// Remote endpoint address, either a resolved IP or a domain requiring DNS lookup.
///
/// Construct via [`RemoteAddr::from_addr`], [`RemoteAddr::from_host`],
/// or `From<SocketAddr>`.
#[derive(Debug, Clone)]
pub struct RemoteAddr {
    pub(crate) kind: RemoteAddrKind,
}

#[derive(Debug, Clone)]
pub(crate) enum RemoteAddrKind {
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
    pub const fn from_addr(addr: SocketAddr) -> Self {
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
}

impl From<SocketAddr> for RemoteAddr {
    fn from(addr: SocketAddr) -> Self {
        Self {
            kind: RemoteAddrKind::Resolved(addr),
        }
    }
}

/// IP version preference for DNS resolution.
#[derive(Debug, Clone, Copy)]
pub enum IpVer {
    /// Prefer IPv6 addresses.
    V6,
    /// Prefer IPv4 addresses.
    V4,
}

#[derive(Clone, Copy)]
pub(crate) enum Protocol {
    #[cfg(feature = "tcp")]
    Tcp,
    #[cfg(feature = "udp")]
    Udp,
}

pub(crate) async fn resolve_dns<T: tokio::net::ToSocketAddrs>(
    host: T,
    ver_preference: Option<IpVer>,
) -> Result<SocketAddr, DnsError> {
    let mut addrs = timeout(TIMEOUT_DURATION, tokio::net::lookup_host(host))
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

#[cfg(feature = "tcp")]
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
    timeout(TIMEOUT_DURATION, stream.writable()).await??;

    // Check if the connection succeeded or failed
    if let Some(e) = stream.take_error()? {
        return Err(e);
    }
    Ok(stream)
}
