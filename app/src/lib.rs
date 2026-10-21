//! Sigma Scribe v0.2: a library of plain `.md` / `.txt` notes in folders,
//! the Files sheet, rename in place, and the v0.1 editor with on-screen
//! keyboard and autosave. Generic over the device so the Forma, the desktop
//! window and the headless emulator run the same code.

use sigma_core::buffer::TabKind;
use sigma_core::gesture::{Gesture, Recognizer};
use sigma_core::keyboard::{Dir, Key};
use sigma_core::library::SCRATCH;
use sigma_core::linefit::Anchor;
use sigma_core::outline::{self, Filter, Heading, Indexer, TodoItem};
use sigma_core::settings::{Orientation, Pad, Side};
use sigma_core::{
    Autosave, Buffer, Change, Entry, FsNoteStore, KeyAction, Keyboard, Kind, Library, NotePath,
    NoteStore, Settings,
};
use sigma_device::{Device, Event, HwKey, Phase, Refresh};
use sigma_render::editor::Model;
use sigma_render::{
    BarAction, Canvas, Doc, DrawerHit, DrawerModel, EditorScreen, FilesHit, FilesModel, Hit,
    KbButton, OptionsHit, OptionsModel, Pane, PreviewModel, Rect, Row, SelUi, Theme,
};
use std::path::PathBuf;

pub const NOTE: &str = "field-notes.md";
pub const SEED: &str = include_str!("../seed/field-notes.md");
pub const SEED_FILES: [(&str, &str); 4] = [
    (NOTE, SEED),
    (
        "drafts/chapter-01.md",
        include_str!("../seed/drafts/chapter-01.md"),
    ),
    ("scratch.txt", include_str!("../seed/scratch.txt")),
    (SCRATCH, ""),
];

/// Where notes live: `SIGMA_NOTES`, else `/mnt/onboard/sigma-scribe/notes`
/// on the Kobo (visible over USB), else `./target/emu-notes` on a desktop.
#[must_use]
pub fn notes_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SIGMA_NOTES") {
        return p.into();
    }
    if cfg!(target_arch = "arm") {
        "/mnt/onboard/sigma-scribe/notes".into()
    } else {
        "target/emu-notes".into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drag {
    /// Dragging a handle; the other end stays put.
    Handle { fixed: usize },
    /// Moving after a long-press on a word: extend word by word.
    Word { a: usize, b: usize },
}

fn rank(r: Refresh) -> u8 {
    match r {
        Refresh::Fast => 0,
        Refresh::Partial => 1,
        Refresh::Full => 2,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RenameTarget {
    Current,
    Entry(NotePath),
}

#[derive(Clone, Debug, Default)]
pub struct FilesState {
    pub folder: NotePath,
    pub entries: Vec<Entry>,
    pub more: Option<usize>,
    pub armed: Option<usize>,
    pub moving: Option<NotePath>,
    pub targets: Vec<NotePath>,
    pub message: Option<String>,
    pub first: usize,
}

/// Preview mode (v0.3): the laid-out note and the page shown. Read-only.
#[derive(Clone, Debug, Default)]
pub struct PreviewState {
    pub doc: Doc,
    pub page: usize,
}

pub struct App {
    pub library: Library,
    pub current: NotePath,
    pub buffer: Buffer,
    pub keyboard: Keyboard,
    pub autosave: Autosave,
    pub keys_on: bool,
    pub screen: EditorScreen,
    store: FsNoteStore,
    pub files: Option<FilesState>,
    rename: Option<(RenameTarget, Buffer)>,
    pub canvas: Canvas,
    shown: Canvas,
    gestures: Recognizer,
    /// Touch selection chrome (v0.2.2).
    pub sel_ui: Option<SelUi>,
    drag: Option<Drag>,
    /// `Some` while the note is shown rendered (eye ↔ pencil).
    pub preview: Option<PreviewState>,
    /// Every Options setting (v0.4), kept in `notes/.sigma-settings`.
    pub settings: Settings,
    /// The Options sheet is open.
    pub options: bool,
    /// Panel size as the device reports it (portrait on the Forma).
    phys: (u32, u32),
    /// Rotated copy of the canvas sent to the panel in landscape.
    phys_canvas: Canvas,
    /// Side drawer (v0.5): open panes (Contents, To-dos, Scratch).
    pub drawer: [bool; 3],
    pub filter: Filter,
    /// The scratch pad (`.scratchpad.md`), always loaded.
    pub scratch: Buffer,
    scratch_save: Autosave,
    /// The keyboard types into the scratch pad instead of the page.
    pub scratch_focus: bool,
    indexer: Indexer,
    /// To-dos of every note on disk (from the background indexer).
    disk_todos: Vec<TodoItem>,
    pub running: bool,
}

impl App {
    /// Opens the library at `root`, seeding it on first run, and restores
    /// the last opened note.
    #[must_use]
    pub fn new(width: u32, height: u32, root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let library = Library::new(&root);
        let _ = library.seed(&SEED_FILES);
        let current = library
            .last_opened()
            .or_else(|| NotePath::parse(NOTE).ok().filter(|p| library.exists(p)))
            .or_else(|| library.all_notes().ok().and_then(|n| n.into_iter().next()))
            .unwrap_or_else(|| NotePath::parse(NOTE).unwrap_or_default());
        let store = FsNoteStore::new(&root);
        let body = store
            .load(current.as_str())
            .ok()
            .flatten()
            .unwrap_or_default();
        let buffer = Buffer::new(body);
        let _ = library.set_last_opened(&current);
        let mut settings = load_settings(&root);
        let (lw, lh) = logical_size((width, height), settings.orientation);
        let mut screen = EditorScreen::new(lw, lh);
        screen.configure(&mut settings, None);
        let scratch = Buffer::new(store.load(SCRATCH).ok().flatten().unwrap_or_default());
        Self {
            autosave: Autosave::new(buffer.revision()),
            library,
            current,
            buffer,
            keyboard: Keyboard::with_layout(settings.keyboard),
            keys_on: true,
            screen,
            store,
            files: None,
            rename: None,
            gestures: Recognizer::default(),
            sel_ui: None,
            drag: None,
            preview: None,
            canvas: Canvas::new(lw, lh, 255),
            shown: Canvas::new(lw, lh, 0),
            settings,
            options: false,
            phys: (width, height),
            phys_canvas: Canvas::new(width, height, 255),
            drawer: [false; 3],
            filter: Filter::All,
            scratch_save: Autosave::new(scratch.revision()),
            scratch,
            scratch_focus: false,
            indexer: Indexer::new(),
            disk_todos: Vec::new(),
            running: true,
        }
    }

    /// Writes every setting to `notes/.sigma-settings` (atomic).
    fn save_settings(&mut self) {
        self.settings.keyboard = self.keyboard.layout;
        let _ = sigma_core::store::write_atomic(
            &self.library.root().join(SETTINGS),
            &self.settings.serialize(),
        );
    }

    /// The page theme (Options → Paper).
    #[must_use]
    pub fn theme(&self) -> Theme {
        Theme::for_paper(self.settings.paper)
    }

    #[must_use]
    pub fn is_landscape(&self) -> bool {
        self.settings.orientation == Orientation::Landscape
    }

    /// Applies one Options / keyboard-bar change: refits Line mode,
    /// re-lays out (and resizes on rotation), saves the settings file.
    /// Paper and orientation need a full (flashing) refresh.
    pub fn change(&mut self, c: Change) -> Refresh {
        self.settings.apply(c);
        if let Change::Keyboard(l) = c {
            self.keyboard = Keyboard {
                layout: l,
                clipboard: std::mem::take(&mut self.keyboard.clipboard),
                ..Keyboard::default()
            };
        }
        let anchor = match c.refit() {
            Some(Some(true)) => Some(Anchor::Size),
            Some(Some(false)) => Some(Anchor::Margin),
            _ => None,
        };
        let (w, h) = logical_size(self.phys, self.settings.orientation);
        if (self.screen.width, self.screen.height) != (w, h) {
            self.screen.resize(w, h);
            self.canvas = Canvas::new(w, h, 255);
            self.shown = Canvas::new(w, h, 0);
            self.sel_ui = None;
        }
        self.screen.configure(&mut self.settings, anchor);
        self.save_settings();
        if c.needs_full_refresh() {
            Refresh::Full
        } else {
            Refresh::Partial
        }
    }

    fn save_scratch(&mut self) {
        let rev = self.scratch.revision();
        self.scratch_save
            .sync(&mut self.store, SCRATCH, self.scratch.text(), rev);
    }

    /// The drawer is drawn and takes taps (write mode, no sheet).
    #[must_use]
    pub fn drawer_visible(&self) -> bool {
        self.drawer.iter().any(|&o| o) && self.in_editor()
    }

    fn scratch_focused(&self) -> bool {
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
        out.extend(live(&scratch, self.scratch.text()));
        out
    }

    fn drawer_model<'a>(&'a self, hs: &'a [Heading], todos: &'a [TodoItem]) -> DrawerModel<'a> {
        DrawerModel {
            side: self.settings.drawer,
            contents: self.drawer[0].then_some(hs),
            todos: self.drawer[1].then_some((todos, self.filter)),
            scratch: self.drawer[2].then_some((&self.scratch, self.scratch_focused())),
            theme: self.theme(),
        }
    }

    fn drawer_hit_at(&self, x: i32, y: i32) -> DrawerHit {
        if !self.drawer_visible() {
            return DrawerHit::Outside;
        }
        let (hs, todos) = (self.headings(), self.todo_list());
        let base = self.screen.layout(self.keys_visible(), &self.keyboard);
        self.screen
            .drawer_hit(&base, &self.drawer_model(&hs, &todos), x, y)
    }

    fn in_drawer(&self, x: i32, y: i32) -> bool {
        self.drawer_hit_at(x, y) != DrawerHit::Outside
    }

    /// Opens or closes one drawer pane (no animation, partial refresh).
    pub fn toggle_pane(&mut self, p: Pane) -> Refresh {
        let i = match p {
            Pane::Contents => 0,
            Pane::Todos => 1,
            Pane::Scratch => 2,
        };
        self.drawer[i] = !self.drawer[i];
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

    fn reindex(&mut self) {
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

    fn drawer_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let hit = self.drawer_hit_at(x, y);
        self.sel_ui = None;
        let page = self.page_rect();
        match hit {
            DrawerHit::Heading(line) => {
                self.scratch_focus = false;
                self.screen.jump_to_line(&mut self.buffer, page, line);
                Some(Refresh::Partial)
            }
            DrawerHit::TodoBox(i) => {
                let it = self.todo_list().get(i)?.clone();
                self.toggle_todo(&it)
            }
            DrawerHit::TodoOpen(i) => {
                let it = self.todo_list().get(i)?.clone();
                if it.note.as_str() == SCRATCH {
                    self.drawer[2] = true;
                    self.scratch_focus = true;
                    let at =
                        outline::line_range(self.scratch.text(), it.todo.line).map_or(0, |r| r.1);
                    self.scratch.set_caret(at);
                    return Some(Refresh::Partial);
                }
                let full = it.note != self.current;
                if full {
                    self.open(&it.note);
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
            DrawerHit::Filter => {
                self.filter = self.filter.next();
                Some(Refresh::Partial)
            }
            DrawerHit::Scratch(at) => {
                self.scratch_focus = true;
                self.scratch.set_caret(at);
                Some(Refresh::Fast)
            }
            DrawerHit::Inside | DrawerHit::Outside => None,
        }
    }

    fn scratch_key(&mut self, k: Key) -> Option<Refresh> {
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
            KeyAction::Page { .. } | KeyAction::Ignored => None,
        }
    }

    fn scratch_hw(&mut self, k: HwKey) -> Option<Refresh> {
        let b = &mut self.scratch;
        match k {
            HwKey::Char(c) => b.insert(&c.to_string()),
            HwKey::Backspace => b.backspace(),
            HwKey::Delete => b.delete_forward(),
            HwKey::Return => b.newline(),
            HwKey::Tab => b.insert_tab(self.settings.tab_kind()),
            HwKey::Left => b.left(),
            HwKey::Right => b.right(),
            HwKey::Up => b.up(),
            HwKey::Down => b.down(),
            HwKey::Escape => {
                self.scratch_focus = false;
                return Some(Refresh::Fast);
            }
        }
        self.save_scratch();
        Some(Refresh::Fast)
    }

    /// Opens the Options sheet (closes preview and selection chrome).
    pub fn open_options(&mut self) {
        self.save();
        self.preview = None;
        self.sel_ui = None;
        self.drag = None;
        self.options = true;
    }

    fn options_model(&self) -> OptionsModel<'_> {
        OptionsModel {
            settings: &self.settings,
            theme: self.theme(),
            keyboard: &self.keyboard,
        }
    }

    fn options_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        match self.screen.options_hit(&self.options_model(), x, y) {
            OptionsHit::Close => {
                self.options = false;
                Some(Refresh::Full)
            }
            OptionsHit::Change(c) => Some(self.change(c)),
            OptionsHit::Nothing => None,
        }
    }

    fn kb_button(&mut self, b: KbButton) -> Option<Refresh> {
        let c = match b {
            KbButton::Mode(m) if m == self.settings.land_keys => return None,
            KbButton::Mode(m) => Change::LandKeys(m),
            KbButton::PadToggle => Change::Pad(match self.settings.pad {
                Pad::Numbers => Pad::Arrows,
                Pad::Arrows => Pad::Numbers,
            }),
            KbButton::PadSide => Change::PadSide(match self.settings.pad_side {
                Side::Left => Side::Right,
                Side::Right => Side::Left,
            }),
        };
        Some(self.change(c))
    }

    /// Device coordinates → screen coordinates (landscape turns the
    /// picture 90° clockwise on the portrait panel).
    #[must_use]
    pub fn to_logical(&self, e: Event) -> Event {
        if !self.is_landscape() {
            return e;
        }
        let lh = self.canvas.height as i32;
        let m = |x: i32, y: i32| (y, lh - 1 - x);
        match e {
            Event::Tap { x, y } => {
                let (x, y) = m(x, y);
                Event::Tap { x, y }
            }
            Event::Touch { phase, x, y, t } => {
                let (x, y) = m(x, y);
                Event::Touch { phase, x, y, t }
            }
            e => e,
        }
    }

    /// What the panel shows, turned back to the screen's orientation
    /// (emulator screenshots).
    #[must_use]
    pub fn view(&self, panel: &Canvas) -> Canvas {
        if !self.is_landscape() {
            return panel.clone();
        }
        let (lw, lh) = (self.canvas.width as usize, self.canvas.height as usize);
        let pw = panel.width as usize;
        let mut out = Canvas::new(lw as u32, lh as u32, 255);
        for y in 0..lh {
            for x in 0..lw {
                out.pixels[y * lw + x] = panel.pixels[x * pw + (lh - 1 - y)];
            }
        }
        out
    }

    fn present(&mut self, dev: &mut dyn Device, r: Rect, refresh: Refresh) -> std::io::Result<()> {
        if !self.is_landscape() {
            return dev.present(&self.canvas, r, refresh);
        }
        let (lw, lh) = (self.canvas.width as i32, self.canvas.height as i32);
        let pw = self.phys_canvas.width as usize;
        let (x0, y0) = (r.x.max(0), r.y.max(0));
        let (x1, y1) = ((r.x + r.w).min(lw), (r.y + r.h).min(lh));
        for y in y0..y1 {
            for x in x0..x1 {
                let v = self.canvas.pixels[(y * lw + x) as usize];
                let (px, py) = ((lh - 1 - y) as usize, x as usize);
                self.phys_canvas.pixels[py * pw + px] = v;
            }
        }
        // Width and height swap, so the >= 16 px minimum still holds.
        let pr = Rect::new(lh - y1, x0, y1 - y0, x1 - x0);
        dev.present(&self.phys_canvas, pr, refresh)
    }

    fn after_key(&mut self, action: KeyAction) -> Option<Refresh> {
        let page = self.screen.layout(self.keys_visible(), &self.keyboard).page;
        match action {
            KeyAction::Edited => {
                self.save();
                Some(Refresh::Fast)
            }
            KeyAction::Moved | KeyAction::Relabelled => Some(Refresh::Fast),
            KeyAction::Vertical { down, extend } => {
                self.screen
                    .caret_row_ext(&mut self.buffer, page, down, extend);
                Some(Refresh::Fast)
            }
            KeyAction::Page { forward, extend } => {
                self.screen
                    .page_turn_ext(&mut self.buffer, page, forward, extend);
                Some(Refresh::Partial)
            }
            KeyAction::Relayout => {
                self.save_settings();
                Some(Refresh::Full)
            }
            KeyAction::Escape | KeyAction::Ignored => None,
        }
    }

    fn save(&mut self) {
        let rev = self.buffer.revision();
        self.autosave.sync(
            &mut self.store,
            self.current.as_str(),
            self.buffer.text(),
            rev,
        );
    }

    /// Saves the open note and opens another.
    pub fn open(&mut self, p: &NotePath) {
        self.save();
        let body = self
            .store
            .load(p.as_str())
            .ok()
            .flatten()
            .unwrap_or_default();
        self.current = p.clone();
        self.buffer = Buffer::new(body);
        self.autosave = Autosave::new(self.buffer.revision());
        self.screen.scroll = 0;
        let _ = self.library.set_last_opened(p);
        if self.drawer[1] {
            self.reindex();
        }
    }

    #[must_use]
    pub fn is_renaming(&self) -> bool {
        self.rename.is_some()
    }

    #[must_use]
    pub fn rename_text(&self) -> Option<&str> {
        self.rename.as_ref().map(|(_, b)| b.text())
    }

    fn keys_visible(&self) -> bool {
        self.rename.is_some()
            || (self.files.is_none() && self.preview.is_none() && !self.options && self.keys_on)
    }

    /// Names a `[[wiki]]` link may use for an existing note: the file name
    /// or the path from the library root, with or without the extension.
    fn note_names(&self) -> std::collections::HashSet<String> {
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

    /// Switches to preview at the page showing the top of the write page.
    /// Full refresh: the whole page changes.
    pub fn enter_preview(&mut self) {
        self.save();
        self.sel_ui = None;
        self.drag = None;
        let names = self.note_names();
        let known = |n: &str| names.contains(&n.trim().to_lowercase());
        let doc = self
            .screen
            .build_preview(self.buffer.text(), &self.keyboard, &known);
        let page = self.page_rect();
        let top = self.screen.top_source_line(&self.buffer, page);
        let page = doc.page_of_line(top);
        self.preview = Some(PreviewState { doc, page });
    }

    /// Back to write mode; caret, selection and scroll are as they were.
    pub fn exit_preview(&mut self) {
        self.preview = None;
    }

    /// Preview input: pencil, Files, page turns, Escape. Everything else is
    /// ignored (read-only, decision S3-D1).
    fn preview_event(&mut self, e: Event) -> Option<Refresh> {
        match e {
            Event::Tap { x, y } => match self.screen.preview_hit(&self.keyboard, x, y) {
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
                _ => None,
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
            Event::Key(HwKey::Escape) => {
                self.exit_preview();
                Some(Refresh::Full)
            }
            _ => None,
        }
    }

    fn rows(&self) -> Vec<Row> {
        let Some(f) = &self.files else {
            return Vec::new();
        };
        if f.moving.is_some() {
            return f
                .targets
                .iter()
                .map(|p| Row {
                    name: if p.is_root() {
                        "Files (top level)".into()
                    } else {
                        p.as_str().replace('/', " / ")
                    },
                    detail: "Folder".into(),
                    folder: true,
                    current: false,
                })
                .collect();
        }
        f.entries
            .iter()
            .map(|e| Row {
                name: e.path.name().to_owned(),
                detail: match e.kind {
                    Kind::Folder => "Folder".into(),
                    Kind::Note if e.words == 1 => "1 word".into(),
                    Kind::Note => format!("{} words", e.words),
                },
                folder: e.kind == Kind::Folder,
                current: e.kind == Kind::Note && (e.path == self.current),
            })
            .collect()
    }

    fn files_model<'a>(&'a self, rows: &'a [Row]) -> Option<FilesModel<'a>> {
        let f = self.files.as_ref()?;
        Some(FilesModel {
            title: if f.folder.is_root() {
                "Files".into()
            } else {
                format!("Files / {}", f.folder.as_str().replace('/', " / "))
            },
            crumb: (!f.folder.is_root()).then(|| {
                let up = f.folder.parent();
                if up.is_root() {
                    "Up to Files".into()
                } else {
                    format!("Up to {}", up.name())
                }
            }),
            rows,
            more: f.more,
            armed: f.armed,
            moving: f.moving.as_ref().map(NotePath::name),
            message: f.message.as_deref(),
            first: f.first,
            theme: self.theme(),
            rename: self.rename.as_ref().map(|(_, b)| b),
            keyboard: &self.keyboard,
        })
    }

    fn refresh_files(&mut self) {
        let lib = self.library.clone();
        if let Some(f) = &mut self.files {
            f.entries = lib.list(&f.folder).unwrap_or_default();
            f.more = None;
            f.armed = None;
            f.first = f.first.min(f.entries.len().saturating_sub(1));
        }
    }

    /// Opens the Files sheet at the current note's folder.
    pub fn open_files(&mut self) {
        self.save();
        self.files = Some(FilesState {
            folder: self.current.parent(),
            ..FilesState::default()
        });
        self.refresh_files();
    }

    /// Redraws and sends only what changed, with the given refresh class.
    ///
    /// # Errors
    /// The device could not present the frame.
    pub fn render(&mut self, dev: &mut dyn Device, refresh: Refresh) -> std::io::Result<()> {
        let mut c = std::mem::replace(&mut self.canvas, Canvas::new(1, 1, 0));
        if self.options {
            self.screen.draw_options(&mut c, &self.options_model());
        } else if let Some(p) = &self.preview {
            let m = PreviewModel {
                file_name: self.current.name(),
                doc: &p.doc,
                page: p.page,
                words: sigma_core::text::word_count(self.buffer.text()),
                save: &self.autosave.state,
                keyboard: &self.keyboard,
                theme: self.theme(),
            };
            self.screen.draw_preview(&mut c, &m);
        } else if self.files.is_some() {
            let rows = self.rows();
            if let Some(m) = self.files_model(&rows) {
                self.screen.draw_files(&mut c, &m);
            }
        } else {
            let renaming_current = matches!(&self.rename, Some((RenameTarget::Current, _)));
            let model = Model {
                file_name: self.current.name(),
                buffer: &self.buffer,
                keyboard: &self.keyboard,
                keys_on: self.keys_visible(),
                save: &self.autosave.state,
                theme: self.theme(),
                sel_ui: if self.rename.is_some() {
                    None
                } else {
                    self.sel_ui
                },
                rename: if renaming_current {
                    self.rename.as_ref().map(|(_, b)| b)
                } else {
                    None
                },
            };
            self.screen.drawer_open = if self.drawer_visible() {
                self.drawer
            } else {
                [false; 3]
            };
            let l = self.screen.draw(&mut c, &model);
            if self.drawer_visible() {
                let (hs, todos) = (self.headings(), self.todo_list());
                self.screen
                    .draw_drawer(&mut c, &l, &self.drawer_model(&hs, &todos));
            }
        }
        self.canvas = c;
        // Only what changed goes to the panel: a pressed modifier redraws its
        // cap, a typed letter its text row and the word count.
        let regions = if refresh == Refresh::Full {
            vec![self.canvas.bounds()]
        } else {
            self.canvas.diff_regions(&self.shown, 24, 8)
        };
        for r in &regions {
            self.present(dev, *r, refresh)?;
        }
        if !regions.is_empty() {
            self.shown.pixels.copy_from_slice(&self.canvas.pixels);
        }
        Ok(())
    }

    fn start_rename(&mut self, target: RenameTarget) {
        let name = match &target {
            RenameTarget::Current => self.current.name().to_owned(),
            RenameTarget::Entry(p) => p.name().to_owned(),
        };
        let k = &self.keyboard;
        self.keyboard = Keyboard {
            layout: k.layout,
            clipboard: k.clipboard.clone(),
            ..Keyboard::default()
        };
        self.rename = Some((target, Buffer::new(name)));
    }

    fn finish_rename(&mut self, ok: bool) {
        let Some((target, field)) = self.rename.take() else {
            return;
        };
        if !ok {
            return;
        }
        let path = match &target {
            RenameTarget::Current => self.current.clone(),
            RenameTarget::Entry(p) => p.clone(),
        };
        self.save();
        match self.library.rename(&path, field.text()) {
            Ok(new) => {
                if self.current == path {
                    self.current = new;
                } else if self.current.within(&path) && !path.is_root() {
                    if let Ok(p) = NotePath::parse(&format!(
                        "{}{}",
                        new.as_str(),
                        &self.current.as_str()[path.as_str().len()..]
                    )) {
                        self.current = p;
                    }
                }
                let _ = self.library.set_last_opened(&self.current);
                self.refresh_files();
            }
            Err(e) => {
                if let Some(f) = &mut self.files {
                    f.message = Some(e.to_string());
                }
            }
        }
    }

    fn type_into_field(&mut self, key: Key) -> Option<Refresh> {
        let (_, field) = self.rename.as_mut()?;
        match key {
            Key::Return => {
                self.finish_rename(true);
                return Some(Refresh::Full);
            }
            Key::Tab | Key::PgUp | Key::PgDn | Key::Arrow(Dir::Up | Dir::Down) => {}
            Key::Esc => {
                self.finish_rename(false);
                return Some(Refresh::Full);
            }
            k => {
                if self.keyboard.press(k, field, TabKind::Spaces(0)) == KeyAction::Relayout {
                    self.save_settings();
                    return Some(Refresh::Full);
                }
            }
        }
        Some(Refresh::Fast)
    }

    fn hw_into_field(&mut self, k: HwKey) -> Option<Refresh> {
        let (_, field) = self.rename.as_mut()?;
        match k {
            HwKey::Char(c) => field.insert(&c.to_string()),
            HwKey::Backspace => field.backspace(),
            HwKey::Delete => field.delete_forward(),
            HwKey::Left => field.left(),
            HwKey::Right => field.right(),
            HwKey::Return => {
                self.finish_rename(true);
                return Some(Refresh::Full);
            }
            HwKey::Escape => {
                self.finish_rename(false);
                return Some(Refresh::Full);
            }
            HwKey::Tab | HwKey::Up | HwKey::Down => {}
        }
        Some(Refresh::Fast)
    }

    fn files_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let rows = self.rows();
        let hit = self
            .files_model(&rows)
            .map_or(FilesHit::Nothing, |m| self.screen.files_hit(&m, x, y));
        let lib = self.library.clone();
        let f = self.files.as_mut()?;
        let was_armed = f.armed.take();
        f.message = None;
        match hit {
            FilesHit::Close => {
                self.files = None;
                return Some(Refresh::Full);
            }
            FilesHit::Up => {
                f.folder = f.folder.parent();
                f.first = 0;
                self.refresh_files();
            }
            FilesHit::Open(i) => {
                if let Some(target) = f.moving.clone() {
                    let dest = f.targets.get(i).cloned().unwrap_or_default();
                    f.moving = None;
                    self.save();
                    match lib.move_to(&target, &dest) {
                        Ok(new) => {
                            if self.current == target {
                                self.current = new;
                            } else if self.current.within(&target) {
                                if let Ok(p) = NotePath::parse(&format!(
                                    "{}{}",
                                    new.as_str(),
                                    &self.current.as_str()[target.as_str().len()..]
                                )) {
                                    self.current = p;
                                }
                            }
                            let _ = lib.set_last_opened(&self.current);
                            self.refresh_files();
                        }
                        Err(e) => {
                            self.refresh_files();
                            if let Some(f) = &mut self.files {
                                f.message = Some(e.to_string());
                            }
                        }
                    }
                    return Some(Refresh::Partial);
                }
                let e = f.entries.get(i)?.clone();
                if e.kind == Kind::Folder {
                    f.folder = e.path;
                    f.first = 0;
                    self.refresh_files();
                } else {
                    self.files = None;
                    self.open(&e.path);
                    return Some(Refresh::Full);
                }
            }
            FilesHit::More(i) => f.more = if f.more == Some(i) { None } else { Some(i) },
            FilesHit::Rename(i) => {
                let p = f.entries.get(i)?.path.clone();
                self.start_rename(RenameTarget::Entry(p));
            }
            FilesHit::Move(i) => {
                let p = f.entries.get(i)?.path.clone();
                let is_dir = lib.fs_path(&p).is_dir();
                f.targets = lib
                    .folders()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| !(is_dir && t.within(&p)) && *t != p.parent())
                    .collect();
                f.moving = Some(p);
                f.more = None;
                f.first = 0;
            }
            FilesHit::Delete(i) => {
                if was_armed == Some(i) {
                    let p = f.entries.get(i)?.path.clone();
                    match lib.delete(&p) {
                        Ok(()) => {
                            let gone = self.current.within(&p);
                            self.refresh_files();
                            if gone {
                                let next = lib.all_notes().ok().and_then(|n| n.into_iter().next());
                                if let Some(n) = next {
                                    // The deleted note must not be re-saved.
                                    self.autosave = Autosave::new(self.buffer.revision());
                                    self.current = n.clone();
                                    self.open(&n);
                                }
                            }
                        }
                        Err(e) => f.message = Some(e.to_string()),
                    }
                } else {
                    f.armed = Some(i);
                }
            }
            FilesHit::NewNote => {
                let folder = f.folder.clone();
                match lib.create_note(&folder, "untitled") {
                    Ok(p) => {
                        self.files = None;
                        self.open(&p);
                        self.start_rename(RenameTarget::Current);
                        return Some(Refresh::Full);
                    }
                    Err(e) => f.message = Some(e.to_string()),
                }
            }
            FilesHit::NewFolder => {
                let folder = f.folder.clone();
                match lib.create_folder(&folder, "new folder") {
                    Ok(p) => {
                        self.refresh_files();
                        // Brief: a new folder starts in rename.
                        self.start_rename(RenameTarget::Entry(p));
                    }
                    Err(e) => f.message = Some(e.to_string()),
                }
            }
            FilesHit::CancelMove => {
                f.moving = None;
            }
            FilesHit::Key(k) => return self.type_into_field(k),
            FilesHit::RenameOk => self.finish_rename(true),
            FilesHit::RenameCancel => self.finish_rename(false),
            FilesHit::Nothing => {}
        }
        Some(Refresh::Partial)
    }

    /// Applies one event; returns the refresh it needs (or `None`).
    fn in_editor(&self) -> bool {
        self.files.is_none() && self.rename.is_none() && self.preview.is_none() && !self.options
    }

    fn apply(&mut self, gestures: Vec<Gesture>) -> Option<Refresh> {
        let mut out: Option<Refresh> = None;
        for g in gestures {
            if let Some(r) = self.gesture(g) {
                out = Some(match out {
                    Some(o) if rank(o) >= rank(r) => o,
                    _ => r,
                });
            }
        }
        out
    }

    fn select_word_at(&mut self, x: i32, y: i32) -> Option<(usize, usize)> {
        let page = self.page_rect();
        let o = self.screen.offset_at_point(&self.buffer, page, x, y);
        match self.buffer.word_at(o) {
            Some((a, b)) => {
                self.buffer.select(a, b);
                Some((a, b))
            }
            None => {
                self.buffer.set_caret(o);
                None
            }
        }
    }

    fn gesture(&mut self, g: Gesture) -> Option<Refresh> {
        let page = self.page_rect();
        let on_page = |x: i32, y: i32| page.contains(x, y);
        match g {
            Gesture::Press { x, y } => {
                self.drag = None;
                if self.in_editor() && self.sel_ui.is_some() {
                    if let (Hit::Handle(i), Some((a, b))) =
                        (self.editor_hit(x, y), self.buffer.selection())
                    {
                        self.drag = Some(Drag::Handle {
                            fixed: if i == 0 { b } else { a },
                        });
                        self.sel_ui = Some(SelUi { bar: false });
                        return Some(Refresh::Fast);
                    }
                }
                None
            }
            Gesture::Tap { x, y } => {
                if self.drag.take().is_some() {
                    self.sel_ui = Some(SelUi { bar: true });
                    return Some(Refresh::Fast);
                }
                self.handle(Event::Tap { x, y })
            }
            Gesture::DoubleTap { x, y }
                if self.in_editor()
                    && on_page(x, y)
                    && !self.in_drawer(x, y)
                    && self.sel_ui.is_none() =>
            {
                self.select_word_at(x, y);
                self.sel_ui = Some(SelUi { bar: true });
                Some(Refresh::Fast)
            }
            Gesture::DoubleTap { x, y } => self.handle(Event::Tap { x, y }),
            Gesture::LongPress { .. } if self.drag.is_some() => None,
            Gesture::LongPress { x, y }
                if self.in_editor() && on_page(x, y) && !self.in_drawer(x, y) =>
            {
                self.drag = self.select_word_at(x, y).map(|(a, b)| Drag::Word { a, b });
                self.sel_ui = Some(SelUi { bar: true });
                Some(Refresh::Fast)
            }
            // A held key or row still counts as one tap.
            Gesture::LongPress { x, y } => self.handle(Event::Tap { x, y }),
            Gesture::Drag { x, y, .. } => {
                let drag = self.drag?;
                let o = |app: &Self, yy: i32| app.screen.offset_at_point(&app.buffer, page, x, yy);
                match drag {
                    Drag::Handle { fixed } => {
                        // The finger is below the handle; aim at the text row.
                        let at = o(
                            self,
                            y - self.screen.handle_drop() + self.screen.line_px() / 2,
                        );
                        if at != fixed {
                            self.buffer.select(fixed, at);
                        }
                    }
                    Drag::Word { a, b } => {
                        let at = o(self, y);
                        let (s0, e0) = if at >= b {
                            (a, self.buffer.word_at(at).map_or(at, |w| w.1).max(b))
                        } else if at < a {
                            (self.buffer.word_at(at).map_or(at, |w| w.0), b)
                        } else {
                            (a, b)
                        };
                        if at < a {
                            self.buffer.select(e0, s0);
                        } else {
                            self.buffer.select(s0, e0);
                        }
                    }
                }
                self.sel_ui = Some(SelUi { bar: false });
                Some(Refresh::Fast)
            }
            Gesture::Release { x, y, held } => {
                // Throttled drags: the lift position is always applied.
                if self.drag.is_some() {
                    self.gesture(Gesture::Drag { x, y, held });
                }
                if self.drag.take().is_some() || (held && self.sel_ui.is_some()) {
                    self.sel_ui = Some(SelUi { bar: true });
                    Some(Refresh::Fast)
                } else if held {
                    None
                } else {
                    // A slide that never held: a tap where the finger lifted
                    // (keeps fast typing as forgiving as v0.2.1).
                    self.handle(Event::Tap { x, y })
                }
            }
        }
    }

    fn editor_hit(&self, x: i32, y: i32) -> Hit {
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
        self.screen.hit(&model, x, y)
    }

    fn bar_action(&mut self, a: BarAction) -> Option<Refresh> {
        let sel = self.buffer.selected_text().map(str::to_owned);
        match a {
            BarAction::Cut => {
                if let Some(t) = sel {
                    self.keyboard.clipboard = t;
                    self.buffer.backspace();
                    self.save();
                }
                self.sel_ui = None;
            }
            BarAction::Copy => {
                if let Some(t) = sel {
                    self.keyboard.clipboard = t;
                }
                let end = self
                    .buffer
                    .selection()
                    .map_or(self.buffer.caret(), |(_, b)| b);
                self.buffer.set_caret(end);
                self.sel_ui = None;
            }
            BarAction::Paste => {
                if !self.keyboard.clipboard.is_empty() {
                    let t = self.keyboard.clipboard.clone();
                    self.buffer.insert(&t);
                    self.save();
                }
                self.sel_ui = None;
            }
            BarAction::SelectAll => {
                self.buffer.select_all();
                self.sel_ui = Some(SelUi { bar: true });
            }
        }
        Some(Refresh::Fast)
    }

    pub fn handle(&mut self, e: Event) -> Option<Refresh> {
        let page = self.screen.layout(self.keys_visible(), &self.keyboard).page;
        match e {
            Event::Touch { phase, x, y, t } => {
                let g = match phase {
                    Phase::Down => self.gestures.down(x, y, t),
                    Phase::Move => self.gestures.moved(x, y, t),
                    Phase::Up => self.gestures.up(x, y, t),
                };
                self.apply(g)
            }
            Event::Tick { t } => {
                if let Some(items) = self.indexer.poll() {
                    let changed = items != self.disk_todos;
                    self.disk_todos = items;
                    if changed && self.drawer[1] && self.drawer_visible() {
                        return Some(Refresh::Partial);
                    }
                }
                if !self.gestures.finger_down() {
                    return None;
                }
                let g = self.gestures.tick(t);
                self.apply(g)
            }
            Event::Quit | Event::Power => {
                self.save();
                self.save_scratch();
                self.running = false;
                None
            }
            Event::Tap { x, y } if self.options => self.options_tap(x, y),
            Event::Key(HwKey::Escape) if self.options => {
                self.options = false;
                Some(Refresh::Full)
            }
            Event::Key(_) | Event::PageTurn { .. } if self.options => None,
            e @ (Event::Tap { .. } | Event::PageTurn { .. } | Event::Key(_))
                if self.preview.is_some() && self.files.is_none() =>
            {
                self.preview_event(e)
            }
            Event::Key(k) if self.rename.is_some() => self.hw_into_field(k),
            Event::Tap { x, y } if self.files.is_some() => self.files_tap(x, y),
            Event::PageTurn { forward } if self.files.is_some() => {
                let rows = self.rows();
                let visible = self
                    .files_model(&rows)
                    .map_or(1, |m| self.screen.files_layout(&m).visible.max(1));
                let f = self.files.as_mut()?;
                let n = if f.moving.is_some() {
                    f.targets.len()
                } else {
                    f.entries.len()
                };
                f.first = if forward {
                    (f.first + visible).min(n.saturating_sub(1))
                } else {
                    f.first.saturating_sub(visible)
                };
                Some(Refresh::Partial)
            }
            Event::Key(HwKey::Escape) if self.files.is_some() => {
                self.files = None;
                Some(Refresh::Full)
            }
            Event::Key(_) if self.files.is_some() => None,
            Event::PageTurn { forward } => {
                self.screen.page_turn(&mut self.buffer, page, forward);
                Some(Refresh::Partial)
            }
            Event::Key(k) if self.scratch_focused() => self.scratch_hw(k),
            Event::Tap { x, y } if self.in_drawer(x, y) => self.drawer_tap(x, y),
            Event::Key(k) => {
                match k {
                    HwKey::Char(c) => self.buffer.insert(&c.to_string()),
                    HwKey::Backspace => self.buffer.backspace(),
                    HwKey::Delete => self.buffer.delete_forward(),
                    HwKey::Return => self.buffer.newline(),
                    HwKey::Tab => self.buffer.insert_tab(self.settings.tab_kind()),
                    HwKey::Left => self.buffer.left(),
                    HwKey::Right => self.buffer.right(),
                    HwKey::Up => self.screen.caret_row(&mut self.buffer, page, false),
                    HwKey::Down => self.screen.caret_row(&mut self.buffer, page, true),
                    HwKey::Escape => {
                        let had = self.sel_ui.take().is_some();
                        return had.then_some(Refresh::Fast);
                    }
                }
                self.sel_ui = None;
                self.save();
                Some(Refresh::Fast)
            }
            Event::Tap { x, y } => {
                let renaming = matches!(&self.rename, Some((RenameTarget::Current, _)));
                let model = Model {
                    file_name: self.current.name(),
                    buffer: &self.buffer,
                    keyboard: &self.keyboard,
                    keys_on: self.keys_visible(),
                    save: &self.autosave.state,
                    theme: self.theme(),
                    sel_ui: if self.rename.is_some() {
                        None
                    } else {
                        self.sel_ui
                    },
                    rename: if renaming {
                        self.rename.as_ref().map(|(_, b)| b)
                    } else {
                        None
                    },
                };
                let hit = self.screen.hit(&model, x, y);
                if !matches!(hit, Hit::Bar(_) | Hit::Handle(_)) {
                    // Tapping elsewhere dismisses the selection chrome.
                    self.sel_ui = None;
                }
                match hit {
                    Hit::Bar(a) => self.bar_action(a),
                    Hit::Handle(_) => None,
                    Hit::Key(k) if self.rename.is_some() => self.type_into_field(k),
                    Hit::Key(k) if self.scratch_focused() => self.scratch_key(k),
                    Hit::Drawer(p) => Some(self.toggle_pane(p)),
                    Hit::Key(k) => {
                        let tab = self.settings.tab_kind();
                        let a = self.keyboard.press(k, &mut self.buffer, tab);
                        self.after_key(a)
                    }
                    Hit::ToggleKeys => {
                        self.keys_on = !self.keys_on;
                        Some(Refresh::Partial)
                    }
                    Hit::Caret(at) => {
                        self.scratch_focus = false;
                        self.buffer.set_caret(at);
                        Some(Refresh::Fast)
                    }
                    Hit::Files => {
                        self.open_files();
                        Some(Refresh::Full)
                    }
                    Hit::TogglePreview => {
                        self.enter_preview();
                        Some(Refresh::Full)
                    }
                    Hit::Options => {
                        self.open_options();
                        Some(Refresh::Full)
                    }
                    Hit::KbButton(b) => self.kb_button(b),
                    Hit::StartRename => {
                        self.start_rename(RenameTarget::Current);
                        Some(Refresh::Partial)
                    }
                    Hit::RenameOk => {
                        self.finish_rename(true);
                        Some(Refresh::Partial)
                    }
                    Hit::RenameCancel => {
                        self.finish_rename(false);
                        Some(Refresh::Partial)
                    }
                    Hit::Nothing => None,
                }
            }
        }
    }

    /// Main loop until power/quit or end of input.
    ///
    /// # Errors
    /// The device failed to present a frame.
    pub fn run(&mut self, dev: &mut dyn Device) -> std::io::Result<()> {
        self.render(dev, Refresh::Full)?;
        while self.running {
            let Some(e) = dev.next_event() else { break };
            let e = self.to_logical(e);
            if let Some(r) = self.handle(e) {
                self.render(dev, r)?;
            }
        }
        self.save();
        Ok(())
    }

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
        if self.options {
            return self.screen.options_control(&self.options_model(), label);
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
            "Files" => Some(centre(l.files_button)),
            "Preview" | "Pencil" | "Eye" => Some(centre(l.preview_button)),
            "Title" => Some(centre(l.name_rect)),
            "OK" => Some(centre(l.ok_rect)),
            "Cancel" => Some(centre(l.cancel_rect)),
            "Keys" => Some(centre(l.keys_button)),
            "Options" => Some(centre(l.options_button)),
            "Contents" => Some(centre(l.contents_button)),
            "To-dos" => Some(centre(l.todos_button)),
            "Scratch" => Some(centre(l.scratch_button)),
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

    #[must_use]
    pub fn page_rect(&self) -> Rect {
        self.screen.layout(self.keys_visible(), &self.keyboard).page
    }
}

const SETTINGS: &str = ".sigma-settings";

fn load_settings(root: &std::path::Path) -> Settings {
    std::fs::read_to_string(root.join(SETTINGS))
        .map(|s| Settings::parse(&s))
        .unwrap_or_default()
}

/// Screen size for an orientation on a panel of `phys` size.
fn logical_size(phys: (u32, u32), o: Orientation) -> (u32, u32) {
    let (short, long) = (phys.0.min(phys.1), phys.0.max(phys.1));
    match o {
        Orientation::Portrait => (short, long),
        Orientation::Landscape => (long, short),
    }
}

/// Words a Line-mode row holds at the current settings (tests, emulator).
#[must_use]
pub fn words_per_row(app: &App) -> usize {
    app.screen.sample_words_per_row(app.page_rect())
}

fn centre(r: Rect) -> (i32, i32) {
    (r.x + r.w / 2, r.y + r.h / 2)
}

impl App {
    /// Drawer controls for emulator scripts: `Heading:TEXT`, `Todo:TEXT`
    /// (opens), `TodoBox:TEXT` (checks), `Filter`, `ScratchPane` (end of
    /// the scratch text).
    fn drawer_control(&self, name: &str) -> Option<(i32, i32)> {
        if !self.drawer_visible() {
            return None;
        }
        let (hs, todos) = (self.headings(), self.todo_list());
        let base = self.screen.layout(self.keys_visible(), &self.keyboard);
        let l = self
            .screen
            .drawer_layout(&base, &self.drawer_model(&hs, &todos));
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
}
