# nyat

NAT traversal CLI — discover and maintain your public address via STUN.

In `run` mode, nyat outputs one line to stdout each time the mapping changes:

```
<pub_ip> <pub_port> <local_ip> <local_port>
```

Machine-readable, pipe-friendly. Compose with the tools you already have.

## Installation

```sh
cargo install nyat
```

## Quick start

### TCP mode

```sh
nyat run tcp -s turn.cloudflare.com -r example.com
```

### UDP mode

```sh
nyat run udp -s turn.cloudflare.com
```

### Batch mode

```sh
nyat batch -c /path/to/config.toml
```

## Subcommands

### `nyat run` — single mapping task

```
nyat run [OPTIONS] --stun <STUN> <MODE>
```

| Argument | Description |
|----------|-------------|
| `<MODE>` | `tcp` or `udp` |

#### Common options

| Flag | Description |
|------|-------------|
| `-s, --stun <STUN>` | STUN server (`addr[:port]`, default port 3478) |
| `-b, --bind <BIND>` | Local bind address (`[addr:]port`, default `0`) |
| `-k, --keepalive <SECS>` | Keepalive interval (TCP default 30s, UDP default 5s) |
| `-e, --exec <CMD>` | Command to run on mapping change (see [Exec hook](#exec-hook)) |
| `-4, --ipv4` | Prefer IPv4 for DNS resolution |
| `-6, --ipv6` | Prefer IPv6 for DNS resolution |

#### TCP-only

| Flag | Description |
|------|-------------|
| `-r, --remote <REMOTE>` | HTTP server for keepalive (`addr[:port]`, default port 80). **Required.** |

#### UDP-only

| Flag | Description |
|------|-------------|
| `-c, --count <N>` | STUN probe every N keepalive intervals (default 5) |

#### Linux-only

| Flag | Description |
|------|-------------|
| `-i, --iface <IFACE>` | Bind to a specific network interface |
| `-f, --fwmark <MARK>` | Set firewall mark for policy routing |
| `--force-reuse` | **Dangerous.** Force `SO_REUSEPORT` on existing sockets (see warning below) |

> [!WARNING]
> `--force-reuse` uses `pidfd_getfd(2)` to duplicate sockets from
> other processes and mutate their options. This is invasive — it modifies state
> that belongs to other programs without their knowledge. Requires root or
> `CAP_SYS_PTRACE`, Linux 5.6+. Use only as a last resort when the target port
> is held by a service that did not set `SO_REUSEPORT` itself.

### `nyat batch` — multiple mapping tasks

> [!NOTE]
> This subcommand is under development.

Run multiple mapping tasks defined in a TOML config file:

```
nyat batch -c config.toml
```

See [`nyat.toml`](nyat.toml) for a detailed config example.

## Exec hook

When `-e` (or `exec` in batch config) is set, nyat runs the command via
`sh -c` each time the mapping changes. The following environment variables are
available to the command:

| Variable | Description |
|----------|-------------|
| `NYAT_PUB_ADDR` | Public IP address |
| `NYAT_PUB_PORT` | Public port |
| `NYAT_LOCAL_ADDR` | Local IP address |
| `NYAT_LOCAL_PORT` | Local port |

The command's stdin and stdout are redirected to `/dev/null`; stderr is
inherited.

## Examples

```sh
# Bind to a specific port
nyat run tcp -s stun.l.google.com -r example.com -b 4070

# UDP, prefer IPv4, probe every 3 keepalive intervals
nyat run udp -s stun.l.google.com -4 -c 3

# Bind to interface (Linux)
nyat run tcp -s stun.l.google.com -r example.com -i eth0

# Run a script on mapping change
nyat run udp -s stun.l.google.com -e './on-change.sh $NYAT_PUB_ADDR $NYAT_PUB_PORT'

# Pipe to a script — each line has: pub_ip pub_port local_ip local_port
nyat run udp -s stun.l.google.com \
  | while read pub_ip pub_port local_ip local_port; do
      ./on-change.sh "$pub_ip" "$pub_port"
    done
```

## License

GPL-3.0-or-later
