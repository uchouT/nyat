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
#[derive(Debug)]
pub struct TcpMapper {
    remote: RemoteAddr,
    stun: RemoteAddr,
    local: LocalAddr,
    tick_interval: Duration,
    request: String,
}

impl TcpMapper {
    const RETRY_LTD: usize = 5;
    /// Run the keepalive loop, calling `handler` whenever the public address changes.
    ///
    /// Returns only on unrecoverable error or after exhausting retries.
    pub async fn run<H: MappingHandler>(self, mut handler: H) -> Result<(), Error> {
        let mut current_ip = None;
        let mut retry_cnt = 0usize;

        loop {
            match self.stream_and_addr().await {
                Ok((mut stream, pub_addr)) => {
                    retry_cnt = 0;
                    if Some(pub_addr) != current_ip {
                        current_ip = Some(pub_addr);
                        handler.on_change(pub_addr);
                    }

                    let _ = keepalive(&mut stream, &self.request, self.tick_interval).await;
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

    /// Create keepalive tcp stream and get public address via STUN
    async fn stream_and_addr(&self) -> Result<(TcpStream, SocketAddr), Error> {
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

    pub(super) fn new(builder: super::MapperBuilder<super::builder::WithTcpRemote>) -> Self {
        let remote = builder.state.0;
        let request = match &remote.kind {
            crate::net::RemoteAddrKind::Host { domain, .. } => format!(
                "HEAD / HTTP/1.1\r\nHost: {domain}\r\nConnection: keep-alive\r\n\r\n"
            ),
            crate::net::RemoteAddrKind::Resolved(addr) => format!(
                "HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\n\r\n",
                addr.ip()
            ),
        };
        Self {
            remote,
            stun: builder.stun,
            local: builder.local,
            tick_interval: builder.interval.unwrap_or(Duration::from_secs(30)),
            request,
        }
    }
}

/// Send periodic HTTP HEAD requests to keep the TCP connection alive.
async fn keepalive(
    stream: &mut TcpStream,
    request: &str,
    interval: Duration,
) -> Result<(), std::io::Error> {
    let mut interval = tokio::time::interval(interval);
    let mut buf = [0u8; 8192];
    loop {
        tokio::select! {
            _ = interval.tick() => {
                stream.write_all(request.as_bytes()).await?;
            }

            res = stream.read(&mut buf) => match res {
                // receive FIN
                Ok(0) => return Ok(()),
                // ignore response body
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
    }
}
