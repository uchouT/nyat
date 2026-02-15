use crate::{
    mapper::{TcpMapper, UdpMapper},
    net::{LocalAddr, RemoteAddr},
};
use std::{num::NonZeroUsize, time::Duration};

#[doc(hidden)]
#[derive(Debug)]
pub struct UdpConfig {
    pub(super) check_per_tick: NonZeroUsize,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct TcpConfig {
    pub(super) ka_remote: RemoteAddr,
}

/// Builder for [`TcpMapper`] and [`UdpMapper`].
///
/// Use [`new_tcp`](Self::new_tcp) or [`new_udp`](Self::new_udp) to create
/// a builder, configure optional parameters, then call [`build`](Self::build).
///
/// # Examples
///
/// ```no_run
/// use nyat_core::{mapper::MapperBuilder, net::{LocalAddr, RemoteAddr}};
///
/// // TCP mapper
/// let tcp = MapperBuilder::new_tcp(
///     LocalAddr::new("0.0.0.0:8080".parse().unwrap()),
///     RemoteAddr::from_host("stun.example.com", 3478, None),
///     RemoteAddr::from_host("example.com", 80, None),
/// ).build();
///
/// // UDP mapper
/// let udp = MapperBuilder::new_udp(
///     LocalAddr::new("0.0.0.0:8080".parse().unwrap()),
///     RemoteAddr::from_host("stun.example.com", 3478, None),
/// ).build();
/// ```
#[derive(Debug)]
pub struct MapperBuilder<S> {
    pub(super) local: LocalAddr,
    pub(super) stun: RemoteAddr,
    pub(super) interval: Duration,
    pub(super) config: S,
}

impl MapperBuilder<UdpConfig> {
    /// Create a UDP mapper builder.
    ///
    /// Defaults: interval = 5 s, check_per_tick = 5.
    #[must_use]
    pub const fn new_udp(local: LocalAddr, stun_addr: RemoteAddr) -> Self {
        Self {
            local,
            stun: stun_addr,
            interval: Duration::from_secs(5),
            config: UdpConfig {
                check_per_tick: NonZeroUsize::new(5).unwrap(),
            },
        }
    }

    /// Set how many keepalive ticks between STUN probes. Defaults to 5.
    #[must_use]
    pub const fn check_per_tick(mut self, check_per_tick: NonZeroUsize) -> Self {
        self.config.check_per_tick = check_per_tick;
        self
    }

    /// Build a [`UdpMapper`].
    #[must_use]
    pub fn build(self) -> UdpMapper {
        UdpMapper::new(self)
    }
}

impl MapperBuilder<TcpConfig> {
    /// Create a TCP mapper builder.
    ///
    /// `ka_remote` is the HTTP server used for TCP keepalive (typically port 80).
    ///
    /// Defaults: interval = 30 s.
    #[must_use]
    pub const fn new_tcp(local: LocalAddr, stun_addr: RemoteAddr, ka_remote: RemoteAddr) -> Self {
        Self {
            local,
            stun: stun_addr,
            interval: Duration::from_secs(30),
            config: TcpConfig { ka_remote },
        }
    }

    /// Build a [`TcpMapper`].
    #[must_use]
    pub fn build(self) -> TcpMapper {
        TcpMapper::new(self)
    }
}

impl<S> MapperBuilder<S> {
    /// Set the keepalive / STUN probe interval.
    ///
    /// Defaults to 30 s for TCP, 5 s for UDP.
    #[must_use]
    pub const fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }
}
