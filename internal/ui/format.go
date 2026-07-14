package ui

import (
	"fmt"
	"time"

	"github.com/charmbracelet/lipgloss"
)

// fmtMs renders a duration in milliseconds: one decimal under 100ms,
// whole numbers above.
func fmtMs(d time.Duration) string {
	ms := float64(d) / float64(time.Millisecond)
	if ms < 100 {
		return fmt.Sprintf("%.1fms", ms)
	}
	return fmt.Sprintf("%.0fms", ms)
}

func fmtLoss(loss float64) string {
	return fmt.Sprintf("%.0f%%", loss*100)
}

func lossStyle(loss float64) lipgloss.Style {
	switch {
	case loss > 0.05:
		return bad
	case loss > 0:
		return warn
	default:
		return value
	}
}
