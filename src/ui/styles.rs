//! ANSI SGR styles. Basic colors only, so pulse inherits the user's
//! terminal theme.

#[derive(Clone, Copy)]
pub struct Style(&'static str);

pub const BRAND: Style = Style("1;34"); // bold blue
pub const DIM: Style = Style("90"); // labels, hints
pub const VALUE: Style = Style(""); // plain foreground
pub const OK: Style = Style("32");
pub const OK_BOLD: Style = Style("1;32");
pub const WARN: Style = Style("33");
pub const BAD: Style = Style("31");
pub const BAD_BOLD: Style = Style("1;31");
pub const GRAPH: Style = Style("34");

impl Style {
    pub fn render(&self, text: &str) -> String {
        if self.0.is_empty() {
            return text.to_string();
        }
        format!("\x1b[{}m{}\x1b[0m", self.0, text)
    }
}
