//! NAT mapping sessions

use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use crate::net::{LocalAddr, RemoteAddr};

mod tcp;
mod udp;

pub use tcp::TcpMapper;
pub use udp::UdpMapper;

/// public socket address change handler
pub trait MappingHandler {
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

pub struct MapperBuilder<S> {
    local: LocalAddr,
    stun: RemoteAddr,
    interval: Option<Duration>,
    check_per_tick: Option<NonZeroUsize>,
    state: S,
}

impl MapperBuilder<MissingTcpRemote> {
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
    pub fn tcp_remote(self, ka_remote: RemoteAddr) -> MapperBuilder<WithTcpRemote> {
        MapperBuilder {
            local: self.local,
            stun: self.stun,
            interval: self.interval,
            check_per_tick: self.check_per_tick,
            state: WithTcpRemote(ka_remote),
        }
    }

    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    pub fn check_per_tick(mut self, check_per_tick: NonZeroUsize) -> Self {
        self.check_per_tick = Some(check_per_tick);
        self
    }

    pub fn build_udp(self) -> UdpMapper {
        UdpMapper::new(self)
    }
}

impl MapperBuilder<WithTcpRemote> {
    pub fn build_tcp(self) -> TcpMapper {
        TcpMapper::new(self)
    }
}
