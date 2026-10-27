//! The side drawer: Contents, To-dos and the scratch pad.

use crate::preview::PreviewState;
use crate::App;
use sigma_core::keyboard::Key;
use sigma_core::library::SCRATCH;
use sigma_core::outline::{Heading, TodoItem};
use sigma_core::Change;
use sigma_core::{outline, Buffer, KeyAction, NotePath};
use sigma_device::{HwKey, Phase, Refresh};
use sigma_render::drawer_menu::MenuHit;
use sigma_render::{DrawerHit, DrawerModel, Pane};

/// A finger on the drawer's grip or on the closed drawer's tab (v1.16).
#[derive(Clone, Copy, Debug)]
pub(crate) struct GripTouch {
    /// Where it went down.
    from: (i32, i32),
    /// The width stop nearest the finger (the saved one until it moves).
    stop: u8,
    /// It moved past the tap slop: a drag, not a tap.
    moved: bool,
    /// It started on the tab, not on the grip.
    tab: bool,
}

impl App {
    pub(crate) fn save_scratch(&mut self) {
        let rev = self.scratch.revision();
        self.scratch_save
            .sync(&mut self.store, SCRATCH, self.scratch.text(), rev);
    }

    /// The drawer is drawn and takes taps: write mode or preview, never
    /// over a sheet (v1.0.3, S10.3-D1; write mode only before).
    #[must_use]
    pub fn drawer_visible(&self) -> bool {
        self.drawer.iter().any(|&o| o) && self.drawer_allowed()
    }

    /// Write mode or preview, with no sheet (Files, Options, Graph, Help,
    /// rename) open.
    #[must_use]
    pub fn drawer_allowed(&self) -> bool {
        self.files.is_none()
            && self.rename.is_none()
            && !self.options
            && self.graph.is_none()
            && self.plugin_screen.is_none()
            && self.help.is_none()
    }

    /// The layout the drawer sits on: preview has no on-screen keyboard.
    pub(crate) fn drawer_base(&self) -> sigma_render::editor::Layout {
        if self.preview.is_some() {
            self.screen.layout(false, &self.keyboard)
        } else {
            self.screen.layout(self.keys_visible(), &self.keyboard)
        }
    }

    /// Headings for Contents: in preview, the rendered headings only.
    pub(crate) fn drawer_headings(&self) -> Vec<Heading> {
        let hs = self.headings();
        match &self.preview {
            Some(p) => {
                let shown = p.doc.heading_lines();
                hs.into_iter().filter(|h| shown.contains(&h.line)).collect()
            }
            None => hs,
        }
    }

    /// Where the preview page goes when the drawer is open (v1.0.3).
    pub(crate) fn sync_preview_drawer(&mut self, preview: bool) {
        self.screen.preview_drawer =
            (preview && self.drawer.iter().any(|&o| o) && self.drawer_allowed()).then(|| {
                let base = self.screen.layout(false, &self.keyboard);
                self.screen.drawer_rect(&base, self.settings.drawer)
            });
    }

    /// Lays the preview out again (drawer opened/closed, to-do toggled),
    /// keeping the shown source line on screen.
    pub(crate) fn rebuild_preview(&mut self) {
        let Some(p) = &self.preview else { return };
        let src = p
            .doc
            .pages
            .get(p.page)
            .and_then(|r| p.doc.lines[r.clone()].iter().find(|l| !l.spacer))
            .map_or(0, |l| l.src);
        self.sync_preview_drawer(true);
        let doc = self.preview_doc();
        let page = doc.page_of_line(src);
        self.preview = Some(PreviewState { doc, page });
    }

    pub(crate) fn scratch_focused(&self) -> bool {
        self.scratch_focus && self.drawer[2] && self.drawer_visible()
    }

    /// Headings of the open note (Contents).
    #[must_use]
    pub fn headings(&self) -> Vec<Heading> {
        outline::headings(self.buffer.text())
    }

    /// Every to-do: the indexer's view of the disk, with the open note and
    /// the scratch pad taken live from their buffers.
    #[must_use]
    pub fn todo_list(&self) -> Vec<TodoItem> {
        let live = |note: &NotePath, text: &str| -> Vec<TodoItem> {
            outline::todos(text)
                .into_iter()
                .map(|todo| TodoItem {
                    note: note.clone(),
                    todo,
                })
                .collect()
        };
        let scratch = NotePath::parse(SCRATCH).unwrap_or_default();
        let mut out = Vec::new();
        let mut placed = false;
        for it in &self.disk_todos {
            if it.note == self.current {
                if !placed {
                    out.extend(live(&self.current, self.buffer.text()));
                    placed = true;
                }
            } else if it.note != scratch {
                out.push(it.clone());
            }
        }
        if !placed {
            let mut cur = live(&self.current, self.buffer.text());
            cur.extend(out);
            out = cur;
        }
        if self.screen.scratch_on {
            out.extend(live(&scratch, self.scratch.text()));
        }
        out
    }

    pub(crate) fn drawer_model<'a>(
        &'a self,
        hs: &'a [Heading],
        todos: &'a [TodoItem],
    ) -> DrawerModel<'a> {
        DrawerModel {
            side: self.settings.drawer,
            contents: self.drawer[0].then_some(hs),
            todos: self.drawer[1].then_some((todos, self.filter)),
            scratch: self.drawer[2].then_some((&self.scratch, self.scratch_focused())),
            plugin: [0, 1, 2].map(|i| self.pane_frame(i)),
            theme: self.theme(),
        }
    }

    fn drawer_hit_at(&self, x: i32, y: i32) -> DrawerHit {
        if !self.drawer_visible() {
            return DrawerHit::Outside;
        }
        let (hs, todos) = (self.drawer_headings(), self.todo_list());
        let base = self.drawer_base();
        self.screen
            .drawer_hit(&base, &self.drawer_model(&hs, &todos), x, y)
    }

    /// A finger on the drawer's edge grip drags its width: the stop nearest
    /// the finger is applied when it lifts (one redraw, as for the other
    /// sliders). v1.16: dragged to the screen edge, or tapped twice, the
    /// grip closes the drawer; while it is closed that way, a tab at the
    /// screen edge reopens it, tapped or dragged in (to the width where the
    /// finger lifts). `None` when the touch is not the grip's or the tab's.
    pub(crate) fn handle_touch(
        &mut self,
        phase: Phase,
        x: i32,
        y: i32,
        t: u64,
    ) -> Option<Option<Refresh>> {
        let (base, side) = (self.drawer_base(), self.settings.drawer);
        match phase {
            Phase::Down => {
                if self.ink.is_some() || self.options {
                    return None;
                }
                let tab = if self.drawer_visible()
                    && self.screen.drawer_handle(&base, side).contains(x, y)
                {
                    false
                } else if self.on_drawer_tab(x, y) {
                    true
                } else {
                    return None;
                };
                self.handle_drag = Some(GripTouch {
                    from: (x, y),
                    stop: self.settings.drawer_width,
                    moved: false,
                    tab,
                });
                Some(None)
            }
            Phase::Move => {
                let slop = self.gestures.cfg.slop;
                let d = self.handle_drag.as_mut()?;
                d.moved |= (x - d.from.0).abs().max((y - d.from.1).abs()) > slop;
                if d.moved {
                    d.stop = self.screen.drawer_stop_at(&base, side, x);
                }
                Some(None)
            }
            Phase::Up => {
                let d = self.handle_drag.take()?;
                let edge = self.screen.drawer_closes_at(&base, side, x);
                if d.tab {
                    if d.moved && edge {
                        return Some(None);
                    }
                    let r = self.reopen_drawer();
                    if d.moved && d.stop != self.settings.drawer_width {
                        return Some(Some(self.change(Change::DrawerWidth(d.stop))));
                    }
                    return Some(r);
                }
                if !d.moved {
                    let double = self
                        .grip_tap
                        .is_some_and(|p| t.saturating_sub(p) <= self.gestures.cfg.double_tap_ms);
                    if double {
                        self.grip_tap = None;
                        return Some(Some(self.close_drawer()));
                    }
                    self.grip_tap = Some(t);
                    return Some(None);
                }
                if edge {
                    return Some(Some(self.close_drawer()));
                }
                if d.stop == self.settings.drawer_width {
                    return Some(None);
                }
                Some(Some(self.change(Change::DrawerWidth(d.stop))))
            }
        }
    }

    /// The tab at the screen edge is shown: the grip closed the drawer and
    /// it could be open here.
    #[must_use]
    pub fn drawer_tab_shown(&self) -> bool {
        self.drawer_stash.is_some()
            && !self.drawer_visible()
            && self.drawer_allowed()
            && self.ink.is_none()
    }

    /// (`x`, `y`) is on the closed drawer's tab.
    pub(crate) fn on_drawer_tab(&self, x: i32, y: i32) -> bool {
        self.drawer_tab_shown()
            && self
                .screen
                .drawer_tab(&self.drawer_base(), self.settings.drawer)
                .contains(x, y)
    }

    /// (`x`, `y`) is on the open drawer's grip.
    pub(crate) fn on_drawer_grip(&self, x: i32, y: i32) -> bool {
        self.drawer_visible()
            && self
                .screen
                .drawer_handle(&self.drawer_base(), self.settings.drawer)
                .contains(x, y)
    }

    /// Closes the whole drawer from its grip (v1.16), remembering its panes
    /// and the scratch focus for the tab to bring back.
    pub fn close_drawer(&mut self) -> Refresh {
        if !self.drawer.iter().any(|&o| o) {
            return Refresh::Partial;
        }
        self.drawer_stash = Some((self.drawer, self.scratch_focus));
        self.drawer = [false; 3];
        self.scratch_focus = false;
        self.save_scratch();
        self.screen.drawer_menu = false;
        self.sel_ui = None;
        if self.preview.is_some() {
            self.rebuild_preview();
        }
        Refresh::Partial
    }

    /// The tab: the panes the grip closed open again, as they were.
    pub(crate) fn reopen_drawer(&mut self) -> Option<Refresh> {
        let (open, focus) = self.drawer_stash.take()?;
        self.drawer = open;
        self.scratch_focus = focus && open[2];
        if open[1] {
            self.reindex();
        }
        self.sel_ui = None;
        if self.preview.is_some() {
            self.rebuild_preview();
        }
        Some(Refresh::Partial)
    }

    pub(crate) fn in_drawer(&self, x: i32, y: i32) -> bool {
        self.drawer_hit_at(x, y) != DrawerHit::Outside
    }

    /// The drawer button (v1.16): opens or closes the drawer menu.
    pub(crate) fn toggle_drawer_menu(&mut self) -> Refresh {
        self.screen.drawer_menu = !self.screen.drawer_menu;
        self.sel_ui = None;
        Refresh::Partial
    }

    /// The title-bar layout of the mode on screen (preview has no keys).
    fn title_layout(&self) -> sigma_render::editor::Layout {
        let keys = self.preview.is_none() && self.keys_visible();
        self.screen.layout(keys, &self.keyboard)
    }

    /// A tap while the drawer menu is open: a row shows or hides its pane
    /// and the menu stays open; the drawer button or any other place
    /// closes it, and the tap does nothing else.
    pub(crate) fn drawer_menu_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.title_layout();
        match self.screen.drawer_menu_hit(&l, x, y) {
            MenuHit::Pane(p) => {
                let r = self.toggle_pane(p);
                if self.preview.is_some() {
                    self.rebuild_preview();
                }
                Some(r)
            }
            MenuHit::Button | MenuHit::Outside => {
                self.screen.drawer_menu = false;
                Some(Refresh::Partial)
            }
        }
    }

    /// Draws the drawer menu when it is open (over the drawer, under the
    /// ink panel). Leaving write mode and preview closes it.
    pub(crate) fn draw_drawer_menu(&mut self, c: &mut sigma_render::Canvas) {
        if !self.drawer_allowed() {
            self.screen.drawer_menu = false;
        }
        let l = self.title_layout();
        self.screen.draw_drawer_menu(c, &l, self.theme());
    }

    /// Opens or closes one drawer pane (no animation, partial refresh).
    pub fn toggle_pane(&mut self, p: Pane) -> Refresh {
        let i = match p {
            Pane::Contents => 0,
            Pane::Todos => 1,
            Pane::Scratch => 2,
        };
        self.drawer[i] = !self.drawer[i];
        // Showing or hiding a pane by hand forgets what the grip closed.
        self.drawer_stash = None;
        if p == Pane::Todos && self.drawer[1] {
            self.reindex();
        }
        if p == Pane::Scratch {
            self.scratch_focus = self.drawer[2];
            self.save_scratch();
        }
        self.sel_ui = None;
        Refresh::Partial
    }

    pub(crate) fn reindex(&mut self) {
        self.save();
        self.save_scratch();
        self.indexer.request(self.library.root().to_path_buf());
    }

    /// Blocks until the background index is current (emulator, tests).
    /// Returns whether the to-do list changed.
    pub fn settle(&mut self) -> bool {
        match self.indexer.wait() {
            Some(items) => {
                let changed = items != self.disk_todos;
                self.disk_todos = items;
                changed
            }
            None => false,
        }
    }

    /// Checks / unchecks a to-do: the open note and the scratch pad are
    /// edited in their buffers (undo works, autosave writes them); any
    /// other note is rewritten on disk, only that line.
    pub fn toggle_todo(&mut self, it: &TodoItem) -> Option<Refresh> {
        let line = it.todo.line;
        let flip = |b: &mut Buffer| -> bool {
            let Some((at, done)) = outline::box_at(b.text(), line) else {
                return false;
            };
            let (caret, sel) = (b.caret(), b.selection());
            b.select(at, at + 3);
            b.insert(if done { "[ ]" } else { "[x]" });
            match sel {
                Some((x, y)) => b.select(x, y),
                None => b.set_caret(caret),
            }
            true
        };
        if it.note == self.current {
            if flip(&mut self.buffer) {
                self.save();
            }
        } else if it.note.as_str() == SCRATCH {
            if flip(&mut self.scratch) {
                self.save_scratch();
            }
        } else {
            let path = self.library.fs_path(&it.note);
            let body = std::fs::read_to_string(&path).ok()?;
            let new = outline::toggle(&body, line)?;
            sigma_core::store::write_atomic(&path, &new).ok()?;
            if let Some(d) = self
                .disk_todos
                .iter_mut()
                .find(|d| d.note == it.note && d.todo.line == line)
            {
                d.todo.done = !d.todo.done;
            }
            self.reindex();
        }
        Some(Refresh::Partial)
    }

    /// Contents: jumps to the heading at source line `line`.
    pub(crate) fn drawer_heading(&mut self, line: usize) -> Option<Refresh> {
        self.push_history();
        self.scratch_focus = false;
        if let Some(p) = self.preview.as_mut() {
            p.page = p.doc.page_of_line(line);
        } else {
            let page = self.page_rect();
            self.screen.jump_to_line(&mut self.buffer, page, line);
        }
        Some(Refresh::Partial)
    }

    /// To-dos: checks or unchecks to-do number `i` of the list.
    pub(crate) fn drawer_todo_box(&mut self, i: usize) -> Option<Refresh> {
        let it = self.todo_list().get(i)?.clone();
        let r = self.toggle_todo(&it);
        if self.preview.is_some() {
            self.rebuild_preview();
        }
        r
    }

    /// To-dos: opens to-do number `i` of the list at its line.
    pub(crate) fn drawer_todo_open(&mut self, i: usize) -> Option<Refresh> {
        let it = self.todo_list().get(i)?.clone();
        if it.note.as_str() == SCRATCH {
            self.drawer[2] = true;
            self.scratch_focus = true;
            if let Some(f) = &mut self.find {
                f.focus = None;
            }
            let at = outline::line_range(self.scratch.text(), it.todo.line).map_or(0, |r| r.1);
            self.scratch.set_caret(at);
            return Some(Refresh::Partial);
        }
        let full = it.note != self.current;
        self.push_history();
        if full {
            self.open(&it.note);
        }
        if self.preview.is_some() {
            self.scratch_focus = false;
            if full {
                self.enter_preview();
            }
            let p = self.preview.as_mut()?;
            p.page = p.doc.page_of_line(it.todo.line);
            return Some(if full {
                Refresh::Full
            } else {
                Refresh::Partial
            });
        }
        self.scratch_focus = false;
        let page = self.page_rect();
        self.screen
            .jump_to_line(&mut self.buffer, page, it.todo.line);
        Some(if full {
            Refresh::Full
        } else {
            Refresh::Partial
        })
    }

    pub(crate) fn drawer_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let hit = self.drawer_hit_at(x, y);
        self.sel_ui = None;
        match hit {
            DrawerHit::Heading(line) => self.drawer_heading(line),
            DrawerHit::TodoBox(i) => self.drawer_todo_box(i),
            DrawerHit::TodoOpen(i) => self.drawer_todo_open(i),
            DrawerHit::Plugin(p) => self.pane_plugin_tap(p, x, y),
            DrawerHit::Filter => {
                self.filter = self.filter.next();
                Some(Refresh::Partial)
            }
            DrawerHit::Scratch(at) => {
                self.scratch_focus = true;
                if let Some(f) = &mut self.find {
                    f.focus = None;
                }
                self.scratch.set_caret(at);
                Some(Refresh::Fast)
            }
            DrawerHit::Inside | DrawerHit::Outside => None,
        }
    }

    pub(crate) fn scratch_key(&mut self, k: Key) -> Option<Refresh> {
        let tab = self.settings.tab_kind();
        match self.keyboard.press(k, &mut self.scratch, tab) {
            KeyAction::Edited => {
                self.save_scratch();
                Some(Refresh::Fast)
            }
            KeyAction::Vertical { down, .. } => {
                if down {
                    self.scratch.down();
                } else {
                    self.scratch.up();
                }
                Some(Refresh::Fast)
            }
            KeyAction::Relayout => {
                self.save_settings();
                Some(Refresh::Full)
            }
            KeyAction::Escape => {
                self.scratch_focus = false;
                Some(Refresh::Fast)
            }
            KeyAction::Moved | KeyAction::Relabelled => Some(Refresh::Fast),
            KeyAction::Page { .. } | KeyAction::Ignored | KeyAction::Find => None,
        }
    }

    pub(crate) fn scratch_hw(&mut self, k: HwKey) -> Option<Refresh> {
        let b = &mut self.scratch;
        match k {
            HwKey::Char(c) => {
                self.keyboard
                    .composer
                    .type_char(&mut self.scratch, self.settings.kb_layout, c);
            }
            HwKey::Backspace => self.keyboard.composer.backspace(b, self.settings.kb_layout),
            HwKey::Delete => b.delete_forward(),
            HwKey::Return => {
                if !self.keyboard.composer.enter(b) {
                    b.newline();
                }
            }
            HwKey::Tab => b.insert_tab(self.settings.tab_kind()),
            HwKey::Left => b.left(),
            HwKey::Right => b.right(),
            HwKey::Up => b.up(),
            HwKey::Down => b.down(),
            HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown | HwKey::Insert => {
                return None
            }
            HwKey::Escape => {
                self.scratch_focus = false;
                return Some(Refresh::Fast);
            }
        }
        self.save_scratch();
        Some(Refresh::Fast)
    }
}
