//! Startup validation of the path to a host: DNS, TCP, TLS and HTTP.

use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// One startup validation step (dns, tcp, tls, http).
pub struct Check {
    pub name: &'static str,
    pub ok: bool,
    pub skipped: bool,
    pub dur: Duration,
    pub detail: String,
}

impl Check {
    fn ok(name: &'static str, dur: Duration, detail: String) -> Self {
        Self { name, ok: true, skipped: false, dur, detail }
    }
    fn failed(name: &'static str, dur: Duration, detail: String) -> Self {
        Self { name, ok: false, skipped: false, dur, detail }
    }
    fn skipped(name: &'static str, detail: &str) -> Self {
        Self { name, ok: false, skipped: true, dur: Duration::ZERO, detail: detail.into() }
    }
}

/// The startup validation results and the address chosen for probing.
pub struct Checks {
    pub target: String,
    pub ip: Option<IpAddr>,
    pub items: Vec<Check>,
}

/// Validates the path to `host` step by step: DNS resolution, TCP connect, TLS
/// handshake and an HTTP request timed to first byte. Later steps are skipped
/// when an earlier one fails. `port` is used for the TCP check; TLS and HTTP
/// run only against 443.
pub fn run_checks(host: &str, port: u16, timeout: Duration) -> Checks {
    let mut c = Checks { target: host.into(), ip: None, items: Vec::new() };

    // dns
    if let Ok(ip) = host.parse::<IpAddr>() {
        c.ip = Some(ip);
        c.items.push(Check::skipped("dns", "ip literal"));
    } else {
        let start = Instant::now();
        let res = resolve(host, timeout);
        let dur = start.elapsed();
        match res {
            Ok(addrs) if !addrs.is_empty() => {
                c.ip = Some(pick(&addrs));
                c.items.push(Check::ok("dns", dur, join_ips(&addrs)));
            }
            Ok(_) => {
                c.items.push(Check::failed("dns", dur, "no records".into()));
                return c;
            }
            Err(e) => {
                c.items.push(Check::failed("dns", dur, dns_detail(&e)));
                return c;
            }
        }
    }
    let ip = c.ip.expect("set above");

    // tcp
    let start = Instant::now();
    let res = TcpStream::connect_timeout(&SocketAddr::new(ip, port), timeout);
    let dur = start.elapsed();
    match res {
        Ok(_) => c.items.push(Check::ok("tcp", dur, format!(":{port}"))),
        Err(e) => {
            c.items.push(Check::failed("tcp", dur, err_detail(&e)));
            return c;
        }
    }

    if port != 443 {
        return c;
    }

    // tls
    let start = Instant::now();
    let res = tls_handshake(host, ip, timeout);
    let dur = start.elapsed();
    match res {
        Ok(detail) => c.items.push(Check::ok("tls", dur, detail)),
        Err(e) => {
            c.items.push(Check::failed("tls", dur, err_detail(&e)));
            return c;
        }
    }

    // http (time to first byte)
    let start = Instant::now();
    let res = http_head(host, ip, timeout);
    let dur = start.elapsed();
    match res {
        Ok((status, ttfb)) => c.items.push(Check::ok(
            "http",
            dur,
            format!("{status} · ttfb {}ms", ttfb.as_millis()),
        )),
        Err(e) => c.items.push(Check::failed("http", dur, err_detail(&e))),
    }
    c
}

/// Resolves `host`, giving up after `timeout`. std has no resolver timeout, so
/// the lookup runs on its own thread and is abandoned if it overruns.
fn resolve(host: &str, timeout: Duration) -> io::Result<Vec<IpAddr>> {
    let (tx, rx) = mpsc::channel();
    let h = host.to_string();
    thread::spawn(move || {
        let r = (h.as_str(), 0u16)
            .to_socket_addrs()
            .map(|it| it.map(|s| s.ip()).collect::<Vec<_>>());
        let _ = tx.send(r);
    });
    match rx.recv_timeout(timeout) {
        Ok(r) => r,
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "i/o timeout")),
    }
}

/// Prefers an IPv4 address so ICMP works consistently.
fn pick(addrs: &[IpAddr]) -> IpAddr {
    addrs.iter().find(|a| a.is_ipv4()).copied().unwrap_or(addrs[0])
}

fn join_ips(addrs: &[IpAddr]) -> String {
    let shown: Vec<String> = addrs.iter().take(2).map(|a| a.to_string()).collect();
    let mut s = shown.join(", ");
    if addrs.len() > 2 {
        s.push_str(&format!(" +{}", addrs.len() - 2));
    }
    s
}

/// Resolver failures, phrased the way the platform resolver reports them,
/// vary by libc. Normalise to the two cases that matter to the reader.
fn dns_detail(e: &io::Error) -> String {
    match e.kind() {
        io::ErrorKind::TimedOut => "i/o timeout".into(),
        _ => "no such host".into(),
    }
}

/// Trims wrapper context from an error, keeping the innermost cause.
fn err_detail(e: &io::Error) -> String {
    let s = e.to_string();
    match s.rfind(": ") {
        Some(i) => s[i + 2..].to_string(),
        None => s,
    }
}

fn client_config() -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

fn connect_tls(
    host: &str,
    ip: IpAddr,
    timeout: Duration,
) -> io::Result<(rustls::ClientConnection, TcpStream)> {
    let name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid server name"))?;
    let conn = rustls::ClientConnection::new(client_config(), name)
        .map_err(|e| io::Error::other(e.to_string()))?;
    let sock = TcpStream::connect_timeout(&SocketAddr::new(ip, 443), timeout)?;
    sock.set_read_timeout(Some(timeout))?;
    sock.set_write_timeout(Some(timeout))?;
    Ok((conn, sock))
}

/// Handshakes and reports the negotiated version and leaf certificate expiry.
fn tls_handshake(host: &str, ip: IpAddr, timeout: Duration) -> io::Result<String> {
    let (mut conn, mut sock) = connect_tls(host, ip, timeout)?;
    while conn.is_handshaking() {
        if conn.wants_write() {
            conn.write_tls(&mut sock)?;
            continue;
        }
        if conn.wants_read() {
            if conn.read_tls(&mut sock)? == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed"));
            }
            conn.process_new_packets()
                .map_err(|e| io::Error::other(e.to_string()))?;
        }
    }

    let mut detail = version_name(conn.protocol_version()).to_string();
    if let Some(certs) = conn.peer_certificates() {
        if let Some(days) = expiry_days(&certs[0]) {
            detail.push_str(&format!(" · cert {days}d"));
        }
    }
    Ok(detail)
}

fn version_name(v: Option<rustls::ProtocolVersion>) -> &'static str {
    use rustls::ProtocolVersion::*;
    match v {
        Some(TLSv1_3) => "TLS 1.3",
        Some(TLSv1_2) => "TLS 1.2",
        Some(TLSv1_1) => "TLS 1.1",
        Some(TLSv1_0) => "TLS 1.0",
        _ => "unknown",
    }
}

/// Whole days until the certificate's notAfter, truncated toward zero.
fn expiry_days(der: &[u8]) -> Option<i64> {
    let (_, cert) = x509_parser::parse_x509_certificate(der).ok()?;
    let not_after = cert.validity().not_after.timestamp();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    Some((not_after - now) / 86_400)
}

/// Times a GET to first response byte. Redirects are not followed: the first
/// response is exactly the timing we care about.
fn http_head(host: &str, ip: IpAddr, timeout: Duration) -> io::Result<(u16, Duration)> {
    let start = Instant::now();
    let (mut conn, mut sock) = connect_tls(host, ip, timeout)?;
    let mut tls = rustls::Stream::new(&mut conn, &mut sock);
    let req = format!("GET / HTTP/1.1\r\nHost: {host}\r\nUser-Agent: pulse\r\nConnection: close\r\n\r\n");
    tls.write_all(req.as_bytes())?;
    tls.flush()?;

    let mut ttfb = None;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let n = tls.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        if ttfb.is_none() {
            ttfb = Some(start.elapsed());
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let status = status_code(&buf)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "malformed status line"))?;
    Ok((status, ttfb.unwrap_or_default()))
}

/// Parses the status code out of an HTTP/1.x status line.
fn status_code(resp: &[u8]) -> Option<u16> {
    let line = resp.split(|&b| b == b'\n').next()?;
    let mut parts = line.split(|&b| b == b' ');
    parts.next()?; // HTTP/1.1
    std::str::from_utf8(parts.next()?).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn prefers_ipv4() {
        let v6 = IpAddr::V6(Ipv6Addr::LOCALHOST);
        let v4 = IpAddr::V4(Ipv4Addr::LOCALHOST);
        assert_eq!(pick(&[v6, v4]), v4);
        assert_eq!(pick(&[v6]), v6, "falls back to the first address");
    }

    #[test]
    fn joins_at_most_two_ips() {
        let a = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let b = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));
        let c = IpAddr::V4(Ipv4Addr::new(3, 3, 3, 3));
        assert_eq!(join_ips(&[a]), "1.1.1.1");
        assert_eq!(join_ips(&[a, b]), "1.1.1.1, 2.2.2.2");
        assert_eq!(join_ips(&[a, b, c]), "1.1.1.1, 2.2.2.2 +1");
    }

    #[test]
    fn keeps_innermost_error_cause() {
        let e = io::Error::other("dial tcp 1.2.3.4:443: connect: connection refused");
        assert_eq!(err_detail(&e), "connection refused");
        let plain = io::Error::other("no route to host");
        assert_eq!(err_detail(&plain), "no route to host");
    }

    #[test]
    fn parses_status_line() {
        assert_eq!(status_code(b"HTTP/1.1 301 Moved\r\n\r\n"), Some(301));
        assert_eq!(status_code(b"HTTP/1.1 200 OK\r\n"), Some(200));
        assert_eq!(status_code(b"garbage"), None);
    }
}
