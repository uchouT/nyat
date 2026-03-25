use std::io::ErrorKind;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::time::Duration;

use nyat_core::mapper::{Mapper, MapperBuilder, MappingHandler};
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
    pub fn build_task(self) -> Task {
        let mut local = LocalAddr::new(self.bind);
        #[cfg(target_os = "linux")]
        {
            if let Some(fmark) = self.fwmark {
                local = local.with_fmark(fmark);
            }
            if let Some(ref iface) = self.iface {
                local = local.with_iface(iface.as_bytes());
            }
        }

        let inner = match self.mode {
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
        };

        #[cfg(target_os = "linux")]
        let force_reuse = self.force_reuse;
        #[cfg(not(target_os = "linux"))]
        let force_reuse = false;

        Task::new(inner, force_reuse, self.bind.port())
    }
}

pub struct Task {
    inner: Mapper,
    force_reuse: bool,
    port: u16,
}

impl Task {
    pub(crate) const fn new(inner: Mapper, force_reuse: bool, port: u16) -> Self {
        Self {
            inner,
            force_reuse,
            port,
        }
    }

    pub(crate) async fn run<H>(&self, handler: &mut H) -> nyat_core::Result<()>
    where
        H: MappingHandler,
    {
        #[cfg(target_os = "linux")]
        if let Err(err) = self.inner.run(handler).await {
            if let nyat_core::Error::Socket(e) = &err
                && e.kind() == ErrorKind::AddrInUse
                && self.force_reuse
            {
                nyat::force::force_reuse_port(self.port).map_err(nyat_core::Error::Socket)?;
                return self.inner.run(handler).await;
            }
            return Err(err);
        }

        #[cfg(not(target_os = "linux"))]
        self.inner.run(handler).await?;

        Ok(())
    }
}

pub enum RunMode {
    Tcp { remote: RemoteAddr },
    Udp { count: Option<NonZeroUsize> },
}
