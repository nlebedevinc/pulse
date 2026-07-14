package ui

import (
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/lipgloss"

	"github.com/nlebedevinc/pulse/internal/probe"
	"github.com/nlebedevinc/pulse/internal/stats"
)

// Summary renders the end-of-session report printed after the TUI exits.
func Summary(checks *probe.Checks, t *stats.Tracker, kind string, elapsed time.Duration) string {
	var b strings.Builder

	target := checks.Target
	if checks.IP != nil && checks.IP.IP.String() != target {
		target += dim.Render(" (" + checks.IP.IP.String() + ")")
	}
	b.WriteString(brand.Render("pulse") + " summary — " + target + "\n")

	if t.Sent() == 0 {
		b.WriteString(dim.Render("  no probes sent") + "\n")
		return b.String()
	}

	b.WriteString(fmt.Sprintf("  %d probes over %s via %s · %d lost (%s)\n",
		t.Sent(), elapsed, kind, t.Lost(),
		lossStyle(t.Loss()).Render(fmtLoss(t.Loss()))))

	if t.Recv() > 0 {
		b.WriteString("  rtt min/avg/max " +
			value.Render(fmt.Sprintf("%s/%s/%s", fmtMs(t.Min()), fmtMs(t.Avg()), fmtMs(t.Max()))) +
			dim.Render(" · ") + dim.Render("p95 ") + value.Render(fmtMs(t.Percentile(95))) +
			dim.Render(" · ") + dim.Render("jitter ") + value.Render(fmtMs(t.Jitter())) + "\n")
	}

	grade, reason := t.Verdict()
	b.WriteString("  verdict: " + gradeStyle(grade).Render(grade.String()) +
		dim.Render(" — "+reason) + "\n")
	return b.String()
}

func gradeStyle(g stats.Grade) lipgloss.Style {
	switch g {
	case stats.Excellent:
		return ok.Bold(true)
	case stats.Good:
		return ok
	case stats.Degraded:
		return warn
	default:
		return bad.Bold(true)
	}
}
