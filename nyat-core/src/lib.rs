//! NAT traversal library.
//!
//! Discovers and maintains public socket addresses via STUN, keeping NAT
//! mappings alive over TCP or UDP.
//!
//! # Quick start
//!
//! ```no_run
//! use nyat_core::net::{LocalAddr, RemoteAddr};
//! use nyat_core::mapper::MapperBuilder;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), nyat_core::Error> {
//! let local = LocalAddr::new("0.0.0.0:4070".parse().unwrap());
//! let stun = RemoteAddr::from_host("turn.cloudflare.com", 3478, None);
//! let keepalive_remote = RemoteAddr::from_host("example.com", 80, None);
//!
//! let mapper = MapperBuilder::new_tcp(local, stun, keepalive_remote)
//!     .interval(Duration::from_secs(10))
//!     .build();
//!
//! mapper.run(&mut |info: nyat_core::mapper::MappingInfo| {
//!     println!("{} {}", info.pub_addr, info.local_addr);
//! }).await?;
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(not(any(feature = "tcp", feature = "udp")))]
compile_error!("at least one of the `tcp` or `udp` features must be enabled");

mod error;
pub mod mapper;
pub mod net;
mod stun;

pub use error::Error;
