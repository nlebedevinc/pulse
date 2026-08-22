//! A compact column chart of recent round-trip times.

use std::time::Duration;

use crate::probe::Sample;
use crate::ui::styles::{BAD, GRAPH};

const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Renders the last `width` samples as a compact column chart of `height`
/// rows. Columns scale from zero to the window maximum; lost probes show as a
/// red × on the baseline. The newest sample sits at the right edge.
pub fn graph(samples: &[Sample], width: usize, height: usize) -> String {
    if width < 1 || height < 1 {
        return String::new();
    }
    let window = if samples.len() > width {
        &samples[samples.len() - width..]
    } else {
        samples
    };

    // floor so tiny RTTs don't fill the graph
    let mut max = Duration::from_millis(5);
    for s in window {
        if let Some(rtt) = s.rtt {
            if rtt > max {
                max = rtt;
            }
        }
    }

    // eighth-blocks per column, -1 = lost
    let top = (height * 8) as i64;
    let levels: Vec<i64> = window
        .iter()
        .map(|s| match s.rtt {
            None => -1,
            Some(rtt) => {
                let l = (top as i128 * rtt.as_nanos() as i128 / max.as_nanos() as i128) as i64;
                l.clamp(1, top)
            }
        })
        .collect();

    let lost_mark = BAD.render("×");
    let mut rows = Vec::with_capacity(height);
    for row in 0..height {
        let mut b = String::new();
        let base = ((height - 1 - row) * 8) as i64; // eighths below this row
        let mut run = String::new(); // batch plain cells, style once

        b.push_str(&" ".repeat(width - window.len()));
        for &l in &levels {
            if l == -1 {
                if row == height - 1 {
                    flush(&mut b, &mut run);
                    b.push_str(&lost_mark);
                } else {
                    run.push(' ');
                }
                continue;
            }
            let cell = l - base;
            if cell <= 0 {
                run.push(' ');
            } else if cell >= 8 {
                run.push(BLOCKS[7]);
            } else {
                run.push(BLOCKS[cell as usize - 1]);
            }
        }
        flush(&mut b, &mut run);
        rows.push(b);
    }
    rows.join("\n")
}

fn flush(b: &mut String, run: &mut String) {
    if !run.is_empty() {
        b.push_str(&GRAPH.render(run));
        run.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(rtts: &[u64]) -> Vec<Sample> {
        rtts.iter()
            .enumerate()
            .map(|(i, &r)| Sample::reply(i, Duration::from_millis(r)))
            .collect()
    }

    /// Strips ANSI escapes for shape assertions.
    fn plain(s: &str) -> String {
        let mut out = String::new();
        let mut in_esc = false;
        for c in s.chars() {
            match c {
                '\x1b' => in_esc = true,
                'm' if in_esc => in_esc = false,
                _ if !in_esc => out.push(c),
                _ => {}
            }
        }
        out
    }

    #[test]
    fn shape() {
        let g = plain(&graph(&samples(&[10, 50, 100]), 10, 4));
        let rows: Vec<&str> = g.split('\n').collect();
        assert_eq!(rows.len(), 4, "row count");
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.chars().count(), 10, "row {i} width");
        }
        // max sample fills the top row's last column
        assert_eq!(rows[0].chars().next_back(), Some('█'), "top-right cell");
        // newest at right, window right-aligned: leftmost columns are empty
        assert_eq!(rows[3].chars().next(), Some(' '), "bottom-left cell");
    }

    #[test]
    fn loss() {
        let mut rs = samples(&[20, 20]);
        rs.push(Sample::lost(2));
        let g = plain(&graph(&rs, 3, 4));
        let rows: Vec<&str> = g.split('\n').collect();
        let bottom: Vec<char> = rows[3].chars().collect();
        assert_eq!(bottom[2], '×', "lost probe cell");
    }

    #[test]
    fn window() {
        let rs = samples(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let g = plain(&graph(&rs, 4, 2));
        let rows: Vec<&str> = g.split('\n').collect();
        assert_eq!(
            rows[0].chars().count(),
            4,
            "window trims old samples"
        );
    }

    #[test]
    fn empty() {
        assert_eq!(graph(&[], 0, 0), "");
    }
}
