//! Wiki links between notes and the Back history.

use crate::App;
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
        };
        if self.history.last() == Some(&e) {
            return;
        }
        self.history.push(e);
        if self.history.len() > HISTORY_CAP {
            self.history.remove(0);
        }
    }

    /// Back (v1.0.2): the previous note, mode and position. Sheets and the
    /// find bar close. `None` when the history is empty.
    pub fn go_back(&mut self) -> Option<Refresh> {
        let e = self.history.pop()?;
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
        Some(Refresh::Full)
    }

    /// A tapped `[[wiki]]` link: an existing note opens in preview; a
    /// missing one is created (empty) and opens in write mode.
    pub fn follow_link(&mut self, target: &str) -> Option<Refresh> {
        let notes = self.library.all_notes().unwrap_or_default();
        self.push_history();
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
}
