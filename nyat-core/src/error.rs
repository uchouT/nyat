use error_set::error_set;

use crate::reactor::TcpStreamError;
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
#[error("Stun error")]
pub(crate) struct StunError {
    #[source]
    #[from]
    innner: stun::Error,
}
