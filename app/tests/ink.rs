//! v1.5.0 ink panel (plan §12.3–12.4): white panel, pure black DU ink,
//! GL16 cleanup after a pause, undo / clear, SVG + strokes next to the note.

use sigma_core::settings::{Change, Settings};
use sigma_device::desktop::DesktopDevice;
use sigma_device::Phase;
use sigma_device::{Event, Refresh};
use sigma_scribe::{App, INK_CLEANUP_MS};

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str, w: u32, h: u32) -> Self {
        let dir = std::env::temp_dir().join(format!("sigma-ink-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(w, h, dir.clone());
        app.buffer.select_all();
        app.buffer.backspace();
        app.buffer.insert("first line\nsecond");
        app.buffer.set_caret(10);
        let mut dev = DesktopDevice::new(w, h);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self {
            app,
            dev,
            t: 0,
            dir,
        }
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }
    fn touch(&mut self, phase: Phase, (x, y): (i32, i32)) -> Option<Refresh> {
        self.t += 25;
        let t = self.t;
        self.send(Event::Touch { phase, x, y, t })
    }
    fn tap(&mut self, label: &str) {
        let p = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no {label}"));
        self.touch(Phase::Down, p);
        self.touch(Phase::Up, p);
    }
    fn tick(&mut self, ms: u64) {
        let end = self.t + ms;
        while self.t < end {
            self.t += 50;
            let t = self.t;
            self.send(Event::Tick { t });
        }
    }
    /// A zig-zag stroke in the ink area; returns the refreshes it sent.
    fn stroke(&mut self, dx: i32) -> Vec<Refresh> {
        let (cx, cy) = self.app.control_centre("InkArea").unwrap();
        let mut out = Vec::new();
        out.extend(self.touch(Phase::Down, (cx - 200 + dx, cy)));
        for i in 1..=10 {
            out.extend(self.touch(Phase::Move, (cx - 200 + dx + i * 20, cy + (i % 2) * 40)));
        }
        out.extend(self.touch(Phase::Up, (cx + dx, cy)));
        out
    }
    fn black_in_area(&self) -> usize {
        let l = self.app.screen.ink_layout();
        let c = &self.app.canvas;
        (l.area.y..l.area.y + l.area.h)
            .flat_map(|y| (l.area.x..l.area.x + l.area.w).map(move |x| (x, y)))
            .filter(|&(x, y)| c.pixels[(y * c.width as i32 + x) as usize] == 0)
            .count()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn live_ink_is_du_pure_black_and_cleaned_with_gl16_after_a_pause() {
    let mut r = Rig::new("du", 1440, 1920);
    r.tap("Write");
    r.tap("Draw");
    assert!(r.app.ink.is_some());
    // The panel is pure white (buttons aside): DU-exact.
    let l = r.app.screen.ink_layout();
    let c = &r.app.canvas;
    let px = |x: i32, y: i32| c.pixels[(y * c.width as i32 + x) as usize];
    assert_eq!(px(l.area.x + 5, l.area.y + 5), 255);
    r.dev.refreshes.clear();
    let sent = r.stroke(0);
    assert!(
        !sent.is_empty() && sent.iter().all(|&s| s == Refresh::Ink),
        "{sent:?}"
    );
    // Every live region is small and holds only black and white.
    for (rect, rf) in &r.dev.refreshes {
        assert_eq!(*rf, Refresh::Ink);
        assert!(rect.w * rect.h < 200 * 200, "segment region {rect:?}");
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                if l.area.contains(x, y) {
                    let v = r.app.canvas.pixels[(y * 1440 + x) as usize];
                    assert!(v == 0 || v == 255, "grey {v} at {x},{y}");
                }
            }
        }
    }
    assert!(r.black_in_area() > 2000);
    // No cleanup before the pause; one GL16 pass after it, over the ink.
    r.dev.refreshes.clear();
    r.tick(INK_CLEANUP_MS - 100);
    assert!(r.dev.refreshes.is_empty());
    r.tick(200);
    let cl: Vec<_> = r
        .dev
        .refreshes
        .iter()
        .filter(|(_, f)| *f == Refresh::PointerCleanup)
        .collect();
    assert_eq!(cl.len(), 1, "{:?}", r.dev.refreshes);
    r.dev.refreshes.clear();
    r.tick(2000);
    assert!(r.dev.refreshes.is_empty(), "only one cleanup per pause");
}

#[test]
fn a2_setting_switches_the_live_waveform() {
    let mut r = Rig::new("a2", 1440, 1920);
    assert!(!r.app.settings.ink_a2, "A2 is off by default");
    r.app.change(Change::InkA2(true));
    // Fast ink is an experimental feature: it does nothing until they are on.
    assert!(!r.app.settings.fast_ink());
    r.app.change(Change::Experimental(true));
    assert!(r.app.settings.fast_ink());
    r.tap("Write");
    r.tap("Draw");
    let sent = r.stroke(0);
    assert!(sent.iter().all(|&s| s == Refresh::InkA2), "{sent:?}");
    let s = Settings::parse(&r.app.settings.serialize());
    assert!(s.ink_a2);
    assert!(
        !Settings::parse("keyboard=compact\n").ink_a2,
        "old files load with A2 off"
    );
}

#[test]
fn undo_clear_and_touches_outside_the_panel() {
    let mut r = Rig::new("undo", 1440, 1920);
    r.tap("Write");
    r.tap("Draw");
    let base = r.black_in_area();
    r.stroke(0);
    let one = r.black_in_area();
    r.stroke(250);
    assert!(r.black_in_area() > one);
    r.tap("Undo");
    assert_eq!(r.black_in_area(), one);
    r.tap("Clear");
    assert_eq!(r.black_in_area(), base, "only the dotted baseline is left");
    r.tap("Undo");
    assert_eq!(r.black_in_area(), one, "undo brings a Clear back");
    // Above the panel: no ink, the note does not take the touch.
    let before = r.app.buffer.text().to_owned();
    assert_eq!(r.touch(Phase::Down, (700, 200)), None);
    r.touch(Phase::Up, (700, 200));
    assert_eq!(r.app.buffer.text(), before);
    assert!(r.app.ink.is_some());
    r.tap("Cancel");
    assert!(r.app.ink.is_none());
    assert!(!r.dir.join(".ink").exists(), "Cancel writes nothing");
}

#[test]
fn insert_writes_svg_and_strokes_next_to_the_note() {
    let mut r = Rig::new("insert", 1440, 1920);
    let stem = std::path::Path::new(r.app.current.as_str())
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let note_dir = r
        .dir
        .join(r.app.current.as_str())
        .parent()
        .unwrap()
        .to_path_buf();
    r.tap("Write");
    r.tap("Draw");
    r.stroke(0);
    r.tap("Pen");
    assert_eq!(r.app.settings.ink_pen, sigma_core::ink::Pen::Stylus);
    r.tap("Insert");
    assert!(r.app.ink.is_none());
    let svg = note_dir.join(format!(".ink/{stem}/ink-001.svg"));
    let json = note_dir.join(format!(".ink/{stem}/ink-001.strokes.json"));
    let s = std::fs::read_to_string(&svg).unwrap();
    assert!(
        s.starts_with("<svg")
            && s.contains("stroke=\"#000\"")
            && s.contains("stroke-width=\"0.60\"")
    );
    let j = std::fs::read_to_string(&json).unwrap();
    assert!(j.contains("\"format\":\"sigma-scribe-ink\"") && j.contains("\"pen\":\"stylus\""));
    assert_eq!(
        r.app.buffer.text(),
        format!("first line\n\n![ink](.ink/{stem}/ink-001.svg)\n\nsecond")
    );
    // A second block takes the next name; an empty panel inserts nothing.
    r.tap("Write");
    r.tap("Draw");
    r.stroke(0);
    r.tap("Insert");
    assert!(note_dir.join(format!(".ink/{stem}/ink-002.svg")).exists());
    let t = r.app.buffer.text().to_owned();
    r.tap("Write");
    r.tap("Draw");
    r.tap("Insert");
    assert_eq!(r.app.buffer.text(), t);
}

#[test]
fn panel_fits_portrait_and_landscape_at_every_ui_scale() {
    for land in [false, true] {
        let mut r = Rig::new(&format!("fit{land}"), 1440, 1920);
        if land {
            r.app.change(Change::Orientation(
                sigma_core::settings::Orientation::Landscape,
            ));
        }
        let w = r.app.canvas.width;
        for _ in 0..4 {
            r.app.change(Change::UiScale(1));
        }
        r.tap("Write");
        r.tap("Draw");
        let l = r.app.screen.ink_layout();
        for (b, _) in &l.buttons {
            assert!(
                b.h >= 44 && b.w >= 44 && b.x >= 0 && b.x + b.w <= w as i32,
                "{b:?}"
            );
        }
        for (i, (a, _)) in l.buttons.iter().enumerate() {
            for (b, _) in &l.buttons[i + 1..] {
                assert!(
                    a.intersect(b).w <= 0 || a.intersect(b).h <= 0,
                    "overlap {a:?} {b:?}"
                );
            }
        }
        // The keys take a third of the width (v1.16); the rest is for drawing.
        assert!(
            l.area.h > 300 && l.area.w >= w as i32 * 60 / 100,
            "{:?}",
            l.area
        );
        let base = r.black_in_area();
        r.stroke(0);
        assert!(r.black_in_area() > base);
    }
}

#[test]
fn preview_draws_inserted_ink_pure_black_and_pure_white_at_night() {
    for night in [false, true] {
        let mut r = Rig::new(&format!("pv{night}"), 1440, 1920);
        if night {
            r.app
                .change(Change::Paper(sigma_core::settings::Paper::Night));
        }
        r.tap("Write");
        r.tap("Draw");
        r.stroke(0);
        r.tap("Insert");
        r.app.enter_preview();
        r.app.render(&mut r.dev, Refresh::Full).unwrap();
        let doc = &r.app.preview.as_ref().unwrap().doc;
        let line = doc
            .lines
            .iter()
            .find(|l| l.image.is_some())
            .expect("ink line");
        let img = line.image.as_ref().unwrap();
        assert!(img.w > 100 && img.h > 20, "{}x{}", img.w, img.h);
        assert!(
            !doc.lines
                .iter()
                .flat_map(|l| &l.pieces)
                .any(|p| p.text.contains("[image")),
            "no placeholder text"
        );
        let want = if night { 255 } else { 0 };
        let n = r.app.canvas.pixels.iter().filter(|&&p| p == want).count();
        assert!(
            n > img.on.iter().filter(|&&b| b).count() / 2,
            "ink drawn ({n})"
        );
    }
}

#[test]
fn preview_keeps_the_placeholder_for_missing_or_foreign_images() {
    let mut r = Rig::new("pvmissing", 1440, 1920);
    r.app.buffer.select_all();
    r.app
        .buffer
        .insert("![ink](.ink/nope/ink-001.svg)\n\n![pic](../etc/x.svg)\n");
    r.app.enter_preview();
    let doc = &r.app.preview.as_ref().unwrap().doc;
    assert!(doc.lines.iter().all(|l| l.image.is_none()));
    let text: String = doc
        .lines
        .iter()
        .flat_map(|l| &l.pieces)
        .map(|p| p.text.as_str())
        .collect();
    assert!(
        text.contains("[image: ink]") && text.contains("[image: pic]"),
        "{text}"
    );
}

#[test]
fn strokes_record_contact_size_only_when_the_panel_reports_it() {
    let mut r = Rig::new("contact", 1440, 1920);
    r.tap("Write");
    r.tap("Draw");
    r.stroke(0);
    r.tap("Insert");
    let note = r.dir.join(r.app.current.as_str());
    let stem = note.file_stem().unwrap().to_string_lossy().into_owned();
    let j =
        std::fs::read_to_string(note.with_file_name(format!(".ink/{stem}/ink-001.strokes.json")))
            .unwrap();
    // The desktop/emulator input has no ABS_MT_TOUCH_MAJOR: points stay [x,y,t].
    assert!(
        j.contains("\"contact\":null") && j.contains("\"version\":2"),
        "{j}"
    );
}
