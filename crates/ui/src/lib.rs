//! Sigma Scribe screens, built with Cobalt's `ScreenBuilder`.
//!
//! v0.1 (S1) draws the page as Cobalt's measured monospace grid with a block
//! caret, because the SDK has no free-form drawing, no font choice and no
//! multi-line text field (see docs/s0-findings.md). Everything that is not
//! drawing lives in `sigma-core`.

use kobo_sdk::{
    action_id, terminal_grid_for, ActionId, Caret, DisplayMetrics, Screen, ScreenBuilder,
};
use sigma_core::keyboard::Key;
use sigma_core::wrap::{caret_cell, offset_at, wrap, MonoMeasure, VisualLine};
use sigma_core::{Buffer, Keyboard, SaveState};

/// Kobo Forma panel (device code 377): 1440×1920 at 300 ppi.
pub const FORMA_METRICS: DisplayMetrics = DisplayMetrics {
    width: 1440,
    height: 1920,
    pixels_per_inch: 300,
    text_scale: kobo_ui::TextScale::Default,
};

pub const KEYS: &str = "keys";
pub const LEFT: &str = "nav-left";
pub const RIGHT: &str = "nav-right";
pub const UP: &str = "nav-up";
pub const DOWN: &str = "nav-down";

/// What a tap meant, resolved from an opaque `ActionId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Key(Key),
    ToggleKeys,
    Left,
    Right,
    Up,
    Down,
}

fn key_name(row: usize, col: usize) -> String {
    format!("kb.r{row}c{col}")
}

/// Maps an action back to a key (positional) or a control.
#[must_use]
pub fn resolve(action: ActionId, keyboard: &Keyboard) -> Option<Intent> {
    for (name, intent) in [
        (KEYS, Intent::ToggleKeys),
        (LEFT, Intent::Left),
        (RIGHT, Intent::Right),
        (UP, Intent::Up),
        (DOWN, Intent::Down),
    ] {
        if action == action_id(name) {
            return Some(intent);
        }
    }
    for (r, row) in keyboard.rows().iter().enumerate() {
        for (c, &key) in row.iter().enumerate() {
            if action == action_id(&key_name(r, c)) {
                return Some(Intent::Key(key));
            }
        }
    }
    None
}

/// Everything the editor screen shows.
pub struct EditorView<'a> {
    pub file_name: &'a str,
    pub buffer: &'a Buffer,
    pub keyboard: &'a Keyboard,
    pub keys_on: bool,
    pub save: &'a SaveState,
    /// First visual row on screen; updated by [`EditorView::build`].
    pub scroll: usize,
}

fn chrome<'b>(builder: ScreenBuilder, view: &EditorView<'b>) -> ScreenBuilder {
    builder
        .top_bar(view.file_name)
        .top_bar_action(KEYS, if view.keys_on { "Hide keys" } else { "Keys" })
}

fn status(view: &EditorView<'_>) -> String {
    format!(
        "{} words · line {}      Paper · {}",
        sigma_core::text::word_count(view.buffer.text()),
        view.buffer.line_number(),
        match view.save {
            SaveState::Failed(why) => format!("not saved: {why}"),
            other => other.label().to_owned(),
        }
    )
}

fn keyboard_rows(mut b: ScreenBuilder, kb: &Keyboard) -> ScreenBuilder {
    for (r, row) in kb.rows().iter().enumerate() {
        let cells: Vec<(String, String, bool)> = row
            .iter()
            .enumerate()
            .map(|(c, &k)| (key_name(r, c), kb.label(k), k == Key::Shift && kb.shift))
            .collect();
        let cols = u8::try_from(cells.len()).unwrap_or(10);
        b = b.grid_with_selection(cols, false, cells);
    }
    b
}

fn tail(b: ScreenBuilder, view: &EditorView<'_>) -> ScreenBuilder {
    // `fill` anchors status and keys to the foot; the page takes the rest.
    let b = b.fill().secondary(status(view)).grid(
        4,
        false,
        [(LEFT, "←"), (RIGHT, "→"), (UP, "↑"), (DOWN, "↓")],
    );
    if view.keys_on {
        keyboard_rows(b, view.keyboard)
    } else {
        b
    }
}

impl EditorView<'_> {
    /// Grid the page gets on this panel with the current chrome.
    #[must_use]
    pub fn grid(&self, metrics: &DisplayMetrics) -> (u16, u16) {
        let probe = tail(
            chrome(ScreenBuilder::new("editor"), self).terminal(Vec::<String>::new(), None),
            self,
        )
        .build();
        terminal_grid_for(&probe, metrics)
    }

    /// Visual rows of the current text at the measured grid width.
    #[must_use]
    pub fn rows(&self, columns: u16) -> Vec<VisualLine> {
        wrap(
            self.buffer.text(),
            u32::from(columns.max(1)),
            &MonoMeasure::default(),
        )
    }

    /// Builds the screen and keeps the caret row in view.
    pub fn build(&mut self, metrics: &DisplayMetrics) -> Screen {
        let (columns, height) = self.grid(metrics);
        let height = usize::from(height.max(1));
        let m = MonoMeasure::default();
        let text = self.buffer.text();
        let rows = self.rows(columns);
        let (crow, ccol) = caret_cell(text, &rows, self.buffer.caret(), &m);
        if crow < self.scroll {
            self.scroll = crow;
        } else if crow >= self.scroll + height {
            self.scroll = crow + 1 - height;
        }
        let shown: Vec<String> = rows
            .iter()
            .skip(self.scroll)
            .take(height)
            .map(|r| text[r.start..r.end].replace('\t', "    "))
            .collect();
        let caret = Caret::new(
            u16::try_from(crow - self.scroll).unwrap_or(0),
            u16::try_from(ccol).unwrap_or(0),
        );
        tail(
            chrome(ScreenBuilder::new("editor"), self).terminal(shown, Some(caret)),
            self,
        )
        .build()
    }
}

/// Moves the caret one visual row (soft-wrap aware).
pub fn move_vertical(buffer: &mut Buffer, columns: u16, down: bool) {
    let m = MonoMeasure::default();
    let rows = wrap(buffer.text(), u32::from(columns.max(1)), &m);
    let (row, col) = caret_cell(buffer.text(), &rows, buffer.caret(), &m);
    let target = if down { row + 1 } else { row.wrapping_sub(1) };
    match rows.get(target) {
        Some(&r) => {
            let at = offset_at(buffer.text(), r, col, &m);
            buffer.set_caret(at);
        }
        None if down => buffer.set_caret(buffer.text().len()),
        None => buffer.set_caret(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view<'a>(b: &'a Buffer, k: &'a Keyboard, s: &'a SaveState) -> EditorView<'a> {
        EditorView {
            file_name: "field-notes.md",
            buffer: b,
            keyboard: k,
            keys_on: true,
            save: s,
            scroll: 0,
        }
    }

    #[test]
    fn forma_grid_is_usable() {
        let (b, k, s) = (Buffer::new(""), Keyboard::default(), SaveState::Saved);
        let v = view(&b, &k, &s);
        let (cols, rows) = v.grid(&FORMA_METRICS);
        assert!(cols >= 40, "columns {cols}");
        assert!(rows >= 8, "rows {rows}");
    }

    #[test]
    fn screen_builds_without_layout_errors_on_forma() {
        let (b, k, s) = (
            Buffer::new("# Field notes\n\nA page about the screen."),
            Keyboard::default(),
            SaveState::Saved,
        );
        let mut v = view(&b, &k, &s);
        let screen = v.build(&FORMA_METRICS);
        let issues = screen.validate(&FORMA_METRICS);
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == kobo_sdk::DiagnosticSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn every_key_resolves_positionally() {
        let k = Keyboard::default();
        for (r, row) in k.rows().iter().enumerate() {
            for (c, &key) in row.iter().enumerate() {
                assert_eq!(
                    resolve(action_id(&key_name(r, c)), &k),
                    Some(Intent::Key(key))
                );
            }
        }
        assert_eq!(resolve(action_id(KEYS), &k), Some(Intent::ToggleKeys));
    }

    #[test]
    fn caret_stays_in_view() {
        let long = "line\n".repeat(200);
        let (b, k, s) = (Buffer::new(long), Keyboard::default(), SaveState::Saved);
        let mut v = view(&b, &k, &s);
        let _ = v.build(&FORMA_METRICS);
        let (_, h) = v.grid(&FORMA_METRICS);
        assert!(v.scroll + usize::from(h) > 200);
    }

    #[test]
    fn vertical_moves_follow_soft_wrap() {
        let mut b = Buffer::new("aaaa bbbb cccc");
        b.set_caret(0);
        move_vertical(&mut b, 5, true);
        assert_eq!(b.caret(), 5);
        move_vertical(&mut b, 5, true);
        assert_eq!(b.caret(), 10);
        move_vertical(&mut b, 5, false);
        assert_eq!(b.caret(), 5);
    }
}
