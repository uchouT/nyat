//! Error types for nyat-core.

/// Top-level error returned by mapper operations.
///
/// Each variant represents a semantically distinct failure that callers
/// can match on to decide whether to retry or abort.
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

/// DNS resolution error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DnsError {
    /// The system DNS resolver returned an error.
    #[error("DNS lookup failed")]
    Resolve(#[from] std::io::Error),
    /// DNS succeeded but returned no matching addresses.
    #[error("no matching address found")]
    NotFound,
}

/// STUN protocol error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StunError {
    /// The STUN library returned a protocol-level error.
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
