# nyat-core

NAT traversal library — a Rust reimplementation of [natmap](https://github.com/heiher/natmap).

Discovers and maintains public socket addresses via STUN, keeping NAT mappings alive over TCP or UDP.

## Usage

```toml
[dependencies]
nyat-core = "0.1"
```

```rust,no_run
use nyat_core::net::{LocalAddr, RemoteAddr};
use nyat_core::mapper::MapperBuilder;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), nyat_core::Error> {
    let local = LocalAddr::new("0.0.0.0:4070".parse().unwrap());
    let stun = RemoteAddr::from_host("turn.cloudflare.com", 3478, None);
    let remote = RemoteAddr::from_host("example.com", 80, None);

    let mapper = MapperBuilder::new_tcp(local, stun, remote)
        .interval(Duration::from_secs(10))
        .build();

    mapper.run(&mut |addr| {
        println!("public address: {addr}");
    }).await
}
```

## License

MIT
