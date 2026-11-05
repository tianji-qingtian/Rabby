//! TerminalCore facade over wezterm-term. No GPUI, no PTY.

use std::io::Write;
use std::sync::Arc;

use anyhow::Result;
use wezterm_term::color::ColorPalette;
pub use wezterm_term::input::{KeyCode, KeyModifiers};
use wezterm_term::{Terminal, TerminalConfiguration, TerminalSize, UnicodeVersion};

#[derive(Debug)]
struct CoreConfig {
    scrollback: usize,
}

impl TerminalConfiguration for CoreConfig {
    fn scrollback_size(&self) -> usize {
        self.scrollback
    }

    fn color_palette(&self) -> ColorPalette {
        ColorPalette::default()
    }

    fn unicode_version(&self) -> UnicodeVersion {
        UnicodeVersion::new(11)
    }

    fn enable_kitty_keyboard(&self) -> bool {
        false
    }

    fn enable_kitty_graphics(&self) -> bool {
        true
    }
}

fn term_size(cols: usize, rows: usize) -> TerminalSize {
    TerminalSize {
        rows,
        cols,
        pixel_width: cols * 8,
        pixel_height: rows * 17,
        dpi: 96,
    }
}

pub struct TerminalCore {
    term: Terminal,
}

impl TerminalCore {
    pub fn new(cols: usize, rows: usize, scrollback: usize, writer: Box<dyn Write + Send>) -> Self {
        let term = Terminal::new(
            term_size(cols, rows),
            Arc::new(CoreConfig { scrollback }),
            "Rabby",
            "0.1.0",
            writer,
        );
        Self { term }
    }

    pub fn new_headless(cols: usize, rows: usize, scrollback: usize) -> Self {
        Self::new(cols, rows, scrollback, Box::new(Vec::new()))
    }

    pub fn key_down(&mut self, key: KeyCode, mods: KeyModifiers) -> Result<()> {
        self.term.key_down(key, mods)
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.term.advance_bytes(bytes);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.term.resize(term_size(cols, rows));
    }

    pub fn title(&self) -> String {
        self.term.get_title().to_string()
    }

    pub fn visible_rows(&self) -> Vec<String> {
        let screen = self.term.screen();
        let top = screen.phys_row(0);
        screen
            .lines_in_phys_range(top..top + screen.physical_rows)
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_plain_text() {
        let mut core = TerminalCore::new_headless(80, 24, 1000);
        core.feed(b"hello");
        let rows = core.visible_rows();
        assert_eq!(rows.first().map(String::as_str), Some("hello"));
    }

    #[test]
    fn handles_clear_and_cursor_home() {
        let mut core = TerminalCore::new_headless(80, 24, 1000);
        core.feed(b"old\x1b[2J\x1b[Hnew");
        let rows = core.visible_rows();
        assert!(rows.first().map(|row| row.contains("new")).unwrap_or(false));
        assert!(!rows.first().map(|row| row.contains("old")).unwrap_or(true));
    }

    #[test]
    fn resizes_the_screen_model() {
        let mut core = TerminalCore::new_headless(80, 24, 1000);
        core.resize(100, 30);
        assert_eq!(core.visible_rows().len(), 30);
    }
}
