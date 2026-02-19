use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use nyat_core::net::{IpVer, RemoteAddr};

use crate::config::{RunConfig, RunMode};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a single mapping task
    Run {
        /// Protocol mode
        mode: Mode,

        #[command(flatten)]
        shared: SharedArgs,

        /// HTTP server for keepalive (TCP only, addr[:port], default port: 80)
        #[arg(short, long)]
        remote: Option<String>,

        /// STUN check cycle: probe every N keepalive intervals (UDP only, default: 5)
        #[arg(short, long)]
        count: Option<NonZeroUsize>,
    },
    /// Run multiple mapping tasks from a config file
    Batch {
        #[arg(short, long, value_name = "FILE")]
        config: PathBuf,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    /// TCP mode (HTTP keepalive + STUN)
    Tcp,
    /// UDP mode (STUN binding)
    Udp,
}

#[derive(Debug, Args)]
struct SharedArgs {
    /// STUN server address (addr[:port], default port: 3478)
    #[arg(short, long)]
    stun: String,

    /// Local bind address ([addr:]port, default: 0)
    #[arg(short, long, default_value = "0", value_name = "BIND")]
    bind: String,

    /// Keepalive interval in seconds (TCP: 30, UDP: 5)
    #[arg(short, long)]
    keepalive: Option<u64>,

    /// Prefer IPv4 for DNS resolution
    #[arg(short = '4', long, conflicts_with = "ipv6")]
    ipv4: bool,

    /// Prefer IPv6 for DNS resolution
    #[arg(short = '6', long, conflicts_with = "ipv4")]
    ipv6: bool,

    /// Network interface to bind to
    #[cfg(target_os = "linux")]
    #[arg(short, long)]
    iface: Option<String>,

    /// Firewall mark for policy routing
    #[cfg(target_os = "linux")]
    #[arg(short, long)]
    fwmark: Option<u32>,

    /// Force SO_REUSEPORT on existing sockets (requires root)
    #[cfg(target_os = "linux")]
    #[arg(long)]
    force_reuse: bool,
}

pub enum Config {
    Single(RunConfig),
    Multi(PathBuf),
}

impl Config {
    pub fn parse() -> Self {
        let cli = Cli::parse();
        Self::try_from(cli).unwrap_or_else(|e| e.exit())
    }
}

impl TryFrom<Cli> for Config {
    type Error = clap::Error;
    fn try_from(value: Cli) -> Result<Self, Self::Error> {
        match value.command {
            Command::Run {
                shared,
                mode,
                remote,
                count,
            } => {
                let bind = parse_bind(&shared.bind, shared.ipv6)?;
                let stun =
                    parse_with_default_port(&shared.stun, STUN_PORT, shared.ipv4, shared.ipv6)?;

                let mode = match mode {
                    Mode::Tcp => {
                        if count.is_some() {
                            return Err(Cli::command().error(
                                clap::error::ErrorKind::ArgumentConflict,
                                "--count is only valid in UDP mode",
                            ));
                        }
                        let remote_str = remote.ok_or_else(|| {
                            Cli::command().error(
                                clap::error::ErrorKind::MissingRequiredArgument,
                                "TCP mode requires --remote (-r)",
                            )
                        })?;
                        let remote = parse_with_default_port(
                            &remote_str,
                            REMOTE_PORT,
                            shared.ipv4,
                            shared.ipv6,
                        )?;
                        RunMode::Tcp { remote }
                    }
                    Mode::Udp => {
                        if remote.is_some() {
                            return Err(Cli::command().error(
                                clap::error::ErrorKind::ArgumentConflict,
                                "--remote is only valid in TCP mode",
                            ));
                        }
                        RunMode::Udp { count }
                    }
                };

                #[cfg(target_os = "linux")]
                if let Some(ref name) = shared.iface {
                    crate::config::check_iface(name).map_err(|e| {
                        Cli::command()
                            .error(clap::error::ErrorKind::InvalidValue, e.to_string())
                    })?;
                }

                Ok(Config::Single(RunConfig {
                    mode,
                    bind,
                    stun,
                    keepalive: shared.keepalive.map(std::time::Duration::from_secs),
                    #[cfg(target_os = "linux")]
                    iface: shared.iface,
                    #[cfg(target_os = "linux")]
                    fwmark: shared.fwmark,
                    #[cfg(target_os = "linux")]
                    force_reuse: shared.force_reuse,
                }))
            }

            Command::Batch { config } => Ok(Config::Multi(config)),
        }
    }
}

fn parse_bind(s: &str, ipv6: bool) -> Result<SocketAddr, clap::Error> {
    if let Ok(port) = s.parse::<u16>() {
        let ip = if ipv6 {
            IpAddr::V6(Ipv6Addr::UNSPECIFIED)
        } else {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        };
        Ok(SocketAddr::new(ip, port))
    } else {
        s.parse::<SocketAddr>().map_err(|_| {
            Cli::command().error(
                clap::error::ErrorKind::InvalidValue,
                "invalid bind address: expected PORT or ADDR:PORT",
            )
        })
    }
}

const STUN_PORT: u16 = 3478;
const REMOTE_PORT: u16 = 80;

fn parse_with_default_port(
    s: &str,
    default_port: u16,
    v4: bool,
    v6: bool,
) -> Result<RemoteAddr, clap::Error> {
    let ver = match (v4, v6) {
        (_, true) => Some(IpVer::V6),
        _ => Some(IpVer::V4),
    };

    // ip:port or [::1]:port
    if let Ok(addr) = s.parse::<SocketAddr>() {
        return Ok(RemoteAddr::from_addr(addr));
    }

    // host:port
    if let Some((host, port_str)) = s.rsplit_once(':') {
        let port: u16 = port_str.parse().map_err(|_| {
            Cli::command().error(
                clap::error::ErrorKind::InvalidValue,
                "invalid address: expected HOST[:PORT]",
            )
        })?;
        return Ok(RemoteAddr::from_host(host, port, ver));
    }

    // bare ip → from_addr; bare domain → from_host
    if let Ok(addr) = format!("{s}:{default_port}").parse::<SocketAddr>() {
        Ok(RemoteAddr::from_addr(addr))
    } else {
        Ok(RemoteAddr::from_host(s, default_port, ver))
    }
}
