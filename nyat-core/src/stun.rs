//! Minimal STUN client (RFC 5389).
//!
//! Only implements Binding Request and parsing of
//! MAPPED-ADDRESS / XOR-MAPPED-ADDRESS from responses.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use tokio::time::timeout;

#[cfg(feature = "tcp")]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
#[cfg(feature = "udp")]
use tokio::net::{ToSocketAddrs, UdpSocket};

use crate::error::StunError;

const TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(30);

const MAGIC_COOKIE: u32 = 0x2112_A442;
const HEADER_SIZE: usize = 20;
const MAX_BODY_SIZE: usize = 2048;

const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
const FAMILY_IPV4: u8 = 0x01;
const FAMILY_IPV6: u8 = 0x02;

fn random_tx_id() -> [u8; 12] {
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 12];
    for chunk in bytes.chunks_exact_mut(4) {
        let hash = std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish();
        chunk.copy_from_slice(&hash.to_ne_bytes()[..4]);
    }
    bytes
}

fn build_request() -> ([u8; HEADER_SIZE], [u8; 12]) {
    let tx_id = random_tx_id();
    let mut buf = [0u8; HEADER_SIZE];
    buf[0..2].copy_from_slice(&0x0001u16.to_be_bytes()); // Binding Request
    // buf[2..4] = 0 — message length = 0 (no attributes)
    buf[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    buf[8..20].copy_from_slice(&tx_id);
    (buf, tx_id)
}

fn parse_response(data: &[u8], tx_id: &[u8; 12]) -> Result<SocketAddr, StunError> {
    if data.len() < HEADER_SIZE {
        return Err(StunError::Malformed);
    }
    if data[8..20] != *tx_id {
        return Err(StunError::TransactionIdMismatch);
    }

    let body_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let body = data
        .get(HEADER_SIZE..HEADER_SIZE + body_len)
        .ok_or(StunError::Malformed)?;

    let mut offset = 0;
    while offset + 4 <= body.len() {
        let attr_type = u16::from_be_bytes([body[offset], body[offset + 1]]);
        let attr_len = u16::from_be_bytes([body[offset + 2], body[offset + 3]]) as usize;
        let value = body
            .get(offset + 4..offset + 4 + attr_len)
            .ok_or(StunError::Malformed)?;

        match attr_type {
            ATTR_XOR_MAPPED_ADDRESS => return parse_xor_mapped(value, tx_id),
            ATTR_MAPPED_ADDRESS => return parse_mapped(value),
            _ => {}
        }

        // attributes padded to 4-byte boundary
        offset += 4 + ((attr_len + 3) & !3);
    }

    Err(StunError::Malformed)
}

fn parse_xor_mapped(value: &[u8], tx_id: &[u8; 12]) -> Result<SocketAddr, StunError> {
    if value.len() < 8 {
        return Err(StunError::Malformed);
    }
    let family = value[1];
    let port = u16::from_be_bytes([value[2], value[3]]) ^ (MAGIC_COOKIE >> 16) as u16;

    match family {
        FAMILY_IPV4 => {
            let mut b = [0u8; 4];
            b.copy_from_slice(&value[4..8]);
            let cookie = MAGIC_COOKIE.to_be_bytes();
            for (a, m) in b.iter_mut().zip(&cookie) {
                *a ^= m;
            }
            Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(b)), port))
        }
        FAMILY_IPV6 if value.len() >= 20 => {
            let mut b = [0u8; 16];
            b.copy_from_slice(&value[4..20]);
            let mut key = [0u8; 16];
            key[..4].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
            key[4..].copy_from_slice(tx_id);
            for (a, k) in b.iter_mut().zip(&key) {
                *a ^= k;
            }
            Ok(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(b)), port))
        }
        _ => Err(StunError::Malformed),
    }
}

fn parse_mapped(value: &[u8]) -> Result<SocketAddr, StunError> {
    if value.len() < 8 {
        return Err(StunError::Malformed);
    }
    let family = value[1];
    let port = u16::from_be_bytes([value[2], value[3]]);

    match family {
        FAMILY_IPV4 => Ok(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(value[4], value[5], value[6], value[7])),
            port,
        )),
        FAMILY_IPV6 if value.len() >= 20 => {
            let mut b = [0u8; 16];
            b.copy_from_slice(&value[4..20]);
            Ok(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(b)), port))
        }
        _ => Err(StunError::Malformed),
    }
}

#[cfg(feature = "tcp")]
/// Discover public address via STUN over an established TCP stream.
pub(crate) async fn tcp_socket_addr(mut stream: TcpStream) -> Result<SocketAddr, StunError> {
    let (request, tx_id) = build_request();

    let buf = timeout(TIMEOUT_DURATION, async {
        stream.write_all(&request).await?;

        let mut header = [0u8; HEADER_SIZE];
        stream.read_exact(&mut header).await?;

        let body_len = u16::from_be_bytes([header[2], header[3]]) as usize;
        if body_len > MAX_BODY_SIZE {
            return Err(StunError::ResponseTooLarge);
        }

        let mut buf = vec![0u8; HEADER_SIZE + body_len];
        buf[..HEADER_SIZE].copy_from_slice(&header);
        if body_len > 0 {
            stream.read_exact(&mut buf[HEADER_SIZE..]).await?;
        }
        Ok(buf)
    })
    .await
    .map_err(std::io::Error::from)??;

    parse_response(&buf, &tx_id)
}

/// Wrapper around a UDP socket that has been `connect()`ed to a STUN server.
#[derive(Clone, Copy)]
#[cfg(feature = "udp")]
pub(crate) struct StunUdpSocket<'a> {
    pub inner: &'a UdpSocket,
}

#[cfg(feature = "udp")]
impl<'a> StunUdpSocket<'a> {
    pub(crate) async fn new<A: ToSocketAddrs>(
        socket: &'a UdpSocket,
        stun_addr: A,
    ) -> Result<Self, std::io::Error> {
        socket.connect(stun_addr).await?;
        Ok(Self { inner: socket })
    }
}

#[cfg(feature = "udp")]
/// Discover public address via STUN over a connected UDP socket.
pub(crate) async fn udp_socket_addr(socket: StunUdpSocket<'_>) -> Result<SocketAddr, StunError> {
    let socket = socket.inner;
    let (request, tx_id) = build_request();
    let mut buf = [0u8; HEADER_SIZE + MAX_BODY_SIZE];

    socket.send(&request).await?;

    let len = timeout(TIMEOUT_DURATION, socket.recv(&mut buf))
        .await
        .map_err(std::io::Error::from)??;

    if len < HEADER_SIZE {
        return Err(StunError::Malformed);
    }

    parse_response(&buf[..len], &tx_id)
}
