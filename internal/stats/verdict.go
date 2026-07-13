package stats

import "time"

// Grade is an overall connection quality rating.
type Grade int

const (
	Excellent Grade = iota
	Good
	Degraded
	Poor
)

func (g Grade) String() string {
	switch g {
	case Excellent:
		return "excellent"
	case Good:
		return "good"
	case Degraded:
		return "degraded"
	default:
		return "poor"
	}
}

// Verdict grades the connection. Loss and jitter dominate the grade because
// they hurt interactive traffic more than raw latency does; latency adjusts
// the grade within a stable connection.
func (t *Tracker) Verdict() (Grade, string) {
	if t.Recv() == 0 {
		return Poor, "no replies received"
	}

	loss := t.Loss()
	jitter := t.Jitter()
	avg := t.Avg()

	switch {
	case loss > 0.05:
		return Poor, "heavy packet loss"
	case loss > 0.01:
		return Degraded, "intermittent packet loss"
	case jitter > 50*time.Millisecond:
		return Degraded, "unstable latency"
	case avg > 300*time.Millisecond:
		return Degraded, "very high latency"
	case loss > 0 || jitter > 20*time.Millisecond || avg > 150*time.Millisecond:
		return Good, "stable with minor variance"
	default:
		return Excellent, "stable, low latency"
	}
}
