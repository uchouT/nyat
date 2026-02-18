//! NAT mapping sessions.
//!
//! Use [`MapperBuilder::new_tcp`] or [`MapperBuilder::new_udp`] to create
//! a builder, then call [`build`](MapperBuilder::build) and
//! [`run`](TcpMapper::run) with a [`MappingHandler`].

use std::net::SocketAddr;

mod builder;
#[cfg(feature = "tcp")]
mod tcp;
#[cfg(feature = "udp")]
mod udp;

pub use builder::MapperBuilder;
#[cfg(feature = "tcp")]
pub use tcp::TcpMapper;
#[cfg(feature = "udp")]
pub use udp::UdpMapper;

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct MappingInfo {
    pub pub_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

impl MappingInfo {
    pub(crate) const fn new(pub_addr: SocketAddr, local_addr: SocketAddr) -> Self {
        Self {
            pub_addr,
            local_addr,
        }
    }
}

/// Called when the discovered public address changes.
///
/// Automatically implemented for `FnMut(SocketAddr)` closures.
pub trait MappingHandler: Send {
    /// Invoked once each time the public socket address changes.
    fn on_change(&mut self, info: MappingInfo);
}

impl<F: FnMut(MappingInfo) + Send> MappingHandler for F {
    fn on_change(&mut self, info: MappingInfo) {
        self(info)
    }
}

/// Mapper container
#[cfg(all(feature = "tcp", feature = "udp"))]
#[derive(Debug)]
pub enum Mapper {
    Tcp(TcpMapper),
    Udp(UdpMapper),
}

impl From<TcpMapper> for Mapper {
    fn from(m: TcpMapper) -> Self {
        Self::Tcp(m)
    }
}

impl From<UdpMapper> for Mapper {
    fn from(m: UdpMapper) -> Self {
        Self::Udp(m)
    }
}

impl Mapper {
    pub async fn run<H: MappingHandler>(&self, handler: &mut H) -> Result<(), crate::Error> {
        match self {
            Self::Tcp(mapper) => mapper.run(handler).await,
            Self::Udp(mapper) => mapper.run(handler).await,
        }
    }
}
