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
//! # async fn example() -> Result<(), nyat_core::error::Error> {
//! let local = LocalAddr::new("0.0.0.0:4070".parse().unwrap());
//! let stun = RemoteAddr::from_host("turn.cloudflare.com", 3478, None);
//! let keepalive_remote = RemoteAddr::from_host("example.com", 80, None);
//!
//! let mapper = MapperBuilder::new(local, stun)
//!     .tcp_remote(keepalive_remote)
//!     .interval(Duration::from_secs(10))
//!     .build_tcp();
//!
//! mapper.run(&mut |addr| {
//!     println!("public address: {addr}");
//! }).await?;
//! # Ok(())
//! # }
//! ```

mod error;
pub mod mapper;
pub mod net;
mod stun;

pub use error::Error;