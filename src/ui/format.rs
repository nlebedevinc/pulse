//! Number formatting shared by the live view and the exit summary.

use std::time::Duration;

use crate::ui::styles::{Style, BAD, VALUE, WARN};

/// Renders a duration in milliseconds: one decimal under 100ms, whole
/// numbers above.
pub fn fmt_ms(d: Duration) -> String {
    let ms = d.as_nanos() as f64 / 1_000_000.0;
    if ms < 100.0 {
        format!("{ms:.1}ms")
    } else {
        format!("{ms:.0}ms")
    }
}

/// Renders a whole-second duration the way Go's Duration.String does:
/// "34s", "1m2s", "1h2m3s".
pub fn fmt_secs(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m{}s", s / 60, s % 60)
    } else {
        format!("{}h{}m{}s", s / 3600, (s % 3600) / 60, s % 60)
    }
}

pub fn fmt_loss(loss: f64) -> String {
    format!("{:.0}%", loss * 100.0)
}

pub fn loss_style(loss: f64) -> Style {
    if loss > 0.05 {
        BAD
    } else if loss > 0.0 {
        WARN
    } else {
        VALUE
    }
}
