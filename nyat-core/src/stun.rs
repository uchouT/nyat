use std::net::SocketAddr;

use smallvec::SmallVec;
use stun::{
    agent::TransactionId,
    message::{BINDING_REQUEST, Getter, Message},
    xoraddr::XorMappedAddress,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs, UdpSocket},
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StunError {
    #[error("STUN protocol error")]
    Protocol(
        #[source]
        #[from]
        stun::Error,
    ),

    #[error("STUN network I/O error")]
    Network(
        #[source]
        #[from]
        std::io::Error,
    ),

    #[error("STUN transaction ID mismatch")]
    TransactionIdMismatch,
}

fn create_binding_req() -> Result<(impl AsRef<[u8]>, TransactionId), StunError> {
    let mut msg = Message::new();
    let tx_id = TransactionId::new();
    msg.build(&[Box::new(BINDING_REQUEST), Box::new(tx_id)])?;
    Ok((msg.marshal_binary()?, msg.transaction_id))
}

fn parse_pub_socket_addr(data: &[u8], tx_id: TransactionId) -> Result<SocketAddr, StunError> {
    let mut msg = Message::new();
    msg.unmarshal_binary(data)?;
    if msg.transaction_id != tx_id {
        return Err(StunError::TransactionIdMismatch);
    }
    let mut xor_addr = XorMappedAddress::default();
    xor_addr.get_from(&msg)?;
    Ok(SocketAddr::new(xor_addr.ip, xor_addr.port))
}

const BUF_SIZE: usize = 1024;

/// get public socket address from stun server tcp stream
pub(crate) async fn tcp_socket_addr(mut stream: TcpStream) -> Result<SocketAddr, StunError> {
    let (msg, tx_id) = create_binding_req()?;
    stream.write_all(msg.as_ref()).await?;

    let mut buf: SmallVec<[u8; BUF_SIZE]> = smallvec::SmallVec::new();
    buf.resize(20, 0);
    stream.read_exact(&mut buf).await?;

    let total_len = 20 + u16::from_be_bytes([buf[2], buf[3]]) as usize;
    if buf.len() < total_len {
        buf.resize(total_len, 0);
    }

    stream.read_exact(&mut buf[20..total_len]).await?;
    parse_pub_socket_addr(&buf, tx_id)
}

/// Udp socket that has connected to a stun server
#[derive(Clone, Copy)]
pub(crate) struct StunUdpSocket<'a> {
    pub inner: &'a UdpSocket,
}

impl<'a> StunUdpSocket<'a> {
    pub(crate) async fn new<A: ToSocketAddrs>(
        udpsocket: &'a UdpSocket,
        stun_addr: A,
    ) -> Result<Self, std::io::Error> {
        udpsocket.connect(stun_addr).await?;
        Ok(Self { inner: udpsocket })
    }
}
/// get public socket address from stun server udp socket
pub(crate) async fn udp_socket_addr(socket: StunUdpSocket<'_>) -> Result<SocketAddr, StunError> {
    // TODO: error handling
    let socket = socket.inner;
    let (msg, tx_id) = create_binding_req()?;
    let mut buf = [0u8; 1024];
    socket.send(msg.as_ref()).await?;
    let len = socket.recv(&mut buf).await?;
    if len > 0 {
        parse_pub_socket_addr(&buf[..len], tx_id)
    } else {
        todo!("retry")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn test_binding_msg() {
        let res = create_binding_req();
        assert!(res.is_ok());
        let (_, id) = res.unwrap();
    }

    // #[tokio::test]
    // async fn test_udp_stun() {
    //     todo!()
    // }
}
