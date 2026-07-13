// Package probe sends single network probes and reports round-trip times.
package probe

import (
	"context"
	"time"
)

// Result is the outcome of a single probe.
type Result struct {
	Seq int
	RTT time.Duration
	Err error
	At  time.Time
}

// Lost reports whether the probe got no reply.
func (r Result) Lost() bool { return r.Err != nil }

// Prober sends one probe per call.
type Prober interface {
	Probe(ctx context.Context, seq int) Result
	// Kind describes the probe method, e.g. "icmp" or "tcp :443".
	Kind() string
}

// Run probes at the given interval until ctx is cancelled or count probes
// have been sent (count <= 0 means unlimited), sending each result to out.
// It closes out when done.
func Run(ctx context.Context, p Prober, interval time.Duration, count int, out chan<- Result) {
	defer close(out)
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for seq := 0; ; seq++ {
		if count > 0 && seq >= count {
			return
		}
		res := p.Probe(ctx, seq)
		select {
		case out <- res:
		case <-ctx.Done():
			return
		}
		select {
		case <-ticker.C:
		case <-ctx.Done():
			return
		}
	}
}
