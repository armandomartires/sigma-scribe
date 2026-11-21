//! Wiki links between notes and the Back history.

use crate::{App, SearchSnapshot};
use sigma_core::links::Graph;
use sigma_core::{links, NotePath};
use sigma_device::Refresh;

impl App {
    /// Names a `[[wiki]]` link may use for an existing note: the file name
    /// or the path from the library root, with or without the extension.
    pub(crate) fn note_names(&self) -> std::collections::HashSet<String> {
        let mut out = std::collections::HashSet::new();
        for p in self.library.all_notes().unwrap_or_default() {
            for s in [p.as_str(), p.name()] {
                out.insert(s.to_lowercase());
                if let Some((stem, _)) = s.rsplit_once('.') {
                    out.insert(stem.to_lowercase());
                }
            }
        }
        out
    }

    /// The link graph of the library, with the open note's live text.
    #[must_use]
    pub fn link_graph(&self) -> Graph {
        Graph::build(&self.library, Some((&self.current, self.buffer.text())))
    }

    /// Remembers where we are before a jump (v1.0.2). Cap 50, oldest
    /// dropped; an entry equal to the top is not repeated.
    pub fn push_history(&mut self) {
        let e = HistEntry {
            note: self.current.clone(),
            preview: self.preview.as_ref().map(|p| p.page),
            caret: self.buffer.caret(),
            scroll: self.screen.scroll,
            search: self.search_snapshot(),
        };
        if self.history.last() == Some(&e) {
            return;
        }
        self.history.push(e);
        if self.history.len() > HISTORY_CAP {
            self.history.remove(0);
        }
    }

    /// After `from` was renamed or moved to `to`: the open note and every
    /// Back entry at or under `from` follow it, so Back finds the note
    /// where it went, not where it was.
    pub(crate) fn remap_history(&mut self, from: &NotePath, to: &NotePath) {
        if let Some(p) = moved_path(&self.current, from, to) {
            self.current = p;
        }
        for h in &mut self.history {
            if let Some(p) = moved_path(&h.note, from, to) {
                h.note = p;
            }
        }
    }

    /// Drops Back entries whose note is gone (deleted, or moved where
    /// [`App::remap_history`] did not follow). Reopened, such an entry
    /// would be an empty note and a keystroke would recreate the file.
    pub(crate) fn prune_history(&mut self) {
        let lib = &self.library;
        let cur = &self.current;
        self.history
            .retain(|h| h.note == *cur || lib.exists(&h.note));
    }

    /// Back (v1.0.2): the previous note, mode and position. Sheets and the
    /// find bar close; an entry taken from the search results list brings
    /// the list back with its query and page (S22.1); from the review sheet
    /// of Replace in all notes it only closes the sheet (S24.1). Entries whose note
    /// no longer exists are skipped. `None` when no entry is left.
    pub fn go_back(&mut self) -> Option<Refresh> {
        // From the review sheet, Back is Cancel: the results list returns.
        if self.review_shown() {
            return self.review_cancel();
        }
        let e = loop {
            let e = self.history.pop()?;
            if e.note == self.current || self.library.exists(&e.note) {
                break e;
            }
        };
        self.files = None;
        self.close_graph();
        self.help = None;
        self.options = false;
        self.close_find();
        self.exit_preview();
        if e.note != self.current {
            self.open(&e.note);
        }
        let at = e.caret.min(self.buffer.text().len());
        self.buffer.set_caret(at);
        self.screen.scroll = e.scroll;
        self.sel_ui = None;
        if let Some(page) = e.preview {
            self.enter_preview();
            if let Some(p) = &mut self.preview {
                p.page = page.min(p.doc.pages.len().saturating_sub(1));
            }
        }
        if let Some(s) = &e.search {
            self.restore_search(s);
        }
        Some(Refresh::Full)
    }

    /// A tapped `[[wiki]]` link: an existing note opens in preview; a
    /// missing one is created (empty) and opens in write mode. A
    /// `#heading` part is ignored: the note opens at its top (S22-D5); a
    /// bare `[[#heading]]` is this note, shown from its top.
    pub fn follow_link(&mut self, target: &str) -> Option<Refresh> {
        let notes = self.library.all_notes().unwrap_or_default();
        self.push_history();
        if links::note_part(target).is_empty() {
            self.exit_preview();
            self.buffer.set_caret(0);
            self.screen.scroll = 0;
            self.enter_preview();
            return Some(Refresh::Full);
        }
        if let Some(p) = links::resolve(target, &notes).cloned() {
            self.exit_preview();
            self.open(&p);
            self.enter_preview();
            return Some(Refresh::Full);
        }
        let (folder, name) = links::new_note_place(&self.library, &self.current, target);
        let p = self.library.create_note(&folder, &name).ok()?;
        self.exit_preview();
        self.open(&p);
        Some(Refresh::Full)
    }
}

/// Where `p` is after `from` became `to`: `to` itself, or the same place
/// under it when `p` is inside the folder `from`; `None` when `p` did not
/// move.
fn moved_path(p: &NotePath, from: &NotePath, to: &NotePath) -> Option<NotePath> {
    if p == from {
        Some(to.clone())
    } else if p.within(from) && !from.is_root() {
        NotePath::parse(&format!(
            "{}{}",
            to.as_str(),
            &p.as_str()[from.as_str().len()..]
        ))
        .ok()
    } else {
        None
    }
}

/// Navigation history cap (v1.0.2).
pub const HISTORY_CAP: usize = 50;

/// Where the user was before a jump (v1.0.2 Back).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistEntry {
    pub note: NotePath,
    /// Preview page, or `None` for write mode.
    pub preview: Option<usize>,
    pub caret: usize,
    pub scroll: usize,
    /// The search results list that was on screen, if any (S22.1).
    pub search: Option<SearchSnapshot>,
}
