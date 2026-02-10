use std::{net::SocketAddr, num::NonZeroUsize, time::Duration};

use tokio::net::UdpSocket;

use crate::{
	addr::{Local, RemoteAddr},
	error::Error,
	stun::StunUdpSocket,
};

pub struct UdpMapper {
	stun: RemoteAddr,
	local: Local,
	interval: Duration,
	sender: tokio::sync::watch::Sender<SocketAddr>,
	check_per_tick: NonZeroUsize,
}

impl UdpMapper {
	pub async fn run(&self) -> Result<(), Error> {
		let socket_st = self.local.udp_socket().map_err(Error::Socket)?;
		let socket_ka = self.local.udp_socket().map_err(Error::Socket)?;
		let mut current_ip = None;
		// handler to update public address
		let mut handler = |s| {
			if Some(s) != current_ip {
				current_ip = Some(s);
				let _ = self.sender.send(s);
			}
		};
		loop {
			let stun_addr = if matches!(self.stun, RemoteAddr::Resolved(_)) {
				self.stun.socket_addr_resolved()
			} else {
				self.stun.socket_addr().await?
			};

			let socket_st = StunUdpSocket::new(&socket_st, stun_addr)
				.await
				.map_err(Error::Connection)?;
			self.keepalive(socket_st, &socket_ka, stun_addr, &mut handler)
				.await;
		}
	}

	async fn keepalive<F: FnMut(SocketAddr)>(
		&self,
		socket_st: StunUdpSocket<'_>,
		socket_ka: &UdpSocket,
		stun_addr: SocketAddr,
		mut socket_addr: F,
	) -> Result<(), Error> {
		let mut interval = tokio::time::interval(self.interval);
		let mut cnt = 0;
		loop {
			cnt += 1;
			if cnt == self.check_per_tick.get() {
				cnt = 0;
				let socket_pub = crate::stun::udp_socket_addr(socket_st).await?;
				socket_addr(socket_pub);
			} else {
				socket_ka
					.send_to(b"nya", stun_addr)
					.await
					.map_err(Error::Keepalive)?;
			}
			interval.tick().await;
		}
	}
}
