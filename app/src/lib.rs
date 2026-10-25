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

mod plugin_pane;
mod plugin_screen;
pub use plugin_screen::{IdlePlugin, PluginScreen, GRAPH_PLUGIN};
mod drawer;
mod files;
use files::RenameTarget;
mod editor;
mod keyboard;
use editor::Drag;
mod find;
mod graph;
mod ink;
mod nav;
mod preview;
mod render;
mod splash;
pub use splash::Splash;
mod help;
mod paged_bar;
mod pointer;
mod rotation;

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
        let base = sigma_hwr::Recogniser::new(sigma_hwr::Model::default_model());
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

    /// Applies Settings → Logging: the log file on or off, and its level.
    fn apply_logging(&self) {
        sigma_core::log::configure(
            self.settings.log,
            self.settings.log_level,
            Some(self.log_dir.clone()),
        );
    }

    /// Where the log file goes (tests point this at a temp folder).
    pub fn set_log_dir(&mut self, dir: impl Into<PathBuf>) {
        self.log_dir = dir.into();
        self.apply_logging();
    }

    /// The Kobo style board while Enhanced keyboard is off (v1.13); back to
    /// the Compact / Full board (its saved choice) when it is on. Layer and
    /// shift start over when the style changes.
    pub fn sync_keyboard_style(&mut self) {
        use sigma_core::keyboard::{Layer, Style};
        let want = if self.settings.kobo_keyboard() {
            Style::Kobo
        } else {
            Style::Enhanced
        };
        if self.keyboard.style != want {
            self.keyboard.style = want;
            self.keyboard.layer = Layer::Letters;
            self.keyboard.shift = false;
            self.keyboard.shift_lock = false;
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

    /// The interface language (v1.4); normally from the Kobo's locale at
    /// start. Re-lays out so translated labels are measured.
    pub fn set_language(&mut self, l: sigma_render::i18n::Lang) {
        self.system_lang = l;
        sigma_render::i18n::set_lang(resolve_lang(&self.settings, l));
        self.orientation_label = self.orientation_status();
    }

    /// Applies one Options / keyboard-bar change: refits Line mode,
    /// re-lays out (and resizes on rotation), saves the settings file.
    /// Paper and orientation need a full (flashing) refresh.
    pub fn change(&mut self, c: Change) -> Refresh {
        if let Change::Plugin(i, on) = c {
            return self.set_plugin(i, on);
        }
        if let Change::PluginSetting(p, s, o) = c {
            return self.set_plugin_setting(usize::from(p), usize::from(s), usize::from(o));
        }
        if c == Change::TrainSamples {
            self.options = false;
            self.open_ink();
            if let Some(ink) = &mut self.ink {
                ink.mode = sigma_render::ink::InkMode::Train(0);
            }
            return Refresh::Full;
        }
        if c == Change::DeleteSamples {
            let p = samples_path(self.library.root());
            if let Err(e) = std::fs::remove_file(&p) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    sigma_core::warn!("samples", "delete: {e}");
                }
            }
            if let Some(d) = p.parent() {
                let _ = std::fs::remove_dir(d);
            }
            self.hwr = Hwr::default();
            return Refresh::Partial;
        }
        if let Change::Backlight(n) = c {
            // Not a saved setting: the front light keeps its level.
            self.light_step = n.min(sigma_core::settings::BACKLIGHT_MAX);
            self.light_want = Some(self.light_step);
            return Refresh::Partial;
        }
        self.settings.apply(c);
        if matches!(c, Change::Log(_) | Change::LogLevel(_)) {
            self.apply_logging();
            if self.settings.log {
                sigma_core::info!("app", "logging level {}", self.settings.log_level.label());
            }
        }
        sigma_core::debug!("settings", "{c:?}");
        self.screen.ink_words = self.settings.word_mode();
        if c == Change::LangOverride(true) {
            // Start from the language in use, so turning it on changes nothing.
            use sigma_core::settings::UiLang;
            self.settings.ui_lang = UiLang::from_code(self.system_lang.code()).unwrap_or_default();
        }
        if matches!(c, Change::LangOverride(_) | Change::UiLang(_)) {
            sigma_render::i18n::set_lang(resolve_lang(&self.settings, self.system_lang));
            self.orientation_label = self.orientation_status();
        }
        if let Change::Keyboard(l) = c {
            self.keyboard = Keyboard {
                layout: l,
                clipboard: std::mem::take(&mut self.keyboard.clipboard),
                ..Keyboard::default()
            };
        }
        self.keyboard.lang = self.settings.kb_layout;
        self.sync_keyboard_style();
        if !self.keyboard.lang.has_compact() {
            // Too many keys for the compact board (v1.9).
            self.keyboard.layout = sigma_core::keyboard::Layout::Full;
        }
        sigma_device::usbkbd::set_layout(self.settings.kb_layout);
        let anchor = match c.refit() {
            Some(Some(true)) => Some(Anchor::Size),
            Some(Some(false)) => Some(Anchor::Margin),
            _ => None,
        };
        let (w, h) = logical_size(self.phys, self.settings.rotation);
        if (self.screen.width, self.screen.height) != (w, h) {
            self.screen.resize(w, h);
            self.canvas = Canvas::new(w, h, 255);
            self.shown = Canvas::new(w, h, 0);
            self.sel_ui = None;
        }
        self.screen.configure(&mut self.settings, anchor);
        self.orientation_label = self.orientation_status();
        self.save_settings();
        if c.needs_full_refresh() {
            Refresh::Full
        } else {
            Refresh::Partial
        }
    }

    /// Opens the Options sheet (closes preview and selection chrome).
    pub fn open_options(&mut self) {
        sigma_core::debug!("ui", "open settings");
        self.help = None;
        self.close_find();
        self.close_graph();
        self.save();
        self.preview = None;
        self.sel_ui = None;
        self.drag = None;
        self.options = true;
        self.hwr.samples = std::fs::read_to_string(samples_path(self.library.root()))
            .map(|t| sigma_hwr::samples_from_text(&t).len())
            .unwrap_or(0);
        self.settings_page = sigma_render::SettingsPage::Root;
        self.settings_sub = 0;
        self.settings_focus = None;
        self.refresh_plugin_list();
    }

    pub fn options_model(&self) -> OptionsModel<'_> {
        OptionsModel {
            settings: &self.settings,
            theme: self.theme(),
            keyboard: &self.keyboard,
            orientation_status: &self.orientation_label,
            page: self.settings_page,
            sub: self.settings_sub,
            language: sigma_core::settings::UiLang::from_code(sigma_render::i18n::lang().code())
                .unwrap_or_default()
                .name(),
            version: env!("CARGO_PKG_VERSION"),
            focus: self.settings_focus,
            hw_samples: self.hwr.samples,
            light: self.light_have.then_some(self.light_step),
            ui_scale_pending: self.ui_scale_drag,
            plugins: &self.plugin_list,
        }
    }

    /// Settings: one level up, or closed from the top (Esc, back arrow).
    fn settings_back(&mut self) -> Refresh {
        match sigma_render::options::parent(self.settings_page) {
            Some(p) => {
                self.settings_page = p;
                self.settings_sub = 0;
            }
            None => self.options = false,
        }
        Refresh::Full
    }

    /// Next step towards a Settings control that is not on this page.
    #[must_use]
    pub fn settings_step(&self, name: &str) -> Option<String> {
        if !self.options {
            return None;
        }
        self.screen.options_step(&self.options_model(), name)
    }

    fn options_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let focus = self
            .screen
            .options_slider_at(&self.options_model(), x, y)
            .map(|t| t.0);
        let refocus = focus != self.settings_focus;
        self.settings_focus = focus;
        let r = self.options_hit_tap(x, y);
        if refocus && r.is_none() {
            return Some(Refresh::Partial);
        }
        r
    }

    fn options_touch(&mut self, phase: Phase, x: i32, y: i32) -> Option<Option<Refresh>> {
        match phase {
            Phase::Down => {
                let (lab, ch) = self.screen.options_slider_at(&self.options_model(), x, y)?;
                self.slider_touch = Some(lab);
                self.settings_focus = Some(lab);
                // UI scale rescales everything: only the thumb moves while
                // the finger is down, the scale changes on release.
                if let Some(Change::UiScaleTo(v)) = ch {
                    self.ui_scale_drag = Some(v);
                    return Some(Some(Refresh::Partial));
                }
                Some(Some(match ch {
                    Some(c) => self.change(c),
                    None => Refresh::Partial,
                }))
            }
            Phase::Move | Phase::Up => {
                let lab = self.slider_touch?;
                if phase == Phase::Up {
                    self.slider_touch = None;
                }
                let ch = self
                    .screen
                    .options_slider_drag(&self.options_model(), lab, x);
                if lab == "UI scale" {
                    let Some(Change::UiScaleTo(v)) = ch else {
                        return Some(None);
                    };
                    if phase == Phase::Up {
                        self.ui_scale_drag = None;
                        return Some(Some(if v == self.settings.ui_scale {
                            Refresh::Partial
                        } else {
                            self.change(Change::UiScaleTo(v))
                        }));
                    }
                    let moved = self.ui_scale_drag != Some(v);
                    self.ui_scale_drag = Some(v);
                    return Some(moved.then_some(Refresh::Partial));
                }
                Some(match ch {
                    Some(Change::KbSize(v)) if v == self.settings.kb_size => None,
                    Some(Change::SizeTo(v)) if (v - self.settings.size).abs() < 0.05 => None,
                    Some(Change::LeadingTo(v)) if (v - self.settings.leading).abs() < 0.05 => None,
                    Some(Change::MarginTo(v)) if (v - self.settings.margin).abs() < 0.05 => None,
                    Some(Change::UiScaleTo(v)) if v == self.settings.ui_scale => None,
                    // The same light step again (every drag event repeats it): no
                    // redraw, no sysfs write, no log line.
                    Some(Change::Backlight(n)) if n == self.light_step => None,
                    Some(c) => Some(self.change(c)),
                    None => None,
                })
            }
        }
    }

    fn options_hit_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        match self.screen.options_hit(&self.options_model(), x, y) {
            OptionsHit::Close => {
                self.options = false;
                Some(Refresh::Full)
            }
            OptionsHit::Change(c) => Some(self.change(c)),
            OptionsHit::Back => Some(self.settings_back()),
            OptionsHit::Page => {
                let n = self.screen.options_layout(&self.options_model()).pages;
                self.settings_sub = (self.settings_sub + 1) % n.max(1);
                Some(Refresh::Full)
            }
            OptionsHit::Open(p) => {
                self.settings_page = p;
                self.settings_sub = 0;
                Some(Refresh::Full)
            }
            OptionsHit::Pick(c) => {
                self.change(c);
                Some(self.settings_back())
            }
            OptionsHit::Help(i) => {
                self.options = false;
                self.open_help();
                if let Some(h) = self.help.as_mut() {
                    h.page = sigma_render::help::MENU[i].2;
                }
                Some(Refresh::Full)
            }
            OptionsHit::Nothing => None,
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

    /// Applies one event; returns the refresh it needs (or `None`).
    pub fn handle(&mut self, e: Event) -> Option<Refresh> {
        self.sync_pane_plugins();
        match e {
            Event::Tilt { tilt, t } => {
                self.now = self.now.max(t);
                self.pointer_fast = false;
                return self.tilt_event(tilt, t);
            }
            Event::Key(_) | Event::Chord { .. } => self.last_typed = Some(self.now),
            // Touching (on-screen keys included) also pauses Auto for 1 s.
            Event::Touch { t, .. } => {
                self.now = self.now.max(t);
                self.last_typed = Some(self.now);
            }
            Event::Tap { .. } => self.last_typed = Some(self.now),
            _ => {}
        }
        if let Event::Tick { t } = e {
            self.now = self.now.max(t);
            if let Some(r) = self.tilt_tick(t) {
                self.pointer_fast = false;
                let rest = self.handle_inner(e);
                return merge(Some(r), rest);
            }
        }
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
            Event::Tilt { .. } => None,
            Event::Tap { x, y } if self.paged_bar().is_some_and(|(_, b)| b.zone.contains(x, y)) => {
                self.bar_tap(x, y).flatten()
            }
            Event::Tap { x, y } if self.ink.is_some() => {
                let l = self.panel_layout()?;
                if !l.panel.contains(x, y) {
                    return self.place_caret(x, y);
                }
                let b = self.screen.ink_hit(&l, x, y)?;
                self.ink_button(b)
            }
            Event::Touch { phase, x, y, t } => {
                if let Some(r) = self.ink_touch(phase, x, y, t) {
                    return r;
                }
                if let Some(r) = self.bar_touch(phase, x, y) {
                    return r;
                }
                if self.options {
                    if let Some(r) = self.options_touch(phase, x, y) {
                        return r;
                    }
                }
                let g = match phase {
                    Phase::Down => self.gestures.down(x, y, t),
                    Phase::Move => self.gestures.moved(x, y, t),
                    Phase::Up => self.gestures.up(x, y, t),
                };
                self.apply(g)
            }
            Event::Tick { t } => {
                if let Some(r) = self.ink_tick(t) {
                    return Some(r);
                }
                // A plugin screen with more to do (loading, or building a
                // layout) takes its next step while the app is idle.
                if let Some(r) = self.plugin_step() {
                    return Some(r);
                }
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
            Event::Key(
                k @ (HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown | HwKey::Insert),
            ) => self.chord(k, Mods::default()),
            Event::Quit | Event::Power => {
                self.save();
                self.save_scratch();
                self.running = false;
                None
            }
            Event::Tap { x, y } if self.options => self.options_tap(x, y),
            Event::Key(HwKey::Escape) if self.options => {
                self.settings_focus = None;
                Some(self.settings_back())
            }
            Event::Key(k @ (HwKey::Left | HwKey::Right))
                if self.options && self.settings_focus.is_some() =>
            {
                let lab = self.settings_focus?;
                let d = if k == HwKey::Left { -1 } else { 1 };
                let c = self
                    .screen
                    .options_slider_step(&self.options_model(), lab, d)?;
                Some(self.change(c))
            }
            Event::PageTurn { forward } if self.options => {
                let n = self
                    .screen
                    .options_layout(&self.options_model())
                    .pages
                    .max(1);
                self.settings_sub = if forward {
                    (self.settings_sub + 1).min(n - 1)
                } else {
                    self.settings_sub.saturating_sub(1)
                };
                Some(Refresh::Full)
            }
            Event::Key(_) | Event::PageTurn { .. } if self.options => None,
            Event::Tap { x, y } if self.help.is_some() => self.help_tap(x, y),
            Event::PageTurn { forward } if self.help.is_some() => self.help_page(forward),
            Event::Key(HwKey::Escape) if self.help.is_some() => self.help_back(),
            Event::Key(_) if self.help.is_some() => None,
            Event::Tap { x, y } if self.plugin_screen.is_some() => self.plugin_tap(x, y),
            Event::PageTurn { forward } if self.plugin_screen.is_some() => {
                self.plugin_page(forward)
            }
            Event::Key(HwKey::Escape) if self.plugin_screen.is_some() => {
                self.close_graph();
                Some(Refresh::Full)
            }
            Event::Key(_) if self.plugin_screen.is_some() => None,
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
                // Backspace, Return and Esc act on a composition themselves.
                // Left and Right choose the phrase while Japanese is converting.
                let phrase = matches!(k, HwKey::Left | HwKey::Right)
                    && self.keyboard.composer.candidates().is_some();
                if phrase {
                    self.keyboard
                        .composer
                        .phrase_arrow(&mut self.buffer, k == HwKey::Right, false);
                    return Some(Refresh::Fast);
                }
                if !matches!(
                    k,
                    HwKey::Char(_) | HwKey::Backspace | HwKey::Return | HwKey::Escape
                ) {
                    let t = self.keyboard.composer.flush();
                    if !t.is_empty() {
                        self.buffer.insert(&t);
                    }
                }
                match k {
                    HwKey::Char(c) => {
                        // Overwrite is for plain typing, not for composing layouts.
                        let over = self.buffer.overwrite();
                        if self.settings.kb_layout.composes() {
                            self.buffer.set_overwrite(false);
                        }
                        self.keyboard.composer.type_char(
                            &mut self.buffer,
                            self.settings.kb_layout,
                            c,
                        );
                        self.buffer.set_overwrite(over);
                    }
                    HwKey::Backspace => self
                        .keyboard
                        .composer
                        .backspace(&mut self.buffer, self.settings.kb_layout),
                    HwKey::Delete => self.buffer.delete_forward(),
                    HwKey::Return => {
                        if !self.keyboard.composer.enter(&mut self.buffer) {
                            self.buffer.newline();
                        }
                    }
                    HwKey::Tab => self.buffer.insert_tab(self.settings.tab_kind()),
                    HwKey::Left | HwKey::Right => {
                        // Flipped in a right-to-left row (v1.8).
                        if (k == HwKey::Left) != self.caret_rtl() {
                            self.buffer.left();
                        } else {
                            self.buffer.right();
                        }
                    }
                    HwKey::Up => self.screen.caret_row(&mut self.buffer, page, false),
                    HwKey::Down => self.screen.caret_row(&mut self.buffer, page, true),
                    HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown | HwKey::Insert => {
                        return None
                    }
                    HwKey::Escape => {
                        if self.keyboard.composer.escape(&mut self.buffer) {
                            self.save();
                            return Some(Refresh::Fast);
                        }
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
                if self.on_scroll_bar(x, y) {
                    let page = self.page_rect();
                    self.screen.scroll_bar_step(&mut self.buffer, page, y);
                    return Some(Refresh::Partial);
                }
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
                    Hit::Drawer(p) => Some(self.toggle_pane(p)),
                    Hit::Candidate(i) => {
                        self.keyboard.composer.pick(&mut self.buffer, i);
                        self.save();
                        Some(Refresh::Partial)
                    }
                    Hit::Key(k) => {
                        let tab = self.settings.tab_kind();
                        self.keyboard.rtl = self.caret_rtl();
                        let a = self.keyboard.press(k, &mut self.buffer, tab);
                        self.after_key(a)
                    }
                    Hit::ToggleKeys => {
                        self.keys_on = !self.keys_on;
                        Some(Refresh::Partial)
                    }
                    Hit::Ink => {
                        self.open_ink();
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
                sigma_core::error!("usb", "{e}");
            }
        }
    }

    /// Reads the front light once, then sends the step Display → Backlight
    /// asked for (v1.12). A failure is logged; the slider keeps its place.
    pub fn sync_light(&mut self, dev: &mut dyn Device) {
        if !self.light_probed {
            self.light_probed = true;
            if let Some(n) = dev.light_step() {
                self.light_have = true;
                self.light_step = n;
            }
        }
        if let Some(n) = self.light_want.take() {
            if let Err(e) = dev.set_light(n) {
                sigma_core::error!("light", "{e}");
            }
        }
    }

    /// Main loop until power/quit or end of input.
    ///
    /// # Errors
    /// The device failed to present a frame.
    pub fn run(&mut self, dev: &mut dyn Device) -> std::io::Result<()> {
        self.set_gsensor(dev.has_gsensor());
        sigma_device::usbkbd::set_layout(self.settings.kb_layout);
        self.render(dev, Refresh::Full)?;
        while self.running {
            self.sync_usb_host(dev);
            self.sync_light(dev);
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
            sigma_core::info!(
                "perf",
                "{n} keystrokes, keystroke-to-refresh p50 {p50:.1} ms, p95 {p95:.1} ms, max {max:.1} ms"
            );
        }
        if let Some((n, p50, p95, max)) = self.perf.pointer_summary() {
            sigma_core::info!(
                "perf pointer",
                "{n} du updates, move-to-submit p50 {p50:.1} ms, p95 {p95:.1} ms, max {max:.1} ms"
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
}

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
}

/// Touch-target audit (v0.7, brief: 44 px targets at the Forma scale).
impl App {
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

/// The interface language (v1.5.5, decision 57): the Kobo's unless
/// Settings → Language → Override system language is on.
fn resolve_lang(s: &Settings, system: sigma_render::i18n::Lang) -> sigma_render::i18n::Lang {
    if !s.lang_override {
        return system;
    }
    sigma_render::i18n::Lang::from_locale(s.ui_lang.code())
}
