//! NAT mapping sessions

use std::net::SocketAddr;

pub mod tcp;
pub mod udp;

/// public socket address change handler
pub trait SocketHandler {
    fn on_change(&mut self, new_addr: SocketAddr);
}

impl<F: FnMut(SocketAddr)> SocketHandler for F {
    fn on_change(&mut self, new_addr: SocketAddr) {
        self(new_addr)
    }
}
