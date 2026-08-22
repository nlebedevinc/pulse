//! Probes a host by timing a TCP connect to a port.
//!
//! Needs no privileges and works through most firewalls.

use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use crate::probe::{Prober, Sample};

pub struct Tcp {
    addr: SocketAddr,
    port: u16,
    timeout: Duration,
}

impl Tcp {
    /// Returns a TCP prober for an already-resolved address and port.
    pub fn new(ip: IpAddr, port: u16, timeout: Duration) -> Self {
        Self {
            addr: SocketAddr::new(ip, port),
            port,
            timeout,
        }
    }
}

impl Prober for Tcp {
    fn kind(&self) -> String {
        format!("tcp :{}", self.port)
    }

    fn probe(&self, seq: usize) -> Sample {
        let start = Instant::now();
        match TcpStream::connect_timeout(&self.addr, self.timeout) {
            Ok(_) => Sample::reply(seq, start.elapsed()),
            Err(_) => Sample::lost(seq),
        }
    }
}
