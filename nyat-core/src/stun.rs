use std::net::SocketAddr;

use smallvec::SmallVec;
use stun::{
    agent::TransactionId,
    message::{BINDING_REQUEST, Getter, Message},
    xoraddr::XorMappedAddress,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};

use crate::{
    addr::RemoteAddr,
    error::{Error, StunError},
};

fn create_binding_req() -> Result<(impl AsRef<[u8]>, TransactionId), crate::error::StunError> {
    let mut msg = Message::new();
    let tx_id = TransactionId::new();
    msg.build(&[Box::new(BINDING_REQUEST), Box::new(tx_id)])?;
    Ok((msg.marshal_binary()?, msg.transaction_id))
}

fn parse_pub_socket_addr(data: &[u8], tx_id: TransactionId) -> Result<SocketAddr, StunError> {
    let mut msg = Message::new();
    msg.unmarshal_binary(data)?;
    if msg.transaction_id != tx_id {
        return Err(StunError::TnsactionIdMissMatch);
    }
    let mut xor_addr = XorMappedAddress::default();
    xor_addr.get_from(&msg)?;
    Ok(SocketAddr::new(xor_addr.ip, xor_addr.port))
}

const BUF_SIZE: usize = 1024;

pub(crate) async fn tcp_stun(mut stream: TcpStream) -> Result<SocketAddr, StunError> {
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

/// socket must be connected to the stun socket addr
pub(crate) async fn udp_stun(socket: &UdpSocket) -> Result<SocketAddr, StunError> {
    // TODO: error handling
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
