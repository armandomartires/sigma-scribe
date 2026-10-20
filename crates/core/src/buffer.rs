//! The text buffer: one note's body, a caret and an optional selection.
//!
//! Positions are byte offsets that always sit on a `char` boundary. Movement
//! is by `char`; a later sprint can switch to grapheme clusters without
//! changing the API.

/// Tab behaviour from Options (brief §6): a real tab or N spaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabKind {
    Tab,
    Spaces(u8),
}

impl Default for TabKind {
    fn default() -> Self {
        Self::Spaces(4)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Buffer {
    text: String,
    caret: usize,
    /// Selection anchor; the selection is `anchor..caret` (either order).
    anchor: Option<usize>,
    revision: u64,
}

impl Buffer {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self { caret: text.len(), text, anchor: None, revision: 0 }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Increases on every content change; autosave compares against it.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.anchor
            .filter(|&a| a != self.caret)
            .map(|a| (a.min(self.caret), a.max(self.caret)))
    }

    pub fn set_caret(&mut self, at: usize) {
        self.caret = self.floor(at.min(self.text.len()));
        self.anchor = None;
    }

    pub fn select(&mut self, anchor: usize, caret: usize) {
        self.anchor = Some(self.floor(anchor.min(self.text.len())));
        self.caret = self.floor(caret.min(self.text.len()));
    }

    fn floor(&self, mut at: usize) -> usize {
        while !self.text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }

    fn prev_boundary(&self, at: usize) -> usize {
        self.text[..at].char_indices().next_back().map_or(0, |(i, _)| i)
    }

    fn next_boundary(&self, at: usize) -> usize {
        self.text[at..].chars().next().map_or(at, |c| at + c.len_utf8())
    }

    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection() {
            self.text.replace_range(start..end, "");
            self.caret = start;
            self.anchor = None;
            self.revision += 1;
            true
        } else {
            self.anchor = None;
            false
        }
    }

    /// Types text at the caret, replacing any selection.
    pub fn insert(&mut self, s: &str) {
        self.delete_selection();
        if s.is_empty() {
            return;
        }
        self.text.insert_str(self.caret, s);
        self.caret += s.len();
        self.revision += 1;
    }

    pub fn insert_tab(&mut self, kind: TabKind) {
        match kind {
            TabKind::Tab => self.insert("\t"),
            TabKind::Spaces(n) => self.insert(&" ".repeat(usize::from(n))),
        }
    }

    pub fn newline(&mut self) {
        self.insert("\n");
    }

    /// Deletes the char before the caret (or the selection).
    pub fn backspace(&mut self) {
        if self.delete_selection() || self.caret == 0 {
            return;
        }
        let start = self.prev_boundary(self.caret);
        self.text.replace_range(start..self.caret, "");
        self.caret = start;
        self.revision += 1;
    }

    /// Deletes the char after the caret (or the selection).
    pub fn delete_forward(&mut self) {
        if self.delete_selection() || self.caret == self.text.len() {
            return;
        }
        let end = self.next_boundary(self.caret);
        self.text.replace_range(self.caret..end, "");
        self.revision += 1;
    }

    pub fn left(&mut self) {
        self.caret = match self.selection() {
            Some((s, _)) => s,
            None => self.prev_boundary(self.caret),
        };
        self.anchor = None;
    }

    pub fn right(&mut self) {
        self.caret = match self.selection() {
            Some((_, e)) => e,
            None => self.next_boundary(self.caret),
        };
        self.anchor = None;
    }

    /// Start of the logical line containing `at`.
    #[must_use]
    pub fn line_start(&self, at: usize) -> usize {
        self.text[..at].rfind('\n').map_or(0, |i| i + 1)
    }

    /// End of the logical line containing `at` (before its `\n`).
    #[must_use]
    pub fn line_end(&self, at: usize) -> usize {
        self.text[at..].find('\n').map_or(self.text.len(), |i| at + i)
    }

    /// Moves within logical lines, keeping the char column.
    pub fn up(&mut self) {
        let start = self.line_start(self.caret);
        if start == 0 {
            self.set_caret(0);
            return;
        }
        let col = self.text[start..self.caret].chars().count();
        let prev_start = self.line_start(start - 1);
        let target = Self::advance(&self.text[prev_start..start - 1], col);
        self.set_caret(prev_start + target);
    }

    pub fn down(&mut self) {
        let start = self.line_start(self.caret);
        let end = self.line_end(self.caret);
        if end == self.text.len() {
            self.set_caret(end);
            return;
        }
        let col = self.text[start..self.caret].chars().count();
        let next_start = end + 1;
        let next_end = self.line_end(next_start);
        let target = Self::advance(&self.text[next_start..next_end], col);
        self.set_caret(next_start + target);
    }

    fn advance(line: &str, cols: usize) -> usize {
        line.char_indices().nth(cols).map_or(line.len(), |(i, _)| i)
    }

    /// 1-based logical line number of the caret, for the status bar.
    #[must_use]
    pub fn line_number(&self) -> usize {
        self.text[..self.caret].matches('\n').count() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_backspace() {
        let mut b = Buffer::new("");
        b.insert("héllo");
        assert_eq!(b.text(), "héllo");
        b.backspace();
        b.backspace();
        b.backspace();
        b.backspace();
        assert_eq!(b.text(), "h");
        assert_eq!(b.caret(), 1);
        b.backspace();
        b.backspace();
        assert_eq!(b.text(), "");
    }

    #[test]
    fn delete_forward_and_selection() {
        let mut b = Buffer::new("abcdef");
        b.set_caret(1);
        b.delete_forward();
        assert_eq!(b.text(), "acdef");
        b.select(1, 4);
        b.insert("X");
        assert_eq!(b.text(), "aXf");
        assert_eq!(b.caret(), 2);
        b.select(0, 3);
        b.backspace();
        assert_eq!(b.text(), "");
    }

    #[test]
    fn tabs() {
        let mut b = Buffer::new("");
        b.insert_tab(TabKind::Spaces(4));
        b.insert_tab(TabKind::Tab);
        assert_eq!(b.text(), "    \t");
    }

    #[test]
    fn vertical_movement_keeps_column() {
        let mut b = Buffer::new("abcdef\nxy\n12345");
        b.set_caret(4); // after "abcd"
        b.down();
        assert_eq!(b.caret(), 9); // end of "xy"
        b.down();
        assert_eq!(b.caret(), 12); // "12" + col 2
        assert_eq!(b.line_number(), 3);
        b.up();
        b.up();
        assert_eq!(b.caret(), 2);
        b.up();
        assert_eq!(b.caret(), 0);
    }

    #[test]
    fn caret_never_splits_a_char() {
        let mut b = Buffer::new("aé");
        b.set_caret(2); // inside é
        assert_eq!(b.caret(), 1);
        b.right();
        assert_eq!(b.caret(), 3);
        b.left();
        b.left();
        b.left();
        assert_eq!(b.caret(), 0);
    }

    #[test]
    fn revision_tracks_changes_only() {
        let mut b = Buffer::new("a");
        let r = b.revision();
        b.left();
        b.right();
        assert_eq!(b.revision(), r);
        b.insert("b");
        assert!(b.revision() > r);
    }
}
