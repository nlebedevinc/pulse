package stats

import (
	"errors"
	"testing"
	"time"

	"github.com/nlebedevinc/pulse/internal/probe"
)

func ms(n int) time.Duration { return time.Duration(n) * time.Millisecond }

func track(rtts []int, lost int) *Tracker {
	t := &Tracker{}
	for i, r := range rtts {
		t.Add(probe.Result{Seq: i, RTT: ms(r)})
	}
	for i := 0; i < lost; i++ {
		t.Add(probe.Result{Seq: len(rtts) + i, Err: errors.New("timeout")})
	}
	return t
}

func TestBasics(t *testing.T) {
	tr := track([]int{10, 20, 30, 40}, 1)

	if got := tr.Sent(); got != 5 {
		t.Errorf("Sent() = %d, want 5", got)
	}
	if got := tr.Loss(); got != 0.2 {
		t.Errorf("Loss() = %v, want 0.2", got)
	}
	if got := tr.Min(); got != ms(10) {
		t.Errorf("Min() = %v, want 10ms", got)
	}
	if got := tr.Max(); got != ms(40) {
		t.Errorf("Max() = %v, want 40ms", got)
	}
	if got := tr.Avg(); got != ms(25) {
		t.Errorf("Avg() = %v, want 25ms", got)
	}
	if got := tr.Jitter(); got != ms(10) {
		t.Errorf("Jitter() = %v, want 10ms", got)
	}
	if got := tr.Last(); got != ms(40) {
		t.Errorf("Last() = %v, want 40ms", got)
	}
}

func TestPercentile(t *testing.T) {
	tr := track([]int{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}, 0)
	if got := tr.Percentile(50); got != ms(5) {
		t.Errorf("p50 = %v, want 5ms", got)
	}
	if got := tr.Percentile(95); got != ms(10) {
		t.Errorf("p95 = %v, want 10ms", got)
	}
}

func TestEmpty(t *testing.T) {
	tr := &Tracker{}
	if tr.Loss() != 0 || tr.Avg() != 0 || tr.Percentile(95) != 0 || tr.Jitter() != 0 {
		t.Error("empty tracker should return zeros")
	}
	if g, _ := tr.Verdict(); g != Poor {
		t.Errorf("empty verdict = %v, want poor", g)
	}
}

func TestVerdict(t *testing.T) {
	cases := []struct {
		name string
		rtts []int
		lost int
		want Grade
	}{
		{"clean and fast", []int{20, 21, 19, 20}, 0, Excellent},
		{"high but stable", []int{200, 201, 199, 200}, 0, Good},
		{"heavy loss", []int{20, 20, 20, 20}, 1, Poor},
		{"very high latency", []int{400, 401, 399, 400}, 0, Degraded},
	}
	for _, c := range cases {
		if g, _ := track(c.rtts, c.lost).Verdict(); g != c.want {
			t.Errorf("%s: verdict = %v, want %v", c.name, g, c.want)
		}
	}
}
