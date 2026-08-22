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
