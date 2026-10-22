//! v0.8.1 USB mouse / touchpad: raw evdev → decoder → app.

use sigma_device::desktop::DesktopDevice;
use sigma_device::pointer::{raw_button, raw_move, raw_wheel, PointerDecoder, BTN_LEFT, BTN_RIGHT};
use sigma_device::{Event, Refresh};
use sigma_scribe::{App, POINTER_HIDE_MS};

struct Rig {
    app: App,
    dev: DesktopDevice,
    dec: PointerDecoder,
    t: u64,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-ptr-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d);
        app.buffer.select_all();
        app.buffer.backspace();
        app.buffer.insert("alpha beta gamma\ndelta epsilon\n");
        app.buffer.set_caret(0);
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        let mut r = Self {
            app,
            dev,
            dec: PointerDecoder::default(),
            t: 0,
        };
        r.send(Event::PointerPlug(true));
        r
    }
    fn send(&mut self, e: Event) {
        if let Some(r) = self.app.handle(e) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }
    fn raw(&mut self, raws: Vec<sigma_device::evdev::RawEvent>) {
        for r in raws {
            for e in self.dec.feed(r, self.t) {
                self.send(e);
            }
        }
    }
    fn to(&mut self, x: i32, y: i32) {
        self.t += 200;
        let (dx, dy) = (x - self.app.pointer.x, y - self.app.pointer.y);
        self.raw(raw_move(dx, dy));
    }
    fn button(&mut self, code: u16, down: bool) {
        self.t += 40;
        self.raw(raw_button(code, down));
    }
    fn click(&mut self) {
        self.button(BTN_LEFT, true);
        self.button(BTN_LEFT, false);
    }
    fn tick(&mut self, ms: u64) {
        let end = self.t + ms;
        while self.t < end {
            self.t += 50;
            let t = self.t;
            self.send(Event::Tick { t });
        }
    }
}

#[test]
fn redraws_are_throttled_gl16_and_at_least_16px() {
    let mut r = Rig::new("throttle");
    r.to(500, 500);
    let before = r.dev.refreshes.len();
    // 20 moves 10 ms apart = 200 ms: at most 2 redraws, plus the flush.
    for _ in 0..20 {
        r.t += 10;
        r.raw(raw_move(3, 2));
    }
    let during = r.dev.refreshes.len() - before;
    assert!(during <= 3, "{during} redraws in 200 ms");
    assert!(r.app.pointer.pending || during > 0);
    r.tick(200);
    assert!(!r.app.pointer.pending, "pending move flushed on the tick");
    for (rect, kind) in &r.dev.refreshes[before..] {
        assert_eq!(*kind, Refresh::Pointer);
        assert!(rect.w >= 16 && rect.h >= 16, "{rect:?}");
        assert!(
            rect.w <= 200 && rect.h <= 200,
            "only the sprite area: {rect:?}"
        );
    }
    assert_eq!((r.app.pointer.x, r.app.pointer.y), (560, 540));
}

#[test]
fn hides_when_idle_and_when_typing() {
    let mut r = Rig::new("hide");
    r.to(400, 400);
    assert!(r.app.pointer.visible);
    r.tick(POINTER_HIDE_MS + 100);
    assert!(!r.app.pointer.visible);
    r.to(410, 400);
    assert!(r.app.pointer.visible);
    r.send(Event::Key(sigma_device::HwKey::Char('x')));
    assert!(!r.app.pointer.visible);
    // The sprite is gone from the panel: the frame equals a fresh render.
    assert_eq!(r.dev.panel.pixels, r.app.canvas.pixels);
}

#[test]
fn click_is_a_tap_drag_selects_double_click_word() {
    let mut r = Rig::new("click");
    let (x, y) = r.app.control_centre("Help").unwrap();
    r.to(x, y);
    r.click();
    assert!(r.app.help.is_some(), "click pressed Help");
    let (x, y) = r.app.control_centre("Close").unwrap();
    r.to(x, y);
    r.click();
    assert!(r.app.help.is_none());
    // Click places the caret.
    let (x, y) = r.app.text_point("delta", false).unwrap();
    r.to(x, y);
    r.click();
    assert_eq!(
        r.app.buffer.caret(),
        r.app.buffer.text().find("delta").unwrap()
    );
    // Drag selects.
    let (x0, y0) = r.app.text_point("beta", false).unwrap();
    let (x1, y1) = r.app.text_point("gamma", true).unwrap();
    r.to(x0, y0);
    r.button(BTN_LEFT, true);
    r.to(x1, y1);
    r.button(BTN_LEFT, false);
    assert_eq!(r.app.buffer.selected_text(), Some("beta gamma"));
    assert!(r.app.sel_ui.is_some_and(|s| s.bar));
    // Right click does nothing.
    r.button(BTN_RIGHT, true);
    r.button(BTN_RIGHT, false);
    assert_eq!(r.app.buffer.selected_text(), Some("beta gamma"));
    // Double-click selects the word.
    let (x, y) = r.app.word_point("epsilon").unwrap();
    r.to(x, y);
    r.click();
    r.click();
    assert_eq!(r.app.buffer.selected_text(), Some("epsilon"));
    // Slow second click: just a click.
    let (x, y) = r.app.word_point("alpha").unwrap();
    r.to(x, y);
    r.click();
    r.t += 1000;
    r.click();
    assert_eq!(r.app.buffer.selected_text(), None);
}

#[test]
fn wheel_scrolls_note_and_pages() {
    let mut r = Rig::new("wheel");
    r.app.buffer.insert(&"line\n".repeat(200));
    r.app.buffer.set_caret(0);
    r.raw(raw_wheel(-2));
    let after = r.app.buffer.caret();
    assert!(after > 0, "moved down");
    r.raw(raw_wheel(2));
    assert!(r.app.buffer.caret() < after);
    r.app.open_options();
    r.raw(raw_wheel(-1)); // ignored, no panic
}

#[test]
fn unplug_mid_drag_is_safe() {
    let mut r = Rig::new("unplug");
    let (x, y) = r.app.text_point("beta", false).unwrap();
    r.to(x, y);
    r.button(BTN_LEFT, true);
    r.send(Event::PointerPlug(false));
    assert!(!r.app.pointer.visible && r.app.pointer.down.is_none());
    r.raw(raw_move(5, 5)); // stray report after unplug
    assert!(!r.app.pointer.visible);
    r.send(Event::PointerPlug(false));
    r.send(Event::Key(sigma_device::HwKey::Char('q')));
    assert!(r.app.buffer.text().contains('q'));
}

/// v1.0.1 ghost trails: every move is ONE merged old+new region on the
/// pointer (GL16) waveform, and hiding sends one cleanup over the path.
#[test]
fn moves_are_one_merged_region_and_hide_cleans_the_path() {
    let mut r = Rig::new("trail");
    r.to(300, 300);
    let mut lo = (i32::MAX, i32::MAX);
    let mut hi = (0, 0);
    for i in 0..24 {
        let a = f64::from(i) * std::f64::consts::PI / 6.0;
        let (x, y) = (
            600 + (200.0 * a.cos()) as i32,
            700 + (200.0 * a.sin()) as i32,
        );
        let before = r.dev.refreshes.len();
        r.to(x, y);
        let new = &r.dev.refreshes[before..];
        assert_eq!(new.len(), 1, "one region per move: {new:?}");
        let (rect, kind) = new[0];
        assert_eq!(kind, Refresh::Pointer);
        // Old and new sprite both inside the one region.
        assert!(rect.x <= x + 4 && rect.y <= y + 4, "{rect:?} vs {x},{y}");
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x), hi.1.max(y));
    }
    let before = r.dev.refreshes.len();
    r.tick(POINTER_HIDE_MS + 100);
    assert!(!r.app.pointer.visible);
    let cleanup: Vec<_> = r.dev.refreshes[before..]
        .iter()
        .filter(|(_, k)| *k == Refresh::PointerCleanup)
        .collect();
    assert_eq!(cleanup.len(), 1, "{:?}", &r.dev.refreshes[before..]);
    let c = cleanup[0].0;
    assert!(
        c.x <= lo.0 && c.y <= lo.1 && c.x + c.w > hi.0 && c.y + c.h > hi.1,
        "{c:?}"
    );
    assert!(r.app.pointer.trail.is_none());
    assert_eq!(r.dev.panel.pixels, r.app.canvas.pixels);
    // Typing hides it too, with the cleanup.
    r.to(500, 500);
    let before = r.dev.refreshes.len();
    r.send(Event::Key(sigma_device::HwKey::Char('x')));
    assert!(r.dev.refreshes[before..]
        .iter()
        .any(|(_, k)| *k == Refresh::PointerCleanup));
}

/// Emulator latency for the pointer path (app side: draw + diff + merge).
#[test]
fn pointer_update_cost_is_small() {
    let mut r = Rig::new("cost");
    r.to(300, 300);
    let mut v = Vec::new();
    for i in 0..100 {
        r.t += 200;
        let t0 = std::time::Instant::now();
        r.raw(raw_move(if i % 2 == 0 { 7 } else { -5 }, 3));
        v.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    v.sort_by(f64::total_cmp);
    println!("pointer update p50 {:.2} ms p95 {:.2} ms", v[50], v[95]);
    assert!(v[50] < 200.0);
}

#[test]
fn reverse_scroll_flips_wheel_notches() {
    let mut r = Rig::new("reverse");
    r.app.buffer.select_all();
    r.app.buffer.backspace();
    let text: String = (0..200).map(|i| format!("line {i}\n")).collect();
    r.app.buffer.insert(&text);
    r.app.buffer.set_caret(0);
    r.send(Event::Wheel { notches: -2 });
    let traditional = r.app.buffer.caret();
    assert!(traditional > 0, "wheel down moves forward");
    r.app.buffer.set_caret(0);
    r.app
        .settings
        .apply(sigma_core::settings::Change::NaturalScroll(true));
    r.send(Event::Wheel { notches: 2 });
    assert_eq!(
        r.app.buffer.caret(),
        traditional,
        "natural: same move, other notch sign"
    );
    let s = r.app.settings.serialize();
    assert!(s.contains("scroll=natural"), "{s}");
    assert!(sigma_core::settings::Settings::parse(&s).natural_scroll);
}

#[test]
fn v103_moves_are_du_and_cleanup_is_one_gl16_on_hide() {
    let mut r = Rig::new("du");
    r.to(400, 400);
    let n = r.dev.refreshes.len();
    for _ in 0..5 {
        r.to(r.app.pointer.x + 30, 400);
    }
    let moves = &r.dev.refreshes[n..];
    assert!(!moves.is_empty());
    assert!(moves.iter().all(|(_, k)| *k == Refresh::Pointer));
    assert!(Refresh::Pointer.fbink_mode() == 3 && !Refresh::Pointer.waits_for_completion());
    let n = r.dev.refreshes.len();
    r.tick(POINTER_HIDE_MS + 100);
    let cleanups: Vec<_> = r.dev.refreshes[n..]
        .iter()
        .filter(|(_, k)| *k == Refresh::PointerCleanup)
        .collect();
    assert_eq!(cleanups.len(), 1, "one GL16 pass on hide");
    let (c, _) = cleanups[0];
    assert!(c.w >= 16 && c.h >= 16);
    assert!(c.w >= 150, "covers the whole path: {c:?}");
}

#[test]
fn v103_pointer_never_blocks_typing() {
    let mut r = Rig::new("typing");
    r.to(600, 600);
    // Interleave moves and keys: every key is handled at once with a Fast
    // refresh, the text lands, and no refresh class waits for the panel.
    for (i, c) in "abcdef".chars().enumerate() {
        r.t += 20;
        r.raw(raw_move(5, 0));
        let got = r.app.handle(Event::Key(sigma_device::HwKey::Char(c)));
        assert_eq!(got, Some(Refresh::Fast), "key {i}");
        r.app.render(&mut r.dev, Refresh::Fast).unwrap();
        assert!(!r.app.pointer.visible, "typing hides the pointer");
    }
    assert!(r.app.buffer.text().starts_with("abcdef"));
    assert!(r
        .dev
        .refreshes
        .iter()
        .all(|(_, k)| !k.waits_for_completion()));
    assert!(r
        .dev
        .refreshes
        .iter()
        .any(|(_, k)| *k == Refresh::PointerCleanup));
}

#[test]
fn v104_pointer_only_frames_match_a_full_render_and_stay_small() {
    let mut r = Rig::new("fast");
    r.to(300, 300);
    for i in 0..12 {
        r.to(300 + i * 37, 300 + i * 23);
    }
    // The fast path must leave exactly what a full redraw would show.
    let fast = r.dev.panel.pixels.clone();
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert_eq!(fast, r.dev.panel.pixels, "fast path == full frame");
    // Pointer frames only cover the old+new arrow (arrow follows the UI scale, v1.1).
    let n = r.dev.refreshes.len();
    r.to(700, 700);
    for (rect, k) in &r.dev.refreshes[n..] {
        assert_eq!(*k, Refresh::Pointer);
        assert!(
            rect.w <= 300 && rect.h <= 300 && rect.w >= 16 && rect.h >= 16,
            "{rect:?}"
        );
    }
    // Hiding restores the page and runs one cleanup.
    r.tick(POINTER_HIDE_MS + 100);
    let hidden = r.dev.panel.pixels.clone();
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert_eq!(hidden, r.dev.panel.pixels);
}

#[test]
fn v104_backlogged_moves_do_not_hide_the_pointer() {
    let mut r = Rig::new("backlog");
    r.to(400, 400);
    // Ticks run at "now" = 10 s while queued moves still carry old times.
    r.t = 10_000;
    let t = r.t;
    r.send(Event::Tick { t });
    for i in 0..5 {
        let t = 6_000 + i * 10; // read 4 s ago, processed now
        r.send(Event::PointerMove { dx: 3, dy: 0, t });
    }
    let t = r.t + 50;
    r.send(Event::Tick { t });
    assert!(r.app.pointer.visible, "a backlog is not idle time");
}

/// v1.4.1: the arrow's fill pixels on the panel are pure black after
/// moves (fast path), a restore, a full render and the hide cleanup, in
/// Paper and Night; the sprite never has greys.
#[test]
fn v141_arrow_is_pure_black_on_every_path() {
    use sigma_core::settings::{Change, Paper};
    use sigma_render::pointer::{pointer_cell, sprite, FILL, OUTLINE};
    for paper in [Paper::Day, Paper::Night] {
        let mut r = Rig::new(&format!("black-{paper:?}"));
        r.app.change(Change::Paper(paper));
        r.app.render(&mut r.dev, Refresh::Full).unwrap();
        let cell = pointer_cell(r.app.screen.scale().px(1.0) as f32);
        let check = |r: &Rig, what: &str| {
            let (x, y) = (r.app.pointer.x, r.app.pointer.y);
            let w = r.dev.panel.width as i32;
            for (i, v) in sprite().iter().enumerate() {
                let Some(v) = v else { continue };
                let (cx, cy) = (x + (i as i32 % 12) * cell, y + (i as i32 / 12) * cell);
                let got = r.dev.panel.pixels[(cy * w + cx) as usize];
                assert_eq!(got, *v, "{paper:?} {what}: ({cx},{cy})");
                assert!(*v == FILL || *v == OUTLINE);
            }
        };
        r.to(400, 400);
        check(&r, "first move");
        for i in 0..8 {
            r.to(400 + i * 41, 400 + i * 29);
            check(&r, "fast-path move");
        }
        // Over the keyboard and text (restore of grey under-areas).
        r.to(300, 1700);
        check(&r, "over keyboard");
        r.app.render(&mut r.dev, Refresh::Full).unwrap();
        check(&r, "full render");
        r.tick(POINTER_HIDE_MS + 100);
        assert!(!r.app.pointer.visible, "hidden");
        r.to(500, 600);
        check(&r, "after cleanup");
    }
}
