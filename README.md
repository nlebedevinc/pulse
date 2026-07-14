# pulse

> Validate a connection, watch it live, get a verdict.

`pulse` is a minimal terminal tool that answers the question *"is my connection
actually fine?"* — the way `tailscale netcheck` validates a path, with the live
feedback of `ping` and a graph you can read at a glance. When you stop it, you
get a clean summary with an honest verdict.

```
  pulse  google.com (142.250.217.238) · icmp · 1s

           ▂▂▁▂▂▃▂▂▂▂▂▂█▂▂▁▂▂▂▂×▂▂▂▂▃▂▂▂▂▂▂▂▂
  last 23.4ms   min 18.2ms   avg 24.1ms   max 89.3ms   jitter 3.2ms   loss 2%

  ✓ dns     29ms  142.250.217.238, 2607:f8b0:4006:816::200e
  ✓ tcp     13ms  :443
  ✓ tls     47ms  TLS 1.3 · cert 62d
  ✓ http    79ms  301 · ttfb 79ms

  q quit · 34 probes · 34s
```

On exit:

```
pulse summary — google.com (142.250.217.238)
  34 probes over 34s via icmp · 1 lost (3%)
  rtt min/avg/max 18.2ms/24.1ms/89.3ms · p95 31.2ms · jitter 3.2ms
  verdict: degraded — intermittent packet loss
```

## Why not just ping?

`ping` tells you a host replies. `pulse` tells you whether the *path* works:

- **Layered checks up front** — DNS resolution, TCP connect, TLS handshake
  (protocol + certificate expiry) and HTTP time-to-first-byte, each timed
  individually. When something is broken, you see *which layer*.
- **A live RTT graph** — latency spikes and timeouts (`×`) are visible as they
  happen, not buried in scrollback.
- **Jitter and percentiles** — `p95` and jitter matter more than average
  latency for calls, gaming and SSH. `ping` gives you neither.
- **A verdict** — the run ends with `excellent / good / degraded / poor` and a
  reason, graded primarily on loss and jitter (what actually hurts interactive
  traffic), then latency.

## Install

### go install

```sh
go install github.com/nlebedevinc/pulse@latest
```

### From source

```sh
git clone https://github.com/nlebedevinc/pulse.git
cd pulse
make install
```

Requires Go 1.25+.

## Usage

```sh
pulse google.com            # probe with icmp, checks against :443
pulse -c 60 1.1.1.1         # stop automatically after 60 probes
pulse -i 200ms 10.0.0.1     # probe 5× per second
pulse --tcp -p 22 my-server # probe a specific tcp port instead of icmp
```

| Flag | Default | Description |
|------|---------|-------------|
| `-c` | `0` | Stop after this many probes (`0` = until interrupted) |
| `-i` | `1s` | Time between probes |
| `-t` | `2s` | Per-probe timeout |
| `-p` | `443` | Port used for TCP probes and the connect/TLS/HTTP checks |
| `--tcp` | `false` | Probe with TCP connects instead of ICMP |
| `--version` | | Print version and exit |

Quit with `q`, `esc` or `ctrl+c` — the summary prints either way.

## How it works

1. **Checks** — on startup, pulse resolves the host (preferring IPv4), opens a
   TCP connection, performs a TLS handshake and times an HTTP request to first
   byte. Each step is reported independently; later steps are skipped when an
   earlier one fails. TLS/HTTP checks only run when the port is 443.
2. **Probes** — one ICMP echo per interval against the resolved address, each
   individually timed out, so packet loss is accounted for precisely.
3. **Verdict** — loss > 5% is `poor`, any loss or jitter > 50ms is `degraded`;
   an otherwise clean run is graded `excellent` or `good` by latency.

### ICMP permissions

pulse uses unprivileged ICMP datagram sockets — no root needed on **macOS**.
On **Linux**, allow them once with:

```sh
sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"
```

If ICMP is unavailable, pulse automatically falls back to TCP probing and says
so in the footer. You can also force it with `--tcp`.

## Development

```sh
make build   # build ./pulse
make test    # run tests
make vet     # static analysis
```

The TUI is a hand-rolled ANSI renderer — the only dependencies are
[pro-bing](https://github.com/prometheus-community/pro-bing) for ICMP and
`golang.org/x/term` for raw terminal mode, keeping the stripped binary
around 5 MB.

## License

[MIT](LICENSE)
