package ui

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"

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

type checksMsg probe.Checks
type resultMsg probe.Result
type doneMsg struct{}

// Model is the Bubble Tea model for a pulse session.
type Model struct {
	opts    Options
	spin    spinner.Model
	checks  *probe.Checks
	Tracker *stats.Tracker
	Kind    string
	note    string
	results chan probe.Result
	cancel  context.CancelFunc
	width   int
	start   time.Time
	Err     error // fatal: checks could not produce an address
}

// New returns a model ready to run.
func New(opts Options) *Model {
	sp := spinner.New()
	sp.Spinner = spinner.MiniDot
	sp.Style = dim
	return &Model{
		opts:    opts,
		spin:    sp,
		Tracker: &stats.Tracker{},
		width:   80,
		start:   time.Now(),
	}
}

// Elapsed is the wall-clock duration of the session.
func (m *Model) Elapsed() time.Duration { return time.Since(m.start).Round(time.Second) }

// Checks exposes the startup check results for the exit summary.
func (m *Model) Checks() *probe.Checks { return m.checks }

func (m *Model) Init() tea.Cmd {
	return tea.Batch(m.spin.Tick, m.runChecks)
}

func (m *Model) runChecks() tea.Msg {
	return checksMsg(probe.RunChecks(context.Background(), m.opts.Host, m.opts.Port, m.opts.Timeout))
}

func (m *Model) startProbing(p probe.Prober) tea.Cmd {
	ctx, cancel := context.WithCancel(context.Background())
	m.cancel = cancel
	m.Kind = p.Kind()
	m.results = make(chan probe.Result)
	go probe.Run(ctx, p, m.opts.Interval, m.opts.Count, m.results)
	return m.waitResult
}

func (m *Model) waitResult() tea.Msg {
	r, ok := <-m.results
	if !ok {
		return doneMsg{}
	}
	return resultMsg(r)
}

func (m *Model) stop() {
	if m.cancel != nil {
		m.cancel()
	}
}

func (m *Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "q", "esc", "ctrl+c":
			m.stop()
			return m, tea.Quit
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width

	case checksMsg:
		c := probe.Checks(msg)
		m.checks = &c
		if c.IP == nil {
			m.Err = fmt.Errorf("cannot reach %s: %s", m.opts.Host, lastDetail(c))
			return m, tea.Quit
		}
		var p probe.Prober
		if m.opts.TCP {
			p = probe.NewTCP(c.IP, m.opts.Port, m.opts.Timeout)
		} else {
			p = probe.NewICMP(c.IP, m.opts.Timeout)
		}
		return m, m.startProbing(p)

	case resultMsg:
		r := probe.Result(msg)
		// ICMP sockets unavailable: fall back to TCP transparently.
		if r.Seq == 0 && r.Lost() && m.Kind == "icmp" && probe.PermissionError(r.Err) {
			m.stop()
			m.note = "icmp unavailable · falling back to tcp"
			m.Tracker = &stats.Tracker{}
			return m, m.startProbing(probe.NewTCP(m.checks.IP, m.opts.Port, m.opts.Timeout))
		}
		m.Tracker.Add(r)
		return m, m.waitResult

	case doneMsg:
		m.stop()
		return m, tea.Quit

	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spin, cmd = m.spin.Update(msg)
		return m, cmd
	}
	return m, nil
}

func lastDetail(c probe.Checks) string {
	if len(c.Items) == 0 {
		return "checks failed"
	}
	last := c.Items[len(c.Items)-1]
	return last.Name + ": " + last.Detail
}

func (m *Model) View() string {
	var b strings.Builder

	// header
	b.WriteString("\n")
	header := brand.Render("pulse") + "  " + value.Render(m.opts.Host)
	if m.checks != nil && m.checks.IP != nil && m.checks.IP.IP.String() != m.opts.Host {
		header += dim.Render(" (" + m.checks.IP.IP.String() + ")")
	}
	if m.Kind != "" {
		header += dim.Render(" · " + m.Kind + " · " + m.opts.Interval.String())
	}
	b.WriteString(pad.Render(header) + "\n\n")

	if m.checks == nil {
		b.WriteString(pad.Render(m.spin.View()+" "+dim.Render("running checks…")) + "\n")
		return b.String()
	}

	// graph
	gw := m.width - 4
	if gw > maxGraphWidth {
		gw = maxGraphWidth
	}
	if gw < 10 {
		gw = 10
	}
	if m.Tracker.Sent() > 0 {
		b.WriteString(pad.Render(Graph(m.Tracker.Results, gw, graphHeight)) + "\n\n")
		b.WriteString(pad.Render(m.statsLine()) + "\n\n")
	} else {
		b.WriteString(pad.Render(m.spin.View()+" "+dim.Render("probing…")) + "\n\n")
	}

	// checks
	for _, it := range m.checks.Items {
		b.WriteString(pad.Render(checkLine(it)) + "\n")
	}

	// footer
	b.WriteString("\n" + pad.Render(m.footer()) + "\n")
	return b.String()
}

func (m *Model) statsLine() string {
	t := m.Tracker
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

func (m *Model) footer() string {
	f := dim.Render("q quit")
	if n := m.Tracker.Sent(); n > 0 {
		count := fmt.Sprintf("%d", n)
		if m.opts.Count > 0 {
			count += fmt.Sprintf("/%d", m.opts.Count)
		}
		f += dim.Render(fmt.Sprintf(" · %s probes · %s", count, m.Elapsed()))
	}
	if m.note != "" {
		f += "\n" + warn.Render(m.note)
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
