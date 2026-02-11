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

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DnsError {
    #[error("DNS lookup failed")]
    Resolve(#[from] std::io::Error),
    #[error("no matching address found")]
    NotFound,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StunError {
    #[error("STUN protocol error")]
    Protocol(
        #[source]
        #[from]
        stun::Error,
    ),

    #[error("STUN network I/O error")]
    Network(
        #[source]
        #[from]
        std::io::Error,
    ),

    #[error("STUN transaction ID mismatch")]
    TransactionIdMismatch,
}
