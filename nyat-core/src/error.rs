use error_set::error_set;

use crate::{reactor::TcpStreamError, util::DnsError};
// TODO: very awful error handling
error_set!(
    pub Error := {
    Io(std::io::Error),
    TcpStream(TcpStreamError),
    Keepalive,
    Stun(StunError)
    }
);

#[derive(Debug, thiserror::Error)]
pub(crate) enum StunError {
    #[error("Failed to parse stun server DNS")]
    Dns(
        #[source]
        #[from]
        DnsError,
    ),

    // TODO: delete this enum variant after testing
    #[error("Internal stun error")]
    Stun(
        #[source]
        #[from]
        stun::Error,
    ),

    #[error("Failed to interact with stun server")]
    Network(
        #[source]
        #[from]
        std::io::Error,
    ),

    #[error("miss match transaction id")]
    TnsactionIdMissMatch,
}
