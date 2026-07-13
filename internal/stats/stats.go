// Package stats tracks probe results and grades connection quality.
package stats

import (
	"math"
	"sort"
	"time"

	"github.com/nlebedevinc/pulse/internal/probe"
)

// Tracker accumulates probe results.
type Tracker struct {
	Results []probe.Result
	rtts    []time.Duration
}

// Add records a probe result.
func (t *Tracker) Add(r probe.Result) {
	t.Results = append(t.Results, r)
	if !r.Lost() {
		t.rtts = append(t.rtts, r.RTT)
	}
}

func (t *Tracker) Sent() int { return len(t.Results) }
func (t *Tracker) Recv() int { return len(t.rtts) }
func (t *Tracker) Lost() int { return t.Sent() - t.Recv() }

// Loss returns packet loss as a fraction in [0, 1].
func (t *Tracker) Loss() float64 {
	if t.Sent() == 0 {
		return 0
	}
	return float64(t.Lost()) / float64(t.Sent())
}

// Last returns the most recent successful RTT, or 0 if none.
func (t *Tracker) Last() time.Duration {
	if len(t.rtts) == 0 {
		return 0
	}
	return t.rtts[len(t.rtts)-1]
}

func (t *Tracker) Min() time.Duration { return t.fold(func(a, b time.Duration) bool { return a < b }) }
func (t *Tracker) Max() time.Duration { return t.fold(func(a, b time.Duration) bool { return a > b }) }

func (t *Tracker) fold(better func(a, b time.Duration) bool) time.Duration {
	if len(t.rtts) == 0 {
		return 0
	}
	best := t.rtts[0]
	for _, d := range t.rtts[1:] {
		if better(d, best) {
			best = d
		}
	}
	return best
}

func (t *Tracker) Avg() time.Duration {
	if len(t.rtts) == 0 {
		return 0
	}
	var sum time.Duration
	for _, d := range t.rtts {
		sum += d
	}
	return sum / time.Duration(len(t.rtts))
}

// Jitter is the mean absolute difference between consecutive RTTs
// (RFC 3550-style interarrival jitter, unsmoothed).
func (t *Tracker) Jitter() time.Duration {
	if len(t.rtts) < 2 {
		return 0
	}
	var sum time.Duration
	for i := 1; i < len(t.rtts); i++ {
		d := t.rtts[i] - t.rtts[i-1]
		if d < 0 {
			d = -d
		}
		sum += d
	}
	return sum / time.Duration(len(t.rtts)-1)
}

// Percentile returns the p-th percentile RTT (p in [0, 100]).
func (t *Tracker) Percentile(p float64) time.Duration {
	if len(t.rtts) == 0 {
		return 0
	}
	sorted := make([]time.Duration, len(t.rtts))
	copy(sorted, t.rtts)
	sort.Slice(sorted, func(i, j int) bool { return sorted[i] < sorted[j] })
	idx := int(math.Ceil(p/100*float64(len(sorted)))) - 1
	if idx < 0 {
		idx = 0
	}
	if idx >= len(sorted) {
		idx = len(sorted) - 1
	}
	return sorted[idx]
}
