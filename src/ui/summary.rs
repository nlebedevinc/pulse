//! The end-of-session report printed after the TUI exits.

use std::time::Duration;

use crate::checks::Checks;
use crate::stats::Tracker;
use crate::ui::format::{fmt_loss, fmt_ms, fmt_secs, loss_style};
use crate::ui::styles::{Style, BAD_BOLD, BRAND, DIM, OK, OK_BOLD, VALUE, WARN};
use crate::verdict::Grade;

pub fn summary(checks: &Checks, t: &Tracker, kind: &str, elapsed: Duration) -> String {
    let mut b = String::new();

    let mut target = checks.target.clone();
    if let Some(ip) = checks.ip {
        let ip = ip.to_string();
        if ip != target {
            target.push_str(&DIM.render(&format!(" ({ip})")));
        }
    }
    b.push_str(&format!("{} summary — {target}\n", BRAND.render("pulse")));

    if t.sent() == 0 {
        b.push_str(&DIM.render("  no probes sent"));
        b.push('\n');
        return b;
    }

    b.push_str(&format!(
        "  {} probes over {} via {} · {} lost ({})\n",
        t.sent(),
        fmt_secs(elapsed),
        kind,
        t.lost(),
        loss_style(t.loss()).render(&fmt_loss(t.loss()))
    ));

    if t.recv() > 0 {
        b.push_str("  rtt min/avg/max ");
        b.push_str(&VALUE.render(&format!(
            "{}/{}/{}",
            fmt_ms(t.min()),
            fmt_ms(t.avg()),
            fmt_ms(t.max())
        )));
        b.push_str(&DIM.render(" · "));
        b.push_str(&DIM.render("p95 "));
        b.push_str(&VALUE.render(&fmt_ms(t.percentile(95.0))));
        b.push_str(&DIM.render(" · "));
        b.push_str(&DIM.render("jitter "));
        b.push_str(&VALUE.render(&fmt_ms(t.jitter())));
        b.push('\n');
    }

    let (grade, reason) = t.verdict();
    b.push_str("  verdict: ");
    b.push_str(&grade_style(grade).render(&grade.to_string()));
    b.push_str(&DIM.render(&format!(" — {reason}")));
    b.push('\n');
    b
}

fn grade_style(g: Grade) -> Style {
    match g {
        Grade::Excellent => OK_BOLD,
        Grade::Good => OK,
        Grade::Degraded => WARN,
        Grade::Poor => BAD_BOLD,
    }
}
