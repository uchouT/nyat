use crate::net::DnsError;
use crate::stun::StunError;

/// Top-level error type for nyat-core.
///
/// Each variant represents a semantically distinct failure category
/// that callers can meaningfully react to.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Socket creation or bind failed (typically unrecoverable)
    #[error("socket creation/bind failed")]
    Socket(#[source] std::io::Error),

    /// DNS resolution failed
    #[error("DNS resolution failed")]
    Dns(
        #[source]
        #[from]
        DnsError,
    ),

    /// TCP/UDP connection to remote failed
    #[error("connection failed")]
    Connection(#[source] std::io::Error),

    /// STUN protocol interaction failed
    #[error("STUN error")]
    Stun(
        #[source]
        #[from]
        StunError,
    ),

    /// Keepalive I/O failed (connection likely broken)
    #[error("keepalive failed")]
    Keepalive(#[source] std::io::Error),
}
