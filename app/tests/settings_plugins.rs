//! Settings → Plugins: the installed plugins listed with a switch each, the
//! switch writing `plugins.state`, and what it does to the Graph screen.

use sigma_core::plugins;
use sigma_core::settings::{Change, Orientation};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_render::i18n::{self, Lang};
use sigma_render::options::PLUGINS_CATEGORY;
use sigma_render::SettingsPage;
use sigma_scribe::App;
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph-map")
}

struct Rig {
    app: App,
    dev: DesktopDevice,
    plugins: PathBuf,
}

fn conf(dir: &Path, id: &str, name: &str, kind: &str, extra: &str) {
    std::fs::create_dir_all(dir.join(id)).unwrap();
    std::fs::write(
        dir.join(id).join("plugin.conf"),
        format!("id={id}\nname={name}\nversion=1.2.0\nkind={kind}\n{extra}"),
    )
    .unwrap();
}

impl Rig {
    /// A library with a few linked notes and a plugins folder holding the real
    /// graph-map screen plugin (on), a font pack (off) and a plugin that
    /// needs a newer app.
    fn new(tag: &str, with_plugins: bool) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-setplug-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (notes, plugins) = (d.join("notes"), d.join("plugins"));
        std::fs::create_dir_all(&notes).unwrap();
        for (n, body) in [
            ("a.md", "[[b]] [[c]]"),
            ("b.md", "[[c]]"),
            ("c.md", "[[a]]"),
        ] {
            std::fs::write(notes.join(n), body).unwrap();
        }
        std::fs::create_dir_all(&plugins).unwrap();
        if with_plugins {
            conf(
                &plugins,
                "graph-map",
                "Graph",
                "screen",
                "entry=graph.wasm\n",
            );
            std::fs::copy(
                fixture().join("graph.wasm"),
                plugins.join("graph-map/graph.wasm"),
            )
            .unwrap();
            conf(
                &plugins,
                "cjk-font",
                "A very long plugin name that must stay clear of the switch",
                "font",
                "",
            );
            conf(
                &plugins,
                "future",
                "Future theme",
                "theme",
                "min_app=99.0.0\n",
            );
            std::fs::write(plugins.join("plugins.state"), "graph-map=1.0.0,enabled\n").unwrap();
        }
        let mut app = App::new(1440, 1920, notes);
        app.set_plugins_root(plugins.clone());
        app.open(&sigma_core::NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev, plugins }
    }

    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }

    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no control {label}"));
        self.send(Event::Tap { x, y })
    }

    fn open_plugins_page(&mut self) {
        self.tap_on("Options");
        self.tap_on("Plugins");
        assert_eq!(
            self.app.settings_page,
            SettingsPage::Category(PLUGINS_CATEGORY)
        );
    }

    fn state(&self) -> String {
        std::fs::read_to_string(self.plugins.join("plugins.state")).unwrap_or_default()
    }
}

#[test]
fn the_plugins_category_is_on_the_settings_root() {
    let mut r = Rig::new("root", true);
    r.tap_on("Options");
    assert!(r.app.control_centre("Plugins").is_some());
    assert_eq!(
        sigma_render::options::CATEGORIES[PLUGINS_CATEGORY].0,
        "Plugins"
    );
    // Help & about is still the last category and still works.
    r.tap_on("Help & about");
    r.tap_on("App info");
    assert!(!r.app.options && r.app.help.is_some());
}

#[test]
fn installed_plugins_are_listed_and_switchable() {
    let mut r = Rig::new("list", true);
    r.open_plugins_page();
    // On: the state already set names the no-op, the other flips it.
    assert!(r.app.control_centre("Graph/On").is_some());
    assert!(r.app.control_centre("Graph/Off").is_some());
    assert!(r
        .app
        .control_centre("A very long plugin name that must stay clear of the switch/Off")
        .is_some());
    // A plugin that needs a newer app is shown but has no switch.
    assert!(r.app.control_centre("Future theme/On").is_none());
    assert!(r.app.control_centre("Future theme/Off").is_none());
}

#[test]
fn switching_the_graph_plugin_off_and_on_changes_what_the_graph_icon_opens() {
    let mut r = Rig::new("toggle", true);
    // On: the Graph icon opens the plugin.
    r.tap_on("Graph");
    assert!(r.app.plugin_screen_open());
    r.tap_on("Close");
    // Off in Settings.
    r.open_plugins_page();
    assert_eq!(r.tap_on("Graph/Off"), Some(Refresh::Full));
    assert!(
        r.state().contains("graph-map=1.0.0,disabled"),
        "{}",
        r.state()
    );
    assert!(r.app.control_centre("Graph/On").is_some());
    r.tap_on("Close");
    r.tap_on("Graph");
    assert!(
        r.app.graph.is_some() && !r.app.plugin_screen_open(),
        "the built-in Graph"
    );
    r.tap_on("Close");
    // On again.
    r.open_plugins_page();
    r.tap_on("Graph/On");
    assert!(
        r.state().contains("graph-map=1.0.0,enabled"),
        "{}",
        r.state()
    );
    r.tap_on("Close");
    r.tap_on("Graph");
    assert!(r.app.plugin_screen_open(), "the plugin again");
}

#[test]
fn a_plugin_not_in_the_file_is_added_when_switched_on() {
    let mut r = Rig::new("add", true);
    r.open_plugins_page();
    let name = "A very long plugin name that must stay clear of the switch";
    r.tap_on(&format!("{name}/On"));
    let state = plugins::State::parse(&r.state());
    assert!(state.get("cjk-font").unwrap().enabled);
    assert!(state.get("graph-map").unwrap().enabled, "others untouched");
    assert!(plugins::enabled_files_in(&r.plugins, "fonts/cjk.ttf").is_empty());
}

#[test]
fn the_choice_survives_a_restart() {
    let mut r = Rig::new("persist", true);
    r.open_plugins_page();
    r.tap_on("Graph/Off");
    let mut again = App::new(1440, 1920, r.plugins.parent().unwrap().join("notes"));
    again.set_plugins_root(r.plugins.clone());
    let list = plugins::list_in(&r.plugins);
    assert!(
        !list
            .iter()
            .find(|l| l.manifest.id == "graph-map")
            .unwrap()
            .enabled
    );
    again.open(&sigma_core::NotePath::parse("a.md").unwrap());
    let mut dev = DesktopDevice::new(1440, 1920);
    again.render(&mut dev, Refresh::Full).unwrap();
    let (x, y) = again.control_centre("Graph").unwrap();
    again.handle(Event::Tap { x, y });
    assert!(again.graph.is_some() && !again.plugin_screen_open());
}

#[test]
fn no_plugins_installed_says_so_and_has_nothing_to_switch() {
    let mut r = Rig::new("empty", false);
    r.open_plugins_page();
    assert!(r.app.control_centre("Graph/On").is_none());
    assert!(r
        .app
        .touch_targets()
        .iter()
        .all(|(n, _)| !n.contains("/On") && !n.contains("/Off")));
    // The built-in Graph still works with no plugins at all.
    r.tap_on("Close");
    r.tap_on("Graph");
    assert!(r.app.graph.is_some());
}

#[test]
fn a_plugin_removed_while_settings_is_open_does_not_break_the_switch() {
    let mut r = Rig::new("gone", true);
    r.open_plugins_page();
    std::fs::remove_dir_all(r.plugins.join("graph-map")).unwrap();
    let before = r.state();
    r.tap_on("Graph/Off");
    assert_eq!(
        r.state(),
        before,
        "nothing to write for a plugin that is gone"
    );
    assert!(
        r.app.control_centre("Graph/On").is_none() && r.app.control_centre("Graph/Off").is_none()
    );
}

#[test]
fn a_switch_acts_on_the_plugin_the_page_showed_even_if_others_vanish() {
    let mut r = Rig::new("shift", true);
    r.open_plugins_page();
    // The list the page showed: font pack, Future theme, Graph. Remove the
    // first, which moves Graph up the folder's own listing.
    std::fs::remove_dir_all(r.plugins.join("cjk-font")).unwrap();
    r.tap_on("Graph/Off");
    let state = plugins::State::parse(&r.state());
    assert!(!state.get("graph-map").unwrap().enabled, "{}", r.state());
    assert!(state.get("future").is_none(), "no other plugin was touched");
}

#[test]
fn back_and_escape_walk_up_from_the_plugins_page() {
    let mut r = Rig::new("nav", true);
    r.open_plugins_page();
    r.send(Event::Key(HwKey::Escape));
    assert_eq!(r.app.settings_page, SettingsPage::Root);
    r.send(Event::Key(HwKey::Escape));
    assert!(!r.app.options);
}

#[test]
fn the_plugins_page_fits_in_each_language_and_orientation() {
    for l in [Lang::En, Lang::Fr, Lang::Pt, Lang::PseudoLong] {
        let mut r = Rig::new(&format!("fit{l:?}"), true);
        r.app.set_language(l);
        for o in [Orientation::Portrait, Orientation::Landscape] {
            r.app.change(Change::Orientation(o));
            r.app.open_options();
            for p in [SettingsPage::Root, SettingsPage::Category(PLUGINS_CATEGORY)] {
                r.app.settings_page = p;
                r.app.settings_sub = 0;
                loop {
                    r.app.render(&mut r.dev, Refresh::Full).unwrap();
                    let (w, h) = (r.app.canvas.width as i32, r.app.canvas.height as i32);
                    let min = r.app.min_target();
                    for (n, t) in r.app.touch_targets() {
                        assert!(t.w >= min && t.h >= min, "{l:?} {o:?} {p:?}: {n} {t:?}");
                        assert!(
                            t.x >= 0 && t.y >= 0 && t.x + t.w <= w && t.y + t.h <= h,
                            "{l:?} {o:?} {p:?}: {n} {t:?}"
                        );
                    }
                    let before = r.app.settings_sub;
                    r.app.handle(Event::PageTurn { forward: true });
                    if r.app.settings_sub == before {
                        break;
                    }
                }
            }
        }
    }
    i18n::set_lang(Lang::En);
}
