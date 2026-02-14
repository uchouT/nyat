//! NAT mapping sessions.
//!
//! Use [`MapperBuilder`] to construct a [`TcpMapper`] or [`UdpMapper`],
//! then call [`run`](TcpMapper::run) with a [`MappingHandler`] to start
//! the keepalive loop.

use std::net::SocketAddr;

mod builder;
mod tcp;
mod udp;

pub use builder::MapperBuilder;
pub use tcp::TcpMapper;
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
