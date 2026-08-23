A ~74× difference on the same destination. ICMP is being heavily deprioritized here. That single fact frames most of your question.

What pulse is genuinely good at

The sweet spot is the "prove it" moment: something feels wrong, support says everything's fine, and you need evidence plus a conclusion rather than raw numbers. Three things make it good at that:

- Layer localization. ping says "host replies." pulse says which layer broke — DNS resolved but TLS handshake failed, etc. That's the first genuinely useful question during an outage.
- The right statistics. Jitter and p95, not just min/avg/max. Tail latency and variance are what actually wreck calls, gaming and SSH; average latency hides both. This is the most under-appreciated design choice in the tool.
- A verdict. It converts data into a sentence a non-expert can act on or paste into a ticket.

RTT — yes, but the ICMP caveat is the real story

pulse is better than ping here, and the numbers above show why: ICMP RTT frequently does not reflect application RTT. Routers rate-limit and deprioritize ICMP. So ping can report a terrible connection while your actual traffic is fine.

pulse already has the cure and doesn't yet use it — --tcp mode measures something much closer to what applications experience, and the startup checks already time a TCP connect. So pulse is collecting both signals and comparing neither.

That's the most interesting opportunity I see: detect the discrepancy and say so. If ICMP reads 600ms while the TCP handshake reads 8ms, the honest verdict isn't "degraded" — it's "ICMP is deprioritized on this path; your applications are fine." No common tool tells you that, it fits the "evidence-based verdict" thesis exactly, and the data is already in hand. Today pulse would grade that path degraded and be wrong.

DNS — don't chase dig

I'd push back here. pulse does one getaddrinfo at startup and reports IPs plus timing. It can't show you which resolver answered, TTLs, the CNAME chain, or SERVFAIL-vs-NXDOMAIN — and it shouldn't. dig exists, it's excellent, and duplicating it means owning a DNS client, which is exactly the kind of scope the README warns against.

The one legitimate gap: pulse resolves once, so a long session can't see DNS flapping or CDN rotation. A periodic re-resolve would fit the "watch it live" premise. Still scope creep, but honest scope creep.

Traceroute — tempting, and I'd still say no

It's the natural follow-up ("degraded — where?"), and it's now technically cheap since we own unprivileged ICMP; traceroute is the same socket with varying TTL. But:

- mtr already does this well, and does it better than a bolt-on would.
- Traceroute is a famously misleading data source — for exactly the reason we just measured. Hops deprioritize ICMP, routing is asymmetric, and middle hops that look terrible are often perfectly healthy. A tool whose pitch is honest, evidence-based verdicts would be importing its least honest possible input.
- The UI is a different shape. A hop table isn't a latency graph; it's a second tool wearing the same binary.

Same reasoning I used against UDP: it isn't hard, it's dishonest by default.

Where I'd actually invest, once parity lands

1. ICMP-vs-TCP discrepancy detection — novel, nearly free, fixes a case where pulse is currently wrong
2. --json output — the "paste into a ticket" story is the core use case and it's plain text today
3. Periodic re-resolve — makes "watch it live" true for DNS as well

All of these are cheap, and none dilute the tool's identity — which is more than I can say for dig or traceroute.

Worth noting: the ~74× gap is partly this sandboxed environment, so I'd want to conf