use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::time::Duration;

use nyat_core::mapper::{Mapper, MapperBuilder};
use nyat_core::net::{LocalAddr, RemoteAddr};

/// Validate that an interface name fits within `IFNAMSIZ` (16 bytes).
#[cfg(target_os = "linux")]
pub(crate) fn check_iface(name: &str) -> anyhow::Result<()> {
    const IFNAMSIZ: usize = 16;
    anyhow::ensure!(
        name.len() <= IFNAMSIZ,
        "interface name exceeds IFNAMSIZ ({IFNAMSIZ} bytes)"
    );
    Ok(())
}

/// Resolved configuration for a single mapping task.
#[non_exhaustive]
pub struct TaskConfig {
    pub mode: RunMode,
    pub bind: SocketAddr,
    pub stun: RemoteAddr,
    pub keepalive: Option<Duration>,
    pub exec: Option<String>,
    #[cfg(target_os = "linux")]
    pub iface: Option<String>,
    #[cfg(target_os = "linux")]
    pub fwmark: Option<u32>,
    #[cfg(target_os = "linux")]
    pub force_reuse: bool,
}

impl TaskConfig {
    pub fn into_mapper(self) -> Mapper {
        let mut local = LocalAddr::new(self.bind);
        #[cfg(target_os = "linux")]
        {
            if let Some(fmark) = self.fwmark {
                local = local.with_fmark(fmark);
            }
            if let Some(ref iface) = self.iface {
                local = local.with_iface(iface.as_bytes());
            }
            if self.force_reuse {
                local = local.force_reuse_port();
            }
        }

        match self.mode {
            RunMode::Tcp { remote } => {
                let mut builder = MapperBuilder::new_tcp(local, self.stun, remote);
                if let Some(keepalive) = self.keepalive {
                    builder = builder.interval(keepalive);
                }
                builder.build().into()
            }
            RunMode::Udp { count } => {
                let mut builder = MapperBuilder::new_udp(local, self.stun);
                if let Some(count) = count {
                    builder = builder.check_per_tick(count);
                }
                if let Some(keepalive) = self.keepalive {
                    builder = builder.interval(keepalive);
                }
                builder.build().into()
            }
        }
    }
}

pub enum RunMode {
    Tcp { remote: RemoteAddr },
    Udp { count: Option<NonZeroUsize> },
}
