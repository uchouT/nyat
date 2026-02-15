use crate::{
    mapper::{TcpMapper, UdpMapper},
    net::{LocalAddr, RemoteAddr},
};
use std::{num::NonZeroUsize, time::Duration};

#[doc(hidden)]
pub struct MissingTcpRemote;

#[doc(hidden)]
pub struct WithTcpRemote(pub(crate) RemoteAddr);

/// Builder for [`TcpMapper`] and [`UdpMapper`].
///
/// `local` and `stun` are required. Call [`tcp_remote`](Self::tcp_remote)
/// before [`build_tcp`](MapperBuilder::build_tcp) to provide the TCP
/// keepalive target.
#[derive(Debug)]
pub struct MapperBuilder<S> {
    pub(super) local: LocalAddr,
    pub(super) stun: RemoteAddr,
    pub(super) interval: Duration,
    pub(super) check_per_tick: NonZeroUsize,
    pub(super) state: S,
}

impl MapperBuilder<MissingTcpRemote> {
    /// Create a builder with required local bind config and STUN server address.
    #[must_use]
    pub const fn new(local: LocalAddr, stun_addr: RemoteAddr) -> Self {
        Self {
            local,
            stun: stun_addr,
            interval: Duration::from_secs(30),
            check_per_tick: NonZeroUsize::new(5).unwrap(),
            state: MissingTcpRemote,
        }
    }
}

impl<S> MapperBuilder<S> {
    /// Set the TCP keepalive remote target. Required for [`build_tcp`](MapperBuilder::build_tcp).
    #[must_use]
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
    #[must_use]
    pub const fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Set how many keepalive ticks between STUN probes (UDP only). Defaults to 5.
    #[must_use]
    pub const fn check_per_tick(mut self, check_per_tick: NonZeroUsize) -> Self {
        self.check_per_tick = check_per_tick;
        self
    }

    /// Build a [`UdpMapper`].
    #[must_use]
    pub fn build_udp(self) -> UdpMapper {
        UdpMapper::new(self)
    }
}

impl MapperBuilder<WithTcpRemote> {
    /// Build a [`TcpMapper`]. Requires [`tcp_remote`](MapperBuilder::tcp_remote) to have been called.
    #[must_use]
    pub fn build_tcp(self) -> TcpMapper {
        TcpMapper::new(self)
    }
}
