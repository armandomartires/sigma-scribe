//! Preview mode: the laid-out, read-only note.

use crate::{stem, App};
use sigma_device::{Event, HwKey, Refresh};
use sigma_render::{Doc, Hit};

impl App {
    /// Switches to preview at the page showing the top of the write page.
    /// Full refresh: the whole page changes.
    pub fn enter_preview(&mut self) {
        self.close_find();
        self.save();
        self.sel_ui = None;
        self.drag = None;
        self.sync_preview_drawer(true);
        let doc = self.preview_doc();
        let page = self.page_rect();
        let top = self.screen.top_source_line(&self.buffer, page);
        let page = doc.page_of_line(top);
        self.preview = Some(PreviewState { doc, page });
    }

    pub(crate) fn preview_doc(&self) -> sigma_render::preview::Doc {
        let names = self.note_names();
        let known = |n: &str| names.contains(&n.trim().to_lowercase());
        let back: Vec<(String, String)> = self
            .link_graph()
            .backlinks(&self.current)
            .iter()
            .map(|p| (stem(p.as_str()).to_owned(), p.as_str().to_owned()))
            .collect();
        let note = self.library.root().join(self.current.as_str());
        self.screen.build_preview_in(
            self.buffer.text(),
            &self.keyboard,
            &known,
            Some(&back),
            note.parent(),
        )
    }

    /// Back to write mode; caret, selection and scroll are as they were.
    pub fn exit_preview(&mut self) {
        self.preview = None;
    }

    /// Preview input: pencil, Files, page turns, Escape, and (v1.0.3) the
    /// drawer: Contents jumps, to-do toggles, scratch typing. The note
    /// itself stays read-only (decision S3-D1; to-dos are the S10.3-D1
    /// exception).
    pub(crate) fn preview_event(&mut self, e: Event) -> Option<Refresh> {
        match e {
            Event::Tap { x, y } if self.in_drawer(x, y) => self.drawer_tap(x, y),
            Event::Tap { x, y } => match self.screen.preview_hit(&self.keyboard, x, y) {
                Hit::Back => self.go_back(),
                Hit::Drawer(p) => {
                    let r = self.toggle_pane(p);
                    self.rebuild_preview();
                    Some(r)
                }
                Hit::TogglePreview => {
                    self.exit_preview();
                    Some(Refresh::Full)
                }
                Hit::Files => {
                    self.exit_preview();
                    self.open_files();
                    Some(Refresh::Full)
                }
                Hit::Options => {
                    self.open_options();
                    Some(Refresh::Full)
                }
                Hit::Find => {
                    self.exit_preview();
                    self.open_find();
                    Some(Refresh::Full)
                }
                Hit::Graph => {
                    self.exit_preview();
                    self.open_graph();
                    Some(Refresh::Full)
                }
                _ => {
                    let p = self.preview.as_ref()?;
                    let target =
                        self.screen
                            .preview_link_at(&self.keyboard, &p.doc, p.page, x, y)?;
                    self.follow_link(&target)
                }
            },
            Event::PageTurn { forward } => {
                let p = self.preview.as_mut()?;
                let last = p.doc.pages.len().saturating_sub(1);
                let next = if forward {
                    (p.page + 1).min(last)
                } else {
                    p.page.saturating_sub(1)
                };
                (next != p.page).then(|| {
                    p.page = next;
                    Refresh::Partial
                })
            }
            // The scratch pad stays editable in preview; the note does not.
            Event::Key(k) if self.scratch_focused() => self.scratch_hw(k),
            Event::Key(HwKey::Escape) => {
                if !self.history.is_empty() {
                    return self.go_back();
                }
                self.exit_preview();
                Some(Refresh::Full)
            }
            _ => None,
        }
    }
}

/// Preview mode (v0.3): the laid-out note and the page shown. Read-only.
#[derive(Clone, Debug, Default)]
pub struct PreviewState {
    pub doc: Doc,
    pub page: usize,
}
