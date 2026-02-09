use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use tokio::net::UdpSocket;

use crate::{
    addr::{LocalAddr, RemoteAddr},
    error::StunError,
    stun::StunUdpSocket,
};

pub struct UdpReactor {
    stun: RemoteAddr,
    local: LocalAddr,
    interval: Duration,
    sender: tokio::sync::watch::Sender<SocketAddr>,
    check_per_tick: NonZeroUsize,
}

impl UdpReactor {
    pub async fn run(&self) -> Result<(), crate::error::Error> {
        let socket_st = self.local.udp_socket()?;
        let socket_ka = self.local.udp_socket()?;
        let mut current_ip = None;
        // handler to update public address
        let mut handler = |s| {
            if Some(s) != current_ip {
                current_ip = Some(s);
                let _ = self.sender.send(s);
            }
        };
        loop {
            let stun_addr = self.stun.socket_addr().await.map_err(StunError::from)?;
            let socket_st = StunUdpSocket::new(&socket_st, stun_addr).await?;
            self.keepalive(socket_st, &socket_ka, stun_addr, &mut handler)
                .await;
        }
    }

    async fn keepalive<F: FnMut(SocketAddr)>(
        &self,
        socket_st: StunUdpSocket<'_>,
        socket_ka: &UdpSocket,
        stun_addr: SocketAddr,
        mut socket_addr: F,
    ) -> Result<(), crate::error::Error> {
        let mut interval = tokio::time::interval(self.interval);
        let mut cnt = 0;
        loop {
            cnt += 1;
            if cnt == self.check_per_tick.get() {
                cnt = 0;
                let socket_pub = crate::stun::udp_socket_addr(socket_st).await?;
                socket_addr(socket_pub);
            } else {
                socket_ka.send_to(b"nya", stun_addr).await?;
            }
            interval.tick().await;
        }
    }
}
