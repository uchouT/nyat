use std::net::SocketAddr;

use stun::{
    agent::TransactionId,
    message::{BINDING_REQUEST, Message},
};
use tokio::net::{TcpStream, UdpSocket};

use crate::{
    addr::RemoteAddr,
    error::{Error, StunError},
};

fn create_binding_req() -> Result<Vec<u8>, crate::error::StunError> {
    let mut msg = Message::new();
    let tx_id = TransactionId::new();
    msg.build(&[Box::new(BINDING_REQUEST), Box::new(tx_id)])?;
    Ok(msg.marshal_binary()?)
}

fn parse_pub_socket_addr(data: &[u8]) -> Result<SocketAddr, StunError> {
    todo!()
}
// TODO: query the public ip port
pub(crate) async fn tcp_stun(stream: TcpStream) -> Result<SocketAddr, std::io::Error> {
    todo!()
}

/// socket must be connected to the stun socket addr
pub(crate) async fn udp_stun(socket: &UdpSocket) -> Result<SocketAddr, StunError> {
    // TODO: error handling
    let msg = create_binding_req()?;
    let mut buf = [0u8; 1024];
    socket.send(&msg).await?;
    let len = socket.recv(&mut buf).await?;
    if len > 0 {
        parse_pub_socket_addr(&buf[..len])
    } else {
        todo!("retry")
    }
}
