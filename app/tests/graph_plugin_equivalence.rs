//! The Graph as the `graph-map` WASM plugin draws the same pixels, sends the
//! same refresh classes and reaches the same states as the app's built-in
//! Graph. Two apps share one library; one has the plugin installed, the other
//! none. The same steps run on both, and after each the whole screen, the
//! refresh regions and the app state must be identical.
//!
//! The module under test is app/tests/fixtures/graph-map/graph.wasm, built
//! from the sigma-scribe-plugins repo (scripts/update-plugin-fixtures.sh).

use sigma_core::settings::Paper;
use sigma_core::{Change, NotePath};
use sigma_device::desktop::DesktopDevice;
use sigma_device::pointer::{raw_button, raw_move, raw_wheel, PointerDecoder, BTN_LEFT};
use sigma_device::{Event, HwKey, Phase, Refresh};
use sigma_scribe::App;
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graph-map")
}

/// A library with clusters, chains, orphans, a folder, long names, repeated
/// links and links to notes that do not exist.
fn write_library(dir: &Path, count: usize) {
    std::fs::create_dir_all(dir.join("deep/folder")).unwrap();
    let stem = |i: usize| match i % 9 {
        0 => format!("a-really-long-note-name-that-needs-truncating-{i:02}"),
        _ => format!("note-{i:02}"),
    };
    for i in 0..count {
        let mut body = format!("# {}\n", stem(i));
        if i % 4 != 0 {
            body += &format!(
                "[[{}]] [[{}]]\n",
                stem((i * 7 + 3) % count),
                stem((i * 11 + 5) % count)
            );
        }
        if i % 5 == 0 {
            body += "[[missing note]] [[missing note]]\n";
        }
        if i % 6 == 1 {
            body += &format!("[[{}]]\n", stem((i + 1) % count)); // a repeated link
            body += &format!("[[{}]]\n", stem((i + 1) % count));
        }
        let path = if i % 10 == 9 {
            dir.join("deep/folder").join(format!("{}.md", stem(i)))
        } else {
            dir.join(format!("{}.md", stem(i)))
        };
        std::fs::write(path, body).unwrap();
    }
}

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
    dec: PointerDecoder,
}

impl Rig {
    fn new(tag: &str, plugin: bool, w: u32, h: u32, count: usize, night: bool) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-eq-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let (notes, plugins) = (d.join("notes"), d.join("plugins"));
        std::fs::create_dir_all(&notes).unwrap();
        write_library(&notes, count);
        if plugin {
            std::fs::create_dir_all(plugins.join("graph-map")).unwrap();
            for f in ["plugin.conf", "graph.wasm"] {
                std::fs::copy(fixture().join(f), plugins.join("graph-map").join(f)).unwrap();
            }
            std::fs::write(plugins.join("plugins.state"), "graph-map=1.0.0,enabled\n").unwrap();
        }
        let mut app = App::new(w, h, notes);
        app.set_plugins_root(plugins);
        if !plugin {
            // The reference: the app's own Graph, with no plugin installed.
            app.set_builtin_graph(true);
        }
        app.open(&NotePath::parse("note-01.md").unwrap());
        if night {
            app.change(Change::Paper(Paper::Night));
        }
        let mut dev = DesktopDevice::new(w, h);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self {
            app,
            dev,
            t: 0,
            dec: PointerDecoder::default(),
        }
    }

    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }
}

#[derive(Clone, Debug)]
enum Step {
    /// Tap the control with this label (the emulator's controls).
    Tap(&'static str),
    /// Tap the control named by a note, `Node:` or `Graph:` prefixed.
    TapNote(&'static str, usize),
    /// Tap the k-th control of that kind that is on screen, in library order.
    TapVisible(&'static str, usize),
    TapAt(i32, i32),
    PageTurn(bool),
    /// A finger drag from the first point to the second.
    Drag((i32, i32), (i32, i32)),
    Wheel(i32),
    MouseDrag(i32, i32),
    MouseClickOn(&'static str, usize),
    Escape,
}

/// The name of the n-th note in library order, as the app lists it.
fn note_name(r: &Rig, n: usize) -> String {
    let g = r.app.link_graph();
    g.notes[n % g.notes.len()].as_str().to_owned()
}

fn run(r: &mut Rig, step: &Step) -> Option<Refresh> {
    match step {
        Step::Tap(label) => {
            let (x, y) = r
                .app
                .control_centre(label)
                .unwrap_or_else(|| panic!("no control {label}"));
            r.send(Event::Tap { x, y })
        }
        Step::TapNote(prefix, n) => {
            let label = format!("{prefix}{}", note_name(r, *n));
            let (x, y) = r
                .app
                .control_centre(&label)
                .unwrap_or_else(|| panic!("no control {label}"));
            r.send(Event::Tap { x, y })
        }
        Step::TapVisible(prefix, k) => {
            let g = r.app.link_graph();
            let on_screen: Vec<(i32, i32)> = g
                .notes
                .iter()
                .filter_map(|n| r.app.control_centre(&format!("{prefix}{}", n.as_str())))
                .collect();
            assert!(!on_screen.is_empty(), "no {prefix} control on screen");
            let (x, y) = on_screen[k % on_screen.len()];
            r.send(Event::Tap { x, y })
        }
        Step::TapAt(x, y) => r.send(Event::Tap { x: *x, y: *y }),
        Step::PageTurn(f) => r.send(Event::PageTurn { forward: *f }),
        Step::Drag(a, b) => {
            r.t += 30;
            let t = r.t;
            r.send(Event::Touch {
                phase: Phase::Down,
                x: a.0,
                y: a.1,
                t,
            });
            for k in 1..=6 {
                r.t += 30;
                let t = r.t;
                let (x, y) = (a.0 + (b.0 - a.0) * k / 6, a.1 + (b.1 - a.1) * k / 6);
                r.send(Event::Touch {
                    phase: Phase::Move,
                    x,
                    y,
                    t,
                });
            }
            r.t += 30;
            let t = r.t;
            r.send(Event::Touch {
                phase: Phase::Up,
                x: b.0,
                y: b.1,
                t,
            })
        }
        Step::Wheel(n) => feed(r, raw_wheel(*n)),
        Step::MouseDrag(dx, dy) => {
            feed(r, raw_button(BTN_LEFT, true));
            feed(r, raw_move(*dx, *dy));
            feed(r, raw_button(BTN_LEFT, false))
        }
        Step::MouseClickOn(prefix, n) => {
            let label = format!("{prefix}{}", note_name(r, *n));
            let (x, y) = r
                .app
                .control_centre(&label)
                .unwrap_or_else(|| panic!("no control {label}"));
            let (dx, dy) = (x - r.app.pointer.x, y - r.app.pointer.y);
            feed(r, raw_move(dx, dy));
            feed(r, raw_button(BTN_LEFT, true));
            feed(r, raw_button(BTN_LEFT, false))
        }
        Step::Escape => r.send(Event::Key(HwKey::Escape)),
    }
}

fn feed(r: &mut Rig, raws: Vec<sigma_device::evdev::RawEvent>) -> Option<Refresh> {
    let mut last = None;
    for raw in raws {
        r.t += 20;
        let events = r.dec.feed(raw, r.t);
        for e in events {
            last = r.send(e).or(last);
        }
    }
    last
}

/// Runs `steps` on a built-in app and a plugin app and compares them after
/// every step.
fn equivalent(tag: &str, w: u32, h: u32, count: usize, night: bool, steps: &[Step]) {
    equivalent_from(tag, w, h, count, night, 0, steps);
}

/// [`equivalent`], comparing pixels and refresh regions only from step
/// `exact_from` on (step 0 opens the Graph). Large libraries use an
/// approximate map layout, so their map pictures differ by design; the list
/// does not depend on the layout and must still match.
fn equivalent_from(
    tag: &str,
    w: u32,
    h: u32,
    count: usize,
    night: bool,
    exact_from: usize,
    steps: &[Step],
) {
    let mut builtin = Rig::new(&format!("{tag}-b"), false, w, h, count, night);
    let mut plugin = Rig::new(&format!("{tag}-p"), true, w, h, count, night);
    for r in [&mut builtin, &mut plugin] {
        r.send(Event::PointerPlug(true));
    }
    let open = Step::Tap("Graph");
    for (n, step) in std::iter::once(&open).chain(steps).enumerate() {
        let (rb, rp) = (run(&mut builtin, step), run(&mut plugin, step));
        if n == 0 {
            // The plugin loads and builds on idle ticks (a loading picture,
            // maybe a progress bar); the built-in Graph is there at once.
            // Compare once it has finished, with the refreshes starting over.
            plugin.app.plugin_settle(&mut plugin.dev);
            builtin.dev.refreshes.clear();
            plugin.dev.refreshes.clear();
        }
        let ctx = format!("{tag}: step {n} {step:?}");
        if n == 0 {
            assert!(builtin.app.graph.is_some(), "{ctx}: built-in Graph open");
            assert!(
                plugin.app.plugin_screen_open(),
                "{ctx}: the plugin is what opened"
            );
        }
        let exact = n >= exact_from;
        if exact && n > 0 {
            assert_eq!(rb, rp, "{ctx}: refresh class");
            assert_eq!(
                builtin.dev.refreshes, plugin.dev.refreshes,
                "{ctx}: refresh regions"
            );
        }
        assert_eq!(
            builtin.app.current, plugin.app.current,
            "{ctx}: current note"
        );
        assert_eq!(
            builtin.app.preview.is_some(),
            plugin.app.preview.is_some(),
            "{ctx}: preview"
        );
        assert_eq!(
            builtin.app.graph.is_some(),
            plugin.app.plugin_screen_open(),
            "{ctx}: the Graph is open in both or neither"
        );
        assert_eq!(
            builtin.app.history.len(),
            plugin.app.history.len(),
            "{ctx}: history"
        );
        let (cb, cp) = (&builtin.app.canvas, &plugin.app.canvas);
        if exact && cb.pixels != cp.pixels {
            let differing = cb
                .pixels
                .iter()
                .zip(&cp.pixels)
                .filter(|(a, b)| a != b)
                .count();
            panic!("{ctx}: {differing} pixels differ, within {:?}", cb.diff(cp));
        }
    }
}

use Step::*;

const FORMA: (u32, u32) = (1440, 1920);

#[test]
fn map_zoom_select_pan_and_deselect() {
    equivalent(
        "map",
        FORMA.0,
        FORMA.1,
        40,
        false,
        &[
            Tap("Zoom+"),
            Tap("Zoom+"),
            Tap("Zoom+"),
            Tap("Zoom+"),
            Tap("Zoom-"),
            PageTurn(true),
            PageTurn(false),
            Drag((900, 900), (500, 700)),
            Drag((300, 1200), (800, 1100)),
            Tap("Zoom-"),
            Tap("Zoom-"),
            Tap("Zoom-"),
            TapNote("Node:", 12),
            TapNote("Node:", 3),
            TapAt(720, 1700),
            TapNote("Node:", 7),
            Tap("Open"),
        ],
    );
}

#[test]
fn list_paging_toggle_and_open_a_row() {
    equivalent(
        "list",
        FORMA.0,
        FORMA.1,
        40,
        false,
        &[
            Tap("List"),
            Tap("List"),
            PageTurn(true),
            PageTurn(true),
            PageTurn(false),
            Wheel(-1),
            Wheel(1),
            Tap("Map"),
            Tap("List"),
            TapVisible("Graph:", 2),
        ],
    );
}

#[test]
fn select_then_open_in_preview_and_escape_back() {
    equivalent(
        "preview",
        FORMA.0,
        FORMA.1,
        40,
        false,
        &[
            TapNote("Node:", 8),
            TapNote("Node:", 8),
            Escape,
            Tap("Graph"),
            Escape,
        ],
    );
}

#[test]
fn mouse_wheel_click_and_drag() {
    equivalent(
        "mouse",
        FORMA.0,
        FORMA.1,
        40,
        false,
        &[
            Wheel(1),
            Wheel(1),
            Wheel(-1),
            MouseClickOn("Node:", 9),
            MouseDrag(-150, 40),
            MouseClickOn("Node:", 9),
        ],
    );
}

#[test]
fn night_theme_and_another_screen_size() {
    let steps = [
        Tap("Zoom+"),
        TapNote("Node:", 4),
        Tap("List"),
        PageTurn(true),
        Tap("Map"),
    ];
    equivalent("night", FORMA.0, FORMA.1, 40, true, &steps);
    equivalent("small", 1072, 1448, 40, false, &steps);
    equivalent("wide", 1264, 1680, 40, false, &steps);
}

#[test]
fn small_and_busy_libraries() {
    // One note, a handful, and enough notes for dots instead of labels.
    equivalent(
        "one",
        FORMA.0,
        FORMA.1,
        1,
        false,
        &[Tap("List"), Tap("Map")],
    );
    equivalent(
        "few",
        FORMA.0,
        FORMA.1,
        5,
        false,
        &[Tap("Zoom+"), TapNote("Node:", 2)],
    );
    equivalent(
        "busy",
        FORMA.0,
        FORMA.1,
        150,
        false,
        &[
            Tap("Zoom+"),
            Tap("Zoom+"),
            Tap("Zoom+"),
            TapVisible("Node:", 5),
            Tap("List"),
            PageTurn(true),
        ],
    );
}

#[test]
fn large_libraries_use_the_approximate_layout_but_the_list_still_matches() {
    // 320 notes, more than 150 of them linked: the map is laid out by the
    // quadtree, so its pictures differ by design. Everything else matches.
    let steps = [
        Tap("List"),
        PageTurn(true),
        PageTurn(true),
        PageTurn(false),
        Wheel(-1),
        TapVisible("Graph:", 3),
    ];
    equivalent_from("large", FORMA.0, FORMA.1, 320, false, 2, &steps);
}
