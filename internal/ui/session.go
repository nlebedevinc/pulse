package ui

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"golang.org/x/term"

	"github.com/nlebedevinc/pulse/internal/probe"
	"github.com/nlebedevinc/pulse/internal/stats"
)

// Options configure a pulse run.
type Options struct {
	Host     string
	Port     string
	Interval time.Duration
	Timeout  time.Duration
	Count    int  // 0 = until interrupted
	TCP      bool // force TCP probes instead of ICMP
}

const (
	graphHeight   = 4
	maxGraphWidth = 60
)

// Session holds the state of an interactive run for the exit summary.
type Session struct {
	opts    Options
	checks  *probe.Checks
	Tracker *stats.Tracker
	Kind    string
	note    string
	spin    int
	width   int
	start   time.Time
	Err     error // fatal: checks could not produce an address
}

// Checks exposes the startup check results for the exit summary.
func (s *Session) Checks() *probe.Checks { return s.checks }

// Elapsed is the wall-clock duration of the session.
func (s *Session) Elapsed() time.Duration { return time.Since(s.start).Round(time.Second) }

// Run drives the session until the probe count is reached, the user quits or
// the process is signalled. It owns the terminal for its whole lifetime.
func Run(opts Options) (*Session, error) {
	s := &Session{opts: opts, Tracker: &stats.Tracker{}, width: 80, start: time.Now()}

	fd := int(os.Stdin.Fd())
	if !term.IsTerminal(fd) {
		return nil, fmt.Errorf("needs an interactive terminal")
	}
	oldState, err := term.MakeRaw(fd)
	if err != nil {
		return nil, err
	}
	if w, _, err := term.GetSize(fd); err == nil && w > 0 {
		s.width = w
	}
	fmt.Print("\x1b[?25l") // hide cursor
	defer func() {
		fmt.Print("\x1b[?25h")
		_ = term.Restore(fd, oldState)
	}()

	sigc := make(chan os.Signal, 1)
	signal.Notify(sigc, syscall.SIGTERM, os.Interrupt)
	defer signal.Stop(sigc)

	winch := make(chan os.Signal, 1)
	notifyResize(winch)
	defer signal.Stop(winch)

	keys := readKeys()

	checksCh := make(chan probe.Checks, 1)
	go func() {
		checksCh <- probe.RunChecks(context.Background(), opts.Host, opts.Port, opts.Timeout)
	}()

	var results chan probe.Result // nil until checks pass; nil blocks in select
	var probeCancel context.CancelFunc
	defer func() {
		if probeCancel != nil {
			probeCancel()
		}
	}()
	startProbing := func(p probe.Prober) {
		var ctx context.Context
		ctx, probeCancel = context.WithCancel(context.Background())
		s.Kind = p.Kind()
		results = make(chan probe.Result)
		go probe.Run(ctx, p, opts.Interval, opts.Count, results)
	}

	spin := time.NewTicker(80 * time.Millisecond)
	defer spin.Stop()

	scr := &screen{out: os.Stdout}
	scr.paint(s.view())

	for {
		select {
		case <-sigc:
			return s, nil

		case k, open := <-keys:
			if !open {
				keys = nil // stdin closed; keep running until count or signal
				continue
			}
			switch k {
			case 'q', 'Q', 0x1b /* esc */, 0x03 /* ctrl+c */ :
				return s, nil
			}

		case <-winch:
			if w, _, err := term.GetSize(fd); err == nil && w > 0 {
				s.width = w
			}
			scr.paint(s.view())

		case c := <-checksCh:
			s.checks = &c
			if c.IP == nil {
				s.Err = fmt.Errorf("cannot reach %s: %s", opts.Host, lastDetail(c))
				return s, nil
			}
			if opts.TCP {
				startProbing(probe.NewTCP(c.IP, opts.Port, opts.Timeout))
			} else {
				startProbing(probe.NewICMP(c.IP, opts.Timeout))
			}
			scr.paint(s.view())

		case r, open := <-results:
			if !open {
				return s, nil
			}
			// ICMP sockets unavailable: fall back to TCP transparently.
			if r.Seq == 0 && r.Lost() && s.Kind == "icmp" && probe.PermissionError(r.Err) {
				probeCancel()
				s.note = "icmp unavailable · falling back to tcp"
				s.Tracker = &stats.Tracker{}
				startProbing(probe.NewTCP(s.checks.IP, opts.Port, opts.Timeout))
				continue
			}
			s.Tracker.Add(r)
			scr.paint(s.view())

		case <-spin.C:
			if s.Tracker.Sent() == 0 {
				s.spin = (s.spin + 1) % len(spinFrames)
				scr.paint(s.view())
			}
		}
	}
}

// readKeys reads single bytes from stdin (raw mode) on a channel.
func readKeys() <-chan byte {
	ch := make(chan byte)
	go func() {
		buf := make([]byte, 1)
		for {
			n, err := os.Stdin.Read(buf)
			if err != nil {
				close(ch)
				return
			}
			if n > 0 {
				ch <- buf[0]
			}
		}
	}()
	return ch
}

func lastDetail(c probe.Checks) string {
	if len(c.Items) == 0 {
		return "checks failed"
	}
	last := c.Items[len(c.Items)-1]
	return last.Name + ": " + last.Detail
}

// indent prefixes every line with the standard two-space margin.
func indent(s string) string {
	return "  " + strings.ReplaceAll(s, "\n", "\n  ")
}

func (s *Session) view() string {
	var b strings.Builder

	// header
	b.WriteString("\n")
	header := brand.Render("pulse") + "  " + s.opts.Host
	if s.checks != nil && s.checks.IP != nil && s.checks.IP.IP.String() != s.opts.Host {
		header += dim.Render(" (" + s.checks.IP.IP.String() + ")")
	}
	if s.Kind != "" {
		header += dim.Render(" · " + s.Kind + " · " + s.opts.Interval.String())
	}
	b.WriteString(indent(header) + "\n\n")

	if s.checks == nil {
		b.WriteString(indent(dim.Render(spinFrames[s.spin]+" running checks…")) + "\n")
		return b.String()
	}

	// graph
	gw := s.width - 4
	if gw > maxGraphWidth {
		gw = maxGraphWidth
	}
	if gw < 10 {
		gw = 10
	}
	if s.Tracker.Sent() > 0 {
		b.WriteString(indent(Graph(s.Tracker.Results, gw, graphHeight)) + "\n\n")
		b.WriteString(indent(s.statsLine()) + "\n\n")
	} else {
		b.WriteString(indent(dim.Render(spinFrames[s.spin]+" probing…")) + "\n\n")
	}

	// checks
	for _, it := range s.checks.Items {
		b.WriteString(indent(checkLine(it)) + "\n")
	}

	// footer
	b.WriteString("\n" + indent(s.footer()) + "\n")
	return b.String()
}

func (s *Session) statsLine() string {
	t := s.Tracker
	parts := []string{
		dim.Render("last ") + value.Render(fmtMs(t.Last())),
		dim.Render("min ") + value.Render(fmtMs(t.Min())),
		dim.Render("avg ") + value.Render(fmtMs(t.Avg())),
		dim.Render("max ") + value.Render(fmtMs(t.Max())),
		dim.Render("jitter ") + value.Render(fmtMs(t.Jitter())),
		dim.Render("loss ") + lossStyle(t.Loss()).Render(fmtLoss(t.Loss())),
	}
	return strings.Join(parts, "   ")
}

func (s *Session) footer() string {
	f := dim.Render("q quit")
	if n := s.Tracker.Sent(); n > 0 {
		count := fmt.Sprintf("%d", n)
		if s.opts.Count > 0 {
			count += fmt.Sprintf("/%d", s.opts.Count)
		}
		f += dim.Render(fmt.Sprintf(" · %s probes · %s", count, s.Elapsed()))
	}
	if s.note != "" {
		f += "\n" + warn.Render(s.note)
	}
	return f
}

func checkLine(it probe.Check) string {
	icon, name := ok.Render("✓"), dim.Render(fmt.Sprintf("%-5s", it.Name))
	switch {
	case it.Skipped:
		icon = dim.Render("–")
	case !it.OK:
		icon = bad.Render("✗")
	}
	dur := "      "
	if !it.Skipped {
		dur = fmt.Sprintf("%4dms", it.Dur.Milliseconds())
	}
	detail := it.Detail
	if !it.OK && !it.Skipped {
		detail = bad.Render(detail)
	} else {
		detail = dim.Render(detail)
	}
	return fmt.Sprintf("%s %s %s  %s", icon, name, value.Render(dur), detail)
}
