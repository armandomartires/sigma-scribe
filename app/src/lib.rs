//! Σscribe v0.2: a library of plain `.md` / `.txt` notes in folders,
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
use sigma_core::settings::{Orientation, Pad, Rotation, Side};
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

// `App` and its state live here; its methods are split by concern across
// these child modules, each with its own `impl App` block.
mod drawer;
mod editor;
mod events;
mod files;
mod find;
mod graph;
mod help;
mod ink;
mod keyboard;
mod nav;
mod paged_bar;
mod plugin_pane;
mod plugin_screen;
mod pointer;
mod preview;
mod render;
mod rotation;
mod script;
mod settings;
mod splash;

use editor::Drag;
use files::RenameTarget;
pub use plugin_screen::{IdlePlugin, PluginScreen, GRAPH_PLUGIN};
pub use splash::Splash;

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

/// Where the log file goes (Settings → Logging): `SIGMA_LOG_DIR`, else
/// `.adds/sigma-scribe/log` on the Kobo, else `./target/emu-log` on a
/// desktop.
#[must_use]
pub fn log_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SIGMA_LOG_DIR") {
        return p.into();
    }
    if cfg!(target_arch = "arm") {
        "/mnt/onboard/.adds/sigma-scribe/log".into()
    } else {
        "target/emu-log".into()
    }
}

/// Pointer redraws at most this often (ms): e-ink friendly (v0.8.1).
pub const POINTER_THROTTLE_MS: u64 = 150;
/// The pointer hides after this long without movement (ms).
pub const POINTER_HIDE_MS: u64 = 3000;
/// Auto orientation: a reading must hold this long before it is applied.
pub const TILT_STABLE_MS: u64 = 600;
/// Auto orientation waits this long after typing or touching.
pub const TILT_TYPING_PAUSE_MS: u64 = 1000;
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

fn rank(r: Refresh) -> u8 {
    match r {
        Refresh::Fast => 0,
        Refresh::Pointer | Refresh::PointerCleanup | Refresh::Ink | Refresh::InkA2 => 1,
        Refresh::Partial => 2,
        Refresh::Full => 3,
    }
}

/// The writing panel while open (v1.5.0, plan §12.3).
#[derive(Clone, Debug, Default)]
pub struct InkState {
    pub ink: sigma_core::ink::Ink,
    /// Segments drawn into the canvas since the last ink frame.
    pending: Option<Rect>,
    /// Everything inked since the last GL16 cleanup.
    dirty: Option<Rect>,
    /// When the last stroke lifted (ms).
    last_up: Option<u64>,
    /// The next render is an ink-only frame.
    fast: bool,
    /// The next `PointerCleanup` render is the ink cleanup pass.
    cleanup: bool,
    /// A button pressed on Down, acted on at Up over the same button.
    press: Option<sigma_render::ink::InkButton>,
    /// Write errors are shown in the status line.
    pub error: Option<String>,
    /// Draw, Letters or Train (v1.5.2).
    pub mode: sigma_render::ink::InkMode,
    /// Letters: the box the pending strokes are in.
    pub boxi: Option<usize>,
    /// Letters: candidates for the last letter, and that it was typed by
    /// recognition (so a candidate tap may replace it).
    pub cands: Vec<String>,
    replace_ok: bool,
    /// Words: characters typed for the last word (with its space), so a
    /// candidate can replace it; and the lexicon for this panel.
    last_len: usize,
    lex: Option<sigma_hwr::word::Lexicon>,
    /// Free (v1.5.3): first candidate shown, 123, language filter.
    pub first: usize,
    pub digits: bool,
    pub lang: Option<&'static str>,
    /// Train: samples saved this session.
    pub saved: usize,
    /// v1.5.4: where a touch above the panel went down (caret tap).
    tap_at: Option<(i32, i32)>,
}

/// Letters: idle time after a stroke before the box is read (ms).
pub const LETTER_MS: u64 = 800;
/// Words (v1.5.3): idle time before the line is read (ms).
pub const WORD_MS: u64 = 1200;

/// The user's handwriting samples, inside the notes folder (hidden).
#[must_use]
pub fn samples_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".handwriting").join("samples.txt")
}

/// The recogniser and its background fine-tuning (v1.5.2).
#[derive(Default)]
pub struct Hwr {
    pub rec: Option<sigma_hwr::Recogniser>,
    job: Option<std::thread::JoinHandle<sigma_hwr::Recogniser>>,
    pub samples: usize,
}

impl Hwr {
    /// Loads the model and the samples; fine-tuning on the samples runs
    /// on a thread, the plain model answers meanwhile.
    pub fn load(&mut self, root: &std::path::Path) {
        let samples = std::fs::read_to_string(samples_path(root))
            .map(|t| sigma_hwr::samples_from_text(&t))
            .unwrap_or_default();
        self.samples = samples.len();
        let base = sigma_hwr::Recogniser::new(sigma_hwr::Model::default_model())
            .with_pieces(sigma_hwr::Model::pieces_model());
        if samples.is_empty() {
            self.rec = Some(base);
            self.job = None;
            return;
        }
        let mut tuned = base.clone();
        self.rec = Some(base);
        self.job = Some(std::thread::spawn(move || {
            tuned.set_samples(&samples);
            tuned
        }));
    }

    /// Takes the fine-tuned recogniser once ready (`wait`: block for it).
    pub fn poll(&mut self, wait: bool) {
        if self.job.as_ref().is_some_and(|j| wait || j.is_finished()) {
            if let Some(Ok(r)) = self.job.take().map(std::thread::JoinHandle::join) {
                self.rec = Some(r);
            }
        }
    }
}

/// Idle time after a stroke before the GL16 cleanup (plan §12.4).
pub const INK_CLEANUP_MS: u64 = 600;

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
    /// The last full frame without the pointer (v1.0.4 fast path).
    base: Canvas,
    /// The next `Refresh::Pointer` render only moves the arrow: nothing
    /// else changed since the last full frame (v1.0.4).
    pointer_fast: bool,
    /// An accelerometer is present (v1.2; the Kobo build reports none
    /// until the device check, `sigma_device::gsensor::ENABLED`).
    gsensor: bool,
    /// `orientation_status()`, kept for the Options model borrow.
    orientation_label: String,
    /// The Kobo's language (v1.5.5), used while the override is off.
    system_lang: sigma_render::i18n::Lang,
    /// A sensor reading waiting for 0.6 s of stability (Auto).
    tilt: Option<(Rotation, u64)>,
    /// Latest event time seen and the last typing time, ms (Auto pause).
    now: u64,
    last_typed: Option<u64>,
    gestures: Recognizer,
    /// Touch selection chrome (v0.2.2).
    pub sel_ui: Option<SelUi>,
    drag: Option<Drag>,
    /// Map pan by finger: press point and whether it moved past the slop.
    map_drag: Option<(i32, i32, bool)>,
    /// Map layout cache: key over names + links (v1.0).
    map_cache: Option<(u64, std::sync::Arc<MapLayout>)>,
    /// The Graph as a WASM plugin, when one is installed and enabled.
    plugin_screen: Option<PluginScreen>,
    /// A loaded plugin kept for the next open.
    plugin_idle: Option<IdlePlugin>,
    /// The installed plugins, as Settings → Plugins lists them (refreshed
    /// whenever Settings opens or a switch is flipped).
    plugin_list: Vec<sigma_render::options::PluginRow>,
    /// Where plugins are installed.
    plugins_root: PathBuf,
    /// A plugin failed: the built-in Graph serves the rest of the session.
    plugin_failed: bool,
    /// The pane plugins of the side drawer (Contents, To-dos, Scratch): the
    /// enabled one for each, the loaded module, and whether it failed (the
    /// built-in pane then draws for the session).
    pane_slots: [Option<plugin_pane::PaneSlotPlugin>; 3],
    pane_plugins: [Option<plugin_pane::PanePlugin>; 3],
    pane_failed: [bool; 3],
    /// Show the drawer icons and draw the built-in panes with the pane
    /// plugins off (the reference the plugins are tested against, and the
    /// emulator's drawer). Off in the app.
    panes_builtin: bool,
    /// Where the log file goes while Settings → Logging is on.
    log_dir: PathBuf,
    /// Show the Graph icon even with the Graph plugin off, and open the
    /// built-in Graph (the reference the plugin is tested against, and the
    /// emulator's Graph). Off in the app.
    graph_builtin: bool,
    /// `Some` while the note is shown rendered (eye ↔ pencil).
    pub preview: Option<PreviewState>,
    /// Every Options setting (v0.4), kept in `notes/.sigma-settings`.
    pub settings: Settings,
    /// The Options sheet is open.
    pub options: bool,
    /// Settings page shown while `options` (v1.4.2) and its sub-page.
    pub settings_page: sigma_render::SettingsPage,
    pub settings_sub: usize,
    /// Settings slider with keyboard focus (v1.4.4): Left / Right move it.
    pub settings_focus: Option<&'static str>,
    /// A finger went down on a Settings slider (v1.4.4): the drag owns the
    /// touch until it lifts.
    slider_touch: Option<&'static str>,
    /// A finger on a paged scroll bar (Settings, Help, Preview): where it
    /// went down and whether it has moved (v1.13.1).
    bar_drag: Option<(i32, bool)>,
    /// UI scale picked on its slider but not applied until the finger lifts.
    ui_scale_drag: Option<u16>,
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
    /// The front light (v1.12): whether the device has one and where it
    /// stands (read at start, then what Display → Backlight set), and a
    /// step still to send.
    light_have: bool,
    light_step: u8,
    light_want: Option<u8>,
    light_probed: bool,
    /// USB mouse / touchpad (v0.8.1).
    pub pointer: PointerState,
    /// The ink panel (v1.5.0); `Some` while open.
    pub ink: Option<InkState>,
    /// Letter recognition (v1.5.2), loaded on first use.
    pub hwr: Hwr,
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
        // v1.4: the interface follows the Kobo's language (read-only).
        let locale = sigma_core::locale::read_locale(&sigma_core::locale::conf_path());
        let system_lang = locale.map_or(sigma_render::i18n::Lang::En, |l| {
            sigma_render::i18n::Lang::from_locale(&l)
        });
        sigma_render::i18n::set_lang(resolve_lang(&settings, system_lang));
        let (lw, lh) = logical_size((width, height), settings.rotation);
        let mut screen = EditorScreen::new(lw, lh);
        screen.configure(&mut settings, None);
        let scratch = Buffer::new(store.load(SCRATCH).ok().flatten().unwrap_or_default());
        let mut app = Self {
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
            keyboard: Keyboard {
                lang: settings.kb_layout,
                ..Keyboard::with_layout(settings.keyboard)
            },
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
            plugin_screen: None,
            plugin_idle: None,
            plugin_list: Vec::new(),
            plugins_root: sigma_core::plugins::plugins_root(),
            plugin_failed: false,
            pane_slots: [None, None, None],
            pane_plugins: [None, None, None],
            pane_failed: [false; 3],
            panes_builtin: false,
            log_dir: log_dir(),
            graph_builtin: false,
            preview: None,
            canvas: Canvas::new(lw, lh, 255),
            shown: Canvas::new(lw, lh, 0),
            base: Canvas::new(1, 1, 0),
            pointer_fast: false,
            gsensor: false,
            orientation_label: String::new(),
            system_lang,
            tilt: None,
            now: 0,
            last_typed: None,
            settings,
            options: false,
            settings_page: sigma_render::SettingsPage::Root,
            settings_sub: 0,
            settings_focus: None,
            slider_touch: None,
            bar_drag: None,
            ui_scale_drag: None,
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
            light_have: false,
            light_step: 0,
            light_want: None,
            light_probed: false,
            pointer: PointerState::default(),
            ink: None,
            hwr: Hwr::default(),
            history: Vec::new(),
            perf: Perf::default(),
            running: true,
        };
        app.set_gsensor(false);
        app.sync_keyboard_style();
        app.sync_plugin_icons();
        app.apply_logging();
        sigma_core::info!(
            "app",
            "sigma-scribe {} started: screen {width}x{height}, notes {}, plugins {}",
            env!("CARGO_PKG_VERSION"),
            app.library.root().display(),
            app.plugins_root.display()
        );
        app
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
        sigma_core::debug!("app", "open note {}", p.as_str());
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
fn logical_size(phys: (u32, u32), r: Rotation) -> (u32, u32) {
    let (short, long) = (phys.0.min(phys.1), phys.0.max(phys.1));
    if r.is_landscape() {
        (long, short)
    } else {
        (short, long)
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

impl App {}

/// A note path without its extension (link labels).
fn stem(s: &str) -> &str {
    s.rsplit_once('.').map_or(s, |(a, _)| a)
}

/// Find bar and wiki links (v0.6).
impl App {
    fn page_layout(&self) -> sigma_render::editor::Layout {
        self.screen.layout(self.keys_visible(), &self.keyboard)
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
impl App {}

/// Touch-target audit (v0.7, brief: 44 px targets at the Forma scale).
impl App {}

/// The interface language (v1.5.5, decision 57): the Kobo's unless
/// Settings → Language → Override system language is on.
fn resolve_lang(s: &Settings, system: sigma_render::i18n::Lang) -> sigma_render::i18n::Lang {
    if !s.lang_override {
        return system;
    }
    sigma_render::i18n::Lang::from_locale(s.ui_lang.code())
}
