package ui

import (
	"errors"
	"strings"
	"testing"
	"time"

	"github.com/nlebedevinc/pulse/internal/probe"
)

func results(rtts ...int) []probe.Result {
	var rs []probe.Result
	for i, r := range rtts {
		rs = append(rs, probe.Result{Seq: i, RTT: time.Duration(r) * time.Millisecond})
	}
	return rs
}

func plain(s string) string {
	// strip ANSI escapes for shape assertions
	var b strings.Builder
	inEsc := false
	for _, r := range s {
		switch {
		case r == '\x1b':
			inEsc = true
		case inEsc && r == 'm':
			inEsc = false
		case !inEsc:
			b.WriteRune(r)
		}
	}
	return b.String()
}

func TestGraphShape(t *testing.T) {
	g := plain(Graph(results(10, 50, 100), 10, 4))
	rows := strings.Split(g, "\n")
	if len(rows) != 4 {
		t.Fatalf("got %d rows, want 4", len(rows))
	}
	for i, row := range rows {
		if got := len([]rune(row)); got != 10 {
			t.Errorf("row %d width = %d, want 10", i, got)
		}
	}
	// max sample fills the top row's last column
	if r := []rune(rows[0]); r[len(r)-1] != '█' {
		t.Errorf("top-right cell = %q, want full block", r[len(r)-1])
	}
	// newest at right, window right-aligned: leftmost columns are empty
	if r := []rune(rows[3]); r[0] != ' ' {
		t.Errorf("bottom-left cell = %q, want padding space", r[0])
	}
}

func TestGraphLoss(t *testing.T) {
	rs := results(20, 20)
	rs = append(rs, probe.Result{Seq: 2, Err: errors.New("timeout")})
	g := plain(Graph(rs, 3, 4))
	rows := strings.Split(g, "\n")
	bottom := []rune(rows[3])
	if bottom[2] != '×' {
		t.Errorf("lost probe cell = %q, want ×", bottom[2])
	}
}

func TestGraphWindow(t *testing.T) {
	rs := results(1, 2, 3, 4, 5, 6, 7, 8)
	g := plain(Graph(rs, 4, 2))
	rows := strings.Split(g, "\n")
	if got := len([]rune(rows[0])); got != 4 {
		t.Errorf("width = %d, want 4 (window trims old samples)", got)
	}
}

func TestGraphEmpty(t *testing.T) {
	if g := Graph(nil, 0, 0); g != "" {
		t.Errorf("empty graph = %q, want empty string", g)
	}
}
