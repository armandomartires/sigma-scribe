//! v1.13.1: the editor's scroll bar can be dragged, and a tap on it turns a
//! screen up or down from the thumb's centre. The caret goes with the view
//! when it would be left off the screen.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Phase, Refresh};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("sigma-sbar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(1440, 1920, dir.clone());
        app.buffer.select_all();
        app.buffer.backspace();
        app.buffer.insert(&"a line of the note\n".repeat(300));
        app.buffer.move_to(0, false);
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self {
            app,
            dev,
            t: 0,
            dir,
        }
    }
    fn touch(&mut self, phase: Phase, x: i32, y: i32) {
        self.t += 30;
        let t = self.t;
        if let Some(r) = self.app.handle(Event::Touch { phase, x, y, t }) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }
    fn zone(&mut self) -> (sigma_render::canvas::Rect, sigma_render::canvas::Rect) {
        let page = self.app.page_rect();
        self.app
            .screen
            .scroll_bar_zone(&self.app.buffer, page)
            .expect("a long note has a bar")
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn dragging_the_thumb_scrolls_the_note() {
    let mut r = Rig::new();
    let (_, thumb) = r.zone();
    let (x, y) = (thumb.x + thumb.w / 2, thumb.y + thumb.h / 2);
    r.touch(Phase::Down, x, y);
    for k in 1..=10 {
        r.touch(Phase::Move, x, y + k * 40);
    }
    r.touch(Phase::Up, x, y + 400);
    assert!(r.app.screen.scroll > 20, "scrolled {}", r.app.screen.scroll);
    assert!(r.app.buffer.caret() > 0, "the caret went into view");
    let (_, now) = r.zone();
    assert!(now.y > thumb.y + 300, "the thumb followed the finger");
}

#[test]
fn a_tap_below_the_thumb_pages_down_and_above_pages_up() {
    let mut r = Rig::new();
    let (zone, thumb) = r.zone();
    let x = thumb.x + thumb.w / 2;
    let below = thumb.y + thumb.h + 100;
    assert!(zone.contains(x, below));
    r.touch(Phase::Down, x, below);
    r.touch(Phase::Up, x, below);
    let down = r.app.screen.scroll;
    assert!(down > 10, "paged down {down}");
    let (_, thumb) = r.zone();
    let above = thumb.y - 20;
    r.touch(Phase::Down, x, above);
    r.touch(Phase::Up, x, above);
    assert!(r.app.screen.scroll < down, "paged up");
}
