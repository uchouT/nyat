# nyat

NAT traversal toolkit — a Rust reimplementation of [natmap](https://github.com/heiher/natmap) / [natter](https://github.com/MikeWang000000/Natter).

Discovers and maintains public socket addresses via STUN, keeping NAT mappings alive over TCP or UDP.

## Crates

| Crate | Description |
|-------|-------------|
| [nyat-core](nyat-core/) | Library — builder API for NAT mapping sessions |

## Building

```sh
cargo build --release
```

## License

MIT
