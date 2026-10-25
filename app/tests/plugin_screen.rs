//! The Graph as a WASM screen plugin: the gate, the events the plugin
//! receives, the actions it can take, host-owned controls, and the fall
//! back to the built-in Graph when a plugin is missing or fails.
//!
//! The plugin here is a tiny WAT module that answers each event with a
//! fixed frame and records every event it receives in its cache, so the
//! tests can decode exactly what the host sent.

use sigma_core::NotePath;
use sigma_device::desktop::DesktopDevice;
use sigma_device::pointer::{raw_wheel, PointerDecoder};
use sigma_device::{Event, Phase, Refresh};
use sigma_plugin_api::{
    Action, Event as PEvent, Frame, Hit, Op, Phase as PPhase, Rect, Redraw, Tone,
};
use sigma_scribe::App;
use std::path::PathBuf;

const MEM: &str = r#"(memory (export "memory") 4)
  (func (export "sigma_abi") (result i32) i32.const 1)
  (func (export "sigma_alloc") (param i32) (result i32) i32.const 16384)"#;

fn esc(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("\\{b:02x}")).collect()
}

/// A plugin that answers event tag k (0 open, 1 tap, 2 drag, 3 wheel,
/// 4 page, 5 back) with `frames[k]`, and saves every event it gets.
fn guest(frames: &[Frame; 6], trap_on_wheel: bool) -> Vec<u8> {
    let mut data = String::new();
    let mut lens = Vec::new();
    for (k, f) in frames.iter().enumerate() {
        let b = f.encode();
        assert!(b.len() < 1024);
        data += &format!(r#"(data (i32.const {}) "{}")"#, 8192 + 1024 * k, esc(&b));
        lens.extend_from_slice(&(b.len() as u32).to_le_bytes());
    }
    data += &format!(r#"(data (i32.const 4096) "{}")"#, esc(&lens));
    let trap = if trap_on_wheel {
        "(if (i32.eq (local.get $tag) (i32.const 3)) (then unreachable))"
    } else {
        ""
    };
    let wat = format!(
        r#"(module
  (import "sigma" "cache_put" (func $put (param i64 i32 i32) (result i32)))
  {MEM}
  {data}
  (global $n (mut i32) (i32.const 0))
  (func (export "sigma_call") (param $p i32) (param $l i32) (result i64)
    (local $tag i32)
    (local.set $tag (i32.load8_u (local.get $p)))
    (drop (call $put (i64.extend_i32_u (i32.add (i32.const 1000) (global.get $n)))
                     (local.get $p) (local.get $l)))
    (global.set $n (i32.add (global.get $n) (i32.const 1)))
    {trap}
    (i64.or
      (i64.shl (i64.extend_i32_u (i32.add (i32.const 8192) (i32.shl (local.get $tag) (i32.const 10))))
               (i64.const 32))
      (i64.extend_i32_u (i32.load (i32.add (i32.const 4096) (i32.shl (local.get $tag) (i32.const 2))))))))"#
    );
    wat::parse_str(wat).unwrap()
}

fn redraw(r: Redraw) -> Frame {
    Frame {
        redraw: r,
        ..Frame::default()
    }
}

fn open_frame() -> Frame {
    Frame {
        ops: vec![
            Op::Fill {
                r: Rect::new(0, 0, 1440, 1920),
                c: Tone::Rule,
            },
            Op::Pill {
                r: Rect::new(200, 400, 60, 60),
                c: Tone::Ink,
            },
        ],
        hits: vec![
            Hit {
                id: 1,
                rect: Rect::new(200, 400, 60, 60),
                label: "Node:b.md".into(),
            },
            Hit {
                id: 2,
                rect: Rect::new(1000, 300, 80, 56),
                label: "Zoom+".into(),
            },
        ],
        ..Frame::default()
    }
}

fn frames(tap: Vec<Action>) -> [Frame; 6] {
    [
        open_frame(),
        Frame {
            actions: tap,
            ..Frame::default()
        },
        redraw(Redraw::Partial),
        redraw(Redraw::Full),
        redraw(Redraw::Clean),
        Frame {
            actions: vec![Action::Close],
            ..Frame::default()
        },
    ]
}

struct Rig {
    app: App,
    dev: DesktopDevice,
    plugins: PathBuf,
    t: u64,
}

fn dirs(tag: &str) -> (PathBuf, PathBuf) {
    let d = std::env::temp_dir().join(format!("sigma-plug-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("notes")).unwrap();
    std::fs::create_dir_all(d.join("plugins/graph-map")).unwrap();
    (d.join("notes"), d.join("plugins"))
}

impl Rig {
    /// `wasm`: the plugin module, or `None` for no plugin installed.
    fn new(tag: &str, wasm: Option<Vec<u8>>, state: &str) -> Self {
        let (notes, plugins) = dirs(tag);
        for (n, body) in [
            ("a.md", "[[b]] [[c]]"),
            ("b.md", "[[c]]"),
            ("c.md", "[[a]]"),
            ("lone.md", "no links"),
        ] {
            std::fs::write(notes.join(n), body).unwrap();
        }
        if let Some(w) = wasm {
            std::fs::write(plugins.join("graph-map/graph.wasm"), w).unwrap();
            std::fs::write(
                plugins.join("graph-map/plugin.conf"),
                "id=graph-map\nname=Graph\nversion=1.0.0\nkind=screen\nentry=graph.wasm\n",
            )
            .unwrap();
            std::fs::write(plugins.join("plugins.state"), state).unwrap();
        }
        let mut app = App::new(1440, 1920, notes);
        app.set_plugins_root(plugins.clone());
        app.open(&NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self {
            app,
            dev,
            plugins,
            t: 0,
        }
    }
    fn on(tag: &str, frames: [Frame; 6]) -> Self {
        Self::new(
            tag,
            Some(guest(&frames, false)),
            "graph-map=1.0.0,enabled\n",
        )
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.app.control_centre(label).expect(label);
        self.send(Event::Tap { x, y })
    }
    fn touch(&mut self, phase: Phase, x: i32, y: i32) {
        self.t += 30;
        let t = self.t;
        self.send(Event::Touch { phase, x, y, t });
    }
    /// Every event the plugin received, in order.
    fn received(&self) -> Vec<PEvent> {
        (0..16u64)
            .map_while(|n| {
                std::fs::read(
                    self.plugins
                        .join(format!("graph-map/.cache/{:016x}", 1000 + n)),
                )
                .ok()
            })
            .map(|b| PEvent::decode(&b).unwrap())
            .collect()
    }
    fn note_index(&self, name: &str) -> u32 {
        let g = self.app.link_graph();
        g.notes.iter().position(|n| n.as_str() == name).unwrap() as u32
    }
}

#[test]
fn the_graph_icon_opens_the_plugin_and_gives_it_the_library() {
    let mut r = Rig::on("open", frames(vec![]));
    r.tap_on("Graph");
    assert!(r.app.plugin_screen_open());
    assert!(r.app.graph.is_none(), "the built-in Graph stays closed");
    let PEvent::Open(s) = &r.received()[0] else {
        panic!("first event is Open")
    };
    assert_eq!((s.width, s.height), (1440, 1920));
    assert!(s.top > 0 && s.bottom == 1920);
    let idx = |n: &str| s.notes.iter().position(|x| x.name == n).unwrap() as u32;
    assert!(s.notes[idx("a.md") as usize].current);
    assert!(!s.notes[idx("b.md") as usize].current);
    assert!(s
        .notes
        .iter()
        .any(|n| n.name == "lone.md" && n.detail == "No links"));
    assert!(s.edges.contains(&(idx("a.md"), idx("b.md"))));
    assert!(s.edges.contains(&(idx("a.md"), idx("c.md"))));
    assert!(s.edges.contains(&(idx("c.md"), idx("a.md"))));
    assert!(!s.notes.iter().any(|n| n.name.contains(".scratchpad")));
}

#[test]
fn the_plugins_hit_list_names_emulator_controls() {
    let mut r = Rig::on("hits", frames(vec![]));
    r.tap_on("Graph");
    assert_eq!(r.app.control_centre("Node:b.md"), Some((230, 430)));
    assert_eq!(r.app.control_centre("Zoom+"), Some((1040, 328)));
    assert!(r.app.control_centre("Close").is_some());
    assert!(r.app.control_centre("Nope").is_none());
}

#[test]
fn a_tap_reaches_the_plugin_and_open_note_applies() {
    let b = Rig::new("tap", None, "").note_index("b.md");
    let mut r = Rig::on("tap2", frames(vec![Action::OpenNote(b)]));
    assert_eq!(r.note_index("b.md"), b, "same library, same order");
    r.tap_on("Graph");
    assert_eq!(r.tap_on("Node:b.md"), Some(Refresh::Full));
    assert!(!r.app.plugin_screen_open() && r.app.graph.is_none());
    assert_eq!(r.app.current.as_str(), "b.md");
    assert!(r.app.preview.is_none());
    assert!(!r.app.history.is_empty(), "Back returns to a.md");
    let got = r.received();
    assert!(
        matches!(got[1], PEvent::Tap { x: 230, y: 430 }),
        "{:?}",
        got[1]
    );
}

#[test]
fn preview_note_opens_the_note_in_preview() {
    let b = Rig::new("pv0", None, "").note_index("b.md");
    let mut r = Rig::on("pv", frames(vec![Action::PreviewNote(b)]));
    r.tap_on("Graph");
    assert_eq!(r.tap_on("Node:b.md"), Some(Refresh::Full));
    assert!(r.app.preview.is_some() && !r.app.plugin_screen_open());
    assert_eq!(r.app.current.as_str(), "b.md");
}

#[test]
fn close_and_escape_belong_to_the_host() {
    let mut r = Rig::on("close", frames(vec![Action::OpenNote(0)]));
    r.tap_on("Graph");
    assert_eq!(r.tap_on("Close"), Some(Refresh::Full));
    assert!(!r.app.plugin_screen_open());
    assert_eq!(r.app.current.as_str(), "a.md");
    assert!(
        !r.received().iter().any(|e| matches!(e, PEvent::Tap { .. })),
        "the plugin never saw the close tap"
    );
    r.tap_on("Graph");
    assert!(r.app.plugin_screen_open());
    assert_eq!(
        r.send(Event::Key(sigma_device::HwKey::Escape)),
        Some(Refresh::Full)
    );
    assert!(!r.app.plugin_screen_open());
}

#[test]
fn the_plugin_can_close_itself() {
    let mut f = frames(vec![]);
    f[1].actions = vec![Action::Close];
    let mut r = Rig::on("self-close", f);
    r.tap_on("Graph");
    assert_eq!(r.tap_on("Zoom+"), Some(Refresh::Full));
    assert!(!r.app.plugin_screen_open());
}

#[test]
fn wheel_page_and_finger_drag_reach_the_plugin() {
    let mut r = Rig::on("input", frames(vec![]));
    r.send(Event::PointerPlug(true));
    r.tap_on("Graph");
    // Page buttons: the plugin's Clean redraw becomes the cleanup refresh.
    assert_eq!(
        r.send(Event::PageTurn { forward: true }),
        Some(Refresh::PointerCleanup)
    );
    // Mouse wheel.
    let mut dec = PointerDecoder::default();
    let mut full = false;
    for raw in raw_wheel(1) {
        r.t += 20;
        for e in dec.feed(raw, r.t) {
            full |= r.send(e) == Some(Refresh::Full);
        }
    }
    assert!(full, "the wheel got the plugin's Full redraw");
    // A finger drag: Start at the press point, End at the release.
    r.touch(Phase::Down, 700, 1000);
    for k in 1..=6 {
        r.touch(Phase::Move, 700 - 50 * k, 1000);
    }
    r.touch(Phase::Up, 400, 1000);
    assert!(r.app.plugin_screen_open(), "a drag never opens anything");
    let got = r.received();
    let drags: Vec<_> = got
        .iter()
        .filter_map(|e| match e {
            PEvent::Drag { phase, x, y } => Some((*phase, *x, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(
        drags.first(),
        Some(&(PPhase::Start, 700, 1000)),
        "{drags:?}"
    );
    assert_eq!(drags.last(), Some(&(PPhase::End, 400, 1000)), "{drags:?}");
    assert!(drags.iter().any(|d| d.0 == PPhase::Move));
    assert!(got
        .iter()
        .any(|e| matches!(e, PEvent::PageTurn { forward: true })));
    assert!(got.iter().any(|e| matches!(e, PEvent::Wheel { dy: 1, .. })));
    assert!(
        !got.iter().any(|e| matches!(e, PEvent::Tap { .. })),
        "{got:?}"
    );
}

#[test]
fn without_an_enabled_plugin_the_builtin_graph_opens() {
    let mut r = Rig::new("none", None, "");
    r.tap_on("Graph");
    assert!(r.app.graph.is_some() && !r.app.plugin_screen_open());

    let w = guest(&frames(vec![]), false);
    let mut r = Rig::new("off", Some(w), "graph-map=1.0.0,disabled\n");
    r.tap_on("Graph");
    assert!(r.app.graph.is_some() && !r.app.plugin_screen_open());
}

#[test]
fn a_plugin_that_does_not_load_falls_back_to_the_builtin_graph() {
    let mut r = Rig::new(
        "bad",
        Some(b"not wasm".to_vec()),
        "graph-map=1.0.0,enabled\n",
    );
    r.tap_on("Graph");
    assert!(r.app.graph.is_some() && !r.app.plugin_screen_open());
}

#[test]
fn a_plugin_that_traps_is_dropped_for_the_session() {
    let w = guest(&frames(vec![]), true);
    let mut r = Rig::new("trap", Some(w), "graph-map=1.0.0,enabled\n");
    r.send(Event::PointerPlug(true));
    r.tap_on("Graph");
    assert!(r.app.plugin_screen_open());
    let mut dec = PointerDecoder::default();
    for raw in raw_wheel(1) {
        r.t += 20;
        for e in dec.feed(raw, r.t) {
            r.send(e);
        }
    }
    assert!(!r.app.plugin_screen_open(), "the failed plugin is gone");
    assert!(r.app.graph.is_some(), "the built-in Graph took over");
    // It stays off until the plugin is set up again.
    r.send(Event::Key(sigma_device::HwKey::Escape));
    r.tap_on("Graph");
    assert!(r.app.graph.is_some() && !r.app.plugin_screen_open());
}
