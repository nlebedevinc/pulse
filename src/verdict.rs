//! Connection quality grading.

use std::fmt;
use std::time::Duration;

use crate::stats::Tracker;

/// An overall connection quality rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    Excellent,
    Good,
    Degraded,
    Poor,
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Grade::Excellent => "excellent",
            Grade::Good => "good",
            Grade::Degraded => "degraded",
            Grade::Poor => "poor",
        })
    }
}

impl Tracker {
    /// Grades the connection. Loss and jitter dominate the grade because they
    /// hurt interactive traffic more than raw latency does; latency adjusts
    /// the grade within a stable connection.
    pub fn verdict(&self) -> (Grade, &'static str) {
        if self.recv() == 0 {
            return (Grade::Poor, "no replies received");
        }

        let loss = self.loss();
        let jitter = self.jitter();
        let avg = self.avg();

        if loss > 0.05 {
            (Grade::Poor, "heavy packet loss")
        } else if loss > 0.01 {
            (Grade::Degraded, "intermittent packet loss")
        } else if jitter > Duration::from_millis(50) {
            (Grade::Degraded, "unstable latency")
        } else if avg > Duration::from_millis(300) {
            (Grade::Degraded, "very high latency")
        } else if loss > 0.0
            || jitter > Duration::from_millis(20)
            || avg > Duration::from_millis(150)
        {
            (Grade::Good, "stable with minor variance")
        } else {
            (Grade::Excellent, "stable, low latency")
        }
    }
}
