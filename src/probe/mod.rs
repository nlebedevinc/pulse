//! Single network probes and their round-trip times.

pub mod icmp;
pub mod tcp;

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// Why a probe got no reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// No reply arrived in time.
    Timeout,
    /// The probe socket is unavailable; callers should fall back to TCP.
    Permission,
    Other,
}

/// The outcome of a single probe.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub seq: usize,
    /// Round-trip time, or `None` when the probe got no reply.
    pub rtt: Option<Duration>,
    pub failure: Option<Failure>,
}

impl Sample {
    /// A probe that got a reply in `rtt`.
    pub fn reply(seq: usize, rtt: Duration) -> Self {
        Self { seq, rtt: Some(rtt), failure: None }
    }

    /// A probe that timed out.
    pub fn lost(seq: usize) -> Self {
        Self::failed(seq, Failure::Timeout)
    }

    /// A probe that failed for a specific reason.
    pub fn failed(seq: usize, failure: Failure) -> Self {
        Self { seq, rtt: None, failure: Some(failure) }
    }

    /// Reports whether the probe got no reply.
    pub fn is_lost(&self) -> bool {
        self.rtt.is_none()
    }
}

/// Sends one probe per call.
pub trait Prober: Send {
    fn probe(&self, seq: usize) -> Sample;
    /// Describes the probe method, e.g. "icmp" or "tcp :443".
    fn kind(&self) -> String;
}

/// Probes at the given interval until `stop` is signalled (or its sender is
/// dropped) or `count` probes have been sent (`count == 0` means unlimited),
/// sending each result to `out`.
pub fn run(
    p: &dyn Prober,
    interval: Duration,
    count: usize,
    stop: &Receiver<()>,
    out: &Sender<Sample>,
) {
    let start = Instant::now();
    for seq in 0.. {
        if count > 0 && seq >= count {
            return;
        }
        if out.send(p.probe(seq)).is_err() {
            return;
        }
        // Pace off the start instant so slow probes don't drift the schedule.
        let deadline = start + interval.saturating_mul(seq as u32 + 1);
        let wait = deadline.saturating_duration_since(Instant::now());
        match stop.recv_timeout(wait) {
            Err(RecvTimeoutError::Timeout) => {}
            _ => return, // stop signalled, or the session went away
        }
    }
}
