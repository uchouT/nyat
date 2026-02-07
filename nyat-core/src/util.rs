use socket2::Socket;
use std::{
    mem::{MaybeUninit, transmute},
    net::SocketAddr,
    num::NonZeroUsize,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};

#[derive(Clone, Copy)]
pub(crate) enum Protocol {
    Tcp,
    Udp,
}

pub(crate) enum IpVer {
    V6,
    V4,
}

error_set::error_set! {
    DnsError {
        Resolve(std::io::Error),
        Notfound,
    }
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
    .ok_or(DnsError::Notfound)
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

const BUF_SIZE: usize = 1024;

/// send tick to keep the tcp connection alive
pub(crate) async fn keepalive(
    stream: &mut TcpStream,
    interval: Duration,
) -> Result<(), std::io::Error> {
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

pub(crate) async fn keepalive_udp<F: FnMut(SocketAddr)>(
    socket_st: &UdpSocket,
    socket_ka: &UdpSocket,
    stun_addr: SocketAddr,
    tick_interval: std::time::Duration,
    check_per_tick: NonZeroUsize,
    mut socket_addr: F,
) -> Result<(), crate::error::Error> {
    let mut interval = tokio::time::interval(tick_interval);
    let mut cnt = 0;
    loop {
        cnt += 1;
        if cnt == check_per_tick.get() {
            cnt = 0;
            let socket_pub = crate::stun::udp_stun(socket_st).await?;
            socket_addr(socket_pub);
        } else {
            socket_ka.send_to(b"nya", stun_addr).await?;
        }
        interval.tick().await;
    }
}
