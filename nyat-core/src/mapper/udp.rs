use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use tokio::net::UdpSocket;

use crate::{
    error::Error,
    mapper::MappingHandler,
    net::{LocalAddr, RemoteAddr},
    stun::StunUdpSocket,
};

/// Sends UDP keepalive packets and periodically discovers the public address via STUN.
#[derive(Debug)]
pub struct UdpMapper {
    stun: RemoteAddr,
    local: LocalAddr,
    interval: Duration,
    check_per_tick: NonZeroUsize,
    #[cfg(feature = "reuse_port")]
    reuse_port: bool,
}

impl UdpMapper {
    const RETRY_LTD: usize = 5;

    /// Run the keepalive loop, calling `handler` whenever the public address changes.
    pub async fn run<H: MappingHandler>(self, mut handler: H) -> Result<(), Error> {
        let socket_st = self
            .local
            .udp_socket(
                #[cfg(feature = "reuse_port")]
                self.reuse_port,
            )
            .map_err(Error::Socket)?;
        let socket_ka = self
            .local
            .udp_socket(
                #[cfg(feature = "reuse_port")]
                self.reuse_port,
            )
            .map_err(Error::Socket)?;
        let mut current_ip = None;
        let mut retry_cnt = 0usize;

        loop {
            let stun_addr = self.stun.socket_addr().await?;

            let socket_st = StunUdpSocket::new(&socket_st, stun_addr)
                .await
                .map_err(Error::Connection)?;

            match self
                .keepalive(
                    socket_st,
                    &socket_ka,
                    &stun_addr,
                    &mut current_ip,
                    &mut handler,
                )
                .await
            {
                Ok(()) => retry_cnt = 0,
                Err(e) if matches!(e, Error::Socket(_)) => return Err(e),
                Err(e) => {
                    retry_cnt += 1;
                    if retry_cnt >= Self::RETRY_LTD {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
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
        // initial STUN probe — discover public address immediately
        let pub_addr = crate::stun::udp_socket_addr(socket_st).await?;
        if current_ip != &Some(pub_addr) {
            *current_ip = Some(pub_addr);
            handler.on_change(pub_addr);
        }

        let mut cnt = 0usize;
        let mut consecutive_failures = 0usize;
        loop {
            tokio::time::sleep(self.interval).await;
            cnt += 1;
            if cnt >= self.check_per_tick.get() {
                // get public addr every `check_per_tick` ticks
                cnt = 0;
                match crate::stun::udp_socket_addr(socket_st).await {
                    Ok(pub_addr) => {
                        consecutive_failures = 0;
                        if current_ip != &Some(pub_addr) {
                            *current_ip = Some(pub_addr);
                            handler.on_change(pub_addr);
                        }
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        if consecutive_failures >= Self::RETRY_LTD {
                            return Err(e.into());
                        }
                    }
                }
            } else {
                // send keepalive packet, tolerate individual failures
                if let Err(e) = socket_ka.send_to(b"nya", stun_addr).await {
                    consecutive_failures += 1;
                    if consecutive_failures >= Self::RETRY_LTD {
                        return Err(Error::Keepalive(e));
                    }
                } else {
                    consecutive_failures = 0;
                }
            }
        }
    }

    pub(super) fn new<S>(builder: super::MapperBuilder<S>) -> Self {
        Self {
            stun: builder.stun,
            local: builder.local,
            interval: builder.interval,
            check_per_tick: builder.check_per_tick,
            #[cfg(feature = "reuse_port")]
            reuse_port: builder.reuse_port,
        }
    }
}
