//! The side-drawer panes as WASM pane plugins: the icon gate (a pane's icon
//! is in the title bar only while its plugin is on), switching a plugin off
//! closes its pane, and a plugin that does not load or traps is dropped for
//! the session in favour of the built-in pane.
//!
//! What the plugins draw and do is compared with the built-in panes in
//! pane_plugin_equivalence.rs; these tests cover the host's side.

use sigma_core::NotePath;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_plugin_api::Frame;
use sigma_render::{options::PLUGINS_CATEGORY, SettingsPage};
use sigma_scribe::App;
use std::path::{Path, PathBuf};

const PANES: [(&str, &str); 3] = [
    ("contents-pane", "Contents"),
    ("todo-pane", "To-dos"),
    ("scratch-pane", "Scratch"),
];

fn fixture(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(id)
}

struct Rig {
    app: App,
    dev: DesktopDevice,
    plugins: PathBuf,
}

impl Rig {
    /// `module`: replaces the module of the plugin named, e.g. a broken one.
    fn new(tag: &str, install: bool, module: Option<(&str, Vec<u8>)>) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-pane-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (notes, plugins) = (d.join("notes"), d.join("plugins"));
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(
            notes.join("a.md"),
            "# Plan\n- [ ] first\n- [x] second\n## Later\n",
        )
        .unwrap();
        let mut state = String::new();
        if install {
            for (id, _) in PANES {
                std::fs::create_dir_all(plugins.join(id)).unwrap();
                for f in ["plugin.conf", "pane.wasm"] {
                    std::fs::copy(fixture(id).join(f), plugins.join(id).join(f)).unwrap();
                }
                state += &format!("{id}=1.0.0,enabled\n");
            }
        }
        if let Some((id, wasm)) = module {
            std::fs::write(plugins.join(id).join("pane.wasm"), wasm).unwrap();
        }
        std::fs::write(plugins.join("plugins.state"), state).unwrap();
        let mut app = App::new(1440, 1920, notes);
        app.set_plugins_root(plugins.clone());
        app.open(&NotePath::parse("a.md").unwrap());
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
fn with_the_pane_plugins_installed_each_pane_has_its_icon_and_is_drawn_by_its_plugin() {
    let mut r = Rig::new("on", true, None);
    for (_, icon) in PANES {
        assert!(r.app.control_centre(icon).is_some(), "{icon}");
    }
    r.tap_on("Contents");
    r.tap_on("To-dos");
    assert!(r.app.drawer[0] && r.app.drawer[1]);
    // The plugins name their rows for the emulator.
    assert!(r.app.control_centre("Heading:Plan").is_some());
    assert!(r.app.control_centre("TodoBox:first").is_some());
    assert!(r.app.control_centre("Filter").is_some());
}

#[test]
fn without_the_plugins_the_pane_icons_are_gone_and_the_rest_close_up() {
    let mut r = Rig::new("none", false, None);
    for (_, icon) in PANES {
        assert!(r.app.control_centre(icon).is_none(), "{icon}");
        assert!(r.app.touch_targets().iter().all(|(n, _)| n != icon));
    }
    for icon in ["Files", "Preview", "Find", "Options"] {
        assert!(r.app.control_centre(icon).is_some(), "{icon}");
    }
    // Nothing opens by accident: a tap where an icon used to be is not a pane.
    let (x, y) = r.app.control_centre("Find").unwrap();
    r.send(Event::Tap { x: x - 100, y });
    assert!(r.app.drawer.iter().all(|&o| !o));
}

#[test]
fn switching_a_pane_plugin_off_removes_its_icon_and_closes_its_pane() {
    let mut r = Rig::new("toggle", true, None);
    r.tap_on("To-dos");
    r.tap_on("Scratch");
    assert!(r.app.drawer[1] && r.app.drawer[2] && r.app.scratch_focus);
    r.open_plugins_page();
    r.tap_on("To-dos/Off");
    assert!(
        r.state().contains("todo-pane=1.0.0,disabled"),
        "{}",
        r.state()
    );
    r.tap_on("Scratch/Off");
    r.tap_on("Close");
    assert!(r.app.control_centre("To-dos").is_none());
    assert!(r.app.control_centre("Scratch").is_none());
    assert!(
        r.app.control_centre("Contents").is_some(),
        "the other stays"
    );
    assert!(!r.app.drawer[1] && !r.app.drawer[2], "their panes closed");
    assert!(!r.app.scratch_focus, "the keyboard is the page's again");
    // On again: the icon is back.
    r.open_plugins_page();
    r.tap_on("To-dos/On");
    r.tap_on("Close");
    r.tap_on("To-dos");
    assert!(r.app.drawer[1]);
    assert!(r.app.control_centre("TodoBox:first").is_some());
}

#[test]
fn a_pane_plugin_that_does_not_load_falls_back_to_the_builtin_pane() {
    let mut r = Rig::new("broken", true, Some(("todo-pane", b"not wasm".to_vec())));
    r.tap_on("To-dos");
    assert!(r.app.drawer[1], "the pane opened");
    // The built-in pane answers the same controls.
    assert!(r.app.control_centre("Filter").is_some());
    assert!(r.app.control_centre("TodoBox:first").is_some());
    r.tap_on("TodoBox:first");
    assert!(
        r.app.buffer.text().contains("- [x] first"),
        "the built-in pane worked"
    );
}

/// A module that answers `Open` with an empty frame and traps on a tap.
fn trapping_guest() -> Vec<u8> {
    let frame: String = Frame::default()
        .encode()
        .iter()
        .map(|b| format!("\\{b:02x}"))
        .collect();
    let len = Frame::default().encode().len();
    wat::parse_str(format!(
        r#"(module
  (memory (export "memory") 4)
  (func (export "sigma_abi") (result i32) i32.const {abi})
  (func (export "sigma_alloc") (param i32) (result i32) i32.const 16384)
  (data (i32.const 8192) "{frame}")
  (func (export "sigma_call") (param $p i32) (param $l i32) (result i64)
    (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 1)) (then unreachable))
    (i64.or (i64.shl (i64.const 8192) (i64.const 32)) (i64.const {len}))))"#,
        abi = sigma_plugin_api::ABI
    ))
    .unwrap()
}

#[test]
fn a_pane_plugin_that_traps_is_dropped_for_the_session() {
    let mut r = Rig::new("trap", true, Some(("contents-pane", trapping_guest())));
    r.tap_on("Contents");
    // The plugin loaded and drew an empty pane: no rows to tap.
    assert!(r.app.control_centre("Heading:Plan").is_none());
    let base = r.app.screen.layout(false, &r.app.keyboard);
    let d = r.app.screen.drawer_rect(&base, r.app.settings.drawer);
    r.send(Event::Tap {
        x: d.x + d.w / 2,
        y: d.y + d.h / 2,
    });
    // It trapped; the built-in Contents pane draws instead, from then on.
    assert!(r.app.drawer[0], "the pane stays open");
    assert!(r.app.control_centre("Heading:Plan").is_some());
    r.tap_on("Contents");
    r.tap_on("Contents");
    assert!(
        r.app.control_centre("Heading:Plan").is_some(),
        "and stays built-in"
    );
}
