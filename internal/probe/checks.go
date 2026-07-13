package probe

import (
	"context"
	"crypto/tls"
	"fmt"
	"net"
	"net/http"
	"net/http/httptrace"
	"strings"
	"time"
)

// Check is one startup validation step (dns, tcp, tls, http).
type Check struct {
	Name    string
	OK      bool
	Skipped bool
	Dur     time.Duration
	Detail  string
}

// Checks holds the startup validation results and the address chosen for probing.
type Checks struct {
	Target string
	IP     *net.IPAddr
	Items  []Check
}

// RunChecks validates the path to host step by step: DNS resolution, TCP
// connect, TLS handshake and an HTTP request timed to first byte. Later steps
// are skipped when an earlier one fails. port is used for the TCP check; TLS
// and HTTP run only against 443.
func RunChecks(ctx context.Context, host, port string, timeout time.Duration) Checks {
	c := Checks{Target: host}

	// dns
	ip := net.ParseIP(host)
	if ip != nil {
		c.IP = &net.IPAddr{IP: ip}
		c.Items = append(c.Items, Check{Name: "dns", Skipped: true, Detail: "ip literal"})
	} else {
		start := time.Now()
		rctx, cancel := context.WithTimeout(ctx, timeout)
		addrs, err := net.DefaultResolver.LookupIPAddr(rctx, host)
		cancel()
		dur := time.Since(start)
		if err != nil || len(addrs) == 0 {
			c.Items = append(c.Items, Check{Name: "dns", Dur: dur, Detail: errDetail(err, "no records")})
			return c
		}
		c.IP = pick(addrs)
		c.Items = append(c.Items, Check{Name: "dns", OK: true, Dur: dur, Detail: joinIPs(addrs)})
	}

	// tcp
	d := net.Dialer{Timeout: timeout}
	start := time.Now()
	conn, err := d.DialContext(ctx, "tcp", net.JoinHostPort(c.IP.IP.String(), port))
	dur := time.Since(start)
	if err != nil {
		c.Items = append(c.Items, Check{Name: "tcp", Dur: dur, Detail: errDetail(err, "")})
		return c
	}
	_ = conn.Close()
	c.Items = append(c.Items, Check{Name: "tcp", OK: true, Dur: dur, Detail: ":" + port})

	if port != "443" {
		return c
	}

	// tls
	start = time.Now()
	tconn, err := tls.DialWithDialer(&d, "tcp", net.JoinHostPort(c.IP.IP.String(), "443"),
		&tls.Config{ServerName: host})
	dur = time.Since(start)
	if err != nil {
		c.Items = append(c.Items, Check{Name: "tls", Dur: dur, Detail: errDetail(err, "")})
		return c
	}
	state := tconn.ConnectionState()
	detail := tls.VersionName(state.Version)
	if len(state.PeerCertificates) > 0 {
		days := int(time.Until(state.PeerCertificates[0].NotAfter).Hours() / 24)
		detail += fmt.Sprintf(" · cert %dd", days)
	}
	_ = tconn.Close()
	c.Items = append(c.Items, Check{Name: "tls", OK: true, Dur: dur, Detail: detail})

	// http (time to first byte)
	var ttfb time.Duration
	reqStart := time.Now()
	trace := &httptrace.ClientTrace{
		GotFirstResponseByte: func() { ttfb = time.Since(reqStart) },
	}
	req, err := http.NewRequestWithContext(httptrace.WithClientTrace(ctx, trace),
		http.MethodGet, "https://"+host+"/", nil)
	if err != nil {
		c.Items = append(c.Items, Check{Name: "http", Detail: errDetail(err, "")})
		return c
	}
	client := &http.Client{
		Timeout: timeout,
		CheckRedirect: func(*http.Request, []*http.Request) error {
			return http.ErrUseLastResponse
		},
	}
	resp, err := client.Do(req)
	dur = time.Since(reqStart)
	if err != nil {
		c.Items = append(c.Items, Check{Name: "http", Dur: dur, Detail: errDetail(err, "")})
		return c
	}
	_ = resp.Body.Close()
	c.Items = append(c.Items, Check{Name: "http", OK: true, Dur: dur,
		Detail: fmt.Sprintf("%d · ttfb %dms", resp.StatusCode, ttfb.Milliseconds())})
	return c
}

// pick prefers an IPv4 address so ICMP works consistently.
func pick(addrs []net.IPAddr) *net.IPAddr {
	for i := range addrs {
		if addrs[i].IP.To4() != nil {
			return &addrs[i]
		}
	}
	return &addrs[0]
}

func joinIPs(addrs []net.IPAddr) string {
	var parts []string
	for _, a := range addrs {
		parts = append(parts, a.IP.String())
		if len(parts) == 2 {
			break
		}
	}
	s := strings.Join(parts, ", ")
	if len(addrs) > 2 {
		s += fmt.Sprintf(" +%d", len(addrs)-2)
	}
	return s
}

func errDetail(err error, fallback string) string {
	if err == nil {
		return fallback
	}
	s := err.Error()
	if i := strings.LastIndex(s, ": "); i >= 0 {
		s = s[i+2:]
	}
	return s
}
