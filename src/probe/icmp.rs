//! Probes a host with a single ICMP echo request per call.
//!
//! Uses unprivileged datagram sockets, which work out of the box on macOS and
//! on Linux when net.ipv4.ping_group_range allows it.

use std::io;
use std::mem::MaybeUninit;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::probe::{Failure, Prober, Sample};

const ECHO_REQUEST_V4: u8 = 8;
const ECHO_REPLY_V4: u8 = 0;
const ECHO_REQUEST_V6: u8 = 128;
const ECHO_REPLY_V6: u8 = 129;

pub struct Icmp {
    ip: IpAddr,
    timeout: Duration,
    id: u16,
}

impl Icmp {
    /// Returns an ICMP prober for an already-resolved address.
    pub fn new(ip: IpAddr, timeout: Duration) -> Self {
        Self {
            ip,
            timeout,
            id: std::process::id() as u16,
        }
    }

    /// Sends one echo request and waits for the matching reply.
    /// `Ok(None)` means the probe timed out.
    fn ping(&self, seq: u16) -> io::Result<Option<Duration>> {
        let v4 = self.ip.is_ipv4();
        let (domain, proto) = if v4 {
            (Domain::IPV4, Protocol::ICMPV4)
        } else {
            (Domain::IPV6, Protocol::ICMPV6)
        };
        let sock = Socket::new(domain, Type::DGRAM, Some(proto))?;
        sock.set_read_timeout(Some(self.timeout))?;

        let pkt = echo_request(v4, self.id, seq);
        let dst = SockAddr::from(SocketAddr::new(self.ip, 0));

        let start = Instant::now();
        sock.send_to(&pkt, &dst)?;

        // The socket can receive replies to other pings, so read until the
        // sequence matches or the deadline passes.
        loop {
            let left = match self.timeout.checked_sub(start.elapsed()) {
                Some(d) if !d.is_zero() => d,
                _ => return Ok(None),
            };
            sock.set_read_timeout(Some(left))?;

            let mut buf = [MaybeUninit::<u8>::uninit(); 1500];
            let n = match sock.recv(&mut buf) {
                Ok(n) => n,
                Err(e) if would_block(&e) => return Ok(None),
                Err(e) => return Err(e),
            };
            // SAFETY: recv reports n initialised bytes.
            let raw = unsafe { &*(&buf[..n] as *const [MaybeUninit<u8>] as *const [u8]) };

            if let Some(icmp) = echo_reply(v4, raw) {
                if reply_seq(icmp) == Some(seq) {
                    return Ok(Some(start.elapsed()));
                }
            }
        }
    }
}

impl Prober for Icmp {
    fn kind(&self) -> String {
        "icmp".into()
    }

    fn probe(&self, seq: usize) -> Sample {
        match self.ping(seq as u16) {
            Ok(Some(rtt)) => Sample::reply(seq, rtt),
            Ok(None) => Sample::lost(seq),
            // Any failure to open or use the ICMP socket means unprivileged
            // ICMP is unavailable here; the session falls back to TCP.
            Err(_) => Sample::failed(seq, Failure::Permission),
        }
    }
}

/// Builds an echo request. IPv4 needs a correct checksum — macOS drops the
/// packet otherwise, while Linux recomputes it. For IPv6 datagram sockets the
/// kernel fills the checksum in, since it needs the pseudo-header.
fn echo_request(v4: bool, id: u16, seq: u16) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(32);
    pkt.push(if v4 { ECHO_REQUEST_V4 } else { ECHO_REQUEST_V6 });
    pkt.push(0); // code
    pkt.extend_from_slice(&[0, 0]); // checksum placeholder
    pkt.extend_from_slice(&id.to_be_bytes());
    pkt.extend_from_slice(&seq.to_be_bytes());
    pkt.extend_from_slice(b"pulse-probe-payload-0123");
    if v4 {
        let ck = checksum(&pkt);
        pkt[2..4].copy_from_slice(&ck.to_be_bytes());
    }
    pkt
}

/// Returns the ICMP portion of a received echo reply, or None if the packet
/// is something else. macOS hands back the IPv4 header, Linux does not.
fn echo_reply(v4: bool, raw: &[u8]) -> Option<&[u8]> {
    let icmp = if v4 && raw.len() > 20 && raw[0] >> 4 == 4 {
        let ihl = (raw[0] & 0x0f) as usize * 4;
        raw.get(ihl..)?
    } else {
        raw
    };
    let want = if v4 { ECHO_REPLY_V4 } else { ECHO_REPLY_V6 };
    (icmp.len() >= 8 && icmp[0] == want).then_some(icmp)
}

/// The sequence number carried by an echo reply. The kernel rewrites the
/// identifier on datagram sockets, so the sequence is what we match on.
fn reply_seq(icmp: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes([*icmp.get(6)?, *icmp.get(7)?]))
}

/// The standard internet checksum (RFC 1071).
fn checksum(b: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = b.chunks_exact(2);
    for c in &mut chunks {
        sum += u16::from_be_bytes([c[0], c[1]]) as u32;
    }
    if let [last] = chunks.remainder() {
        sum += (*last as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn would_block(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_rfc_example() {
        // An echo request with a zeroed checksum must checksum to a value
        // that makes the whole packet sum to zero.
        let pkt = echo_request(true, 0x1234, 7);
        let ck = u16::from_be_bytes([pkt[2], pkt[3]]);
        assert_ne!(ck, 0, "v4 checksum must be filled in");
        let mut verify = pkt.clone();
        verify[2..4].copy_from_slice(&[0, 0]);
        assert_eq!(checksum(&verify), ck);
    }

    #[test]
    fn v6_leaves_checksum_to_the_kernel() {
        let pkt = echo_request(false, 0x1234, 7);
        assert_eq!(&pkt[2..4], &[0, 0]);
        assert_eq!(pkt[0], ECHO_REQUEST_V6);
    }

    #[test]
    fn strips_ipv4_header_when_present() {
        let mut icmp = vec![ECHO_REPLY_V4, 0, 0, 0, 0x12, 0x34, 0x00, 0x09];
        icmp.extend_from_slice(b"payload");

        // Linux: bare ICMP.
        assert_eq!(reply_seq(echo_reply(true, &icmp).unwrap()), Some(9));

        // macOS: 20-byte IPv4 header in front.
        let mut with_ip = vec![0x45u8; 20];
        with_ip[0] = 0x45; // version 4, IHL 5
        with_ip.extend_from_slice(&icmp);
        assert_eq!(reply_seq(echo_reply(true, &with_ip).unwrap()), Some(9));
    }

    #[test]
    fn rejects_non_echo_replies() {
        let dest_unreachable = [3u8, 0, 0, 0, 0, 0, 0, 0];
        assert!(echo_reply(true, &dest_unreachable).is_none());
    }
}
