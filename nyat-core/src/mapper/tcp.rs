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
    pub async fn run<H: MappingHandler>(&self, handler: &mut H) -> Result<(), Error> {
        let mut current_ip = None;
        let mut retry_cnt = 0usize;

        loop {
            match TcpMapperReactor::new(&self.local, &self.remote, &self.stun).await {
                Ok(mut actor) => {
                    retry_cnt = 0;
                    let pub_addr = actor.pub_addr;
                    if Some(pub_addr) != current_ip {
                        current_ip = Some(pub_addr);
                        handler.on_change(super::MappingInfo::new(pub_addr, actor.local_addr));
                    }

                    let _ =
                        keepalive(&mut actor.tcp_stream, &self.request, self.tick_interval).await;
                }

                Err(e) if !e.is_recoverable() => return Err(e),
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

    pub(super) fn new(builder: super::MapperBuilder<super::builder::TcpConfig>) -> Self {
        let remote = builder.config.ka_remote;
        let request = match &remote.kind {
            crate::net::RemoteAddrKind::Host { domain, .. } => {
                format!("HEAD / HTTP/1.1\r\nHost: {domain}\r\nConnection: keep-alive\r\n\r\n")
            }
            crate::net::RemoteAddrKind::Resolved(addr) => format!(
                "HEAD / HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\n\r\n",
                addr.ip()
            ),
        };
        Self {
            remote,
            stun: builder.stun,
            local: builder.local,
            tick_interval: builder.interval,
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

struct TcpMapperReactor {
    local_addr: SocketAddr,
    tcp_stream: TcpStream,
    pub_addr: SocketAddr,
}

impl TcpMapperReactor {
    async fn new(
        local: &LocalAddr,
        ka_remote: &RemoteAddr,
        stun: &RemoteAddr,
    ) -> Result<Self, Error> {
        let socket_ka = local
            .socket(crate::net::Protocol::Tcp)
            .map_err(Error::Socket)?;

        let local_addr = socket_ka
            .local_addr()
            .map_err(Error::Socket)?
            .as_socket()
            .unwrap();

        let socket_st = local
            .socket_from_addr(local_addr, crate::net::Protocol::Tcp)
            .map_err(Error::Socket)?;

        let (addr_ka, addr_st) = try_join!(ka_remote.socket_addr(), stun.socket_addr())?;

        // tcp connect
        let tcp_stream = connect_remote(socket_ka, addr_ka)
            .await
            .map_err(Error::Connection)?;

        let stun_stream = connect_remote(socket_st, addr_st)
            .await
            .map_err(Error::Connection)?;
        let pub_addr = crate::stun::tcp_socket_addr(stun_stream)
            .await
            .map_err(Error::from)?;

        Ok(Self {
            tcp_stream,
            local_addr,
            pub_addr,
        })
    }
}
