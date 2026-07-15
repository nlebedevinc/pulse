# pulse

**Validate a connection, watch it live, get a verdict.**

`pulse` is a minimal terminal tool for answering one question: *is my
connection actually fine?* It validates every layer of the path to a host —
DNS, TCP, TLS, HTTP — then probes continuously with a live latency graph, and
ends with an honest, evidence-based verdict.

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

## Contents

- [Features](#features)
- [Why pulse?](#why-pulse)
- [Installation](#installation)
- [Usage](#usage)
- [How it works](#how-it-works)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)

## Features

- **Layered connection checks** — DNS resolution, TCP connect, TLS handshake
  (protocol version and certificate expiry) and HTTP time-to-first-byte, each
  timed individually. When something is broken, pulse shows *which layer*.
- **Live RTT graph** — a compact in-terminal chart where latency spikes and
  timeouts (`×`) are visible the moment they happen.
- **Statistics that matter** — jitter and p95 alongside min/avg/max, because
  tail latency is what actually hurts calls, gaming and SSH sessions.
- **Precise loss accounting** — every probe is individually timed out, so
  packet loss is exact, not inferred.
- **A verdict** — every run ends with `excellent / good / degraded / poor`
  and the reason, so the result can go straight into a bug report.
- **Minimal by design** — a single static binary around 5 MB, two direct
  dependencies, no configuration files, and plain ANSI colors that inherit
  your terminal theme.

## Why pulse?

`ping` proves a host replies. `pulse` proves the *path* works — name
resolution, the transport handshake, the TLS session and the first HTTP byte
— and quantifies how well, over time, with the numbers that predict real
application behavior.

| | `ping` | `pulse` |
|---|:---:|:---:|
| Reachability | ✓ | ✓ |
| DNS / TCP / TLS / HTTP timing | — | ✓ |
| Live latency graph | — | ✓ |
| Jitter and percentiles | — | ✓ |
| Per-probe timeout and loss accounting | — | ✓ |
| End-of-run quality verdict | — | ✓ |

### When plain `ping` is the better tool

In the spirit of honesty: `ping` is preinstalled everywhere, weighs a few
hundred kilobytes, and is the right choice for scripting, flood testing, or
environments where installing binaries isn't an option. `pulse` is for the
interactive moments — debugging a flaky call, validating a new network,
proving to your ISP that the problem is real.

## Installation

### Go

```sh
go install github.com/nlebedevinc/pulse@latest
```

### From source

```sh
git clone https://github.com/nlebedevinc/pulse.git
cd pulse
make install
```

Requires Go 1.25 or later. `make install` produces a smaller binary than
plain `go install` (symbols stripped, inlining disabled).

### Pre-built binaries

Download the archive for your platform from the
[releases page](https://github.com/nlebedevinc/pulse/releases), extract it,
and place `pulse` on your `PATH`.

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

1. **Checks.** On startup, pulse resolves the host (preferring IPv4), opens a
   TCP connection, performs a TLS handshake and times an HTTP request to
   first byte. Each step is reported independently; later steps are skipped
   when an earlier one fails. TLS and HTTP checks only run when the port
   is 443.
2. **Probes.** One ICMP echo per interval against the resolved address, each
   with its own timeout, so loss is accounted for exactly. If ICMP sockets
   are unavailable, pulse falls back to TCP probing automatically and says so
   in the footer.
3. **Verdict.** Loss and jitter dominate the grade because they hurt
   interactive traffic more than raw latency does: loss above 5% is `poor`;
   any loss, jitter above 50ms or average latency above 300ms is `degraded`;
   an otherwise clean run is graded `excellent` or `good` by latency.

## Troubleshooting

**ICMP permission errors on Linux.** pulse uses unprivileged ICMP datagram
sockets — no root required on macOS. On Linux, allow them once with:

```sh
sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"
```

Or skip ICMP entirely with `--tcp`.

**"needs an interactive terminal".** pulse is an interactive tool and
requires a TTY; it does not support being piped or run from cron. For
scriptable output, run with `-c` and capture the summary it prints on exit.

**The HTTP check shows `301`.** That's the site redirecting `/` — the check
reports the first response without following redirects, which is exactly the
timing you care about.

## Contributing

Contributions are welcome. Before opening a pull request:

```sh
make test   # unit tests
make vet    # static analysis
make build  # release-equivalent build
```

Please keep the tool's scope minimal — new flags and features should earn
their place.

## License

[MIT](LICENSE)
