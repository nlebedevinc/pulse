//! Single network probes and their round-trip times.

use std::time::Duration;

/// The outcome of a single probe.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub seq: usize,
    /// Round-trip time, or `None` when the probe got no reply.
    pub rtt: Option<Duration>,
}

impl Sample {
    /// A probe that got a reply in `rtt`.
    pub fn reply(seq: usize, rtt: Duration) -> Self {
        Self { seq, rtt: Some(rtt) }
    }

    /// A probe that got no reply.
    pub fn lost(seq: usize) -> Self {
        Self { seq, rtt: None }
    }

    /// Reports whether the probe got no reply.
    pub fn is_lost(&self) -> bool {
        self.rtt.is_none()
    }
}
