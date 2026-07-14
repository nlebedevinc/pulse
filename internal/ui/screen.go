package ui

import (
	"fmt"
	"io"
	"strings"
)

// screen repaints a block of lines in place: it moves the cursor back to the
// top of the previous frame, clears each line and writes the new one. The
// terminal is in raw mode, so lines end with \r\n.
type screen struct {
	out   io.Writer
	lines int
}

func (s *screen) paint(frame string) {
	lines := strings.Split(frame, "\n")
	var b strings.Builder
	if s.lines > 0 {
		fmt.Fprintf(&b, "\x1b[%dF", s.lines)
	}
	for _, l := range lines {
		b.WriteString("\x1b[2K" + l + "\r\n")
	}
	b.WriteString("\x1b[J") // clear leftovers when the frame shrank
	s.lines = len(lines)
	_, _ = io.WriteString(s.out, b.String())
}
