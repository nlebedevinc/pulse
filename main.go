// Command pulse validates a network path with live probes, a compact RTT
// graph and an end-of-run health verdict.
package main

import (
	"flag"
	"fmt"
	"os"
	"time"

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

	s, err := ui.Run(ui.Options{
		Host:     flag.Arg(0),
		Port:     *port,
		Interval: *interval,
		Timeout:  *timeout,
		Count:    *count,
		TCP:      *tcp,
	})
	if err != nil {
		fmt.Fprintln(os.Stderr, "pulse:", err)
		os.Exit(1)
	}
	if s.Err != nil {
		fmt.Fprintln(os.Stderr, "pulse:", s.Err)
		os.Exit(1)
	}
	if s.Checks() != nil {
		fmt.Print(ui.Summary(s.Checks(), s.Tracker, s.Kind, s.Elapsed()))
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
