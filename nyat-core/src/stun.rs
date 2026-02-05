use std::net::SocketAddr;

use stun::{
    agent::TransactionId,
    message::{BINDING_REQUEST, Message},
};
use tokio::net::TcpStream;

use crate::{addr::RemoteAddr, error::Error};

fn create_binding_req() -> Result<Vec<u8>, crate::error::StunError> {
    let mut msg = Message::new();
    let tx_id = TransactionId::new();
    msg.build(&[Box::new(BINDING_REQUEST), Box::new(tx_id)])?;
    Ok(msg.marshal_binary()?)
}

// TODO: query the public ip port
pub(crate) async fn tcp_stun(stream: TcpStream) -> Result<SocketAddr, std::io::Error> {
    todo!()
}
