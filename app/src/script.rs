//! Hooks the emulator scripts and tests use to find and tap controls.

use super::*;

impl App {
    /// Centre of the on-screen key with this label (emulator scripts).
    #[must_use]
    pub fn key_centre(&self, label: &str) -> Option<(i32, i32)> {
        let rows = self.rows();
        let keys = match self.files_model(&rows) {
            Some(m) if self.rename.is_some() => self.screen.files_layout(&m).base.keys,
            _ => self.screen.layout(self.keys_visible(), &self.keyboard).keys,
        };
        keys.iter()
            .find(|(_, k)| self.keyboard.label(*k) == label)
            .map(|(r, _)| centre(*r))
    }

    /// Centre of a named control (emulator scripts): `Files`, `Title`,
    /// `OK`, `Cancel`, `Close`, `Up`, `New note`, `New folder`, `Rename`,
    /// `Move`, `Delete`, `More:<row name>`, or a row name.
    #[must_use]
    pub fn control_centre(&self, label: &str) -> Option<(i32, i32)> {
        if self.ink.is_some() {
            use sigma_render::ink::InkButton;
            let l = self.panel_layout()?;
            if let Some(i) = label
                .strip_prefix("Box")
                .and_then(|n| n.parse::<usize>().ok())
            {
                return l.boxes.get(i).map(|b| centre(*b));
            }
            let want = match label {
                "Undo" => InkButton::Undo,
                "Line" | "Guide" => InkButton::Guide,
                "Redo" => InkButton::Redo,
                "Clear" => InkButton::Clear,
                "Pen" => InkButton::Pen,
                "Cancel" | "Done" => InkButton::Cancel,
                "Insert" => InkButton::Insert,
                "Letters" | "Draw" | "Words" | "Sketch" | "Write" | "Mode" | "Swap" => {
                    InkButton::Mode
                }
                "Prev" => InkButton::Prev,
                "Next" => InkButton::Next,
                "Close" | "×" => InkButton::Cancel,
                "123" | "Digits" => InkButton::Digits,
                "Globe" | "Lang" => InkButton::Lang,
                "," | "." | "?" | "!" | "@" => InkButton::Type(label.chars().next()?),
                "Space" => InkButton::Space,
                "Erase" => InkButton::Back,
                "Return" => InkButton::Enter,
                "Home" => InkButton::Home,
                "End" => InkButton::End,
                "PgUp" => InkButton::PgUp,
                "PgDn" => InkButton::PgDn,
                "Ins" => InkButton::Ins,
                "Train" => InkButton::Train,
                "Skip" => InkButton::Skip,
                "Save" => InkButton::Save,
                "InkArea" => return Some(centre(l.area)),
                _ => {
                    let k = label.strip_prefix("Cand")?.parse().ok()?;
                    InkButton::Cand(k)
                }
            };
            return l
                .buttons
                .iter()
                .find(|(_, b)| *b == want)
                .map(|(r, _)| centre(*r));
        }
        if self.options {
            return self.screen.options_control(&self.options_model(), label);
        }
        if let Some(h) = &self.help {
            return self.screen.help_control(&self.help_model(h), label);
        }
        if self.plugin_screen.is_some() {
            return self.plugin_control(label);
        }
        if let Some(g) = &self.graph {
            let m = self.graph_model(g);
            let l = self.screen.graph_layout(&m);
            if label == "Close" {
                return Some(centre(l.close));
            }
            let want = match label {
                "Map" => Some(GraphHit::Map),
                "List" => Some(GraphHit::List),
                "Zoom+" => Some(GraphHit::ZoomIn),
                "Zoom-" => Some(GraphHit::ZoomOut),
                "Open" => Some(GraphHit::Open),
                _ => None,
            };
            if let Some(h) = want {
                return l
                    .toolbar
                    .iter()
                    .find(|(g, _)| *g == h)
                    .map(|(_, r)| centre(*r));
            }
            if let Some(name) = label.strip_prefix("Node:") {
                let i = g.rows.iter().position(|r| r.name == name)?;
                return self.screen.map_node_point(&m, i);
            }
            let name = label.strip_prefix("Graph:")?;
            return l
                .rows
                .iter()
                .find(|(i, _)| g.rows[*i].name == name)
                .map(|(_, r)| (r.x + r.w / 3, r.y + r.h / 2));
        }
        if let (Some(p), Some(name)) = (&self.preview, label.strip_prefix("Link:")) {
            return self
                .screen
                .preview_link_point(&self.keyboard, &p.doc, p.page, name);
        }
        let rows = self.rows();
        if let Some(m) = self.files_model(&rows) {
            let l = self.screen.files_layout(&m);
            let find_row = |name: &str| l.rows.iter().find(|(i, _, _)| rows[*i].name == name);
            return match label {
                "OK" => Some(centre(l.base.ok_rect)),
                "Cancel" if self.rename.is_some() => Some(centre(l.base.cancel_rect)),
                "Cancel" => Some(centre(l.bottom[0].union(&l.bottom[1]))),
                "Close" => Some(centre(l.close)),
                "Up" => Some(centre(l.crumb)),
                "New note" => Some(centre(l.bottom[0])),
                "New folder" => Some(centre(l.bottom[1])),
                "Rename" => l.actions.map(|a| centre(a[0])),
                "Move" => l.actions.map(|a| centre(a[1])),
                "Delete" => l.actions.map(|a| centre(a[2])),
                s if s.starts_with("More:") => find_row(&s[5..]).map(|(_, _, m)| centre(*m)),
                s => find_row(s).map(|(_, r, _)| (r.x + r.w / 3, r.y + r.h / 2)),
            };
        }
        if let Some((_, p)) = self.bar_buttons().into_iter().find(|(l, _)| *l == label) {
            return Some(p);
        }
        let l = self.screen.layout(self.keys_visible(), &self.keyboard);
        match label {
            "Back" if self.screen.back_on => Some(centre(l.back_button)),
            "Files" => Some(centre(l.files_button)),
            "Preview" | "Pencil" | "Eye" => Some(centre(l.preview_button)),
            "Title" => Some(centre(l.name_rect)),
            "OK" => Some(centre(l.ok_rect)),
            "Cancel" => Some(centre(l.cancel_rect)),
            "Keys" => Some(centre(l.keys_button)),
            "Write" => Some(centre(l.ink_button)),
            "Options" => Some(centre(l.options_button)),
            "Contents" if self.screen.contents_on => Some(centre(l.contents_button)),
            "To-dos" if self.screen.todos_on => Some(centre(l.todos_button)),
            "Scratch" if self.screen.scratch_on => Some(centre(l.scratch_button)),
            "Find" => Some(centre(l.find_button)),
            "Graph" if self.screen.graph_on => Some(centre(l.graph_button)),
            "FindField" | "ReplaceField" | "FindClose" | "Prev" | "Next" | "Replace" | "All"
            | "Aa" => self.screen.find_control(&l, label),
            s if s.starts_with("Heading:")
                || s.starts_with("Todo:")
                || s.starts_with("TodoBox:")
                || s == "Filter"
                || s == "ScratchPane" =>
            {
                self.drawer_control(s)
            }
            s => l
                .kb_buttons
                .iter()
                .find(|(_, b)| self.screen.kb_button_label(*b) == s)
                .map(|(r, _)| centre(*r)),
        }
    }

    /// Point just before (`end = false`) or after a word, on its text row
    /// (emulator pointer drags).
    #[must_use]
    pub fn text_point(&self, word: &str, end: bool) -> Option<(i32, i32)> {
        let at = self.buffer.text().rfind(word)? + if end { word.len() } else { 0 };
        let (x, y) = self.screen.point_of(&self.buffer, self.page_rect(), at)?;
        Some((x, y + self.screen.line_px() / 2))
    }

    /// Screen point in the middle of the last occurrence of `word`.
    #[must_use]
    pub fn word_point(&self, word: &str) -> Option<(i32, i32)> {
        let at = self.buffer.text().rfind(word)?;
        let page = self.page_rect();
        let (x0, y) = self.screen.point_of(&self.buffer, page, at)?;
        let (x1, _) = self.screen.point_of(&self.buffer, page, at + word.len())?;
        Some(((x0 + x1) / 2, y + self.screen.line_px() / 2))
    }

    /// Where a finger dragging a handle sits to put that end just after
    /// (`end`) or before the last occurrence of `word`.
    #[must_use]
    pub fn handle_point(&self, word: &str, end: bool) -> Option<(i32, i32)> {
        let at = self.buffer.text().rfind(word)? + if end { word.len() } else { 0 };
        let (x, y) = self.screen.point_of(&self.buffer, self.page_rect(), at)?;
        Some((x, y + self.screen.handle_drop()))
    }

    /// Centre of selection handle 0 (start) or 1 (end), if shown.
    #[must_use]
    pub fn handle_centre(&self, i: usize) -> Option<(i32, i32)> {
        let l = self.screen.layout(self.keys_visible(), &self.keyboard);
        let model = Model {
            file_name: self.current.name(),
            buffer: &self.buffer,
            keyboard: &self.keyboard,
            keys_on: self.keys_visible(),
            save: &self.autosave.state,
            theme: self.theme(),
            sel_ui: self.sel_ui,
            rename: None,
        };
        self.screen
            .selection_geometry(&model, &l)
            .handles
            .get(i)
            .copied()
            .flatten()
            .map(centre)
    }

    /// Labels of the action bar buttons, and their centres.
    #[must_use]
    pub fn bar_buttons(&self) -> Vec<(&'static str, (i32, i32))> {
        let l = self.screen.layout(self.keys_visible(), &self.keyboard);
        let model = Model {
            file_name: self.current.name(),
            buffer: &self.buffer,
            keyboard: &self.keyboard,
            keys_on: self.keys_visible(),
            save: &self.autosave.state,
            theme: self.theme(),
            sel_ui: self.sel_ui,
            rename: None,
        };
        self.screen
            .selection_geometry(&model, &l)
            .buttons
            .into_iter()
            .map(|(r, a)| (a.label(), centre(r)))
            .collect()
    }

    /// Drawer controls for emulator scripts: `Heading:TEXT`, `Todo:TEXT`
    /// (opens), `TodoBox:TEXT` (checks), `Filter`, `ScratchPane` (end of
    /// the scratch text).
    fn drawer_control(&self, name: &str) -> Option<(i32, i32)> {
        if !self.drawer_visible() {
            return None;
        }
        let (hs, todos) = (self.drawer_headings(), self.todo_list());
        let base = self.drawer_base();
        let l = self
            .screen
            .drawer_layout(&base, &self.drawer_model(&hs, &todos));
        if let Some((_, r)) = self.pane_hits().iter().find(|(label, _)| label == name) {
            return Some(centre(*r));
        }
        if name == "Filter" {
            return l.filter.map(centre);
        }
        if name == "ScratchPane" {
            return l.scratch.map(|r| (r.x + r.w - 4, r.y + r.h - 4));
        }
        if let Some(t) = name.strip_prefix("Heading:") {
            let line = hs.iter().find(|h| h.text == t)?.line;
            return l
                .headings
                .iter()
                .find(|(_, ln)| *ln == line)
                .map(|(r, _)| centre(*r));
        }
        let (boxed, t) = match name.strip_prefix("TodoBox:") {
            Some(t) => (true, t),
            None => (false, name.strip_prefix("Todo:")?),
        };
        l.todos
            .iter()
            .find(|(_, _, i)| todos[*i].todo.text == t)
            .map(|(b, tx, _)| centre(if boxed { *b } else { *tx }))
    }

    /// Every tappable control on the current screen, named.
    #[must_use]
    pub fn touch_targets(&self) -> Vec<(String, Rect)> {
        let mut out: Vec<(String, Rect)> = Vec::new();
        let mut add = |n: &str, r: Rect| out.push((n.to_owned(), r));
        if self.options {
            for (n, r) in self.screen.options_controls(&self.options_model()) {
                add(&n, r);
            }
            return out;
        }
        if let Some(h) = &self.help {
            let m = self.help_model(h);
            let l = self
                .screen
                .help_layout(m.keyboard, h.page != HelpPage::Menu);
            add("Close", l.close);
            if h.page == HelpPage::Menu {
                add("App info", l.rows[0]);
                add("Markdown reference", l.rows[1]);
                add("USB keyboard, mouse & export", l.rows[2]);
            } else if let Some(b) = l.back {
                add("Back", b);
            }
            return out;
        }
        if let Some(g) = &self.graph {
            let l = self.screen.graph_layout(&self.graph_model(g));
            add("Close", l.close);
            for (h, r) in &l.toolbar {
                add(&format!("graph {h:?}"), *r);
            }
            for (i, r) in &l.rows {
                add(&format!("graph row {i}"), *r);
            }
            for (i, r) in &l.nodes {
                add(
                    &format!("graph node {i}"),
                    self.screen.map_target(*r).intersect(&l.area),
                );
            }
            return out;
        }
        let rows = self.rows();
        if let Some(m) = self.files_model(&rows) {
            let l = self.screen.files_layout(&m);
            add("Close", l.close);
            if l.crumb.w > 0 && l.crumb.h > 0 {
                add("Up", l.crumb);
            }
            for (i, r, more) in &l.rows {
                add(&format!("file row {i}"), *r);
                add(&format!("file more {i}"), *more);
            }
            for (k, r) in l.actions.iter().flatten().enumerate() {
                add(&format!("action {k}"), *r);
            }
            add("New note", l.bottom[0]);
            add("New folder", l.bottom[1]);
            return out;
        }
        let l = self.page_layout();
        if self.screen.back_on {
            add("Back", l.back_button);
        }
        add("Files", l.files_button);
        add("Preview", l.preview_button);
        add("Find", l.find_button);
        if self.screen.graph_on {
            add("Graph", l.graph_button);
        }
        add("Options", l.options_button);
        if self.preview.is_some() {
            return out;
        }
        for (on, name, r) in [
            (self.screen.scratch_on, "Scratch", l.scratch_button),
            (self.screen.todos_on, "To-dos", l.todos_button),
            (self.screen.contents_on, "Contents", l.contents_button),
        ] {
            if on {
                add(name, r);
            }
        }
        add("Keys", l.keys_button);
        for (r, k) in &l.keys {
            add(&format!("key {}", self.keyboard.label(*k)), *r);
        }
        for (r, b) in &l.kb_buttons {
            add(self.screen.kb_button_label(*b), *r);
        }
        if self.find.is_some() {
            let f = self.screen.find_layout(&l);
            for (n, r) in [
                ("FindField", f.find),
                ("ReplaceField", f.replace),
                ("FindClose", f.close),
                ("Prev", f.prev),
                ("Next", f.next),
                ("Replace", f.replace_one),
                ("All", f.all),
                ("Aa", f.case),
            ] {
                add(n, r);
            }
        }
        if self.drawer_visible() {
            let (hs, todos) = (self.headings(), self.todo_list());
            let d = self
                .screen
                .drawer_layout(&l, &self.drawer_model(&hs, &todos));
            for (r, line) in &d.headings {
                add(&format!("heading {line}"), *r);
            }
            for (b, t, i) in &d.todos {
                add(&format!("todo box {i}"), *b);
                add(&format!("todo {i}"), *t);
            }
            if let Some(f) = d.filter {
                add("Filter", f);
            }
            for (label, r) in self.pane_hits() {
                add(&label, r);
            }
        }
        out
    }

    /// The smallest side a target may have: 44 px at the screen's scale.
    #[must_use]
    pub fn min_target(&self) -> i32 {
        // 44 px at the device scale; the UI scale only ever enlarges.
        self.screen.page_scale().px(44.0)
    }
}
