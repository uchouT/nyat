use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use tokio::net::UdpSocket;

use crate::{
    error::Error,
    mapper::MappingHandler,
    net::{LocalAddr, RemoteAddr},
    stun::StunUdpSocket,
};

/// Sends UDP keepalive packets and periodically discovers the public address via STUN.
pub struct UdpMapper {
    stun: RemoteAddr,
    local: LocalAddr,
    interval: Duration,
    check_per_tick: NonZeroUsize,
}

impl UdpMapper {
    /// Run the keepalive loop, calling `handler` whenever the public address changes.
    pub async fn run<H: MappingHandler>(&self, handler: &mut H) -> Result<(), Error> {
        let socket_st = self.local.udp_socket().map_err(Error::Socket)?;
        let socket_ka = self.local.udp_socket().map_err(Error::Socket)?;
        let mut current_ip = None;

        loop {
            let stun_addr = if self.stun.is_resolved() {
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

    async fn keepalive<H: MappingHandler>(
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
                    *current_ip = Some(socket_pub);
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

    pub(super) fn new<S>(builder: super::MapperBuilder<S>) -> Self {
        Self {
            stun: builder.stun,
            local: builder.local,
            interval: builder.interval.unwrap_or(Duration::from_secs(30)),
            check_per_tick: builder
                .check_per_tick
                .unwrap_or(NonZeroUsize::new(5).unwrap()),
        }
    }
}
