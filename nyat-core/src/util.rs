use socket2::Socket;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use crate::error::Error;

#[derive(Clone, Copy)]
pub(crate) enum Protocol {
    Tcp,
    Udp,
}

/// resolve dns, domain without trailing "/"
/// TODO: accept v6 or v4 hint
pub(crate) async fn resolve_dns(domain: &str, port: u16) -> Result<SocketAddr, Error> {
    tokio::net::lookup_host(format!("{domain}:{port}"))
        .await?
        .find(SocketAddr::is_ipv4)
        .ok_or(Error::DNS)
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

/// send tick to keep the tcp connection alive
pub(crate) async fn keepalive(stream: &TcpStream) -> Result<(), std::io::Error> {
    todo!()
}
