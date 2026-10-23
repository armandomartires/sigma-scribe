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
    /// Undo snapshots (text, caret), oldest first; capped at [`UNDO_LIMIT`].
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
    /// Kind of the last edit, for coalescing typing into word-sized steps.
    last: Edit,
}

/// Undo steps kept per open note.
pub const UNDO_LIMIT: usize = 200;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Edit {
    /// Next edit starts a new undo step.
    #[default]
    Break,
    Typing,
    Deleting,
}

impl Buffer {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            caret: text.len(),
            text,
            anchor: None,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            last: Edit::Break,
        }
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
        self.last = Edit::Break;
    }

    /// Moves the caret; with `extend` the selection grows from the anchor.
    pub fn move_to(&mut self, at: usize, extend: bool) {
        if extend {
            let anchor = self.anchor.unwrap_or(self.caret);
            self.select(anchor, at);
        } else {
            self.set_caret(at);
        }
        self.last = Edit::Break;
    }

    pub fn select_all(&mut self) {
        self.select(0, self.text.len());
    }

    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        self.selection().map(|(a, b)| &self.text[a..b])
    }

    /// The word (alphanumeric run, plus `'` / `-` inside it) at or just
    /// before `at`; `None` on whitespace or punctuation (long-press).
    #[must_use]
    pub fn word_at(&self, at: usize) -> Option<(usize, usize)> {
        let at = self.floor(at.min(self.text.len()));
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let joiner = |c: char| c == '\'' || c == '-' || c == '\u{2019}';
        let here = self.text[at..].chars().next().filter(|&c| is_word(c));
        let probe = if here.is_some() {
            at
        } else {
            let p = self.prev_boundary(at);
            if at > 0 && self.text[p..].chars().next().is_some_and(is_word) {
                // Pressing just after a word's last letter still picks it.
                p
            } else {
                return None;
            }
        };
        let mut a = probe;
        while a > 0 {
            let p = self.prev_boundary(a);
            let c = self.text[p..].chars().next().unwrap_or(' ');
            let joined = joiner(c)
                && p > 0
                && self.text[self.prev_boundary(p)..]
                    .chars()
                    .next()
                    .is_some_and(is_word);
            if is_word(c) || joined {
                a = p;
            } else {
                break;
            }
        }
        let mut b = probe;
        while b < self.text.len() {
            let c = self.text[b..].chars().next().unwrap_or(' ');
            let n = self.next_boundary(b);
            let joined = joiner(c) && self.text[n..].chars().next().is_some_and(is_word);
            if is_word(c) || joined {
                b = n;
            } else {
                break;
            }
        }
        (a < b).then_some((a, b))
    }

    /// Start of the previous word (Ctrl+Left).
    #[must_use]
    pub fn word_left(&self, at: usize) -> usize {
        let before = &self.text[..at];
        let trimmed = before.trim_end_matches(|c: char| !c.is_alphanumeric());
        trimmed
            .char_indices()
            .rev()
            .find(|(_, c)| !c.is_alphanumeric())
            .map_or(0, |(i, c)| i + c.len_utf8())
    }

    /// End of the next word (Ctrl+Right).
    #[must_use]
    pub fn word_right(&self, at: usize) -> usize {
        let after = &self.text[at..];
        let skip = after.len()
            - after
                .trim_start_matches(|c: char| !c.is_alphanumeric())
                .len();
        let rest = &after[skip..];
        at + skip
            + rest
                .find(|c: char| !c.is_alphanumeric())
                .unwrap_or(rest.len())
    }

    /// Previous char boundary (for Shift+Left).
    #[must_use]
    pub fn prev_pos(&self, at: usize) -> usize {
        self.prev_grapheme(at)
    }

    #[must_use]
    pub fn next_pos(&self, at: usize) -> usize {
        self.next_grapheme(at)
    }

    fn record(&mut self, kind: Edit) {
        self.redo.clear();
        if kind == Edit::Break || kind != self.last {
            self.undo.push((self.text.clone(), self.caret));
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
        }
        self.last = kind;
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Restores the text before the last step. Returns false if none.
    pub fn undo(&mut self) -> bool {
        let Some((t, c)) = self.undo.pop() else {
            return false;
        };
        self.redo
            .push((std::mem::replace(&mut self.text, t), self.caret));
        self.caret = self.floor(c.min(self.text.len()));
        self.anchor = None;
        self.revision += 1;
        self.last = Edit::Break;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some((t, c)) = self.redo.pop() else {
            return false;
        };
        self.undo
            .push((std::mem::replace(&mut self.text, t), self.caret));
        self.caret = self.floor(c.min(self.text.len()));
        self.anchor = None;
        self.revision += 1;
        self.last = Edit::Break;
        true
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

    /// Start of the extended grapheme cluster before `at` (v1.9): the caret
    /// and Delete move over a letter with its marks (क्षि, é, ก็), Backspace
    /// still takes one code point.
    fn prev_grapheme(&self, at: usize) -> usize {
        use unicode_segmentation::GraphemeCursor;
        GraphemeCursor::new(at, self.text.len(), true)
            .prev_boundary(&self.text, 0)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.prev_boundary(at))
    }

    fn next_grapheme(&self, at: usize) -> usize {
        use unicode_segmentation::GraphemeCursor;
        GraphemeCursor::new(at, self.text.len(), true)
            .next_boundary(&self.text, 0)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.next_boundary(at))
    }

    fn prev_boundary(&self, at: usize) -> usize {
        self.text[..at]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    fn next_boundary(&self, at: usize) -> usize {
        self.text[at..]
            .chars()
            .next()
            .map_or(at, |c| at + c.len_utf8())
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

    /// Replaces the whole text as one undo step (Replace All) and puts the
    /// caret at `caret`.
    pub fn replace_text(&mut self, text: String, caret: usize) {
        if text == self.text {
            return;
        }
        self.record(Edit::Break);
        self.text = text;
        self.anchor = None;
        self.caret = self.floor(caret.min(self.text.len()));
        self.revision += 1;
    }

    /// Types text at the caret, replacing any selection.
    pub fn insert(&mut self, s: &str) {
        if s.is_empty() && self.selection().is_none() {
            return;
        }
        let word_char = s.chars().count() == 1 && s.chars().all(char::is_alphanumeric);
        self.record(if self.selection().is_some() || !word_char {
            Edit::Break
        } else {
            Edit::Typing
        });
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
        if self.selection().is_none() && self.caret == 0 {
            return;
        }
        self.record(if self.selection().is_some() {
            Edit::Break
        } else {
            Edit::Deleting
        });
        if self.delete_selection() {
            return;
        }
        let start = self.prev_boundary(self.caret);
        self.text.replace_range(start..self.caret, "");
        self.caret = start;
        self.revision += 1;
    }

    /// Deletes the char after the caret (or the selection).
    pub fn delete_forward(&mut self) {
        if self.selection().is_none() && self.caret == self.text.len() {
            return;
        }
        self.record(if self.selection().is_some() {
            Edit::Break
        } else {
            Edit::Deleting
        });
        if self.delete_selection() {
            return;
        }
        let end = self.next_grapheme(self.caret);
        self.text.replace_range(self.caret..end, "");
        self.revision += 1;
    }

    pub fn left(&mut self) {
        self.caret = match self.selection() {
            Some((s, _)) => s,
            None => self.prev_grapheme(self.caret),
        };
        self.anchor = None;
        self.last = Edit::Break;
    }

    pub fn right(&mut self) {
        self.caret = match self.selection() {
            Some((_, e)) => e,
            None => self.next_grapheme(self.caret),
        };
        self.anchor = None;
        self.last = Edit::Break;
    }

    /// Start of the logical line containing `at`.
    #[must_use]
    pub fn line_start(&self, at: usize) -> usize {
        self.text[..at].rfind('\n').map_or(0, |i| i + 1)
    }

    /// End of the logical line containing `at` (before its `\n`).
    #[must_use]
    pub fn line_end(&self, at: usize) -> usize {
        self.text[at..]
            .find('\n')
            .map_or(self.text.len(), |i| at + i)
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
    #[test]
    fn the_caret_moves_by_cluster_and_backspace_by_code_point() {
        let mut b = Buffer::new("कीक्षि");
        b.set_caret(b.text().len());
        b.left();
        assert_eq!(&b.text()[b.caret()..], "क्षि");
        b.left();
        assert_eq!(b.caret(), 0);
        b.right();
        assert_eq!(&b.text()[..b.caret()], "की");
        b.delete_forward();
        assert_eq!(b.text(), "की");
        // Backspace takes the sign only, then the letter.
        b.set_caret(b.text().len());
        b.backspace();
        assert_eq!(b.text(), "क");
        let mut t = Buffer::new("ก็");
        t.set_caret(t.text().len());
        t.left();
        assert_eq!(t.caret(), 0);
    }

    use super::*;

    #[test]
    fn word_at_picks_words_and_refuses_space() {
        let b = Buffer::new("Don't stop — re-read it, ünïcode!");
        let w = |at| b.word_at(at).map(|(x, y)| &b.text()[x..y]);
        assert_eq!(w(0), Some("Don't"));
        assert_eq!(w(3), Some("Don't"));
        assert_eq!(w(5), Some("Don't"), "just after the last letter");
        assert_eq!(w(7), Some("stop"));
        let rr = b.text().find("re-read").unwrap();
        assert_eq!(w(rr + 4), Some("re-read"));
        let u = b.text().find("ünïcode").unwrap();
        assert_eq!(w(u + 3), Some("ünïcode"));
        let dash = b.text().find('—').unwrap();
        assert_eq!(
            w(dash + 1),
            None,
            "inside a multibyte char snaps, punctuation has no word"
        );
        let sp = Buffer::new("a   b");
        assert_eq!(sp.word_at(2), None, "whitespace");
        assert_eq!(Buffer::new("").word_at(0), None);
    }

    #[test]
    fn undo_groups_words_and_redo_restores() {
        let mut b = Buffer::new("");
        for c in "hello world".chars() {
            b.insert(&c.to_string());
        }
        assert!(b.undo()); // "world"
        assert_eq!(b.text(), "hello ");
        assert!(b.undo()); // " "
        assert!(b.undo()); // "hello"
        assert_eq!(b.text(), "");
        assert!(!b.undo());
        assert!(b.redo() && b.redo() && b.redo());
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.caret(), 11);
        b.backspace();
        b.backspace();
        b.undo();
        assert_eq!(b.text(), "hello world", "a run of backspaces is one step");
        b.undo();
        b.insert("X");
        assert!(!b.redo(), "a new edit clears redo");
    }

    #[test]
    fn undo_is_capped_and_bumps_revision() {
        let mut b = Buffer::new("");
        for _ in 0..UNDO_LIMIT + 50 {
            b.insert(" ");
        }
        let mut n = 0;
        let r = b.revision();
        while b.undo() {
            n += 1;
        }
        assert_eq!(n, UNDO_LIMIT);
        assert!(b.revision() > r);
    }

    #[test]
    fn word_jumps_and_selection_extend() {
        let mut b = Buffer::new("one, two  three");
        assert_eq!(b.word_left(15), 10);
        assert_eq!(b.word_left(10), 5);
        assert_eq!(b.word_left(5), 0);
        assert_eq!(b.word_right(0), 3);
        assert_eq!(b.word_right(3), 8);
        assert_eq!(b.word_right(8), 15);
        b.set_caret(5);
        b.move_to(8, true);
        b.move_to(15, true);
        assert_eq!(b.selected_text(), Some("two  three"));
        b.move_to(3, true);
        assert_eq!(b.selected_text(), Some(", "));
        b.select_all();
        assert_eq!(b.selected_text(), Some("one, two  three"));
        b.insert("x");
        assert_eq!(b.text(), "x");
        b.undo();
        assert_eq!(b.text(), "one, two  three");
        let u = Buffer::new("é ü");
        assert_eq!(u.word_left(u.text().len()), 3);
    }

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
