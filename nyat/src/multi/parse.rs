//! parse conf from file
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use nyat_core::net::{IpVer, RemoteAddr};
use serde::Deserialize;

use crate::config::{RunConfig, RunMode};

#[derive(Debug, Clone)]
struct Server {
    host: String,
    port: u16,
}

impl Server {
    /// Both present → Ok(Some), both absent → Ok(None), partial → Err.
    fn try_from_pair(host: Option<String>, port: Option<u16>, label: &str) -> Result<Option<Self>> {
        match (host, port) {
            (Some(host), Some(port)) => Ok(Some(Self { host, port })),
            (None, None) => Ok(None),
            _ => bail!("{label}-host and {label}-port must both be specified"),
        }
    }

    fn into_remote_addr(self, ver: Option<IpVer>) -> RemoteAddr {
        if let Ok(addr) = format!("{}:{}", self.host, self.port).parse::<SocketAddr>() {
            RemoteAddr::from_addr(addr)
        } else {
            RemoteAddr::from_host(self.host, self.port, ver)
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct BatchFile {
    log_level: Option<String>,
    #[serde(default)]
    default: Defaults,
    task: HashMap<String, TaskEntry>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Defaults {
    stun_host: Option<String>,
    stun_port: Option<u16>,
    remote_host: Option<String>,
    remote_port: Option<u16>,
    keepalive: Option<u64>,
    ipv6: Option<bool>,
    #[cfg(target_os = "linux")]
    iface: Option<String>,
    #[cfg(target_os = "linux")]
    fwmark: Option<u32>,
    #[cfg(target_os = "linux")]
    force_reuse: Option<bool>,
}

impl Defaults {
    /// parse stun and remote
    fn into_parsed(self) -> Result<ParsedDefaults> {
        let stun =
            Server::try_from_pair(self.stun_host, self.stun_port, "stun").context("STUN server")?;

        let remote = Server::try_from_pair(self.remote_host, self.remote_port, "remote")
            .context("remote server")?;

        #[cfg(target_os = "linux")]
        if let Some(ref name) = self.iface {
            crate::config::check_iface(name).context("[default] iface")?;
        }

        Ok(ParsedDefaults {
            stun,
            remote,
            keepalive: self.keepalive,
            ipv6: self.ipv6,
            #[cfg(target_os = "linux")]
            iface: self.iface,
            #[cfg(target_os = "linux")]
            fwmark: self.fwmark,
            #[cfg(target_os = "linux")]
            force_reuse: self.force_reuse,
        })
    }
}

struct ParsedDefaults {
    stun: Option<Server>,
    remote: Option<Server>,
    keepalive: Option<u64>,
    ipv6: Option<bool>,
    #[cfg(target_os = "linux")]
    iface: Option<String>,
    #[cfg(target_os = "linux")]
    fwmark: Option<u32>,
    #[cfg(target_os = "linux")]
    force_reuse: Option<bool>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum TaskMode {
    Tcp,
    Udp,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct TaskEntry {
    mode: TaskMode,
    bind: String,
    stun_host: Option<String>,
    stun_port: Option<u16>,
    remote_host: Option<String>,
    remote_port: Option<u16>,
    keepalive: Option<u64>,
    count: Option<NonZeroUsize>,
    ipv6: Option<bool>,
    #[cfg(target_os = "linux")]
    iface: Option<String>,
    #[cfg(target_os = "linux")]
    fwmark: Option<u32>,
    #[cfg(target_os = "linux")]
    force_reuse: Option<bool>,
}

fn parse_bind(s: &str, ipv6: bool) -> Result<SocketAddr> {
    if let Ok(port) = s.parse::<u16>() {
        let ip = if ipv6 {
            IpAddr::V6(Ipv6Addr::UNSPECIFIED)
        } else {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        };
        Ok(SocketAddr::new(ip, port))
    } else {
        s.parse::<SocketAddr>()
            .context("invalid bind: expected PORT or ADDR:PORT")
    }
}

impl TaskEntry {
    fn into_config(self, name: &str, defaults: &ParsedDefaults) -> Result<RunConfig> {
        let ctx = |msg: &str| format!("task '{name}': {msg}");

        let ipv6 = self.ipv6.or(defaults.ipv6).unwrap_or(false);
        let ver = if ipv6 {
            Some(IpVer::V6)
        } else {
            Some(IpVer::V4)
        };

        let stun = Server::try_from_pair(self.stun_host, self.stun_port, "stun")
            .context(ctx("STUN server"))?
            .or(defaults.stun.clone())
            .context(ctx("requires stun server"))?
            .into_remote_addr(ver);

        let bind = parse_bind(&self.bind, ipv6).context(ctx("bind"))?;

        let keepalive = self
            .keepalive
            .or(defaults.keepalive)
            .map(Duration::from_secs);

        let mode = match self.mode {
            TaskMode::Tcp => {
                let remote = Server::try_from_pair(self.remote_host, self.remote_port, "remote")
                    .context(ctx("remote server"))?
                    .or(defaults.remote.clone())
                    .context(ctx("tcp mode requires remote-host and remote-port"))?
                    .into_remote_addr(ver);
                RunMode::Tcp { remote }
            }
            TaskMode::Udp => {
                if self.remote_host.is_some() || self.remote_port.is_some() {
                    bail!(
                        "{}",
                        ctx("remote-host/remote-port are not valid in udp mode")
                    );
                }
                RunMode::Udp { count: self.count }
            }
        };

        #[cfg(target_os = "linux")]
        if let Some(ref name) = self.iface {
            crate::config::check_iface(name).context(ctx("iface"))?;
        }
        #[cfg(target_os = "linux")]
        let iface = self.iface.or_else(|| defaults.iface.clone());

        Ok(RunConfig {
            mode,
            bind,
            stun,
            keepalive,
            #[cfg(target_os = "linux")]
            iface,
            #[cfg(target_os = "linux")]
            fwmark: self.fwmark.or(defaults.fwmark),
            #[cfg(target_os = "linux")]
            force_reuse: self.force_reuse.or(defaults.force_reuse).unwrap_or(false),
        })
    }
}

#[non_exhaustive]
pub struct MultiConfig {
    pub log_level: Option<String>,
    pub tasks: HashMap<String, RunConfig>,
}

impl MultiConfig {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let file: BatchFile = toml::from_str(&content).context("failed to parse config")?;

        if file.task.is_empty() {
            bail!("no [task.*] entries in {}", path.display());
        }

        let default = file
            .default
            .into_parsed()
            .context("Failed to parse default config")?;

        let configs = file
            .task
            .into_iter()
            .map(|(name, t)| {
                let config = t.into_config(&name, &default)?;
                Ok((name, config))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            log_level: file.log_level,
            tasks: configs,
        })
    }
}
