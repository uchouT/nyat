use crate::error::StunError;
use smallvec::SmallVec;
use std::net::SocketAddr;
use stun::{
    agent::TransactionId,
    message::{BINDING_REQUEST, Getter, Message},
    xoraddr::XorMappedAddress,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs, UdpSocket},
    time::timeout,
};

fn create_binding_req() -> (impl AsRef<[u8]>, TransactionId) {
    let mut msg = Message::new();
    let tx_id = TransactionId::new();
    msg.build(&[Box::new(BINDING_REQUEST), Box::new(tx_id)])
        .unwrap();
    (msg.marshal_binary().unwrap(), msg.transaction_id)
}

fn parse_pub_socket_addr(data: &[u8], tx_id: TransactionId) -> Result<SocketAddr, StunError> {
    let mut msg = Message::new();
    msg.unmarshal_binary(data)?;
    if msg.transaction_id != tx_id {
        return Err(StunError::StunTransactionIdMismatch);
    }
    let mut xor_addr = XorMappedAddress::default();
    xor_addr.get_from(&msg)?;
    Ok(SocketAddr::new(xor_addr.ip, xor_addr.port))
}

/// get public socket address from stun server tcp stream
pub(crate) async fn tcp_socket_addr(mut stream: TcpStream) -> Result<SocketAddr, StunError> {
    let (msg, tx_id) = create_binding_req();
    let buf = timeout(crate::TIMEOUT_DURATION, async {
        stream.write_all(msg.as_ref()).await?;

        let mut buf: SmallVec<[u8; crate::BUF_SIZE]> = smallvec::SmallVec::new();
        buf.resize(20, 0);
        stream.read_exact(&mut buf).await?;

        let total_len = 20 + u16::from_be_bytes([buf[2], buf[3]]) as usize;
        if total_len > crate::BUF_SIZE {
            return Err(StunError::StunResponseTooLarge);
        }
        if buf.len() < total_len {
            buf.resize(total_len, 0);
        }

        stream.read_exact(&mut buf[20..total_len]).await?;
        Ok::<_, StunError>(buf)
    })
    .await
    .map_err(std::io::Error::from)??;

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
    let socket = socket.inner;
    let (msg, tx_id) = create_binding_req();
    let mut buf = [0u8; crate::BUF_SIZE];
    socket.send(msg.as_ref()).await?;

    let len = timeout(crate::TIMEOUT_DURATION, socket.recv(&mut buf))
        .await
        .map_err(std::io::Error::from)??;

    if len > 0 {
        parse_pub_socket_addr(&buf[..len], tx_id)
    } else {
        Err(StunError::StunNetwork(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "empty STUN response",
        )))
    }
}

#[cfg(test)]
mod test {

    // #[tokio::test]
    // async fn test_udp_stun() {
    //     todo!()
    // }
}
