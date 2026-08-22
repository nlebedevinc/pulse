# pulse

[![ci](https://github.com/nlebedevinc/pulse/actions/workflows/ci.yml/badge.svg)](https://github.com/nlebedevinc/pulse/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Validate a connection, watch it live, get a verdict.**

`pulse` answers one question: *is my connection actually fine?* It checks every
layer of the path to a host — DNS, TCP, TLS, HTTP — then probes continuously
with a live latency graph, and ends with an honest, evidence-based verdict.

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

On exit it prints a summary you can paste straight into a bug report:

```
pulse summary — google.com (142.250.217.238)
  34 probes over 34s via icmp · 1 lost (3%)
  rtt min/avg/max 18.2ms/24.1ms/89.3ms · p95 31.2ms · jitter 3.2ms
  verdict: degraded — intermittent packet loss
```

## Why pulse

`ping` proves a host replies. `pulse` proves the *path* works — and tells you
which part of it doesn't.

- **It shows which layer broke.** "DNS resolved but the TLS handshake failed"
  is a different problem from "host unreachable", and it's the first thing
  worth knowing during an outage.
- **It measures what actually hurts.** Jitter and p95, not just an average.
  Tail latency and variance are what wreck calls, gaming and SSH; a mean
  figure hides both.
- **It reaches a conclusion.** Every run ends in `excellent / good / degraded
  / poor` with a reason — so the result means something to someone who doesn't
  read latency graphs for a living.

|                               | `ping` | `pulse` |
|-------------------------------|:------:|:-------:|
| DNS / TCP / TLS / HTTP timing  |   —    |    ✓    |
| Live latency graph             |   —    |    ✓    |
| Jitter and percentiles         |   —    |    ✓    |
| End-of-run quality verdict     |   —    |    ✓    |

Reach for it when a call keeps dropping, when you're sizing up unfamiliar
Wi-Fi, or when you need to show an ISP the problem is real. For scripting,
flood testing, or anywhere you can't install a binary, plain `ping` remains
the better tool.

## Install

Pre-built binaries for macOS and Linux are on the
[releases page](https://github.com/nlebedevinc/pulse/releases), or build from
source with Rust 1.75 or later:

```sh
cargo install --git https://github.com/nlebedevinc/pulse
```

A single 1.1 MB binary — no runtime, no config files, nothing to set up.

## Usage

```sh
pulse google.com            # icmp probes, checks against :443
pulse -c 60 1.1.1.1         # stop automatically after 60 probes
pulse -i 200ms 10.0.0.1     # probe 5× per second
pulse --tcp -p 22 my-server # probe a tcp port instead of icmp
```

Quit with `q`, `esc` or `ctrl+c` — the summary prints either way. Run
`pulse --help` for the full flag list.

## Notes

Loss and jitter dominate the verdict because they hurt interactive traffic
more than raw latency does. TLS and HTTP checks run only when the port is 443.

Routers routinely deprioritise ICMP, so `ping`-style latency can read far
worse than the traffic you actually care about. If the numbers look
implausible, compare against `--tcp`.

pulse is interactive and needs a TTY. On Linux, unprivileged ICMP sockets may
need enabling once — or just use `--tcp`:

```sh
sudo sysctl -w net.ipv4.ping_group_range="0 2147483647"
```

## Contributing

Run `make test`, `make vet` and `make build` before opening a pull request.
Please keep the tool's scope minimal — new flags should earn their place.

## License

[MIT](LICENSE)
