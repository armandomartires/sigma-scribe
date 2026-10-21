//! v1.0 (S9.1) Graph map: layout cache, select / open in preview, toggle,
//! zoom, pan (finger, page buttons, mouse), refresh classes and sizes.

use sigma_device::desktop::DesktopDevice;
use sigma_device::pointer::{raw_button, raw_move, raw_wheel, PointerDecoder, BTN_LEFT};
use sigma_device::{Event, Phase, Refresh};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-map-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d.clone());
        for (n, body) in [
            ("a.md", "[[b]] [[c]]"),
            ("b.md", "[[c]]"),
            ("c.md", "[[a]]"),
            ("lone.md", "no links"),
        ] {
            std::fs::write(d.join(n), body).unwrap();
        }
        app.open(&sigma_core::NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev, t: 0 }
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
    fn open_graph(&mut self) {
        self.tap_on("Graph");
        assert!(self.app.graph.as_ref().is_some_and(|g| g.map));
    }
    fn touch(&mut self, phase: Phase, x: i32, y: i32) {
        self.t += 30;
        let t = self.t;
        self.send(Event::Touch { phase, x, y, t });
    }
    fn regions_since(&self, n: usize) -> &[(sigma_render::Rect, Refresh)] {
        &self.dev.refreshes[n..]
    }
}

fn assert_min(r: &[(sigma_render::Rect, Refresh)]) {
    for (rect, _) in r {
        assert!(rect.w >= 16 && rect.h >= 16, "{rect:?}");
    }
}

#[test]
fn opens_on_the_map_select_then_open_in_preview() {
    let mut r = Rig::new("select");
    r.open_graph();
    let g = r.app.graph.as_ref().unwrap();
    assert_eq!(g.zoom, 0);
    assert!(g.layout_us > 0, "first open computes the layout");
    assert!(!g.rows.iter().any(|row| row.name.contains(".scratchpad")));
    let lone = g.rows.iter().position(|row| row.name == "lone.md").unwrap();
    assert!(g.layout.orphan[lone]);
    let n = r.dev.refreshes.len();
    assert_eq!(r.tap_on("Node:b.md"), Some(Refresh::Partial));
    assert_min(r.regions_since(n));
    let g = r.app.graph.as_ref().unwrap();
    assert_eq!(g.selected.map(|i| g.rows[i].name.as_str()), Some("b.md"));
    assert_eq!(r.tap_on("Node:b.md"), Some(Refresh::Full));
    assert!(r.app.graph.is_none() && r.app.preview.is_some());
    assert_eq!(r.app.current.as_str(), "b.md");
    // Reopening with unchanged links reuses the cached layout.
    r.send(Event::Key(sigma_device::HwKey::Escape));
    r.open_graph();
    assert_eq!(r.app.graph.as_ref().unwrap().layout_us, 0, "cached");
}

#[test]
fn toggle_zoom_and_pan_refresh_classes() {
    let mut r = Rig::new("zoom");
    r.open_graph();
    assert_eq!(r.tap_on("List"), Some(Refresh::Full));
    assert!(!r.app.graph.as_ref().unwrap().map);
    assert!(
        r.app.control_centre("Zoom+").is_none(),
        "no zoom in the list"
    );
    assert_eq!(r.tap_on("Map"), Some(Refresh::Full));
    for _ in 0..5 {
        r.tap_on("Zoom+");
    }
    assert_eq!(r.app.graph.as_ref().unwrap().zoom, 3);
    assert_eq!(r.tap_on("Zoom-"), Some(Refresh::Full));
    assert_eq!(r.app.graph.as_ref().unwrap().zoom, 2);
    // Page buttons pan, partial.
    let n = r.dev.refreshes.len();
    assert_eq!(
        r.send(Event::PageTurn { forward: true }),
        Some(Refresh::Partial)
    );
    assert!(r.app.graph.as_ref().unwrap().pan.1 > 0.0);
    assert_min(r.regions_since(n));
    // Finger drag pans and never selects or opens.
    let before = r.app.graph.as_ref().unwrap().pan;
    let n = r.dev.refreshes.len();
    r.touch(Phase::Down, 700, 1000);
    for k in 1..=6 {
        r.touch(Phase::Move, 700 - 50 * k, 1000);
    }
    r.touch(Phase::Up, 400, 1000);
    let g = r.app.graph.as_ref().expect("still on the graph");
    assert!(g.pan.0 > before.0, "{:?} → {:?}", before, g.pan);
    assert!(g.selected.is_none());
    for (rect, kind) in r.regions_since(n) {
        assert_ne!(*kind, Refresh::Full, "{rect:?}");
    }
    assert_min(r.regions_since(n));
    // Back to zoom 0 recentres.
    r.tap_on("Zoom-");
    r.tap_on("Zoom-");
    assert_eq!(r.app.graph.as_ref().unwrap().pan, (0.0, 0.0));
}

#[test]
fn mouse_wheel_zooms_click_selects_drag_pans() {
    let mut r = Rig::new("mouse");
    r.send(Event::PointerPlug(true));
    r.open_graph();
    let mut dec = PointerDecoder::default();
    let mut feed = |r: &mut Rig, raws: Vec<sigma_device::evdev::RawEvent>| {
        for raw in raws {
            r.t += 20;
            for e in dec.feed(raw, r.t) {
                r.send(e);
            }
        }
    };
    feed(&mut r, raw_wheel(1));
    assert_eq!(r.app.graph.as_ref().unwrap().zoom, 1);
    feed(&mut r, raw_wheel(-1));
    assert_eq!(r.app.graph.as_ref().unwrap().zoom, 0);
    let (x, y) = r.app.control_centre("Node:c.md").unwrap();
    let (dx, dy) = (x - r.app.pointer.x, y - r.app.pointer.y);
    feed(&mut r, raw_move(dx, dy));
    feed(&mut r, raw_button(BTN_LEFT, true));
    feed(&mut r, raw_button(BTN_LEFT, false));
    let g = r.app.graph.as_ref().unwrap();
    assert_eq!(g.selected.map(|i| g.rows[i].name.as_str()), Some("c.md"));
    let before = g.pan;
    feed(&mut r, raw_button(BTN_LEFT, true));
    feed(&mut r, raw_move(-150, 0));
    feed(&mut r, raw_button(BTN_LEFT, false));
    let g = r.app.graph.as_ref().expect("still on the graph");
    assert!(g.pan.0 > before.0);
    assert_eq!(g.selected.map(|i| g.rows[i].name.as_str()), Some("c.md"));
}

#[test]
fn three_hundred_notes_lay_out_quickly() {
    let names: Vec<String> = (0..300).map(|i| format!("n{i:03}.md")).collect();
    let edges: Vec<(usize, usize)> = (0..300)
        .filter(|i| i % 10 != 9)
        .flat_map(|i| [(i, (i + 1) % 300), (i, (i * 7 / 10) % 20)])
        .filter(|&(a, b)| a != b && b % 10 != 9)
        .collect();
    let t0 = std::time::Instant::now();
    let a = sigma_core::graphmap::layout(names.len(), &edges, sigma_scribe::MAP_SEED);
    let ms = t0.elapsed().as_millis();
    let b = sigma_core::graphmap::layout(names.len(), &edges, sigma_scribe::MAP_SEED);
    assert_eq!(a, b);
    // Debug builds are slow; this only guards against a blow-up.
    assert!(ms < 5000, "{ms} ms");
}
