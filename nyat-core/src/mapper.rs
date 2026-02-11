//! NAT mapping sessions.
//!
//! Use [`MapperBuilder`] to construct a [`TcpMapper`] or [`UdpMapper`],
//! then call [`run`](TcpMapper::run) with a [`MappingHandler`] to start
//! the keepalive loop.

use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use crate::net::{LocalAddr, RemoteAddr};

mod tcp;
mod udp;

pub use tcp::TcpMapper;
pub use udp::UdpMapper;

/// Called when the discovered public address changes.
///
/// Automatically implemented for `FnMut(SocketAddr)` closures.
pub trait MappingHandler {
    /// Invoked once each time the public socket address changes.
    fn on_change(&mut self, new_addr: SocketAddr);
}

impl<F: FnMut(SocketAddr)> MappingHandler for F {
    fn on_change(&mut self, new_addr: SocketAddr) {
        self(new_addr)
    }
}

#[doc(hidden)]
pub struct MissingTcpRemote;

#[doc(hidden)]
pub struct WithTcpRemote(RemoteAddr);

/// Builder for [`TcpMapper`] and [`UdpMapper`].
///
/// `local` and `stun` are required. Call [`tcp_remote`](Self::tcp_remote)
/// before [`build_tcp`](MapperBuilder::build_tcp) to provide the TCP
/// keepalive target.
pub struct MapperBuilder<S> {
    local: LocalAddr,
    stun: RemoteAddr,
    interval: Option<Duration>,
    check_per_tick: Option<NonZeroUsize>,
    state: S,
}

impl MapperBuilder<MissingTcpRemote> {
    /// Create a builder with required local bind config and STUN server address.
    pub fn new(local: LocalAddr, stun_addr: RemoteAddr) -> Self {
        Self {
            local,
            stun: stun_addr,
            interval: None,
            check_per_tick: None,
            state: MissingTcpRemote,
        }
    }
}

impl<S> MapperBuilder<S> {
    /// Set the TCP keepalive remote target. Required for [`build_tcp`](MapperBuilder::build_tcp).
    pub fn tcp_remote(self, ka_remote: RemoteAddr) -> MapperBuilder<WithTcpRemote> {
        MapperBuilder {
            local: self.local,
            stun: self.stun,
            interval: self.interval,
            check_per_tick: self.check_per_tick,
            state: WithTcpRemote(ka_remote),
        }
    }

    /// Set the keepalive / STUN probe interval. Defaults to 30 s.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Set how many keepalive ticks between STUN probes (UDP only). Defaults to 5.
    pub fn check_per_tick(mut self, check_per_tick: NonZeroUsize) -> Self {
        self.check_per_tick = Some(check_per_tick);
        self
    }

    /// Build a [`UdpMapper`].
    pub fn build_udp(self) -> UdpMapper {
        UdpMapper::new(self)
    }
}

impl MapperBuilder<WithTcpRemote> {
    /// Build a [`TcpMapper`]. Requires [`tcp_remote`](MapperBuilder::tcp_remote) to have been called.
    pub fn build_tcp(self) -> TcpMapper {
        TcpMapper::new(self)
    }
}
