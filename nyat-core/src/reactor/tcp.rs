use std::{net::SocketAddr, time::Duration};

use tokio::{net::TcpStream, try_join};

use crate::{
    addr::{LocalAddr, RemoteAddr},
    reactor::TcpStreamError,
    util::{connect_remote, keepalive},
};

pub struct TcpReactor {
    remote: RemoteAddr,
    stun: RemoteAddr,
    local: LocalAddr,
    tick_interval: Duration,
    sender: tokio::sync::watch::Sender<SocketAddr>,
}

impl TcpReactor {
    const RETRY_LTD: usize = 5;
    async fn run(&self) -> Result<(), crate::error::Error> {
        let mut current_ip = None;
        let mut retry_cnt = 0usize;

        // handler to update public address
        let mut handler = |s| {
            if Some(s) != current_ip {
                current_ip = Some(s);
                let _ = self.sender.send(s);
            }
        };
        
        loop {
            match self.stream_and_for_address(&mut handler).await {
                Ok(mut stream) => {
                    if let Err(e) = keepalive(&mut stream, self.tick_interval).await {
                        if {
                            retry_cnt += 1;
                            retry_cnt
                        } >= TcpReactor::RETRY_LTD
                        {
                            break Err(e)?;
                        }
                    } else {
                        retry_cnt = 0;
                    }
                }
                Err(e) => match e {
                    TcpStreamError::Socket(_) => return Err(e)?,
                    _ => {
                        if {
                            retry_cnt += 1;
                            retry_cnt
                        } >= TcpReactor::RETRY_LTD
                        {
                            return Err(e)?;
                        }
                    }
                },
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Create keepalive tcp stream and
    async fn stream_and_for_address<F: FnMut(SocketAddr)>(
        &self,
        mut pub_addr: F,
    ) -> Result<TcpStream, TcpStreamError> {
        let socket_ka = self
            .local
            .socket(crate::util::Protocol::Tcp)
            .map_err(TcpStreamError::Socket)?;
        let socket_st = self
            .local
            .socket(crate::util::Protocol::Tcp)
            .map_err(TcpStreamError::Socket)?;

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
