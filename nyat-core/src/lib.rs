//! NAT traversal library — a Rust reimplementation of [natmap](https://github.com/heiher/natmap).
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
//! let stun = RemoteAddr::from_host("stun.l.google.com", 3478, None);
//! let remote = RemoteAddr::from_addr("1.2.3.4:80".parse().unwrap());
//!
//! let mapper = MapperBuilder::new(local, stun)
//!     .interval(Duration::from_secs(10))
//!     .tcp_remote(remote)
//!     .build_tcp();
//!
//! mapper.run(&mut |addr| {
//!     println!("public address: {addr}");
//! }).await?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod mapper;
pub mod net;
mod stun;
