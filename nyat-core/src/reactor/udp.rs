use std::{num::NonZeroUsize, os::unix::net::SocketAddr, time::Duration};

use crate::{
    addr::{LocalAddr, RemoteAddr},
    error::StunError,
};

pub struct UdpReactor {
    stun: RemoteAddr,
    local: LocalAddr,
    interval: Duration,
    keepalive_interval: tokio::sync::watch::Sender<SocketAddr>,
    check_per_tick: NonZeroUsize,
}

impl UdpReactor {
    pub async fn run(&self) -> Result<(), crate::error::Error> {
        let socket_st = self.local.udp_socket()?;
        let socket_ka = self.local.udp_socket()?;
        loop {
            let stun_addr = self.stun.socket_addr().await.map_err(StunError::from)?;
            socket_st.connect(stun_addr).await?;
        }
        todo!()
    }
}
