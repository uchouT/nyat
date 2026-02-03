//! event loop

use std::{net::SocketAddr, time::Duration};

use tokio::{net::TcpStream, try_join};

use crate::{
    addr::{LocalAddr, RemoteAddr},
    error::Error,
    util::{connect_remote, keepalive},
};

pub struct TcpReactor {
    remote: RemoteAddr,
    stun: RemoteAddr,
    local: LocalAddr,
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
                        self.sender.send(s);
                    }
                })
                .await
            {
                Ok(stream) => keepalive(&stream).await?,
                Err(e) => {
                    todo!("retry or abort")
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Create keepalive tcp stream
    async fn stream_and_for_address<F: FnOnce(SocketAddr)>(
        &self,
        socket_addr: F,
    ) -> Result<TcpStream, Error> {
        let socket_ka = self.local.socket(crate::util::Protocol::Tcp)?;
        let socket_st = self.local.socket(crate::util::Protocol::Tcp)?;

        let (addr_ka, addr_st) = try_join!(self.remote.socket_addr(), self.stun.socket_addr())?;
        // tcp connect
        let res = try_join!(connect_remote(socket_ka, addr_ka), async {
            let stun_stream = connect_remote(socket_st, addr_st).await?;
            crate::stun::stun_action_tcp(stun_stream, socket_addr).await?;
            Ok(())
        });
        res.map(|(stream, _)| stream).map_err(Error::from)
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
