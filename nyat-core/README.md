# nyat-core

NAT traversal library — discover and maintain public addresses via STUN.

The library does one thing: it runs a loop that keeps a NAT mapping alive and
calls you back when the public address changes. No policy, no I/O beyond the
mapping itself. You decide what to do with the address.

## Usage

```toml
[dependencies]
nyat-core = "0.2"
```

### TCP

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

    mapper.run(&mut |info: nyat_core::mapper::MappingInfo| {
        println!("{} {}", info.pub_addr, info.local_addr);
    }).await
}
```

### UDP

```rust,no_run
use nyat_core::net::{LocalAddr, RemoteAddr};
use nyat_core::mapper::MapperBuilder;
use std::num::NonZeroUsize;

#[tokio::main]
async fn main() -> Result<(), nyat_core::Error> {
    let local = LocalAddr::new("0.0.0.0:0".parse().unwrap());
    let stun = RemoteAddr::from_host("stun.l.google.com", 19302, None);

    let mapper = MapperBuilder::new_udp(local, stun)
        .check_per_tick(NonZeroUsize::new(3).unwrap())
        .build();

    mapper.run(&mut |info: nyat_core::mapper::MappingInfo| {
        println!("{} {}", info.pub_addr, info.local_addr);
    }).await
}
```

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `tcp` | yes | TCP keepalive + STUN mapping |
| `udp` | yes | UDP STUN mapping |
| `reuse_port` | no | **Dangerous.** Force `SO_REUSEPORT` on sockets owned by other processes via `pidfd_getfd(2)`. Linux 5.6+, requires root or `CAP_SYS_PTRACE`. Last resort only. |

## Architecture

```
MapperBuilder::new_tcp / new_udp
    → .interval()  .check_per_tick()
    → .build()
    → TcpMapper / UdpMapper
        → .run(&mut handler)    // async loop
            → MappingHandler::on_change(MappingInfo)
```

`MappingHandler` is auto-implemented for `FnMut(MappingInfo)`, so a closure
works out of the box.

## License

MIT
