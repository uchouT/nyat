//! event loop

use std::{net::SocketAddr, time::Duration};

use tokio::{net::TcpStream, try_join};

use crate::{
    addr::{LocalAddr, RemoteAddr},
    util::{DnsError, connect_remote, keepalive},
};

error_set::error_set! {
    TcpStreamError := {
        SocketCreation(std::io::Error),
        Dns(DnsError),
        Connection(std::io::Error),
        Stun,
    }
}

pub struct TcpReactor {
    remote: RemoteAddr,
    stun: RemoteAddr,
    local: LocalAddr,
    tick_interval: Duration,
    sender: tokio::sync::watch::Sender<SocketAddr>,
}

impl TcpReactor {
    async fn run(&self) -> Result<(), crate::error::Error> {
        let mut current_ip = None;
        loop {
            match self
                .stream_and_for_address(|s| {
                    if Some(s) != current_ip {
                        current_ip = Some(s);
                        let _ = self.sender.send(s);
                    }
                })
                .await
            {
                Ok(mut stream) => {
                    if let Err(e) = keepalive(&mut stream, self.tick_interval).await {
                        todo!()
                    }
                }
                Err(e) => {
                    todo!("retry or abort")
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Create keepalive tcp stream and
    async fn stream_and_for_address<F: FnOnce(SocketAddr)>(
        &self,
        pub_addr: F,
    ) -> Result<TcpStream, TcpStreamError> {
        let socket_ka = self
            .local
            .socket(crate::util::Protocol::Tcp)
            .map_err(TcpStreamError::SocketCreation)?;
        let socket_st = self
            .local
            .socket(crate::util::Protocol::Tcp)
            .map_err(TcpStreamError::SocketCreation)?;

        let (addr_ka, addr_st) = try_join!(self.remote.socket_addr(), self.stun.socket_addr())?;

        // tcp connect
        try_join!(
            async {
                connect_remote(socket_ka, addr_ka)
                    .await
                    .map_err(TcpStreamError::Connection)
            },
            async {
                let stun_stream = connect_remote(socket_st, addr_st)
                    .await
                    .map_err(TcpStreamError::Connection)?;
                let socket_addr = crate::stun::tcp_stun(stun_stream)
                    .await
                    .map_err(|_| TcpStreamError::Stun)?;
                pub_addr(socket_addr);
                Ok(())
            }
        )
        .map(|(stream, _)| stream)
    }
}

pub struct UdpReactor {
    stun: RemoteAddr,
    local: LocalAddr,
}

// impl UdpReactor {
//     pub async fn run(&self) -> Result<(), crate::error::Error> {
//         let socket = self.local.socket(crate::util::Protocol::Udp)?;
//         todo!("change socket to UdpSocket");
//         loop {
//             let stun_socket = self.stun.socket_addr().await?;
//             let mut n = 0u16;
//             loop {}
//         }
//     }
// }
