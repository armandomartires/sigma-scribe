//! Sigma Scribe v0.2: a library of plain `.md` / `.txt` notes in folders,
//! the Files sheet, rename in place, and the v0.1 editor with on-screen
//! keyboard and autosave. Generic over the device so the Forma, the desktop
//! window and the headless emulator run the same code.

use sigma_core::buffer::TabKind;
use sigma_core::gesture::{Gesture, Recognizer};
use sigma_core::graphmap::{self, MapLayout};
use sigma_core::keyboard::{Dir, Key};
use sigma_core::library::SCRATCH;
use sigma_core::linefit::Anchor;
use sigma_core::links::{self, Graph};
use sigma_core::outline::{self, Filter, Heading, Indexer, TodoItem};
use sigma_core::settings::{Orientation, Pad, Side};
use sigma_core::{
    Autosave, Buffer, Change, Entry, FsNoteStore, KeyAction, Keyboard, Kind, Library, NotePath,
    NoteStore, Settings,
};
use sigma_device::{Button, Device, Event, HwKey, Mods, Phase, Refresh};
use sigma_render::editor::Model;
use sigma_render::{
    BarAction, Canvas, Doc, DrawerHit, DrawerModel, EditorScreen, FilesHit, FilesModel, FindField,
    FindHit, FindModel, GraphHit, GraphModel, GraphRow, HelpHit, HelpModel, HelpPage, Hit,
    KbButton, MapView, OptionsHit, OptionsModel, Pane, PreviewModel, Rect, Row, SelUi, Theme,
    ZOOMS,
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

/// Pointer redraws at most this often (ms): e-ink friendly (v0.8.1).
pub const POINTER_THROTTLE_MS: u64 = 150;
/// The pointer hides after this long without movement (ms).
pub const POINTER_HIDE_MS: u64 = 3000;
/// Two presses within this time and distance are a double-click.
pub const DOUBLE_CLICK_MS: u64 = 400;
pub const DOUBLE_CLICK_PX: i32 = 8;

/// A left button held down (v0.8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseDown {
    pub x: i32,
    pub y: i32,
    /// Text offset under the press, when it was on the note.
    pub anchor: Option<usize>,
    pub dragged: bool,
}

/// USB mouse / touchpad state (v0.8.1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PointerState {
    pub live: bool,
    pub x: i32,
    pub y: i32,
    pub visible: bool,
    pub last_move: u64,
    pub last_draw: u64,
    /// Moved since the last redraw (throttled); flushed on the tick.
    pub pending: bool,
    pub down: Option<MouseDown>,
    pub last_press: Option<(i32, i32, u64)>,
    /// Screen area the arrow has covered since the last cleanup (v1.0.1).
    pub trail: Option<Rect>,
    /// A cleanup refresh over `trail` is due (the arrow just hid).
    pub cleanup: bool,
    /// Where the arrow is on the panel now (v1.0.4 fast path).
    pub drawn: Option<Rect>,
    /// Latest tick time: idle is measured in processing time, so a
    /// backlog of old move events cannot hide the arrow (v1.0.4).
    pub last_tick: u64,
}

fn merge(a: Option<Refresh>, b: Option<Refresh>) -> Option<Refresh> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if rank(x) >= rank(y) { x } else { y }),
        (x, None) => x,
        (None, y) => y,
    }
}

/// Grows a region to at least 16×16 (refresh rule), inside `b`.
fn min16(r: Rect, b: Rect) -> Rect {
    let w = r.w.max(16).min(b.w);
    let h = r.h.max(16).min(b.h);
    let x = r.x.min(b.x + b.w - w).max(b.x);
    let y = r.y.min(b.y + b.h - h).max(b.y);
    Rect::new(x, y, w, h)
}

/// Copies rectangle `r` of `src` into `dst` (same size canvases).
fn copy_rect(dst: &mut Canvas, src: &Canvas, r: Rect) {
    let r = r.intersect(&dst.bounds());
    if r.w <= 0 || r.h <= 0 || dst.width != src.width || dst.height != src.height {
        return;
    }
    let w = dst.width as usize;
    for y in r.y..r.y + r.h {
        let a = y as usize * w + r.x as usize;
        let e = a + r.w as usize;
        dst.pixels[a..e].copy_from_slice(&src.pixels[a..e]);
    }
}

fn rank(r: Refresh) -> u8 {
    match r {
        Refresh::Fast => 0,
        Refresh::Pointer | Refresh::PointerCleanup => 1,
        Refresh::Partial => 2,
        Refresh::Full => 3,
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

/// The Find bar (v0.6). The query is not persisted (brief §8).
#[derive(Clone, Debug, Default)]
pub struct FindState {
    pub query: Buffer,
    pub replace: Buffer,
    /// Field the keyboard types into; `None` when the page has the caret.
    pub focus: Option<FindField>,
    /// Aa: match case (off by default).
    pub case: bool,
}

/// The Graph sheet (v0.6): rows built when it opens.
#[derive(Clone, Debug, Default)]
pub struct GraphState {
    pub notes: Vec<NotePath>,
    pub rows: Vec<GraphRow>,
    pub edges: usize,
    pub first: usize,
    /// v1.0 map view (default) or the list.
    pub map: bool,
    /// Links between notes (from, to) and their layout.
    pub links: Vec<(usize, usize)>,
    pub layout: std::sync::Arc<MapLayout>,
    pub zoom: usize,
    pub pan: (f64, f64),
    pub selected: Option<usize>,
    /// Layout time in microseconds (0 when it came from the cache).
    pub layout_us: u64,
}

/// Seed of the map layout: fixed, so the same notes give the same picture.
pub const MAP_SEED: u64 = 0x5349_474d_414d_4150;

/// The Help sheet (v0.7): the page shown and its laid-out text.
#[derive(Clone, Debug)]
pub struct HelpState {
    pub page: HelpPage,
    pub doc: Option<Doc>,
    pub at: usize,
}

/// Keystroke-to-present timings (v0.7 perf report): microseconds from an
/// input event to the end of its refresh call, for key presses.
#[derive(Clone, Debug, Default)]
pub struct Perf {
    pub samples: Vec<u32>,
    /// Pointer moves: event to the end of the (never waited) submit, µs
    /// (v1.0.3).
    pub pointer: Vec<u32>,
}

impl Perf {
    /// (count, p50, p95, max) in milliseconds.
    #[must_use]
    pub fn summary(&self) -> Option<(usize, f64, f64, f64)> {
        Self::stats(&self.samples)
    }

    /// (count, p50, p95, max) of the pointer samples, in milliseconds.
    #[must_use]
    pub fn pointer_summary(&self) -> Option<(usize, f64, f64, f64)> {
        Self::stats(&self.pointer)
    }

    fn stats(samples: &[u32]) -> Option<(usize, f64, f64, f64)> {
        if samples.is_empty() {
            return None;
        }
        let mut v = samples.to_vec();
        v.sort_unstable();
        let at = |q: f64| f64::from(v[((v.len() - 1) as f64 * q).round() as usize]) / 1000.0;
        Some((v.len(), at(0.5), at(0.95), at(1.0)))
    }
}

/// Preview mode (v0.3): the laid-out note and the page shown. Read-only.
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
    /// The last full frame without the pointer (v1.0.4 fast path).
    base: Canvas,
    /// The next `Refresh::Pointer` render only moves the arrow: nothing
    /// else changed since the last full frame (v1.0.4).
    pointer_fast: bool,
    gestures: Recognizer,
    /// Touch selection chrome (v0.2.2).
    pub sel_ui: Option<SelUi>,
    drag: Option<Drag>,
    /// Map pan by finger: press point and whether it moved past the slop.
    map_drag: Option<(i32, i32, bool)>,
    /// Map layout cache: key over names + links (v1.0).
    map_cache: Option<(u64, std::sync::Arc<MapLayout>)>,
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
    /// Find bar (v0.6).
    pub find: Option<FindState>,
    /// Graph sheet (v0.6).
    pub graph: Option<GraphState>,
    /// Help sheet (v0.7).
    pub help: Option<HelpState>,
    /// The open note could not be read: autosave is held back so a blank
    /// buffer never overwrites it (v0.7 hardening).
    pub read_only: bool,
    /// A USB keyboard is connected (v0.8, from the device's hot-plug watcher).
    pub usb_live: bool,
    /// The USB host-mode state last asked of the device (v0.8).
    pub usb_host: bool,
    /// USB mouse / touchpad (v0.8.1).
    pub pointer: PointerState,
    /// Back stack (v1.0.2), this session only, newest last.
    pub history: Vec<HistEntry>,
    pub perf: Perf,
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
        let (body, unreadable) = load_note(&store, &current);
        let buffer = Buffer::new(body);
        let _ = library.set_last_opened(&current);
        let mut settings = load_settings(&root);
        let (lw, lh) = logical_size((width, height), settings.orientation);
        let mut screen = EditorScreen::new(lw, lh);
        screen.configure(&mut settings, None);
        let scratch = Buffer::new(store.load(SCRATCH).ok().flatten().unwrap_or_default());
        Self {
            autosave: if unreadable {
                let mut a = Autosave::new(buffer.revision());
                a.failed("could not read the note");
                a
            } else {
                Autosave::new(buffer.revision())
            },
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
            map_drag: None,
            map_cache: None,
            preview: None,
            canvas: Canvas::new(lw, lh, 255),
            shown: Canvas::new(lw, lh, 0),
            base: Canvas::new(1, 1, 0),
            pointer_fast: false,
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
            find: None,
            graph: None,
            help: None,
            read_only: unreadable,
            usb_live: false,
            usb_host: false,
            pointer: PointerState::default(),
            history: Vec::new(),
            perf: Perf::default(),
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
            && self.help.is_none()
    }

    /// The layout the drawer sits on: preview has no on-screen keyboard.
    fn drawer_base(&self) -> sigma_render::editor::Layout {
        if self.preview.is_some() {
            self.screen.layout(false, &self.keyboard)
        } else {
            self.screen.layout(self.keys_visible(), &self.keyboard)
        }
    }

    /// Headings for Contents: in preview, the rendered headings only.
    fn drawer_headings(&self) -> Vec<Heading> {
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
    fn sync_preview_drawer(&mut self, preview: bool) {
        self.screen.preview_drawer =
            (preview && self.drawer.iter().any(|&o| o) && self.drawer_allowed()).then(|| {
                let base = self.screen.layout(false, &self.keyboard);
                self.screen.drawer_rect(&base, self.settings.drawer)
            });
    }

    /// Lays the preview out again (drawer opened/closed, to-do toggled),
    /// keeping the shown source line on screen.
    fn rebuild_preview(&mut self) {
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
        let (hs, todos) = (self.drawer_headings(), self.todo_list());
        let base = self.drawer_base();
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
            DrawerHit::Heading(line) if self.preview.is_some() => {
                self.push_history();
                self.scratch_focus = false;
                let p = self.preview.as_mut()?;
                p.page = p.doc.page_of_line(line);
                Some(Refresh::Partial)
            }
            DrawerHit::TodoBox(i) if self.preview.is_some() => {
                let it = self.todo_list().get(i)?.clone();
                let r = self.toggle_todo(&it);
                self.rebuild_preview();
                r
            }
            DrawerHit::Heading(line) => {
                self.push_history();
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
                    if let Some(f) = &mut self.find {
                        f.focus = None;
                    }
                    let at =
                        outline::line_range(self.scratch.text(), it.todo.line).map_or(0, |r| r.1);
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
            KeyAction::Page { .. } | KeyAction::Ignored | KeyAction::Find => None,
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
            HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown => return None,
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
        self.help = None;
        self.close_find();
        self.graph = None;
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

    /// Redraw for the pointer, at most every `POINTER_THROTTLE_MS`.
    fn pointer_redraw(&mut self, t: u64) -> Option<Refresh> {
        if t.saturating_sub(self.pointer.last_draw) >= POINTER_THROTTLE_MS {
            self.pointer.last_draw = t;
            self.pointer.pending = false;
            Some(Refresh::Pointer)
        } else {
            self.pointer.pending = true;
            None
        }
    }

    /// Ticks: flush a throttled move, hide after `POINTER_HIDE_MS` idle.
    fn pointer_tick(&mut self, t: u64) -> Option<Refresh> {
        let p = &mut self.pointer;
        p.last_tick = p.last_tick.max(t);
        if !p.visible {
            return None;
        }
        if p.down.is_none() && t.saturating_sub(p.last_move) >= POINTER_HIDE_MS {
            p.visible = false;
            p.pending = false;
            p.cleanup = true;
            return Some(Refresh::Pointer);
        }
        if p.pending {
            return self.pointer_redraw(t);
        }
        None
    }

    fn pointer_move(&mut self, dx: i32, dy: i32, t: u64) -> Option<Refresh> {
        // A report racing an unplug is dropped.
        if !self.pointer.live {
            return None;
        }
        let (w, h) = (self.canvas.width as i32, self.canvas.height as i32);
        let p = &mut self.pointer;
        p.x = (p.x + dx).clamp(0, w - 1);
        p.y = (p.y + dy).clamp(0, h - 1);
        p.last_move = t.max(p.last_tick);
        let was = std::mem::replace(&mut p.visible, true);
        let (x, y) = (p.x, p.y);
        if let Some(d) = &mut self.pointer.down {
            if (x - d.x).abs() + (y - d.y).abs() > DOUBLE_CLICK_PX / 2 {
                d.dragged = true;
            }
            if let (Some(a), true) = (d.anchor, d.dragged) {
                let at = self
                    .screen
                    .offset_at_point(&self.buffer, self.page_rect(), x, y);
                if at == a {
                    self.buffer.set_caret(a);
                } else {
                    self.buffer.select(a, at);
                }
                self.sel_ui = Some(SelUi { bar: false });
            }
        }
        if was {
            self.pointer_redraw(t)
        } else {
            self.pointer.last_draw = t;
            Some(Refresh::Pointer)
        }
    }

    /// Left click = a tap at the pointer; drag on the note selects;
    /// double-click on the note selects the word. Other buttons: nothing.
    fn pointer_button(&mut self, button: Button, down: bool, t: u64) -> Option<Refresh> {
        if button != Button::Left || !self.pointer.live {
            return None;
        }
        let (x, y) = (self.pointer.x, self.pointer.y);
        self.pointer.visible = true;
        self.pointer.last_move = t;
        let on_note = self.in_editor()
            && self.rename.is_none()
            && self.page_rect().contains(x, y)
            && !self.in_drawer(x, y)
            && !self.in_find_bar(x, y);
        if down {
            let double = self.pointer.last_press.is_some_and(|(px, py, pt)| {
                t.saturating_sub(pt) <= DOUBLE_CLICK_MS
                    && (x - px).abs() <= DOUBLE_CLICK_PX
                    && (y - py).abs() <= DOUBLE_CLICK_PX
            });
            if double && on_note {
                self.pointer.last_press = None;
                self.pointer.down = None;
                self.select_word_at(x, y);
                self.sel_ui = Some(SelUi { bar: true });
                return Some(Refresh::Fast);
            }
            self.pointer.last_press = Some((x, y, t));
            let anchor = on_note.then(|| {
                self.screen
                    .offset_at_point(&self.buffer, self.page_rect(), x, y)
            });
            self.pointer.down = Some(MouseDown {
                x,
                y,
                anchor,
                dragged: false,
            });
            return None;
        }
        let d = self.pointer.down.take()?;
        if d.dragged && self.on_map(d.x, d.y) {
            let r = self.graph_pan(x - d.x, y - d.y);
            return merge(Some(Refresh::Fast), r);
        }
        if d.dragged && d.anchor.is_some() {
            self.sel_ui = self.buffer.selection().map(|_| SelUi { bar: true });
            self.pointer.last_draw = t;
            self.pointer.pending = false;
            return Some(Refresh::Fast);
        }
        let r = self.handle_inner(Event::Tap { x: d.x, y: d.y });
        merge(Some(Refresh::Fast), r)
    }

    /// Wheel: the note moves 3 rows per notch (the caret follows); preview
    /// and the scrolling sheets turn one page per event.
    fn wheel(&mut self, notches: i32) -> Option<Refresh> {
        if notches == 0 || self.options {
            return None;
        }
        // Reverse scroll (v1.0.1): Natural flips every notch, everywhere.
        let notches = if self.settings.natural_scroll {
            -notches
        } else {
            notches
        };
        let forward = notches < 0;
        if self.graph.as_ref().is_some_and(|g| g.map) {
            return self.graph_zoom(notches > 0);
        }
        if self.preview.is_some()
            || self.files.is_some()
            || self.graph.is_some()
            || self.help.is_some()
        {
            return self.handle_inner(Event::PageTurn { forward });
        }
        if !self.in_editor() {
            return None;
        }
        let page = self.page_rect();
        for _ in 0..(notches.unsigned_abs() * 3).min(15) {
            self.screen.caret_row(&mut self.buffer, page, forward);
        }
        self.sel_ui = None;
        Some(Refresh::Partial)
    }

    /// A physical key with modifiers (USB keyboard, v0.8). Routed through
    /// the on-screen keyboard's own key and shortcut code (S2.5) with the
    /// chord's modifiers, so Ctrl+Z/Y/A/C/X/V/F and Shift/Ctrl+arrows behave
    /// exactly like their on-screen versions. The on-screen modifier state is
    /// restored afterwards. Sheets ignore chords; Ctrl+F in preview opens Find.
    fn chord(&mut self, key: HwKey, mods: Mods) -> Option<Refresh> {
        let ctrl_f = mods.ctrl && key == HwKey::Char('f');
        // Alt+Left = Back (v1.0.2), in write mode and preview.
        if mods.alt && !mods.ctrl && key == HwKey::Left && !self.history.is_empty() {
            if self.options || self.help.is_some() || self.graph.is_some() || self.files.is_some() {
                return None;
            }
            return self.go_back();
        }
        if self.options || self.help.is_some() || self.graph.is_some() || self.files.is_some() {
            return None;
        }
        if self.preview.is_some() {
            return if ctrl_f { Some(self.open_find()) } else { None };
        }
        let k = match key {
            HwKey::Char(c) => Key::Char(c.to_ascii_lowercase()),
            HwKey::Backspace => Key::Back,
            HwKey::Delete => Key::Del,
            HwKey::Return => Key::Return,
            HwKey::Tab => Key::Tab,
            HwKey::Left => Key::Arrow(Dir::Left),
            HwKey::Right => Key::Arrow(Dir::Right),
            HwKey::Up => Key::Arrow(Dir::Up),
            HwKey::Down => Key::Arrow(Dir::Down),
            HwKey::Escape => Key::Esc,
            HwKey::Home => Key::Home,
            HwKey::End => Key::End,
            HwKey::PageUp => Key::PgUp,
            HwKey::PageDown => Key::PgDn,
        };
        let kb = &mut self.keyboard;
        let saved = (kb.shift, kb.ctrl, kb.alt);
        (kb.shift, kb.ctrl, kb.alt) = (mods.shift, mods.ctrl, mods.alt);
        let r = if self.rename.is_some() {
            self.type_into_field(k)
        } else if self.find_typing() && !ctrl_f {
            self.find_key(k)
        } else if self.scratch_focused() {
            self.scratch_key(k)
        } else {
            let tab = self.settings.tab_kind();
            let a = self.keyboard.press(k, &mut self.buffer, tab);
            self.after_key(a)
        };
        let kb = &mut self.keyboard;
        (kb.shift, kb.ctrl, kb.alt) = saved;
        r
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
            KeyAction::Find => Some(self.open_find()),
            KeyAction::Escape if self.find.is_some() => {
                self.close_find();
                Some(Refresh::Full)
            }
            KeyAction::Escape | KeyAction::Ignored => None,
        }
    }

    fn save(&mut self) {
        if self.read_only {
            return;
        }
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
        let (body, unreadable) = load_note(&self.store, p);
        self.current = p.clone();
        self.buffer = Buffer::new(body);
        self.autosave = Autosave::new(self.buffer.revision());
        self.read_only = unreadable;
        if unreadable {
            self.autosave.failed("could not read the note");
        }
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
            || (self.files.is_none()
                && self.preview.is_none()
                && !self.options
                && self.graph.is_none()
                && self.help.is_none()
                && self.keys_on)
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

    fn preview_doc(&self) -> sigma_render::preview::Doc {
        let names = self.note_names();
        let known = |n: &str| names.contains(&n.trim().to_lowercase());
        let back: Vec<(String, String)> = self
            .link_graph()
            .backlinks(&self.current)
            .iter()
            .map(|p| (stem(p.as_str()).to_owned(), p.as_str().to_owned()))
            .collect();
        self.screen
            .build_preview_with(self.buffer.text(), &self.keyboard, &known, Some(&back))
    }

    /// Back to write mode; caret, selection and scroll are as they were.
    pub fn exit_preview(&mut self) {
        self.preview = None;
    }

    /// Preview input: pencil, Files, page turns, Escape, and (v1.0.3) the
    /// drawer: Contents jumps, to-do toggles, scratch typing. The note
    /// itself stays read-only (decision S3-D1; to-dos are the S10.3-D1
    /// exception).
    fn preview_event(&mut self, e: Event) -> Option<Refresh> {
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
                Hit::Help => {
                    self.exit_preview();
                    self.open_help();
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
        self.help = None;
        self.close_find();
        self.graph = None;
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
        if refresh == Refresh::Pointer
            && std::mem::take(&mut self.pointer_fast)
            && self.base.pixels.len() == self.canvas.pixels.len()
        {
            return self.render_pointer_only(dev);
        }
        self.pointer_fast = false;
        self.screen.back_on = !self.history.is_empty();
        let mut c = std::mem::replace(&mut self.canvas, Canvas::new(1, 1, 0));
        if self.options {
            self.screen.draw_options(&mut c, &self.options_model());
        } else if let Some(h) = &self.help {
            self.screen.draw_help(&mut c, &self.help_model(h));
        } else if let Some(g) = &self.graph {
            self.screen.draw_graph(&mut c, &self.graph_model(g));
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
            self.screen.drawer_open = if self.drawer_visible() {
                self.drawer
            } else {
                [false; 3]
            };
            let l = self.screen.draw_preview(&mut c, &m);
            if self.drawer_visible() {
                let (hs, todos) = (self.drawer_headings(), self.todo_list());
                self.screen
                    .draw_drawer(&mut c, &l, &self.drawer_model(&hs, &todos));
            }
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
            if let Some(f) = &self.find {
                let label = self.find_label();
                self.screen.draw_find(
                    &mut c,
                    &l,
                    &FindModel {
                        query: &f.query,
                        replace: &f.replace,
                        focus: f.focus,
                        case: f.case,
                        label: &label,
                        theme: self.theme(),
                    },
                );
            }
            if self.drawer_visible() {
                let (hs, todos) = (self.headings(), self.todo_list());
                self.screen
                    .draw_drawer(&mut c, &l, &self.drawer_model(&hs, &todos));
            }
        }
        if self.pointer.live {
            self.base.clone_from(&c);
        }
        self.pointer.drawn = None;
        if self.pointer.visible && self.pointer.live {
            let cell = sigma_render::pointer::pointer_cell(self.screen.scale().px(1.0) as f32);
            sigma_render::pointer::draw_pointer(
                &mut c,
                self.pointer.x,
                self.pointer.y,
                cell,
                self.theme(),
            );
            let pr = sigma_render::pointer::pointer_rect(self.pointer.x, self.pointer.y, cell)
                .intersect(&c.bounds());
            self.pointer.trail = Some(self.pointer.trail.map_or(pr, |t| t.union(&pr)));
            self.pointer.drawn = Some(pr);
        }
        self.canvas = c;
        // Only what changed goes to the panel: a pressed modifier redraws its
        // cap, a typed letter its text row and the word count.
        let mut regions = if refresh == Refresh::Full {
            vec![self.canvas.bounds()]
        } else {
            self.canvas.diff_regions(&self.shown, 24, 8)
        };
        // v1.0.1: the arrow's erase and draw go out as ONE update, so the
        // panel never shows the old arrow's residue next to the new one.
        if matches!(refresh, Refresh::Pointer | Refresh::PointerCleanup) && regions.len() > 1 {
            let u = regions.iter().skip(1).fold(regions[0], |a, r| a.union(r));
            regions = vec![u];
        }
        let regions: Vec<Rect> = regions
            .into_iter()
            .map(|r| min16(r, self.canvas.bounds()))
            .collect();
        for r in &regions {
            self.present(dev, *r, refresh)?;
        }
        // When the arrow hides: one non-flashing GL16 pass over its path
        // since the last cleanup, clearing the DU trails (v1.0.3, S10.3-D2).
        if std::mem::take(&mut self.pointer.cleanup) {
            if let Some(t) = self.pointer.trail.take() {
                if refresh != Refresh::Full {
                    self.present(dev, min16(t, self.canvas.bounds()), Refresh::PointerCleanup)?;
                }
            }
        }
        if !regions.is_empty() {
            self.shown.pixels.copy_from_slice(&self.canvas.pixels);
        }
        Ok(())
    }

    /// v1.0.4: a pointer-only frame. Restores the old arrow's pixels from
    /// the last full frame and draws the new one; no full redraw (the
    /// 1.0.3 device log showed ~170 ms of CPU per move from full redraws,
    /// which backed up the event queue).
    fn render_pointer_only(&mut self, dev: &mut dyn Device) -> std::io::Result<()> {
        let b = self.canvas.bounds();
        let mut region = self.pointer.drawn.take();
        if let Some(o) = region {
            copy_rect(&mut self.canvas, &self.base, o);
        }
        if self.pointer.visible && self.pointer.live {
            let cell = sigma_render::pointer::pointer_cell(self.screen.scale().px(1.0) as f32);
            let theme = self.theme();
            sigma_render::pointer::draw_pointer(
                &mut self.canvas,
                self.pointer.x,
                self.pointer.y,
                cell,
                theme,
            );
            let pr = sigma_render::pointer::pointer_rect(self.pointer.x, self.pointer.y, cell)
                .intersect(&b);
            self.pointer.trail = Some(self.pointer.trail.map_or(pr, |t| t.union(&pr)));
            self.pointer.drawn = Some(pr);
            region = Some(region.map_or(pr, |r| r.union(&pr)));
        }
        if let Some(r) = region {
            let r = min16(r, b);
            self.present(dev, r, Refresh::Pointer)?;
            copy_rect(&mut self.shown, &self.canvas, r);
        }
        if std::mem::take(&mut self.pointer.cleanup) {
            if let Some(t) = self.pointer.trail.take() {
                let t = min16(t, b);
                self.present(dev, t, Refresh::PointerCleanup)?;
            }
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
            HwKey::Tab
            | HwKey::Up
            | HwKey::Down
            | HwKey::Home
            | HwKey::End
            | HwKey::PageUp
            | HwKey::PageDown => {}
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
                    if e.path != self.current {
                        self.push_history();
                    }
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
        self.files.is_none()
            && self.rename.is_none()
            && self.preview.is_none()
            && !self.options
            && self.graph.is_none()
            && self.help.is_none()
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
            Gesture::Press { x, y } if self.on_map(x, y) => {
                self.drag = None;
                self.map_drag = Some((x, y, false));
                None
            }
            Gesture::Drag { x, y, .. } if self.map_drag.is_some() => {
                let (x0, y0, _) = self.map_drag?;
                self.map_drag = Some((x, y, true));
                self.graph_pan(x - x0, y - y0)
            }
            Gesture::Release { x, y, .. } if self.map_drag.is_some_and(|d| d.2) => {
                let (x0, y0, _) = self.map_drag.take()?;
                self.graph_pan(x - x0, y - y0).or(Some(Refresh::Partial))
            }
            Gesture::Press { x, y } => {
                self.drag = None;
                self.map_drag = None;
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
        match e {
            Event::Tick { t } => {
                let p = self.pointer_tick(t);
                let r = self.handle_inner(e);
                if r.is_some() {
                    self.pointer_fast = false;
                } else if p == Some(Refresh::Pointer) && self.pointer.down.is_none() {
                    self.pointer_fast = true;
                }
                return merge(p, r);
            }
            Event::PointerMove { .. } => {
                let r = self.handle_inner(e);
                if r == Some(Refresh::Pointer) && self.pointer.down.is_none() {
                    self.pointer_fast = true;
                } else if r.is_some() {
                    self.pointer_fast = false;
                }
                return r;
            }
            // Typing or touching hides the pointer (v0.8.1).
            _ => self.pointer_fast = false,
        }
        match e {
            Event::Key(_) | Event::Chord { .. } | Event::Touch { .. } | Event::Tap { .. }
                if self.pointer.visible =>
            {
                self.pointer.visible = false;
                self.pointer.pending = false;
                self.pointer.cleanup = true;
                let r = self.handle_inner(e);
                return merge(Some(Refresh::Fast), r);
            }
            _ => {}
        }
        self.handle_inner(e)
    }

    fn handle_inner(&mut self, e: Event) -> Option<Refresh> {
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
            Event::Usb(on) => {
                self.usb_live = on;
                self.screen.usb_live = on || self.pointer.live;
                Some(Refresh::Partial)
            }
            Event::PointerPlug(on) => {
                self.pointer.live = on;
                if !on {
                    self.pointer.visible = false;
                    self.pointer.down = None;
                    self.pointer.pending = false;
                } else if self.pointer.x == 0 && self.pointer.y == 0 {
                    self.pointer.x = self.canvas.width as i32 / 2;
                    self.pointer.y = self.canvas.height as i32 / 2;
                }
                self.screen.usb_live = self.usb_live || on;
                Some(Refresh::Partial)
            }
            Event::PointerMove { dx, dy, t } => self.pointer_move(dx, dy, t),
            Event::PointerButton { button, down, t } => self.pointer_button(button, down, t),
            Event::Wheel { notches } => self.wheel(notches),
            Event::Chord { key, mods } => self.chord(key, mods),
            // Nav keys only a USB keyboard has: the on-screen key path.
            Event::Key(k @ (HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown)) => {
                self.chord(k, Mods::default())
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
            Event::Tap { x, y } if self.help.is_some() => self.help_tap(x, y),
            Event::PageTurn { forward } if self.help.is_some() => self.help_page(forward),
            Event::Key(HwKey::Escape) if self.help.is_some() => self.help_back(),
            Event::Key(_) if self.help.is_some() => None,
            Event::Tap { x, y } if self.graph.is_some() => self.graph_tap(x, y),
            Event::PageTurn { forward } if self.graph.is_some() => self.graph_page(forward),
            Event::Key(HwKey::Escape) if self.graph.is_some() => {
                self.graph = None;
                Some(Refresh::Full)
            }
            Event::Key(_) if self.graph.is_some() => None,
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
            Event::Key(k) if self.find_typing() => self.find_hw(k),
            Event::Key(k) if self.scratch_focused() => self.scratch_hw(k),
            Event::Tap { x, y } if self.in_find_bar(x, y) => self.find_tap(x, y),
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
                    HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown => return None,
                    HwKey::Escape => {
                        let had = self.sel_ui.take().is_some();
                        if !had && !self.history.is_empty() {
                            return self.go_back();
                        }
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
                    Hit::Back => self.go_back(),
                    Hit::Bar(a) => self.bar_action(a),
                    Hit::Handle(_) => None,
                    Hit::Key(k) if self.rename.is_some() => self.type_into_field(k),
                    Hit::Key(k) if self.find_typing() => self.find_key(k),
                    Hit::Key(k) if self.scratch_focused() => self.scratch_key(k),
                    Hit::Find => Some(if self.find.is_some() {
                        self.close_find();
                        Refresh::Full
                    } else {
                        self.open_find()
                    }),
                    Hit::Graph => {
                        self.open_graph();
                        Some(Refresh::Full)
                    }
                    Hit::Help => {
                        self.open_help();
                        Some(Refresh::Full)
                    }
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
                        if let Some(f) = &mut self.find {
                            f.focus = None;
                        }
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

    /// Puts the USB port in host mode while Options → USB keyboard is On,
    /// and back when it is turned off (v0.8). A failure is logged and
    /// retried only when the setting changes again.
    pub fn sync_usb_host(&mut self, dev: &mut dyn Device) {
        if self.settings.usb != self.usb_host {
            self.usb_host = self.settings.usb;
            if let Err(e) = dev.usb_host(self.usb_host) {
                eprintln!("usb: {e}");
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
            self.sync_usb_host(dev);
            let Some(e) = dev.next_event() else { break };
            let t0 = std::time::Instant::now();
            let e = self.to_logical(e);
            let typing = self.is_key_tap(&e);
            if let Some(r) = self.handle(e) {
                self.render(dev, r)?;
                let us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
                if typing {
                    self.perf.samples.push(us);
                } else if r == Refresh::Pointer {
                    self.perf.pointer.push(us);
                }
            }
        }
        self.save();
        if self.usb_host {
            let _ = dev.usb_host(false);
            self.usb_host = false;
        }
        if let Some((n, p50, p95, max)) = self.perf.summary() {
            eprintln!(
                "perf: {n} keystrokes, keystroke-to-refresh p50 {p50:.1} ms, p95 {p95:.1} ms, max {max:.1} ms"
            );
        }
        if let Some((n, p50, p95, max)) = self.perf.pointer_summary() {
            eprintln!(
                "perf pointer: {n} du updates, move-to-submit p50 {p50:.1} ms, p95 {p95:.1} ms, max {max:.1} ms"
            );
        }
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
        if let Some(h) = &self.help {
            return self.screen.help_control(&self.help_model(h), label);
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
            "Options" => Some(centre(l.options_button)),
            "Contents" => Some(centre(l.contents_button)),
            "To-dos" => Some(centre(l.todos_button)),
            "Scratch" => Some(centre(l.scratch_button)),
            "Find" => Some(centre(l.find_button)),
            "Graph" => Some(centre(l.graph_button)),
            "Help" => Some(centre(l.help_button)),
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
        let (hs, todos) = (self.drawer_headings(), self.todo_list());
        let base = self.drawer_base();
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

/// A note path without its extension (link labels).
fn stem(s: &str) -> &str {
    s.rsplit_once('.').map_or(s, |(a, _)| a)
}

/// Find bar and wiki links (v0.6).
impl App {
    /// The link graph of the library, with the open note's live text.
    #[must_use]
    pub fn link_graph(&self) -> Graph {
        Graph::build(&self.library, Some((&self.current, self.buffer.text())))
    }

    fn find_typing(&self) -> bool {
        self.in_editor() && self.find.as_ref().is_some_and(|f| f.focus.is_some())
    }

    fn in_find_bar(&self, x: i32, y: i32) -> bool {
        self.in_editor() && self.find.is_some() && self.page_layout().find_bar.contains(x, y)
    }

    fn page_layout(&self) -> sigma_render::editor::Layout {
        self.screen.layout(self.keys_visible(), &self.keyboard)
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
        // Show the seeded / previous query's match at the caret.
        let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
        self.select_match(sigma_core::find::at_or_after(&self.find_matches(), from));
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

    /// `Find`, `None`, or `i of n`.
    #[must_use]
    pub fn find_label(&self) -> String {
        let ms = self.find_matches();
        let q = self.find.as_ref().map_or("", |f| f.query.text());
        let cur = sigma_core::find::selected(&ms, self.buffer.selection());
        sigma_core::find::count_label(q, &ms, cur)
    }

    fn select_match(&mut self, i: Option<usize>) {
        if let Some(&(a, b)) = i.and_then(|i| self.find_matches().get(i).copied()).as_ref() {
            self.buffer.select(a, b);
            self.screen.follow_caret(&self.buffer, self.page_rect());
        }
    }

    fn find_step(&mut self, forward: bool) -> Option<Refresh> {
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

    fn find_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.page_layout();
        match self.screen.find_hit(&l, x, y) {
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
            FindHit::Prev => self.find_step(false),
            FindHit::Next => self.find_step(true),
            FindHit::Replace => self.find_replace_one(),
            FindHit::All => self.find_replace_all(),
            FindHit::Case => {
                let f = self.find.as_mut()?;
                f.case = !f.case;
                let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
                self.select_match(sigma_core::find::at_or_after(&self.find_matches(), from));
                Some(Refresh::Partial)
            }
            FindHit::Bar | FindHit::Outside => None,
        }
    }

    /// After the query changed: select the first match from the start of
    /// the current selection (incremental search).
    fn query_changed(&mut self) {
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
    fn find_key(&mut self, k: Key) -> Option<Refresh> {
        match k {
            Key::Return => return self.find_step(true),
            Key::Esc => {
                self.close_find();
                return Some(Refresh::Full);
            }
            Key::Tab => {
                let f = self.find.as_mut()?;
                f.focus = Some(match f.focus {
                    Some(FindField::Find) => FindField::Replace,
                    _ => FindField::Find,
                });
                return Some(Refresh::Fast);
            }
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
    fn find_hw(&mut self, k: HwKey) -> Option<Refresh> {
        let key = match k {
            HwKey::Char(c) => Key::Char(c),
            HwKey::Backspace => Key::Back,
            HwKey::Delete => Key::Del,
            HwKey::Return => Key::Return,
            HwKey::Tab => Key::Tab,
            HwKey::Left => Key::Arrow(Dir::Left),
            HwKey::Right => Key::Arrow(Dir::Right),
            HwKey::Up
            | HwKey::Down
            | HwKey::Home
            | HwKey::End
            | HwKey::PageUp
            | HwKey::PageDown => return None,
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

    /// Forgets the cached map layout (perf measurements).
    pub fn clear_map_cache(&mut self) {
        self.map_cache = None;
    }

    /// Opens the Graph sheet with fresh rows.
    pub fn open_graph(&mut self) {
        self.help = None;
        self.close_find();
        self.save();
        self.sel_ui = None;
        self.drag = None;
        let g = self.link_graph();
        let name = |i: usize| stem(g.notes[i].name()).to_owned();
        let rows = (0..g.notes.len())
            .map(|i| {
                let to: Vec<String> = g.out[i]
                    .iter()
                    .map(|l| match l.to {
                        Some(j) => name(j),
                        None => format!("{} (new)", l.target),
                    })
                    .collect();
                let from: Vec<String> = g
                    .backlinks(&g.notes[i])
                    .iter()
                    .map(|p| stem(p.name()).to_owned())
                    .collect();
                let mut parts = Vec::new();
                if !to.is_empty() {
                    parts.push(format!("\u{2192} {}", to.join(", ")));
                }
                if !from.is_empty() {
                    parts.push(format!("\u{2190} {}", from.join(", ")));
                }
                GraphRow {
                    name: g.notes[i].as_str().to_owned(),
                    detail: if parts.is_empty() {
                        "No links".into()
                    } else {
                        parts.join("  \u{b7}  ")
                    },
                    current: g.notes[i] == self.current,
                }
            })
            .collect();
        let mut links: Vec<(usize, usize)> = g
            .out
            .iter()
            .enumerate()
            .flat_map(|(i, out)| out.iter().filter_map(move |l| l.to.map(|j| (i, j))))
            .filter(|(a, b)| a != b)
            .collect();
        links.sort_unstable();
        links.dedup();
        let names: Vec<String> = g.notes.iter().map(|n| n.as_str().to_owned()).collect();
        let key = graphmap::layout_key(&names, &links);
        let (layout, layout_us) = match &self.map_cache {
            Some((k, l)) if *k == key => (l.clone(), 0),
            _ => {
                let t0 = std::time::Instant::now();
                let l = std::sync::Arc::new(graphmap::layout(names.len(), &links, MAP_SEED));
                let us = t0.elapsed().as_micros() as u64;
                self.map_cache = Some((key, l.clone()));
                (l, us.max(1))
            }
        };
        let map = self.graph.as_ref().is_none_or(|g| g.map);
        self.graph = Some(GraphState {
            edges: g.edges(),
            notes: g.notes,
            rows,
            first: 0,
            map,
            links,
            layout,
            zoom: 0,
            pan: (0.0, 0.0),
            selected: None,
            layout_us,
        });
    }

    fn graph_model<'a>(&'a self, g: &'a GraphState) -> GraphModel<'a> {
        GraphModel {
            rows: &g.rows,
            first: g.first,
            edges: g.edges,
            theme: self.theme(),
            keyboard: &self.keyboard,
            map: g.map.then_some(MapView {
                pos: &g.layout.pos,
                orphan: &g.layout.orphan,
                edges: &g.links,
                zoom: g.zoom,
                pan: g.pan,
                selected: g.selected,
            }),
        }
    }

    fn graph_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let g = self.graph.as_ref()?;
        let hit = self.screen.graph_hit(&self.graph_model(g), x, y);
        match hit {
            GraphHit::Close => {
                self.graph = None;
                Some(Refresh::Full)
            }
            GraphHit::Row(i) => {
                let p = g.notes.get(i)?.clone();
                self.graph = None;
                if p != self.current {
                    self.push_history();
                }
                self.open(&p);
                Some(Refresh::Full)
            }
            GraphHit::Map | GraphHit::List => {
                let map = matches!(hit, GraphHit::Map);
                let g = self.graph.as_mut()?;
                if g.map == map {
                    return None;
                }
                g.map = map;
                g.selected = None;
                Some(Refresh::Full)
            }
            GraphHit::ZoomIn | GraphHit::ZoomOut => {
                self.graph_zoom(matches!(hit, GraphHit::ZoomIn))
            }
            GraphHit::Open => {
                let s = g.selected?;
                self.graph_open_preview(s)
            }
            GraphHit::Node(i) => {
                if g.selected == Some(i) {
                    return self.graph_open_preview(i);
                }
                self.graph.as_mut()?.selected = Some(i);
                // v1.0.2: the pill grows in one non-flashing GL16 region
                // (v1.0.4: the GL16 class again; 1.0.3 sent it as DU).
                Some(Refresh::PointerCleanup)
            }
            GraphHit::Nothing => {
                let g = self.graph.as_mut()?;
                if g.map && g.selected.take().is_some() {
                    return Some(Refresh::PointerCleanup);
                }
                None
            }
        }
    }

    /// Opens a map node in preview (the sheet closes).
    fn graph_open_preview(&mut self, i: usize) -> Option<Refresh> {
        let p = self.graph.as_ref()?.notes.get(i)?.clone();
        self.graph = None;
        self.map_drag = None;
        if p != self.current {
            self.push_history();
            self.open(&p);
        }
        self.enter_preview();
        Some(Refresh::Full)
    }

    /// One zoom step; the view stays centred where it is. Full refresh.
    fn graph_zoom(&mut self, zoom_in: bool) -> Option<Refresh> {
        let g = self.graph.as_mut()?;
        if !g.map {
            return None;
        }
        let z = if zoom_in {
            (g.zoom + 1).min(ZOOMS.len() - 1)
        } else {
            g.zoom.saturating_sub(1)
        };
        if z == g.zoom {
            return None;
        }
        g.zoom = z;
        if z == 0 {
            g.pan = (0.0, 0.0);
        }
        Some(Refresh::Full)
    }

    /// Pans the map by a screen distance (the picture follows the finger).
    fn graph_pan(&mut self, dx: i32, dy: i32) -> Option<Refresh> {
        let g = self.graph.as_ref()?;
        if !g.map || (dx == 0 && dy == 0) {
            return None;
        }
        let scale = self
            .screen
            .graph_layout(&self.graph_model(g))
            .scale
            .max(1e-6);
        let g = self.graph.as_mut()?;
        let lim = 2.0;
        g.pan.0 = (g.pan.0 - f64::from(dx) / scale).clamp(-lim, lim);
        g.pan.1 = (g.pan.1 - f64::from(dy) / scale).clamp(-lim, lim);
        Some(Refresh::Partial)
    }

    /// Whether a point is on the map drawing area.
    fn on_map(&self, x: i32, y: i32) -> bool {
        self.graph.as_ref().is_some_and(|g| {
            g.map
                && self
                    .screen
                    .graph_layout(&self.graph_model(g))
                    .area
                    .contains(x, y)
        })
    }

    fn graph_page(&mut self, forward: bool) -> Option<Refresh> {
        let g = self.graph.as_ref()?;
        if g.map {
            // Page buttons pan half a view down (forward) or up.
            let h = self.screen.graph_layout(&self.graph_model(g)).area.h / 2;
            return self.graph_pan(0, if forward { -h } else { h });
        }
        let visible = self
            .screen
            .graph_layout(&self.graph_model(g))
            .visible
            .max(1);
        let n = g.rows.len();
        let g = self.graph.as_mut()?;
        let next = if forward {
            if g.first + visible < n {
                g.first + visible
            } else {
                g.first
            }
        } else {
            g.first.saturating_sub(visible)
        };
        (next != g.first).then(|| {
            g.first = next;
            Refresh::Partial
        })
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
        self.graph = None;
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

/// Loads a note. A file that exists but cannot be read as text comes back
/// empty with `true`, and the app then never saves over it.
fn load_note(store: &FsNoteStore, p: &NotePath) -> (String, bool) {
    match store.load(p.as_str()) {
        Ok(b) => (b.unwrap_or_default(), false),
        Err(_) => (String::new(), true),
    }
}

/// Help sheet and perf (v0.7).
impl App {
    /// A key event for the perf sample: hardware keys, and taps (or
    /// lifts) on the on-screen keyboard.
    fn is_key_tap(&self, e: &Event) -> bool {
        let on_keys =
            |x: i32, y: i32| self.in_editor() && self.page_layout().keyboard.contains(x, y);
        match *e {
            Event::Key(_) => true,
            Event::Tap { x, y } => on_keys(x, y),
            Event::Touch {
                phase: Phase::Up,
                x,
                y,
                ..
            } => on_keys(x, y),
            _ => false,
        }
    }

    /// App info rows: version, licence, author, where notes live, credits.
    #[must_use]
    pub fn about(&self) -> Vec<(String, String)> {
        vec![
            (
                "Sigma Scribe".into(),
                format!(
                    "Version {}. A notebook for the Kobo Forma: plain Markdown files on an e-ink page, typed on the glass.",
                    env!("CARGO_PKG_VERSION")
                ),
            ),
            (
                "License".into(),
                "AGPL-3.0-only. The source is on GitHub: armandomartires/sigma-scribe.".into(),
            ),
            ("Author".into(), "Sigma Recon.".into()),
            (
                "Your notes".into(),
                format!(
                    "Plain .md and .txt files in {}. Connect the Kobo over USB to copy them. The scratch pad is the hidden file .scratchpad.md.",
                    self.library.root().display()
                ),
            ),
            (
                "Buttons".into(),
                "The page buttons scroll the page. The power button saves and goes back to the reader. Every change is saved as you type.".into(),
            ),
            (
                "USB keyboard & mouse".into(),
                "Needs a powered hub or a Y-cable: the reader does not power the port. See Help \u{2192} USB keyboard, mouse & export.".into(),
            ),
            (
                "Credits".into(),
                "Literata and Atkinson Hyperlegible fonts (SIL Open Font License 1.1). FBInk by NiLuJe (GPL-3.0-or-later). NickelMenu by pgaskin. Icons: Lucide (ISC licence), lucide.dev.".into(),
            ),
        ]
    }

    pub fn open_help(&mut self) {
        self.close_find();
        self.graph = None;
        self.files = None;
        self.options = false;
        self.save();
        self.sel_ui = None;
        self.drag = None;
        self.help = Some(HelpState {
            page: HelpPage::Menu,
            doc: None,
            at: 0,
        });
    }

    fn help_model<'a>(&'a self, h: &'a HelpState) -> HelpModel<'a> {
        HelpModel {
            page: h.page,
            doc: h.doc.as_ref(),
            at: h.at,
            theme: self.theme(),
            keyboard: &self.keyboard,
        }
    }

    fn help_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let h = self.help.as_ref()?;
        match self.screen.help_hit(&self.help_model(h), x, y) {
            HelpHit::Close => {
                self.help = None;
                Some(Refresh::Full)
            }
            HelpHit::Back => self.help_back(),
            HelpHit::Open(page) => {
                let doc = self.screen.build_help(&self.keyboard, page, &self.about());
                self.help = Some(HelpState {
                    page,
                    doc: Some(doc),
                    at: 0,
                });
                Some(Refresh::Full)
            }
            HelpHit::Nothing => None,
        }
    }

    /// Escape / Back: a sub-page goes back to the menu, the menu closes.
    fn help_back(&mut self) -> Option<Refresh> {
        let h = self.help.as_mut()?;
        if h.page == HelpPage::Menu {
            self.help = None;
        } else {
            h.page = HelpPage::Menu;
            h.doc = None;
            h.at = 0;
        }
        Some(Refresh::Full)
    }

    fn help_page(&mut self, forward: bool) -> Option<Refresh> {
        let h = self.help.as_mut()?;
        let n = h.doc.as_ref()?.pages.len();
        let next = if forward {
            (h.at + 1).min(n.saturating_sub(1))
        } else {
            h.at.saturating_sub(1)
        };
        (next != h.at).then(|| {
            h.at = next;
            Refresh::Partial
        })
    }
}

/// Touch-target audit (v0.7, brief: 44 px targets at the Forma scale).
impl App {
    /// Every tappable control on the current screen, named.
    #[must_use]
    pub fn touch_targets(&self) -> Vec<(String, Rect)> {
        let mut out: Vec<(String, Rect)> = Vec::new();
        let mut add = |n: &str, r: Rect| out.push((n.to_owned(), r));
        if self.options {
            let l = self.screen.options_layout(&self.options_model());
            add("Close", l.close);
            for (i, (_, cells)) in l.rows.iter().enumerate() {
                for (j, c) in cells.iter().enumerate() {
                    add(&format!("option {i}/{j}"), *c);
                }
            }
            return out;
        }
        if let Some(h) = &self.help {
            let m = self.help_model(h);
            let l = self.screen.help_layout(m.keyboard);
            add("Close", l.close);
            if h.page == HelpPage::Menu {
                add("App info", l.rows[0]);
                add("Markdown reference", l.rows[1]);
                add("USB keyboard, mouse & export", l.rows[2]);
            } else {
                add("Back", l.crumb);
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
        add("Graph", l.graph_button);
        add("Options", l.options_button);
        add("Help", l.help_button);
        if self.preview.is_some() {
            return out;
        }
        add("Scratch", l.scratch_button);
        add("To-dos", l.todos_button);
        add("Contents", l.contents_button);
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
