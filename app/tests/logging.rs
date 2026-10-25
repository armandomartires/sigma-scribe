//! Settings → Logging: the switch and the level, the log file they control,
//! and the lines the app and its plugins write to it.
//!
//! The logger is global to the process, so these tests take turns.

use sigma_core::log::{Level, FILE_NAME};
use sigma_core::NotePath;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_render::options::LOGGING_CATEGORY;
use sigma_render::SettingsPage;
use sigma_scribe::App;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static TURN: Mutex<()> = Mutex::new(());

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph-map")
}

struct Rig {
    app: App,
    dev: DesktopDevice,
    log: PathBuf,
    notes: PathBuf,
    plugins: PathBuf,
    _turn: MutexGuard<'static, ()>,
}

impl Drop for Rig {
    fn drop(&mut self) {
        // Leave the global logger off for whoever is next.
        sigma_core::log::configure(false, Level::Info, None);
    }
}

impl Rig {
    /// A library with a few linked notes, the real Graph plugin installed and
    /// on, and the log folder in a temp directory.
    fn new(tag: &str) -> Self {
        let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = std::env::temp_dir().join(format!("sigma-logging-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (notes, plugins, log) = (d.join("notes"), d.join("plugins"), d.join("log"));
        std::fs::create_dir_all(plugins.join("graph-map")).unwrap();
        std::fs::create_dir_all(&notes).unwrap();
        for (n, body) in [
            ("a.md", "[[b]] [[c]]"),
            ("b.md", "[[c]]"),
            ("c.md", "[[a]]"),
        ] {
            std::fs::write(notes.join(n), body).unwrap();
        }
        for f in ["plugin.conf", "graph.wasm"] {
            std::fs::copy(fixture().join(f), plugins.join("graph-map").join(f)).unwrap();
        }
        std::fs::write(plugins.join("plugins.state"), "graph-map=1.0.0,enabled\n").unwrap();
        let mut app = App::new(1440, 1920, notes.clone());
        app.set_plugins_root(plugins.clone());
        app.set_log_dir(log.clone());
        app.open(&NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self {
            app,
            dev,
            log,
            notes,
            plugins,
            _turn: turn,
        }
    }

    fn tap_on(&mut self, label: &str) {
        let (x, y) = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no control {label}"));
        if let Some(r) = self.app.handle(Event::Tap { x, y }) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }

    /// Opens Settings → Logging.
    fn open_logging_page(&mut self) {
        self.tap_on("Options");
        self.tap_on("Logging");
        assert_eq!(
            self.app.settings_page,
            SettingsPage::Category(LOGGING_CATEGORY)
        );
    }

    fn set_level(&mut self, name: &str) {
        self.tap_on("Level");
        self.tap_on(&format!("Level/{name}"));
    }

    fn text(&self) -> String {
        std::fs::read_to_string(self.log.join(FILE_NAME)).unwrap_or_default()
    }

    fn close_settings(&mut self) {
        self.tap_on("Close");
    }

    fn open_files_and_back(&mut self) {
        self.tap_on("Files");
        self.tap_on("Close");
    }

    fn open_graph(&mut self) {
        self.tap_on("Graph");
        self.app.plugin_settle(&mut self.dev);
        self.tap_on("Close");
    }
}

#[test]
fn logging_is_off_by_default_and_writes_nothing() {
    let mut r = Rig::new("off");
    assert!(!r.app.settings.log && r.app.settings.log_level == Level::Info);
    r.open_files_and_back();
    r.open_graph();
    assert!(!r.log.exists(), "no log folder, no file");
    // The page has the switch, but no level row until it is on.
    r.open_logging_page();
    assert!(r.app.control_centre("Logging/On").is_some());
    assert!(r.app.control_centre("Logging/Off").is_some());
    assert!(
        r.app.control_centre("Level").is_none(),
        "the level waits for the switch"
    );
    assert!(!r.log.exists());
}

#[test]
fn switching_logging_on_starts_the_file_and_shows_the_level() {
    let mut r = Rig::new("on");
    r.open_logging_page();
    r.tap_on("Logging/On");
    assert!(r.app.settings.log);
    let text = r.text();
    assert!(text.contains("logging on (level INFO)"), "{text}");
    assert!(text.contains("app: logging level INFO"), "{text}");
    assert!(
        r.app.control_centre("Level").is_some(),
        "now there is a level"
    );
    // The choice is saved in the settings file.
    let saved = std::fs::read_to_string(r.notes.join(".sigma-settings")).unwrap();
    assert!(
        saved.contains("log=on") && saved.contains("log_level=info"),
        "{saved}"
    );
    // Every line has a timestamp, an uptime, a level and a source.
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(5, ' ').collect();
        assert!(parts[2].starts_with('+') && parts[3].len() >= 4, "{line}");
        assert!(
            ["INFO", "DEBUG", "WARNING", "ERROR", "CRITICAL"].contains(&parts[3]),
            "{line}"
        );
    }
}

#[test]
fn the_level_decides_what_the_file_gets() {
    let mut r = Rig::new("levels");
    r.open_logging_page();
    r.tap_on("Logging/On");
    // INFO (the default): the milestones, not the UI chatter.
    r.close_settings();
    r.open_files_and_back();
    assert!(!r.text().contains("ui: open files"), "{}", r.text());
    // DEBUG: that too.
    r.open_logging_page();
    r.set_level("Debug");
    assert_eq!(r.app.settings.log_level, Level::Debug);
    r.close_settings();
    r.open_files_and_back();
    let text = r.text();
    assert!(text.contains("DEBUG    ui: open files"), "{text}");
    assert!(
        text.contains("DEBUG    settings: LogLevel(Debug)"),
        "{text}"
    );
    // ERROR: neither.
    r.open_logging_page();
    r.set_level("Error");
    r.close_settings();
    let before = r.text().len();
    r.open_files_and_back();
    assert_eq!(r.text().len(), before, "nothing new at ERROR");
    // The root page shows the level; "Off" when it is off.
    r.open_logging_page();
    assert_eq!(r.app.settings.log_level, Level::Error);
}

#[test]
fn switching_off_stops_the_file_and_on_again_appends() {
    let mut r = Rig::new("toggle");
    r.open_logging_page();
    r.tap_on("Logging/On");
    r.set_level("Debug");
    r.tap_on("Logging/Off");
    let off = r.text();
    assert!(!r.app.settings.log);
    r.close_settings();
    r.open_files_and_back();
    r.open_graph();
    assert_eq!(r.text(), off, "nothing is written while it is off");
    assert!(
        r.app.control_centre("Level").is_none(),
        "the level row is hidden again"
    );
    r.open_logging_page();
    r.tap_on("Logging/On");
    let text = r.text();
    assert!(text.starts_with(&off), "appended, not replaced");
    // One marker per time the file was opened (the level had changed between).
    assert_eq!(text.matches("logging on (level ").count(), 2, "{text}");
    assert!(text.contains("(level INFO)") && text.contains("(level DEBUG)"));
}

#[test]
fn plugin_lines_are_tagged_with_the_plugin_and_follow_the_level() {
    let mut r = Rig::new("plugin");
    r.open_logging_page();
    r.tap_on("Logging/On");
    r.set_level("Debug");
    r.close_settings();
    r.open_graph();
    let text = r.text();
    // The plugin's own lines, tagged by the host.
    for want in [
        "DEBUG    plugin/graph-map: open: ",
        "plugin/graph-map: layout: building",
        "INFO     plugin/graph-map: layout: built in",
        "DEBUG    plugin/graph-map: layout: step 1,",
    ] {
        assert!(text.contains(want), "{want} in\n{text}");
    }
    // The app's lines about the plugin.
    assert!(
        text.contains("INFO     plugin: graph-map: loaded in"),
        "{text}"
    );
    // At INFO the plugin's DEBUG lines are not written (nor even built).
    r.open_logging_page();
    r.set_level("Info");
    r.close_settings();
    let before = r.text().len();
    // A new library (a note added) builds again, at INFO.
    std::fs::write(r.notes.join("d.md"), "[[a]]").unwrap();
    r.open_graph();
    let new: String = r.text()[before..].to_owned();
    assert!(
        new.contains("INFO     plugin/graph-map: layout: building"),
        "{new}"
    );
    assert!(!new.contains("DEBUG"), "{new}");
}

#[test]
fn a_problem_in_a_plugin_is_logged_as_an_error() {
    let mut r = Rig::new("broken");
    r.open_logging_page();
    r.tap_on("Logging/On");
    r.set_level("Warning");
    r.close_settings();
    // Break the module: it no longer loads, and the app falls back.
    std::fs::write(r.plugins.join("graph-map/graph.wasm"), b"not wasm").unwrap();
    r.app.set_builtin_graph(true);
    r.app.set_builtin_panes(true);
    r.tap_on("Graph");
    r.app.plugin_settle(&mut r.dev);
    let text = r.text();
    assert!(text.contains("ERROR    plugin: graph-map: "), "{text}");
    assert!(text.contains("the built-in Graph opens instead"), "{text}");
}

#[test]
fn a_saved_setting_starts_logging_with_the_app() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("sigma-logging-start-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let (notes, log) = (d.join("notes"), d.join("log"));
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join(".sigma-settings"), "log=on\nlog_level=debug\n").unwrap();
    // The folder comes from the environment, as on the Kobo and in the
    // emulator. No other test runs while this one holds its turn.
    std::env::set_var("SIGMA_LOG_DIR", &log);
    let app = App::new(1440, 1920, notes);
    std::env::remove_var("SIGMA_LOG_DIR");
    assert!(app.settings.log && app.settings.log_level == Level::Debug);
    let text = std::fs::read_to_string(log.join(FILE_NAME)).unwrap();
    assert!(text.contains("logging on (level DEBUG)"), "{text}");
    assert!(text.contains("app: sigma-scribe "), "{text}");
    assert!(text.contains("started: screen 1440x1920"), "{text}");
    sigma_core::log::configure(false, Level::Info, None);
}
