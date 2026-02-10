use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use tokio::net::UdpSocket;

use crate::{
    addr::{Local, RemoteAddr},
    error::Error,
    mapper::SocketHandler,
    stun::StunUdpSocket,
};

pub struct UdpMapper {
    stun: RemoteAddr,
    local: Local,
    interval: Duration,
    check_per_tick: NonZeroUsize,
}

impl UdpMapper {
    pub async fn run<H: SocketHandler>(&self, handler: &mut H) -> Result<(), Error> {
        let socket_st = self.local.udp_socket().map_err(Error::Socket)?;
        let socket_ka = self.local.udp_socket().map_err(Error::Socket)?;
        let mut current_ip = None;

        loop {
            let stun_addr = if matches!(self.stun, RemoteAddr::Resolved(_)) {
                self.stun.socket_addr_resolved()
            } else {
                self.stun.socket_addr().await?
            };

            let socket_st = StunUdpSocket::new(&socket_st, stun_addr)
                .await
                .map_err(Error::Connection)?;

            // TODO: error handling, retry logic
            self.keepalive(socket_st, &socket_ka, &stun_addr, &mut current_ip, handler)
                .await;
        }
    }

    async fn keepalive<H: SocketHandler>(
        &self,
        socket_st: StunUdpSocket<'_>,
        socket_ka: &UdpSocket,
        stun_addr: &SocketAddr,
        current_ip: &mut Option<SocketAddr>,
        handler: &mut H,
    ) -> Result<(), Error> {
        let mut interval = tokio::time::interval(self.interval);
        let mut cnt = 0;
        loop {
            cnt += 1;
            if cnt == self.check_per_tick.get() {
                cnt = 0;
                let socket_pub = crate::stun::udp_socket_addr(socket_st).await?;
                if current_ip != &Some(socket_pub) {
                    handler.on_change(socket_pub);
                }
            } else {
                socket_ka
                    .send_to(b"nya", stun_addr)
                    .await
                    .map_err(Error::Keepalive)?;
            }
            interval.tick().await;
        }
    }
}
