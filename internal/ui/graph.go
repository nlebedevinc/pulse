package ui

import (
	"strings"
	"time"

	"github.com/nlebedevinc/pulse/internal/probe"
)

var blocks = []rune("▁▂▃▄▅▆▇█")

// Graph renders the last `width` results as a compact column chart of
// `height` rows. Columns scale from zero to the window maximum; lost probes
// show as a red × on the baseline. The newest sample sits at the right edge.
func Graph(results []probe.Result, width, height int) string {
	if width < 1 || height < 1 {
		return ""
	}
	window := results
	if len(window) > width {
		window = window[len(window)-width:]
	}

	max := 5 * time.Millisecond // floor so tiny RTTs don't fill the graph
	for _, r := range window {
		if !r.Lost() && r.RTT > max {
			max = r.RTT
		}
	}

	levels := make([]int, len(window)) // eighth-blocks per column, -1 = lost
	top := height * 8
	for i, r := range window {
		if r.Lost() {
			levels[i] = -1
			continue
		}
		l := int(int64(top) * int64(r.RTT) / int64(max))
		if l < 1 {
			l = 1
		}
		if l > top {
			l = top
		}
		levels[i] = l
	}

	lostMark := bad.Render("×")
	var rows []string
	for row := 0; row < height; row++ {
		var b strings.Builder
		base := (height - 1 - row) * 8 // eighths below this row
		var run strings.Builder        // batch plain cells, style once
		flush := func() {
			if run.Len() > 0 {
				b.WriteString(graphSt.Render(run.String()))
				run.Reset()
			}
		}
		b.WriteString(strings.Repeat(" ", width-len(window)))
		for _, l := range levels {
			if l == -1 {
				if row == height-1 {
					flush()
					b.WriteString(lostMark)
				} else {
					run.WriteRune(' ')
				}
				continue
			}
			cell := l - base
			switch {
			case cell <= 0:
				run.WriteRune(' ')
			case cell >= 8:
				run.WriteRune(blocks[7])
			default:
				run.WriteRune(blocks[cell-1])
			}
		}
		flush()
		rows = append(rows, b.String())
	}
	return strings.Join(rows, "\n")
}
