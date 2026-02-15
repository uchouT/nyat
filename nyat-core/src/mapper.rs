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
/// Called when the discovered public address changes.
///
/// Automatically implemented for `FnMut(SocketAddr)` closures.
pub trait MappingHandler: Send {
    /// Invoked once each time the public socket address changes.
    fn on_change(&mut self, new_addr: SocketAddr);
}

impl<F: FnMut(SocketAddr) + Send> MappingHandler for F {
    fn on_change(&mut self, new_addr: SocketAddr) {
        self(new_addr)
    }
}
