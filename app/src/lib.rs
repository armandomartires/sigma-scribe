//! Σscribe v0.2: a library of plain `.md` / `.txt` notes in folders,
//! the Files sheet, rename in place, and the v0.1 editor with on-screen
//! keyboard and autosave. Generic over the device so the Forma, the desktop
//! window and the headless emulator run the same code.

use sigma_core::gesture::Recognizer;
use sigma_core::graphmap::MapLayout;
use sigma_core::library::SCRATCH;
use sigma_core::outline::{Filter, Indexer, TodoItem};
use sigma_core::settings::Rotation;
use sigma_core::{Autosave, Buffer, FsNoteStore, Keyboard, Library, NotePath, NoteStore, Settings};
use sigma_device::Refresh;
use sigma_render::{Canvas, EditorScreen, GraphRow, Rect, SelUi};
use std::path::PathBuf;

// `App` and its state live here; its methods are split by concern across
// these child modules, each with its own `impl App` block.
mod bootstrap;
mod drawer;
mod editor;
mod events;
mod files;
mod find;
mod graph;
mod handwriting;
mod help;
mod ink;
mod keyboard;
mod light_menu;
mod nav;
mod paged_bar;
mod perf;
mod plugin_pane;
mod plugin_repo;
mod plugin_screen;
mod pointer;
mod preview;
mod render;
mod rotation;
mod script;
mod settings;
mod splash;
mod sync;

use crate::files::RenameTarget;
use crate::ink::ReadInk;
use crate::settings::{load_settings, resolve_lang};
use editor::Drag;

pub use bootstrap::{log_dir, notes_dir, NOTE, SEED, SEED_FILES};
pub use files::FilesState;
pub use find::FindState;
pub use graph::{GraphState, MAP_SEED};
pub use handwriting::{all_samples, learned_path, samples_path, Hwr, LEARN_PER_CHAR};
pub use help::HelpState;
pub use ink::{InkState, INK_CLEANUP_MS, LETTER_MS, WORD_MS};
pub use nav::{HistEntry, HISTORY_CAP};
pub use perf::Perf;
pub use plugin_screen::{IdlePlugin, PluginScreen, GRAPH_PLUGIN};
pub use pointer::{
    MouseDown, PointerState, DOUBLE_CLICK_MS, DOUBLE_CLICK_PX, POINTER_HIDE_MS, POINTER_THROTTLE_MS,
};
pub use preview::PreviewState;
pub use rotation::{TILT_STABLE_MS, TILT_TYPING_PAUSE_MS};
pub use splash::Splash;
pub use sync::{sync_config, syncthing_binary, syncthing_home};

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
    /// What the plugins repository listed at the last check.
    repo_index: Option<sigma_core::plugins::Index>,
    /// Its rows for Settings → Plugins.
    repo_rows: Vec<sigma_render::options::RepoRow>,
    /// How the last check or install went.
    repo_note: String,
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
    /// Syncthing, while Settings → Sync is on (`docs/sync.md`), how to run
    /// it, and what it last reported (`sync_gen` counts the supervisor's
    /// changes the app has taken in).
    sync: Option<sigma_core::sync::Supervisor>,
    sync_cfg: sigma_core::sync::Config,
    sync_view: sigma_core::sync::View,
    sync_gen: u64,
    /// The open note's and the scratch pad's files as the app last read or
    /// wrote them, to notice a change made by someone else.
    note_stamp: Option<sync::Stamp>,
    scratch_stamp: Option<sync::Stamp>,
    /// Hashes of the text those files held then.
    note_hash: u64,
    scratch_hash: u64,
    /// When the stamps are next compared, ms.
    watch_at: u64,
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
    /// A finger on the drawer's edge grip, or on the closed drawer's tab.
    handle_drag: Option<drawer::GripTouch>,
    /// When the last tap on the grip lifted: a second one soon after
    /// closes the drawer (v1.16).
    grip_tap: Option<u64>,
    /// The panes (and scratch focus) the grip closed, which the tab at the
    /// screen edge reopens (v1.16). Not kept across a restart.
    drawer_stash: Option<([bool; 3], bool)>,
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
    /// The warm-to-cool mixer (v1.17): whether the device has one, the step
    /// shown, and a step waiting to be sent. The first sync takes the saved
    /// step, else what the device reads back, else the cool end.
    warmth_have: bool,
    warmth_step: u8,
    warmth_want: Option<u8>,
    warmth_probed: bool,
    /// The light-menu slider the finger is on (a drag moves it).
    light_touch: Option<sigma_render::light_menu::LightRow>,
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
            repo_index: None,
            repo_rows: Vec::new(),
            repo_note: String::new(),
            plugins_root: sigma_core::plugins::plugins_root(),
            plugin_failed: false,
            pane_slots: [None, None, None],
            pane_plugins: [None, None, None],
            pane_failed: [false; 3],
            panes_builtin: false,
            log_dir: log_dir(),
            sync: None,
            sync_cfg: sync::sync_config(&root, &log_dir()),
            sync_view: sigma_core::sync::View::default(),
            sync_gen: 0,
            note_stamp: None,
            scratch_stamp: None,
            note_hash: 0,
            scratch_hash: 0,
            watch_at: 0,
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
            handle_drag: None,
            grip_tap: None,
            drawer_stash: None,
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
            warmth_have: false,
            warmth_step: 0,
            warmth_want: None,
            warmth_probed: false,
            light_touch: None,
            pointer: PointerState::default(),
            ink: None,
            hwr: Hwr::default(),
            history: Vec::new(),
            perf: Perf::default(),
            running: true,
        };
        app.remember_note_stamp();
        app.remember_scratch_stamp();
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
        let dirty = self.autosave.dirty(rev);
        // Syncthing may have delivered a newer version of this note since
        // it was read: never write over it (docs/sync.md §9).
        if dirty && self.guard_note_write() {
            return;
        }
        self.autosave.sync(
            &mut self.store,
            self.current.as_str(),
            self.buffer.text(),
            rev,
        );
        if dirty && self.autosave.state == sigma_core::SaveState::Saved {
            self.remember_note_stamp();
        }
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
        self.remember_note_stamp();
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
