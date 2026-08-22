//! Tracks probe results and grades connection quality.

use std::time::Duration;

use crate::probe::Sample;

/// Accumulates probe results.
#[derive(Debug, Default)]
pub struct Tracker {
    pub samples: Vec<Sample>,
    rtts: Vec<Duration>,
}

impl Tracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a probe result.
    pub fn add(&mut self, s: Sample) {
        self.samples.push(s);
        if let Some(rtt) = s.rtt {
            self.rtts.push(rtt);
        }
    }

    pub fn sent(&self) -> usize {
        self.samples.len()
    }

    pub fn recv(&self) -> usize {
        self.rtts.len()
    }

    pub fn lost(&self) -> usize {
        self.sent() - self.recv()
    }

    /// Packet loss as a fraction in [0, 1].
    pub fn loss(&self) -> f64 {
        if self.sent() == 0 {
            return 0.0;
        }
        self.lost() as f64 / self.sent() as f64
    }

    /// The most recent successful RTT, or zero if none.
    pub fn last(&self) -> Duration {
        self.rtts.last().copied().unwrap_or_default()
    }

    pub fn min(&self) -> Duration {
        self.rtts.iter().copied().min().unwrap_or_default()
    }

    pub fn max(&self) -> Duration {
        self.rtts.iter().copied().max().unwrap_or_default()
    }

    pub fn avg(&self) -> Duration {
        if self.rtts.is_empty() {
            return Duration::ZERO;
        }
        self.rtts.iter().sum::<Duration>() / self.rtts.len() as u32
    }

    /// The mean absolute difference between consecutive RTTs
    /// (RFC 3550-style interarrival jitter, unsmoothed).
    pub fn jitter(&self) -> Duration {
        if self.rtts.len() < 2 {
            return Duration::ZERO;
        }
        let sum: Duration = self
            .rtts
            .windows(2)
            .map(|w| if w[1] > w[0] { w[1] - w[0] } else { w[0] - w[1] })
            .sum();
        sum / (self.rtts.len() - 1) as u32
    }

    /// The p-th percentile RTT (p in [0, 100]).
    pub fn percentile(&self, p: f64) -> Duration {
        if self.rtts.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted = self.rtts.clone();
        sorted.sort_unstable();
        let idx = (p / 100.0 * sorted.len() as f64).ceil() as isize - 1;
        let idx = idx.clamp(0, sorted.len() as isize - 1) as usize;
        sorted[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::Grade;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn track(rtts: &[u64], lost: usize) -> Tracker {
        let mut t = Tracker::new();
        for (i, &r) in rtts.iter().enumerate() {
            t.add(Sample::reply(i, ms(r)));
        }
        for i in 0..lost {
            t.add(Sample::lost(rtts.len() + i));
        }
        t
    }

    #[test]
    fn basics() {
        let tr = track(&[10, 20, 30, 40], 1);
        assert_eq!(tr.sent(), 5);
        assert_eq!(tr.loss(), 0.2);
        assert_eq!(tr.min(), ms(10));
        assert_eq!(tr.max(), ms(40));
        assert_eq!(tr.avg(), ms(25));
        assert_eq!(tr.jitter(), ms(10));
        assert_eq!(tr.last(), ms(40));
    }

    #[test]
    fn percentile() {
        let tr = track(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 0);
        assert_eq!(tr.percentile(50.0), ms(5));
        assert_eq!(tr.percentile(95.0), ms(10));
    }

    #[test]
    fn empty() {
        let tr = Tracker::new();
        assert_eq!(tr.loss(), 0.0);
        assert_eq!(tr.avg(), Duration::ZERO);
        assert_eq!(tr.percentile(95.0), Duration::ZERO);
        assert_eq!(tr.jitter(), Duration::ZERO);
        assert_eq!(tr.verdict().0, Grade::Poor);
    }

    #[test]
    fn verdict() {
        let cases: &[(&str, &[u64], usize, Grade)] = &[
            ("clean and fast", &[20, 21, 19, 20], 0, Grade::Excellent),
            ("high but stable", &[200, 201, 199, 200], 0, Grade::Good),
            ("heavy loss", &[20, 20, 20, 20], 1, Grade::Poor),
            ("very high latency", &[400, 401, 399, 400], 0, Grade::Degraded),
        ];
        for &(name, rtts, lost, want) in cases {
            assert_eq!(track(rtts, lost).verdict().0, want, "{name}");
        }
    }
}
