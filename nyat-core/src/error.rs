//! Error types for nyat-core.

error_set::error_set! {
    DnsError := {
        /// The system DNS resolver returned an error.
        #[display("DNS lookup failed")]
        DnsResolve(std::io::Error),

        /// No addresses matched the requested IP version preference.
        #[display("no matching address found")]
        AddrNotFound,
    }

    StunError := {
        /// The STUN response could not be parsed (missing or invalid attributes).
        #[display("malformed STUN response")]
        StunMalformed,

        /// The STUN response body exceeded the maximum allowed size.
        #[display("STUN response too large")]
        StunResponseTooLarge,

        /// Network I/O error during STUN operations.
        #[display("STUN network I/O error")]
        StunNetwork(std::io::Error),

        /// The STUN response transaction ID did not match the request.
        #[display("STUN transaction ID mismatch")]
        StunTransactionIdMismatch,
    }

/// Top-level error returned by mapper operations.
///
/// Each variant represents a semantically distinct failure that callers
/// can match on to decide whether to retry or abort.
    Error := StunError || DnsError || {
        #[display("socket creation/bind failed")]
        Socket(std::io::Error),

        #[display("connection failed")]
        Connection(std::io::Error),

        /// Keepalive I/O failed (connection likely broken)
        #[display("keepalive failed")]
        Keepalive(std::io::Error),
    }
}
