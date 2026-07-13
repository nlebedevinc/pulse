package probe

import (
	"context"
	"fmt"
	"net"
	"time"
)

// TCP probes a host by timing a TCP connect to a port.
// It needs no privileges and works through most firewalls.
type TCP struct {
	addr    string
	port    string
	timeout time.Duration
}

// NewTCP returns a TCP prober for an already-resolved address and port.
func NewTCP(ip *net.IPAddr, port string, timeout time.Duration) *TCP {
	return &TCP{addr: net.JoinHostPort(ip.IP.String(), port), port: port, timeout: timeout}
}

func (p *TCP) Kind() string { return "tcp :" + p.port }

func (p *TCP) Probe(ctx context.Context, seq int) Result {
	at := time.Now()
	d := net.Dialer{Timeout: p.timeout}
	start := time.Now()
	conn, err := d.DialContext(ctx, "tcp", p.addr)
	rtt := time.Since(start)
	if err != nil {
		return Result{Seq: seq, Err: fmt.Errorf("connect: %w", err), At: at}
	}
	_ = conn.Close()
	return Result{Seq: seq, RTT: rtt, At: at}
}
