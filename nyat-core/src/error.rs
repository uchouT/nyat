//! Error types for nyat-core.

use std::io;

/// DNS resolution error.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DnsError {
    /// The system DNS resolver returned an error.
    #[error("DNS lookup failed")]
    Resolve(#[from] io::Error),

    /// No addresses matched the requested IP version preference.
    #[error("no matching address found")]
    AddrNotFound,
}

/// STUN protocol error.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StunError {
    /// The STUN response could not be parsed (missing or invalid attributes).
    #[error("malformed STUN response")]
    Malformed,

    /// The STUN response body exceeded the maximum allowed size.
    #[error("STUN response too large")]
    ResponseTooLarge,

    /// Network I/O error during STUN operations.
    #[error("STUN network I/O error")]
    Network(#[from] io::Error),

    /// The STUN response transaction ID did not match the request.
    #[error("STUN transaction ID mismatch")]
    TransactionIdMismatch,
}

/// Top-level error returned by mapper operations.
///
/// Each variant represents a semantically distinct failure that callers
/// can match on to decide whether to retry or abort.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The STUN response could not be parsed.
    #[error("malformed STUN response")]
    StunMalformed,

    /// The STUN response body exceeded the maximum allowed size.
    #[error("STUN response too large")]
    StunResponseTooLarge,

    /// Network I/O error during STUN operations.
    #[error("STUN network I/O error")]
    StunNetwork(#[source] io::Error),

    /// The STUN response transaction ID did not match the request.
    #[error("STUN transaction ID mismatch")]
    StunTransactionIdMismatch,

    /// The system DNS resolver returned an error.
    #[error("DNS lookup failed")]
    DnsResolve(#[source] io::Error),

    /// No addresses matched the requested IP version preference.
    #[error("no matching address found")]
    AddrNotFound,

    /// Socket creation or binding failed.
    #[error("socket creation/bind failed")]
    Socket(#[source] io::Error),

    /// TCP connection failed.
    #[error("connection failed")]
    Connection(#[source] io::Error),

    /// Keepalive I/O failed (connection likely broken).
    #[error("keepalive failed")]
    Keepalive(#[source] io::Error),
}

impl From<StunError> for Error {
    fn from(e: StunError) -> Self {
        match e {
            StunError::Malformed => Self::StunMalformed,
            StunError::ResponseTooLarge => Self::StunResponseTooLarge,
            StunError::Network(e) => Self::StunNetwork(e),
            StunError::TransactionIdMismatch => Self::StunTransactionIdMismatch,
        }
    }
}

impl From<DnsError> for Error {
    fn from(e: DnsError) -> Self {
        match e {
            DnsError::Resolve(e) => Self::DnsResolve(e),
            DnsError::AddrNotFound => Self::AddrNotFound,
        }
    }
}
