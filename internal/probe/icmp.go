package probe

import (
	"context"
	"errors"
	"net"
	"strings"
	"time"

	probing "github.com/prometheus-community/pro-bing"
)

// ICMP probes a host with a single ICMP echo request per call.
// It uses unprivileged datagram sockets, which work out of the box on macOS
// and on Linux when net.ipv4.ping_group_range allows it.
type ICMP struct {
	ip      *net.IPAddr
	timeout time.Duration
}

// NewICMP returns an ICMP prober for an already-resolved address.
func NewICMP(ip *net.IPAddr, timeout time.Duration) *ICMP {
	return &ICMP{ip: ip, timeout: timeout}
}

func (p *ICMP) Kind() string { return "icmp" }

// ErrTimeout is reported when a probe receives no reply in time.
var ErrTimeout = errors.New("timeout")

func (p *ICMP) Probe(ctx context.Context, seq int) Result {
	at := time.Now()
	pinger := probing.New("")
	pinger.SetIPAddr(p.ip)
	pinger.Count = 1
	pinger.Timeout = p.timeout
	pinger.SetPrivileged(false)
	pinger.SetLogger(nil)

	if err := pinger.RunWithContext(ctx); err != nil {
		return Result{Seq: seq, Err: err, At: at}
	}
	st := pinger.Statistics()
	if st.PacketsRecv == 0 {
		return Result{Seq: seq, Err: ErrTimeout, At: at}
	}
	return Result{Seq: seq, RTT: st.AvgRtt, At: at}
}

// PermissionError reports whether err means ICMP sockets are unavailable,
// in which case callers should fall back to TCP probing.
func PermissionError(err error) bool {
	if err == nil {
		return false
	}
	s := err.Error()
	return strings.Contains(s, "permission denied") ||
		strings.Contains(s, "operation not permitted") ||
		strings.Contains(s, "socket: ")
}
