//! In-place repainting of the live view.

use std::io::Write;

/// Repaints a block of lines in place: it moves the cursor back to the top of
/// the previous frame, clears each line and writes the new one. The terminal
/// is in raw mode, so lines end with \r\n.
pub struct Screen<W: Write> {
    out: W,
    lines: usize,
}

impl<W: Write> Screen<W> {
    pub fn new(out: W) -> Self {
        Self { out, lines: 0 }
    }

    pub fn paint(&mut self, frame: &str) {
        let lines: Vec<&str> = frame.split('\n').collect();
        let mut b = String::new();
        if self.lines > 0 {
            b.push_str(&format!("\x1b[{}F", self.lines));
        }
        for l in &lines {
            b.push_str("\x1b[2K");
            b.push_str(l);
            b.push_str("\r\n");
        }
        b.push_str("\x1b[J"); // clear leftovers when the frame shrank
        self.lines = lines.len();
        let _ = self.out.write_all(b.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint_all(frames: &[&str]) -> String {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut s = Screen::new(&mut buf);
            for f in frames {
                s.paint(f);
            }
        }
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn first_frame_has_no_cursor_move() {
        let out = paint_all(&["a\nb"]);
        assert_eq!(out, "\x1b[2Ka\r\n\x1b[2Kb\r\n\x1b[J");
    }

    #[test]
    fn later_frames_rewind_by_previous_line_count() {
        let out = paint_all(&["a\nb", "c"]);
        assert!(out.ends_with("\x1b[2F\x1b[2Kc\r\n\x1b[J"), "got {out:?}");
    }
}
