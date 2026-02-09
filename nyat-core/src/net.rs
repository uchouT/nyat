use socket2::Socket;
use std::net::SocketAddr;
use tokio::net::TcpStream;

#[derive(Clone, Copy)]
pub(crate) enum Protocol {
    Tcp,
    Udp,
}

pub(crate) enum IpVer {
    V6,
    V4,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DnsError {
    #[error("DNS lookup failed")]
    Resolve(#[from] std::io::Error),
    #[error("no matching address found")]
    NotFound,
}

pub(crate) async fn resolve_dns<T: tokio::net::ToSocketAddrs>(
    host: T,
    ver_prefered: Option<IpVer>,
) -> Result<SocketAddr, DnsError> {
    let mut addrs = tokio::net::lookup_host(host).await?;

    if let Some(ver) = ver_prefered {
        addrs.find(|s| match ver {
            IpVer::V6 => s.is_ipv6(),
            IpVer::V4 => s.is_ipv4(),
        })
    } else {
        addrs.next()
    }
    .ok_or(DnsError::NotFound)
}

/// create tcp stream
pub(crate) async fn connect_remote(
    socket: Socket,
    remote_addr: SocketAddr,
) -> Result<TcpStream, std::io::Error> {
    match socket.connect(&remote_addr.into()) {
        Ok(_) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(e) => return Err(e),
    };

    let stream = TcpStream::from_std(socket.into())?;
    stream.writable().await?;

    if let Some(e) = stream.take_error()? {
        return Err(e);
    }

    Ok(stream)
}
