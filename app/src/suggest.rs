//! Wiki-link autocomplete (S23): the suggestion bar that follows `[[` and
//! `[[note#`, on the strip the Japanese candidates use.
//!
//! The matching is `sigma_core::complete`; this module decides when the bar
//! is up, keeps the cached note names while it is, and applies a pick as
//! one undo step.

use crate::App;
use sigma_core::complete::{link_query, suggest_headings, suggest_notes, Suggestion};
use sigma_core::{links, NotePath, NoteStore};
use sigma_device::{HwKey, Refresh};
use sigma_render::Rect;

/// Chips offered at most; the bar scrolls to the chosen one.
const CAP: usize = 12;

/// What a bar keeps from the one before it while the bar stays up: the
/// library's note paths and the heading source.
type Cache = (Option<Vec<NotePath>>, Option<(NotePath, String)>);

/// The suggestion bar while it is up.
pub struct SuggestState {
    /// Byte offset where the query being replaced starts.
    pub start: usize,
    /// The chips, best first.
    pub chips: Vec<Suggestion>,
    /// The chips' labels, as the renderer takes them.
    pub labels: Vec<String>,
    /// The chosen chip (USB Left / Right).
    pub sel: usize,
    /// The library's note paths, read when the bar opened (no note is read
    /// for them); dropped with the bar and when the library changes.
    notes: Option<Vec<NotePath>>,
    /// The note whose headings are suggested, and its text.
    head: Option<(NotePath, String)>,
}

/// The fence a code block opens with on `line`, as the parser sees it (up
/// to three columns of indent).
fn fence_open(line: &str) -> Option<&'static str> {
    let line = line.trim_end();
    let indent: usize = line
        .chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum();
    let t = line.trim_start();
    if indent >= 4 {
        None
    } else if t.starts_with("```") {
        Some("```")
    } else if t.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

/// Whether the line starting at byte `line_start` is inside a fenced code
/// block opened above it.
fn in_fence(text: &str, line_start: usize) -> bool {
    let mut fence: Option<&str> = None;
    for l in text[..line_start].lines() {
        fence = match fence {
            Some(f) if l.trim_start().starts_with(f) => None,
            Some(f) => Some(f),
            None => fence_open(l),
        };
    }
    fence.is_some()
}

impl App {
    /// The bar's chips and the chosen one, for the `Model`.
    pub(crate) fn suggest_chips(&self) -> Option<(&[String], usize)> {
        self.suggest.as_ref().map(|s| (&s.labels[..], s.sel))
    }

    /// Where the bar may show at all.
    fn suggest_allowed(&self) -> bool {
        self.in_editor()
            && self.find.is_none()
            && self.ink.is_none()
            && !self.scratch_focused()
            && !self.read_only
            && self.buffer.selection().is_none()
            && self.keyboard.composer.candidates().is_none()
    }

    /// Brings the bar in step with the text and the caret: opens it, closes
    /// it or changes its chips. `Partial` when it appeared or went (the text
    /// area changes size), `Fast` when only the chips changed, `None` when
    /// nothing did. Cheap when the text and caret are as last time.
    pub(crate) fn refresh_suggest(&mut self) -> Option<Refresh> {
        if !self.suggest_allowed() {
            self.suggest_seen = None;
            return self.suggest.take().map(|_| Refresh::Partial);
        }
        let key = (
            self.current.clone(),
            self.buffer.revision(),
            self.buffer.caret(),
        );
        if self.suggest_seen.as_ref() == Some(&key) {
            return None;
        }
        self.suggest_seen = Some(key);
        let old = self.suggest.take();
        let cache = old.as_ref().map(|o| (o.notes.clone(), o.head.clone()));
        let new = self.compute_suggest(cache);
        let r = match (&old, &new) {
            (None, None) => None,
            (Some(_), None) | (None, Some(_)) => Some(Refresh::Partial),
            (Some(a), Some(b)) => {
                (a.labels != b.labels || a.start != b.start).then_some(Refresh::Fast)
            }
        };
        // The same chips for the same query keep their selection.
        self.suggest = match (old, new) {
            (Some(a), Some(mut b)) if a.labels == b.labels && a.start == b.start => {
                b.sel = a.sel;
                Some(b)
            }
            (_, b) => b,
        };
        r
    }

    /// The bar for the text before the caret, if there is one. `cache` is
    /// the previous bar's, kept while the bar stays up.
    fn compute_suggest(&self, cache: Option<Cache>) -> Option<SuggestState> {
        let text = self.buffer.text();
        let caret = self.buffer.caret();
        let line = self.buffer.line_start(caret);
        let (at, q) = link_query(&text[line..caret])?;
        if in_fence(text, line) {
            return None;
        }
        let start = line + at;
        let (mut notes, mut head) = cache.unwrap_or((None, None));
        if notes.is_none() {
            notes = Some(self.library.note_paths().unwrap_or_default());
        }
        let paths = notes.as_deref().unwrap_or_default();
        let (span, chips) = if let Some((note, h)) = q.split_once('#') {
            // A bare `[[#` names this note.
            let src = if links::note_part(note).is_empty() {
                text.to_owned()
            } else {
                let p = links::resolve(note, paths)?.clone();
                match &head {
                    Some((hp, t)) if *hp == p => t.clone(),
                    _ => {
                        let t = self.store.load(p.as_str()).ok().flatten()?;
                        head = Some((p, t.clone()));
                        t
                    }
                }
            };
            (start + note.len() + 1, suggest_headings(&src, h, CAP))
        } else {
            let mut recent: Vec<NotePath> = self
                .history
                .iter()
                .rev()
                .map(|h| h.note.clone())
                .chain(self.library.last_opened())
                .filter(|p| *p != self.current)
                .collect();
            recent.dedup();
            (start, suggest_notes(q, paths, &recent, CAP))
        };
        if chips.is_empty() {
            return None;
        }
        Some(SuggestState {
            start: span,
            labels: chips.iter().map(|c| c.label.clone()).collect(),
            chips,
            sel: 0,
            notes,
            head,
        })
    }

    /// The library changed (a note arrived by sync): the cached names and
    /// heading source are read again at the next look.
    pub(crate) fn drop_suggest_cache(&mut self) {
        if let Some(s) = &mut self.suggest {
            s.notes = None;
            s.head = None;
        }
        self.suggest_seen = None;
    }

    /// A chip was picked (tap or USB): the query becomes the choice and
    /// `]]`, unless `]]` already follows the caret, and the caret goes past
    /// them. One undo step.
    pub(crate) fn pick_suggest(&mut self, i: usize) -> Option<Refresh> {
        let st = self.suggest.take()?;
        let chip = st.chips.get(i)?;
        let text = self.buffer.text();
        let caret = self.buffer.caret();
        let (head, tail) = (&text[..st.start.min(caret)], &text[caret..]);
        let close = if tail.starts_with("]]") { "" } else { "]]" };
        let new = format!("{head}{}{close}{tail}", chip.insert);
        let at = head.len() + chip.insert.len() + 2;
        self.buffer.replace_text(new, at);
        self.sel_ui = None;
        self.save();
        // The caret is past the link: no bar until the next edit.
        self.suggest_seen = Some((
            self.current.clone(),
            self.buffer.revision(),
            self.buffer.caret(),
        ));
        Some(Refresh::Partial)
    }

    /// A USB key while the bar is up: Left / Right choose, Enter and Tab
    /// pick, Esc closes.
    pub(crate) fn suggest_key(&mut self, k: HwKey) -> Option<Refresh> {
        let s = self.suggest.as_mut()?;
        match k {
            HwKey::Left | HwKey::Right => {
                let was = s.sel;
                s.sel = if k == HwKey::Left {
                    s.sel.saturating_sub(1)
                } else {
                    (s.sel + 1).min(s.chips.len().saturating_sub(1))
                };
                (s.sel != was).then_some(Refresh::Fast)
            }
            HwKey::Return | HwKey::Tab => {
                let i = s.sel;
                self.pick_suggest(i)
            }
            _ => {
                self.suggest = None;
                Some(Refresh::Partial)
            }
        }
    }

    /// Whether a USB key belongs to the bar.
    pub(crate) fn suggest_takes(&self, k: HwKey) -> bool {
        self.suggest.is_some()
            && matches!(
                k,
                HwKey::Left | HwKey::Right | HwKey::Return | HwKey::Tab | HwKey::Escape
            )
    }

    /// The chips' rectangles as drawn (the strip's geometry is the
    /// renderer's).
    pub(crate) fn suggest_rects(&self) -> Vec<(usize, Rect)> {
        if self.suggest.is_none() {
            return Vec::new();
        }
        self.screen.cand_rects(&self.editor_model())
    }
}
