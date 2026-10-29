//! How long the Graph plugin takes to open: the first time (the layout runs in
//! the WASM interpreter), again with the library unchanged (one cache read),
//! and after a small change (a warm start). Prints the times:
//!
//!     cargo test --release -p sigma-scribe --test graph_plugin_perf -- --ignored --nocapture
//!
//! "Something on screen" is the loading picture (or, for a loaded plugin, the
//! progress bar), which is what stops the app looking hung.
//!
//! Ignored by default: timings depend on the machine. On the Forma, expect
//! roughly 8 to 10 times the desktop numbers (docs/dev/perf.md).

use sigma_core::NotePath;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_scribe::App;
use std::path::Path;
use std::time::Instant;

fn write_notes(dir: &Path, count: usize, extra: bool) {
    for i in 0..count {
        let mut body = String::new();
        if i % 4 != 0 {
            body += &format!(
                "[[n{:03}]] [[n{:03}]]\n",
                (i * 7 + 3) % count,
                (i * 11 + 5) % count
            );
        }
        std::fs::write(dir.join(format!("n{i:03}.md")), body).unwrap();
    }
    if extra {
        std::fs::write(dir.join("zz-new.md"), "[[n003]] [[n040]]").unwrap();
    }
}

/// Opens the Graph; returns the milliseconds until something is on screen (a
/// loading picture or the map) and until the map is ready, bar steps included.
fn open_graph(app: &mut App, dev: &mut DesktopDevice) -> (f64, f64) {
    let (x, y) = app.control_centre("Graph").unwrap();
    let t = Instant::now();
    if let Some(r) = app.handle(Event::Tap { x, y }) {
        app.render(dev, r).unwrap();
    }
    let first = t.elapsed().as_secs_f64() * 1e3;
    app.plugin_settle(dev);
    let ms = (first, t.elapsed().as_secs_f64() * 1e3);
    assert!(app.plugin_screen_open() && !app.plugin_busy());
    let (x, y) = app.control_centre("Close").unwrap();
    if let Some(r) = app.handle(Event::Tap { x, y }) {
        app.render(dev, r).unwrap();
    }
    ms
}

#[test]
#[ignore = "prints timings; run with --ignored --nocapture"]
fn open_cold_cached_and_warm() {
    let d = std::env::temp_dir().join(format!("sigma-perf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let (notes, plugins) = (d.join("notes"), d.join("plugins/graph-map"));
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::create_dir_all(&plugins).unwrap();
    for f in ["plugin.conf", "graph.wasm"] {
        let from = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/graph-map")
            .join(f);
        std::fs::copy(from, plugins.join(f)).unwrap();
    }
    std::fs::write(d.join("plugins/plugins.state"), "graph-map=1.0.0,enabled\n").unwrap();
    for count in [100, 150, 309, 600, 1000] {
        let _ = std::fs::remove_dir_all(plugins.join(".cache"));
        for f in std::fs::read_dir(&notes).unwrap().flatten() {
            let _ = std::fs::remove_file(f.path());
        }
        write_notes(&notes, count, false);
        let mut app = App::new(1440, 1920, notes.clone());
        app.set_plugins_root(d.join("plugins"));
        app.open(&NotePath::parse("n001.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        let cold = open_graph(&mut app, &mut dev);
        let cached = open_graph(&mut app, &mut dev);
        write_notes(&notes, count, true);
        let warm = open_graph(&mut app, &mut dev);
        println!(
            "{count:4} notes: first open {:8.1} ms (something on screen after {:5.1})   unchanged {:7.1} ms   after one new note {:8.1} ms (picture after {:5.1})",
            cold.1, cold.0, cached.1, warm.1, warm.0
        );
    }
}
