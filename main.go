// Command pulse validates a network path with live probes, a compact RTT
// graph and an end-of-run health verdict.
package main

import (
	"flag"
	"fmt"
	"os"
	"time"

	tea "github.com/charmbracelet/bubbletea"

	"github.com/nlebedevinc/pulse/internal/ui"
)

var version = "dev" // set via -ldflags "-X main.version=…"

func main() {
	flag.Usage = usage
	var (
		count    = flag.Int("c", 0, "stop after this many probes (default: until interrupted)")
		interval = flag.Duration("i", time.Second, "time between probes")
		timeout  = flag.Duration("t", 2*time.Second, "per-probe timeout")
		port     = flag.String("p", "443", "port for tcp probes and checks")
		tcp      = flag.Bool("tcp", false, "probe with tcp connects instead of icmp")
		ver      = flag.Bool("version", false, "print version and exit")
	)
	flag.Parse()

	if *ver {
		fmt.Println("pulse", version)
		return
	}
	if flag.NArg() != 1 {
		usage()
		os.Exit(2)
	}

	m := ui.New(ui.Options{
		Host:     flag.Arg(0),
		Port:     *port,
		Interval: *interval,
		Timeout:  *timeout,
		Count:    *count,
		TCP:      *tcp,
	})

	p := tea.NewProgram(m)
	final, err := p.Run()
	if err != nil {
		fmt.Fprintln(os.Stderr, "pulse:", err)
		os.Exit(1)
	}

	fm := final.(*ui.Model)
	if fm.Err != nil {
		fmt.Fprintln(os.Stderr, "pulse:", fm.Err)
		os.Exit(1)
	}
	if fm.Checks() != nil {
		fmt.Print(ui.Summary(fm.Checks(), fm.Tracker, fm.Kind, fm.Elapsed()))
	}
}

func usage() {
	fmt.Fprintf(os.Stderr, `pulse — validate a connection, watch it live, get a verdict

usage:
  pulse [flags] <host>

examples:
  pulse google.com          probe with icmp, checks on :443
  pulse -c 60 1.1.1.1       stop after 60 probes
  pulse --tcp -p 22 host    probe a specific tcp port

flags:
`)
	flag.PrintDefaults()
}
