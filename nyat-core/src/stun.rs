use std::net::SocketAddr;

use stun::{
    agent::TransactionId,
    message::{BINDING_REQUEST, Message},
};
use tokio::net::TcpStream;

use crate::{
    addr::RemoteAddr,
    error::{Error, StunError},
};

fn create_binding_req() -> Result<Vec<u8>, StunError> {
    let mut msg = Message::new();
    let tx_id = TransactionId::new();
    msg.build(&[Box::new(BINDING_REQUEST), Box::new(tx_id)])?;
    Ok(msg.marshal_binary()?)
}

// TODO: query the public ip port and do the following action
pub(crate) async fn stun_action_tcp<F>(stream: TcpStream, handler: F) -> Result<(), std::io::Error>
where
    F: FnOnce(SocketAddr),
{
    todo!()
}
