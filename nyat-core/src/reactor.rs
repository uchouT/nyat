//! event loop

use crate::net::DnsError;

mod tcp;
mod udp;

error_set::error_set! {
    TcpStreamError := {
        Socket(std::io::Error),
        Dns(DnsError),
        Connection(std::io::Error),
        Stun,
    }
}
