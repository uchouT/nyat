# nyat

> [!NOTE]
> This project is under active development. Planned features:
> - [x] Batch mode — run multiple mapping tasks from a `.toml` config file
> - [ ] Forward mode — relay traffic to a local target service

NAT traversal toolkit — discover and maintain your public address via STUN.

A Rust reimplementation of [natmap](https://github.com/heiher/natmap) / [natter](https://github.com/MikeWang000000/Natter).

When every NAT layer is full cone (NAT-1), external hosts can reach you — but
only if you know the current mapping. nyat discovers that mapping and keeps it
alive.

## Crates

| Crate | Description |
|-------|-------------|
| [**nyat**](nyat/) | CLI binary |
| [**nyat-core**](nyat-core/) | Library — async builder API for NAT mapping sessions |

## Building

```sh
cargo build --release
```

## License

GPL-3.0-or-later
