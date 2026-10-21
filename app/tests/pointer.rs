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
fn redraws_are_throttled_fast_and_at_least_16px() {
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
        assert_eq!(*kind, Refresh::Fast);
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
