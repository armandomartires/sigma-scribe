//! The Find and replace bar, and Search across all notes (S22.1).

use crate::App;
use sigma_core::buffer::TabKind;
use sigma_core::keyboard::{Dir, Key};
use sigma_core::search::{SearchRequest, SearchResult};
use sigma_core::{Buffer, KeyAction, NotePath};
use sigma_device::{HwKey, Refresh};
use sigma_render::{
    EditorScreen, FindField, FindHit, Scope, SearchLayout, SearchListHit, SearchListModel,
    SearchRow, SearchState,
};

/// At most this many matches are collected for one query (S22.1): the
/// status line says "first 500 shown" when the cap was hit.
pub const SEARCH_CAP: usize = 500;

impl App {
    pub(crate) fn find_typing(&self) -> bool {
        self.in_editor() && self.find.as_ref().is_some_and(|f| f.focus.is_some())
    }

    pub(crate) fn in_find_bar(&self, x: i32, y: i32) -> bool {
        self.in_editor() && self.find.is_some() && self.page_layout().find_bar.contains(x, y)
    }

    /// The Find bar is open in All notes scope, so the results list is
    /// drawn in place of the page.
    #[must_use]
    pub fn search_list_shown(&self) -> bool {
        self.in_editor() && self.find.as_ref().is_some_and(|f| f.scope == Scope::All)
    }

    /// The scope of the open Find bar (`Note` when it is closed).
    #[must_use]
    pub fn find_scope(&self) -> Scope {
        self.find.as_ref().map_or(Scope::Note, |f| f.scope)
    }

    /// Opens the Find bar (Ctrl+F or the magnifier). A one-line selection
    /// seeds the query. Full refresh: the page moves down.
    pub fn open_find(&mut self) -> Refresh {
        let seed = self
            .buffer
            .selected_text()
            .filter(|t| !t.contains('\n'))
            .map(str::to_owned);
        let f = self.find.get_or_insert_with(FindState::default);
        if let Some(t) = seed {
            f.query = Buffer::new(t);
        }
        f.focus = Some(FindField::Find);
        self.scratch_focus = false;
        self.sel_ui = None;
        self.screen.find_open = true;
        if self.find_scope() == Scope::All {
            self.search_request();
        } else {
            // Show the seeded / previous query's match at the caret.
            let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
            self.select_match(sigma_core::find::at_or_after(&self.find_matches(), from));
        }
        Refresh::Full
    }

    pub fn close_find(&mut self) {
        self.find = None;
        self.screen.find_open = false;
    }

    /// Matches of the query in the open note.
    #[must_use]
    pub fn find_matches(&self) -> Vec<(usize, usize)> {
        self.find.as_ref().map_or_else(Vec::new, |f| {
            sigma_core::find::matches(self.buffer.text(), f.query.text(), f.case)
        })
    }

    /// `Find`, `None`, or `i of n`; in All notes the results status line
    /// ("3 matches in 2 notes · page 1 of 2").
    #[must_use]
    pub fn find_label(&self) -> String {
        if let Some(f) = self.find.as_ref().filter(|f| f.scope == Scope::All) {
            let m = self.search_model(f);
            let l = self.screen.search_layout(self.page_rect(), &m);
            return EditorScreen::search_status(&m, l.visible);
        }
        let ms = self.find_matches();
        let q = self.find.as_ref().map_or("", |f| f.query.text());
        let cur = sigma_core::find::selected(&ms, self.buffer.selection());
        sigma_core::find::count_label(q, &ms, cur)
    }

    fn select_match(&mut self, i: Option<usize>) {
        if self.find_scope() == Scope::All {
            return;
        }
        if let Some(&(a, b)) = i.and_then(|i| self.find_matches().get(i).copied()).as_ref() {
            self.buffer.select(a, b);
            self.screen.follow_caret(&self.buffer, self.page_rect());
        }
    }

    fn find_step(&mut self, forward: bool) -> Option<Refresh> {
        if self.find_scope() == Scope::All {
            return self.search_page(forward);
        }
        let ms = self.find_matches();
        let (a, b) = self
            .buffer
            .selection()
            .unwrap_or((self.buffer.caret(), self.buffer.caret()));
        let i = if forward {
            // After the selected match, or from the caret.
            let from = if sigma_core::find::selected(&ms, Some((a, b))).is_some() {
                b.max(a + 1)
            } else {
                a
            };
            sigma_core::find::next(&ms, from)
        } else {
            sigma_core::find::prev(&ms, a)
        };
        i?;
        self.select_match(i);
        Some(Refresh::Partial)
    }

    fn find_replace_one(&mut self) -> Option<Refresh> {
        if self.find_scope() == Scope::All {
            return None;
        }
        let ms = self.find_matches();
        if sigma_core::find::selected(&ms, self.buffer.selection()).is_none() {
            return self.find_step(true);
        }
        let with = self.find.as_ref()?.replace.text().to_owned();
        // An empty replacement deletes the match (still one undo step).
        self.buffer.insert(&with);
        self.save();
        let at = self.buffer.caret();
        let next = sigma_core::find::at_or_after(&self.find_matches(), at);
        self.select_match(next);
        Some(Refresh::Partial)
    }

    fn find_replace_all(&mut self) -> Option<Refresh> {
        if self.find_scope() == Scope::All {
            return None;
        }
        let ms = self.find_matches();
        if ms.is_empty() {
            return None;
        }
        let with = self.find.as_ref()?.replace.text().to_owned();
        let text = sigma_core::find::replace_all(self.buffer.text(), &ms, &with);
        // Keep the caret near where it was: count the bytes the replaced
        // matches before it gained or lost.
        let at = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
        let shift: isize = ms
            .iter()
            .filter(|m| m.1 <= at)
            .map(|m| with.len() as isize - (m.1 - m.0) as isize)
            .sum();
        let caret = (at as isize + shift).max(0) as usize;
        self.buffer.replace_text(text, caret);
        self.save();
        self.screen.follow_caret(&self.buffer, self.page_rect());
        Some(Refresh::Partial)
    }

    pub(crate) fn find_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.page_layout();
        match self.screen.find_hit(&l, self.find_scope(), x, y) {
            FindHit::Field(fld) => {
                self.scratch_focus = false;
                let f = self.find.as_mut()?;
                f.focus = Some(fld);
                let b = match fld {
                    FindField::Find => &mut f.query,
                    FindField::Replace => &mut f.replace,
                };
                let len = b.text().len();
                b.set_caret(len);
                Some(Refresh::Fast)
            }
            FindHit::Close => {
                self.close_find();
                Some(Refresh::Full)
            }
            FindHit::Scope(s) => self.set_scope(s),
            FindHit::Prev => self.find_step(false),
            FindHit::Next => self.find_step(true),
            FindHit::Replace => self.find_replace_one(),
            FindHit::All => self.find_replace_all(),
            FindHit::Case => {
                let f = self.find.as_mut()?;
                f.case = !f.case;
                if f.scope == Scope::All {
                    f.first = 0;
                    f.selected = None;
                    self.search_request();
                } else {
                    let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
                    self.select_match(sigma_core::find::at_or_after(&self.find_matches(), from));
                }
                Some(Refresh::Partial)
            }
            FindHit::Bar | FindHit::Outside => None,
        }
    }

    /// The scope control: This note or All notes (S22-D1). Switching to
    /// All starts the search of the current query; switching back shows
    /// the note again with its match selected. Session state (S22-D2).
    pub fn set_scope(&mut self, s: Scope) -> Option<Refresh> {
        let f = self.find.as_mut()?;
        if f.scope == s {
            return None;
        }
        f.scope = s;
        f.first = 0;
        f.selected = None;
        f.focus = Some(FindField::Find);
        if s == Scope::All {
            f.results = None;
            f.rows.clear();
            f.notes_total = self.library.all_notes().map_or(0, |n| n.len());
            self.search_request();
        } else {
            self.query_changed();
        }
        Some(Refresh::Full)
    }

    /// After the query changed: select the first match from the start of
    /// the current selection (incremental search). In All notes: a new
    /// search from the first page.
    fn query_changed(&mut self) {
        if self.find_scope() == Scope::All {
            if let Some(f) = &mut self.find {
                f.first = 0;
                f.selected = None;
            }
            self.search_request();
            return;
        }
        let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
        let ms = self.find_matches();
        match sigma_core::find::at_or_after(&ms, from) {
            Some(i) => self.select_match(Some(i)),
            None => {
                let c = self.buffer.caret().min(from);
                self.buffer.set_caret(c);
            }
        }
    }

    fn find_field(&mut self) -> Option<(&mut Buffer, FindField)> {
        let f = self.find.as_mut()?;
        let fld = f.focus?;
        Some((
            match fld {
                FindField::Find => &mut f.query,
                FindField::Replace => &mut f.replace,
            },
            fld,
        ))
    }

    /// On-screen key while a Find field has focus.
    pub(crate) fn find_key(&mut self, k: Key) -> Option<Refresh> {
        let all = self.find_scope() == Scope::All;
        match k {
            Key::Return if all => return self.search_open_selected(),
            Key::Return => return self.find_step(true),
            Key::Esc => {
                self.close_find();
                return Some(Refresh::Full);
            }
            // Replace is off in All notes (S22-D3): Tab stays in the query.
            Key::Tab if all => return None,
            Key::Tab => {
                let f = self.find.as_mut()?;
                f.focus = Some(match f.focus {
                    Some(FindField::Find) => FindField::Replace,
                    _ => FindField::Find,
                });
                return Some(Refresh::Fast);
            }
            Key::Arrow(Dir::Up) if all => return self.search_move(false),
            Key::Arrow(Dir::Down) if all => return self.search_move(true),
            Key::PgUp if all => return self.search_page(false),
            Key::PgDn if all => return self.search_page(true),
            Key::PgUp | Key::PgDn | Key::Arrow(Dir::Up | Dir::Down) => return None,
            _ => {}
        }
        let mut kb = std::mem::take(&mut self.keyboard);
        let (field, fld) = self.find_field()?;
        let before = field.revision();
        let action = kb.press(k, field, TabKind::Spaces(0));
        let edited = field.revision() != before;
        self.keyboard = kb;
        if action == KeyAction::Relayout {
            self.save_settings();
            return Some(Refresh::Full);
        }
        if edited && fld == FindField::Find {
            self.query_changed();
            return Some(Refresh::Partial);
        }
        Some(Refresh::Fast)
    }

    /// Hardware (USB) key while a Find field has focus.
    pub(crate) fn find_hw(&mut self, k: HwKey) -> Option<Refresh> {
        let key = match k {
            HwKey::Char(c) => Key::Char(c),
            HwKey::Backspace => Key::Back,
            HwKey::Delete => Key::Del,
            HwKey::Return => Key::Return,
            HwKey::Tab => Key::Tab,
            HwKey::Left => Key::Arrow(Dir::Left),
            HwKey::Right => Key::Arrow(Dir::Right),
            // Up, Down, PgUp and PgDn move in the results list (S22.1).
            HwKey::Up => Key::Arrow(Dir::Up),
            HwKey::Down => Key::Arrow(Dir::Down),
            HwKey::PageUp => Key::PgUp,
            HwKey::PageDown => Key::PgDn,
            HwKey::Home | HwKey::End | HwKey::Insert => return None,
            HwKey::Escape => Key::Esc,
        };
        if let Key::Char(c) = key {
            let (field, fld) = self.find_field()?;
            field.insert(&c.to_string());
            if fld == FindField::Find {
                self.query_changed();
                return Some(Refresh::Partial);
            }
            return Some(Refresh::Fast);
        }
        self.find_key(key)
    }
}

/// Search across all notes (S22.1).
impl App {
    /// Sends the current query to the search worker. The open note is
    /// saved first: the worker reads from disk.
    fn search_request(&mut self) {
        let Some(f) = self.find.as_ref().filter(|f| f.scope == Scope::All) else {
            return;
        };
        let req = SearchRequest {
            root: self.library.root().to_path_buf(),
            notes: None,
            query: f.query.text().to_owned(),
            case: f.case,
            cap: SEARCH_CAP,
        };
        self.save();
        self.searcher.request(req);
    }

    /// Takes a finished search into the open bar. Returns whether the
    /// results list on screen changed.
    pub(crate) fn search_apply(&mut self, r: SearchResult) -> bool {
        let shown = self.search_list_shown();
        let Some(f) = self.find.as_mut().filter(|f| f.scope == Scope::All) else {
            return false;
        };
        // One line per answered search: the number `docs/dev/perf.md` wants
        // from the Forma (query length, not the query: notes are private).
        // One line per answered search: the number `docs/dev/perf.md` wants
        // from the Forma (query length, not the query: notes are private).
        sigma_core::info!("search", "{}", search_log_line(f.query.text(), &r));
        f.rows = search_rows(&r, f.query.text(), f.case);
        if f.first >= f.rows.len() {
            f.first = 0;
        }
        if f.selected.is_some_and(|i| i >= f.rows.len()) {
            f.selected = None;
        }
        f.results = Some(r);
        shown
    }

    /// Polls the search worker (the idle tick): a new answer redraws the
    /// list.
    pub(crate) fn search_poll(&mut self) -> Option<Refresh> {
        let r = self.searcher.poll()?;
        self.search_apply(r).then_some(Refresh::Partial)
    }

    /// Blocks until the newest search is answered (emulator, tests).
    /// Returns whether the list on screen changed.
    pub fn settle_search(&mut self) -> bool {
        match self.searcher.wait() {
            Some(r) => self.search_apply(r),
            None => false,
        }
    }

    /// The results list model for the bar `f` (open in All notes).
    pub(crate) fn search_model<'a>(&'a self, f: &'a FindState) -> SearchListModel<'a> {
        let state = if f.query.text().is_empty() {
            SearchState::Idle
        } else if self.searcher.pending() || f.results.is_none() {
            SearchState::Searching
        } else {
            SearchState::Done
        };
        let r = f.results.as_ref();
        SearchListModel {
            rows: &f.rows,
            first: f.first,
            selected: f.selected,
            state,
            notes_total: f.notes_total,
            notes_with_hits: r.map_or(0, |r| r.notes_with_hits),
            capped: r.is_some_and(|r| r.capped),
            theme: self.theme(),
        }
    }

    /// The results list's layout, when it is on screen.
    #[must_use]
    pub fn search_layout_now(&self) -> Option<SearchLayout> {
        if !self.search_list_shown() {
            return None;
        }
        let f = self.find.as_ref()?;
        let m = self.search_model(f);
        Some(self.screen.search_layout(self.page_rect(), &m))
    }

    /// A tap on the results list: a row opens its note, ‹ › turn pages.
    pub(crate) fn search_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.search_layout_now()?;
        match self.screen.search_hit(&l, x, y) {
            SearchListHit::Row(i) => self.search_open(i),
            SearchListHit::Prev => self.search_page(false),
            SearchListHit::Next => self.search_page(true),
            SearchListHit::None => None,
        }
    }

    /// Turns a page of the results list (page buttons, ‹ ›).
    pub(crate) fn search_page(&mut self, forward: bool) -> Option<Refresh> {
        let l = self.search_layout_now()?;
        let visible = l.visible.max(1);
        let f = self.find.as_mut()?;
        let n = f.rows.len();
        let first = if forward {
            let next = f.first + visible;
            if next >= n {
                return None;
            }
            next
        } else {
            if f.first == 0 {
                return None;
            }
            f.first.saturating_sub(visible)
        };
        f.first = first;
        f.selected = None;
        Some(Refresh::Partial)
    }

    /// Up/Down: moves the selection one row, turning the page with it.
    fn search_move(&mut self, down: bool) -> Option<Refresh> {
        let l = self.search_layout_now()?;
        let visible = l.visible.max(1);
        let f = self.find.as_mut()?;
        let n = f.rows.len();
        if n == 0 {
            return None;
        }
        let i = match (f.selected, down) {
            (None, _) => f.first,
            (Some(i), true) => (i + 1).min(n - 1),
            (Some(i), false) => i.saturating_sub(1),
        };
        f.selected = Some(i);
        f.first = i / visible * visible;
        Some(Refresh::Partial)
    }

    /// Enter: opens the selected result, or the first one.
    fn search_open_selected(&mut self) -> Option<Refresh> {
        let f = self.find.as_ref()?;
        let i = f.selected.unwrap_or(f.first);
        self.search_open(i)
    }

    /// Opens result `i` in write mode with the caret on the match.
    fn search_open(&mut self, i: usize) -> Option<Refresh> {
        let f = self.find.as_ref()?;
        let h = f.results.as_ref()?.hits.get(i)?;
        let (note, byte) = (h.note.clone(), h.byte);
        Some(self.open_at(&note, byte))
    }

    /// Opens `note` in write mode with the caret at `byte` and the page
    /// scrolled to it, after pushing Back history (which remembers the
    /// results list when the bar is open in All notes).
    pub fn open_at(&mut self, note: &NotePath, byte: usize) -> Refresh {
        self.push_history();
        self.close_find();
        self.exit_preview();
        if *note != self.current {
            self.open(note);
        }
        let at = byte.min(self.buffer.text().len());
        self.buffer.set_caret(at);
        self.screen.follow_caret(&self.buffer, self.page_rect());
        self.sel_ui = None;
        self.scratch_focus = false;
        Refresh::Full
    }

    /// Reopens the Find bar in All notes as a Back entry left it, and
    /// searches again (the notes may have changed).
    pub(crate) fn restore_search(&mut self, s: &SearchSnapshot) {
        let f = FindState {
            query: Buffer::new(s.query.clone()),
            replace: Buffer::default(),
            focus: Some(FindField::Find),
            case: s.case,
            scope: Scope::All,
            results: None,
            rows: Vec::new(),
            first: s.first,
            selected: None,
            notes_total: self.library.all_notes().map_or(0, |n| n.len()),
        };
        self.find = Some(f);
        self.screen.find_open = true;
        self.scratch_focus = false;
        self.search_request();
    }

    /// What Back needs to bring the results list back (S22.1): set while
    /// the bar is open in All notes.
    pub(crate) fn search_snapshot(&self) -> Option<SearchSnapshot> {
        let f = self.find.as_ref().filter(|f| f.scope == Scope::All)?;
        Some(SearchSnapshot {
            query: f.query.text().to_owned(),
            case: f.case,
            first: f.first,
        })
    }
}

/// The renderer's rows for a result: the hit's place inside its line is
/// found again with the same matcher, counting the hits that share a line.
fn search_rows(r: &SearchResult, query: &str, case: bool) -> Vec<SearchRow> {
    let mut out = Vec::with_capacity(r.hits.len());
    let mut k = 0;
    for (i, h) in r.hits.iter().enumerate() {
        let same_line = i > 0 && r.hits[i - 1].note == h.note && r.hits[i - 1].line == h.line;
        k = if same_line { k + 1 } else { 0 };
        let hit = sigma_core::find::matches(&h.text, query, case)
            .get(k)
            .copied()
            .unwrap_or((0, 0));
        out.push(SearchRow {
            path: h.note.as_str().to_owned(),
            line: h.line,
            text: h.text.clone(),
            hit,
        });
    }
    out
}

/// The Find bar (v0.6). The query is not persisted (brief §8).
#[derive(Clone, Debug, Default)]
pub struct FindState {
    pub query: Buffer,
    pub replace: Buffer,
    /// Field the keyboard types into; `None` when the page has the caret.
    pub focus: Option<FindField>,
    /// Aa: match case (off by default).
    pub case: bool,
    /// This note or All notes (S22.1).
    pub scope: Scope,
    /// The newest answer of the search worker (All notes).
    pub results: Option<SearchResult>,
    /// `results` as the renderer draws them.
    pub rows: Vec<SearchRow>,
    /// Index of the first row on the shown page of the results list.
    pub first: usize,
    /// The selected row (Up/Down, Enter).
    pub selected: Option<usize>,
    /// How many notes there are to search (the idle status line).
    pub notes_total: usize,
}

/// The results list as Back remembers it (S22.1): the bar reopens in All
/// notes with this query, case and page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchSnapshot {
    pub query: String,
    pub case: bool,
    pub first: usize,
}

/// The text of the `search:` log line for an answered search.
fn search_log_line(query: &str, r: &SearchResult) -> String {
    // A capped scan stopped early, so the notes left are unknown.
    let read = if r.capped {
        format!("{} notes read", r.scanned)
    } else {
        format!("{0} of {0} notes read", r.scanned)
    };
    format!(
        "{} chars, {}, {} hits in {} notes{}{}: {} ms",
        query.chars().count(),
        read,
        r.hits.len(),
        r.notes_with_hits,
        if r.capped { " (capped)" } else { "" },
        if r.skipped > 0 {
            format!(", {} skipped", r.skipped)
        } else {
            String::new()
        },
        r.elapsed_ms
    )
}

#[cfg(test)]
mod log_line_tests {
    use super::*;

    #[test]
    fn capped_line_does_not_claim_a_total_of_notes() {
        let mut r = SearchResult {
            scanned: 3,
            elapsed_ms: 7,
            ..SearchResult::default()
        };
        assert_eq!(
            search_log_line("lamp", &r),
            "4 chars, 3 of 3 notes read, 0 hits in 0 notes: 7 ms"
        );
        r.capped = true;
        assert_eq!(
            search_log_line("lamp", &r),
            "4 chars, 3 notes read, 0 hits in 0 notes (capped): 7 ms"
        );
    }
}
