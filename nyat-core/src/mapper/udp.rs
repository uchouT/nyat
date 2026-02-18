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
}

impl UdpMapper {
    const RETRY_LTD: usize = 5;

    /// Run the keepalive loop, calling `handler` whenever the public address changes.
    pub async fn run<H: MappingHandler>(&self, handler: &mut H) -> Result<(), Error> {
        let socket_st = self.local.udp_socket().map_err(Error::Socket)?;
        let socket_ka = self
            .local
            .udp_socket_from_addr(socket_st.local_addr().map_err(Error::Socket)?)
            .map_err(Error::Socket)?;
        let mut current_ip = None;
        let mut retry_cnt = 0usize;

        loop {
            // Phase 1: DNS + connect + initial STUN probe (errors → retry_cnt)
            let setup = async {
                let stun_addr = self.stun.socket_addr().await?;
                let stun_socket = StunUdpSocket::new(&socket_st, stun_addr)
                    .await
                    .map_err(Error::Connection)?;
                let pub_addr = crate::stun::udp_socket_addr(stun_socket).await?;
                Ok::<_, Error>((stun_addr, pub_addr))
            }
            .await;

            match setup {
                Ok((stun_addr, pub_addr)) => {
                    retry_cnt = 0;
                    if Some(pub_addr) != current_ip {
                        current_ip = Some(pub_addr);
                        handler.on_change(pub_addr);
                    }

                    let _ = self
                        .keepalive(
                            StunUdpSocket { inner: &socket_st },
                            &socket_ka,
                            &stun_addr,
                            &mut current_ip,
                            handler,
                        )
                        .await;
                }
                Err(e) if matches!(e, Error::Socket(_)) => return Err(e),
                Err(e) => {
                    retry_cnt += 1;
                    if retry_cnt >= Self::RETRY_LTD {
                        return Err(e);
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Keepalive loop: periodic STUN re-probes and keepalive packets.
    ///
    /// STUN re-probe failures are silently tolerated (mapping may still be valid).
    /// Any keepalive send failure exits immediately (like natmap).
    async fn keepalive<H: MappingHandler>(
        &self,
        socket_st: StunUdpSocket<'_>,
        socket_ka: &UdpSocket,
        stun_addr: &SocketAddr,
        current_ip: &mut Option<SocketAddr>,
        handler: &mut H,
    ) -> Result<(), Error> {
        let mut cnt = 1usize;
        let mut consecutive_failures = 0usize;
        loop {
            if cnt >= self.check_per_tick.get() {
                // STUN re-probe: tolerate failures
                if let Ok(pub_addr) = crate::stun::udp_socket_addr(socket_st).await {
                    cnt = 1;
                    consecutive_failures = 0;
                    if current_ip != &Some(pub_addr) {
                        *current_ip = Some(pub_addr);
                        handler.on_change(pub_addr);
                    }
                } else {
                    consecutive_failures += 1;
                }
            } else if let Err(e) = socket_ka.send_to(b"nya", stun_addr).await {
                consecutive_failures += 1;
                if consecutive_failures >= Self::RETRY_LTD {
                    return Err(Error::Keepalive(e));
                }
            } else {
                cnt += 1;
                consecutive_failures = 0;
            }
            tokio::time::sleep(self.interval).await;
        }
    }

    pub(super) fn new(builder: super::MapperBuilder<super::builder::UdpConfig>) -> Self {
        Self {
            stun: builder.stun,
            local: builder.local,
            interval: builder.interval,
            check_per_tick: builder.config.check_per_tick,
        }
    }
}
