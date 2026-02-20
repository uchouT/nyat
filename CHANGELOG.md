# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-02-21

### Added

- `nyat run` — single TCP/UDP mapping task with STUN discovery
- `nyat batch` — run multiple mapping tasks from a TOML config file
- Exec hook (`-e` / `exec`) — run a command on mapping change via environment variables
- IPv4/IPv6 preference for DNS resolution
- Linux: interface binding (`-i`), fwmark (`-f`), force `SO_REUSEPORT` (`--force-reuse`)
- `nyat-core` library crate with async builder API

[Unreleased]: https://github.com/uchouT/nyat/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/uchouT/nyat/releases/tag/v0.1.0
