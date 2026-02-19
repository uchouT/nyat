use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::time::Duration;

use nyat_core::mapper::{Mapper, MapperBuilder};
use nyat_core::net::{LocalAddr, RemoteAddr};

/// Parsed run-mode configuration
#[non_exhaustive]
pub struct RunConfig {
    pub mode: RunMode,
    pub bind: SocketAddr,
    pub stun: RemoteAddr,
    pub keepalive: Option<Duration>,
    #[cfg(target_os = "linux")]
    pub iface: Option<String>,
    #[cfg(target_os = "linux")]
    pub fwmark: Option<u32>,
    #[cfg(target_os = "linux")]
    pub force_reuse: bool,
}

pub enum RunMode {
    Tcp { remote: RemoteAddr },
    Udp { count: Option<NonZeroUsize> },
}

pub fn build_mapper(config: &RunConfig) -> Mapper {
    let mut local = LocalAddr::new(config.bind);
    #[cfg(target_os = "linux")]
    {
        if let Some(fmark) = config.fwmark {
            local = local.with_fmark(fmark);
        }
        if let Some(ref iface) = config.iface {
            local = local.with_iface(iface.as_bytes());
        }
        if config.force_reuse {
            local = local.force_reuse_port();
        }
    }

    match &config.mode {
        RunMode::Tcp { remote } => {
            let mut builder =
                MapperBuilder::new_tcp(local, config.stun.clone(), remote.clone());
            if let Some(keepalive) = config.keepalive {
                builder = builder.interval(keepalive);
            }
            builder.build().into()
        }
        RunMode::Udp { count } => {
            let mut builder = MapperBuilder::new_udp(local, config.stun.clone());
            if let Some(count) = count {
                builder = builder.check_per_tick(*count);
            }
            if let Some(keepalive) = config.keepalive {
                builder = builder.interval(keepalive);
            }
            builder.build().into()
        }
    }
}
