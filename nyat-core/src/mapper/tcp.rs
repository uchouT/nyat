use std::{net::SocketAddr, time::Duration};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    try_join,
};

use crate::{
    error::Error,
    mapper::MappingHandler,
    net::connect_remote,
    net::{LocalAddr, RemoteAddr},
};

/// Maintains a TCP connection and periodically discovers the public address via STUN.
pub struct TcpMapper {
    remote: RemoteAddr,
    stun: RemoteAddr,
    local: LocalAddr,
    tick_interval: Duration,
}

impl TcpMapper {
    const RETRY_LTD: usize = 5;
    /// Run the keepalive loop, calling `handler` whenever the public address changes.
    ///
    /// Returns only on unrecoverable error or after exhausting retries.
    pub async fn run<H: MappingHandler>(&self, handler: &mut H) -> Result<(), Error> {
        let mut current_ip = None;
        let mut retry_cnt = 0usize;

        loop {
            match self.stream_and_for_address().await {
                Ok((mut stream, pub_addr)) => {
                    if Some(pub_addr) != current_ip {
                        current_ip = Some(pub_addr);
                        handler.on_change(pub_addr);
                    }
                    if let Err(e) = keepalive(&mut stream, self.tick_interval).await {
                        if {
                            retry_cnt += 1;
                            retry_cnt
                        } >= TcpMapper::RETRY_LTD
                        {
                            break Err(Error::Keepalive(e));
                        }
                    } else {
                        retry_cnt = 0;
                    }
                }
                Err(e) => match e {
                    Error::Socket(_) => return Err(e),
                    _ => {
                        if {
                            retry_cnt += 1;
                            retry_cnt
                        } >= TcpMapper::RETRY_LTD
                        {
                            return Err(e);
                        }
                    }
                },
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Create keepalive tcp stream and get public address via STUN
    async fn stream_and_for_address(&self) -> Result<(TcpStream, SocketAddr), Error> {
        let socket_ka = self
            .local
            .socket(crate::net::Protocol::Tcp)
            .map_err(Error::Socket)?;
        let socket_st = self
            .local
            .socket(crate::net::Protocol::Tcp)
            .map_err(Error::Socket)?;

        let (addr_ka, addr_st) = try_join!(self.remote.socket_addr(), self.stun.socket_addr())?;

        // tcp connect
        try_join!(
            async {
                connect_remote(socket_ka, addr_ka)
                    .await
                    .map_err(Error::Connection)
            },
            async {
                let stun_stream = connect_remote(socket_st, addr_st)
                    .await
                    .map_err(Error::Connection)?;
                crate::stun::tcp_socket_addr(stun_stream)
                    .await
                    .map_err(Error::from)
            }
        )
    }

    pub(super) fn new(builder: super::MapperBuilder<super::WithTcpRemote>) -> Self {
        Self {
            remote: builder.state.0,
            stun: builder.stun,
            local: builder.local,
            tick_interval: builder.interval.unwrap_or(Duration::from_secs(30)),
        }
    }
}

const BUF_SIZE: usize = 1024;

/// send tick to keep the tcp connection alive
async fn keepalive(stream: &mut TcpStream, interval: Duration) -> Result<(), std::io::Error> {
    let mut interval = tokio::time::interval(interval);
    let mut buf = [0u8; BUF_SIZE];
    stream.write_all(b"nya").await?;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                stream.write_all(b"nya").await?;
            }

            res = stream.read(&mut buf) => match res {
                // receive FIN
                Ok(0) => return Ok(()),
                // ignore
                Ok(_) => {}
                Err(e) => return Err(e)
            }
        }
    }
}
