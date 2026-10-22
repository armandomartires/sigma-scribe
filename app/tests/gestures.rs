//! v1.5.4 gestures (plan §12.3, sprint S15.4): scratch-out deletes, a
//! long flat flick is Backspace (line join) or Space, a tap on the note
//! above the panel moves the caret; in Draw a scratch-out erases strokes.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Phase, Refresh};
use sigma_render::ink::InkMode;
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sigma-gest-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(1440, 1920, dir.clone());
        app.buffer.select_all();
        app.buffer.backspace();
        let mut dev = DesktopDevice::new(1440, 1920);
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
    fn touch(&mut self, phase: Phase, x: i32, y: i32) -> Option<Refresh> {
        self.t += 20;
        let t = self.t;
        self.send(Event::Touch { phase, x, y, t })
    }
    fn tap(&mut self, label: &str) {
        let (x, y) = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no {label}"));
        self.touch(Phase::Down, x, y);
        self.touch(Phase::Up, x, y);
    }
    fn tick(&mut self, ms: u64) {
        let end = self.t + ms;
        while self.t < end {
            self.t += 50;
            let t = self.t;
            self.send(Event::Tick { t });
        }
    }
    /// One stroke, points in area heights from the area's corner.
    fn stroke(&mut self, pts: &[(f32, f32)]) {
        let a = self.app.letter_box(0).expect("area");
        let px =
            |&(x, y): &(f32, f32)| (a.x + (x * a.h as f32) as i32, a.y + (y * a.h as f32) as i32);
        for (k, p) in pts.iter().enumerate() {
            let (x, y) = px(p);
            self.touch(if k == 0 { Phase::Down } else { Phase::Move }, x, y);
        }
        let (x, y) = px(pts.last().unwrap());
        self.touch(Phase::Up, x, y);
    }
    fn strokes(&self) -> usize {
        self.app.ink.as_ref().unwrap().ink.strokes.len()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn zigzag(x: f32, y: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    (0..=48)
        .map(|k| {
            let t = k as f32 / 8.0;
            let f = t.fract();
            let u = if (t as usize) % 2 == 0 { f } else { 1.0 - f };
            (x + u * w, y + h * t / 6.0)
        })
        .collect()
}

fn flick(a: f32, b: f32) -> Vec<(f32, f32)> {
    (0..=12)
        .map(|k| (a + (b - a) * k as f32 / 12.0, 0.5))
        .collect()
}

fn circle(cx: f32) -> Vec<(f32, f32)> {
    (0..=24)
        .map(|k| {
            let a = k as f32 / 24.0 * std::f32::consts::TAU;
            (cx + 0.07 * a.cos(), 0.5 + 0.07 * a.sin())
        })
        .collect()
}

#[test]
fn scratch_out_erases_pending_ink_or_deletes_the_last_word() {
    let mut r = Rig::new("scratch");
    r.tap("Write");
    r.stroke(&circle(0.3));
    assert_eq!(r.strokes(), 1);
    r.stroke(&zigzag(0.2, 0.42, 0.2, 0.16));
    assert_eq!(r.strokes(), 0, "scratch over ink erases it");
    r.tick(1300);
    assert_eq!(r.app.buffer.text(), "", "nothing was typed");
    // Over nothing: the last reading goes.
    r.stroke(&circle(0.3));
    r.tick(1300);
    assert!(!r.app.buffer.text().is_empty());
    r.stroke(&zigzag(0.2, 0.42, 0.3, 0.16));
    r.tick(1300);
    assert_eq!(r.app.buffer.text(), "");
    // Typed text: the word before the caret goes.
    r.app.buffer.insert("hello world");
    r.stroke(&zigzag(0.2, 0.42, 0.3, 0.16));
    assert_eq!(r.app.buffer.text(), "hello ");
}

#[test]
fn flicks_join_lines_and_type_space() {
    let mut r = Rig::new("flick");
    r.tap("Write");
    r.app.buffer.insert("ab\ncd");
    r.app.buffer.set_caret(3);
    r.stroke(&flick(0.9, 0.2));
    assert_eq!(
        r.app.buffer.text(),
        "abcd",
        "flick left at a line start joins"
    );
    r.stroke(&flick(0.2, 0.9));
    assert_eq!(r.app.buffer.text(), "ab cd");
    assert_eq!(r.strokes(), 0, "gestures leave no ink");
    // A short dash stays ink.
    r.stroke(&flick(0.3, 0.4));
    assert_eq!(r.strokes(), 1);
}

#[test]
fn caret_tap_above_the_panel_and_draw_scratch() {
    let mut r = Rig::new("caret");
    r.app.buffer.insert("first line\nsecond line\n");
    r.tap("Write");
    let end = r.app.buffer.caret();
    let panel_top = r.app.letter_box(0).unwrap().y - 200;
    // The first line of the note is near the top of the page.
    let (x, y) = (60, 180);
    assert!(y < panel_top);
    r.touch(Phase::Down, x, y);
    r.touch(Phase::Up, x, y);
    assert!(r.app.buffer.caret() < end, "caret moved");
    assert!(r.app.ink.is_some(), "panel stays open");
    // Draw: a scratch-out erases crossed strokes only.
    r.tap("Draw");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Draw);
    let a = r.app.panel_layout().unwrap().area;
    let at = |x: f32, y: f32| (a.x + (x * a.h as f32) as i32, a.y + (y * a.h as f32) as i32);
    for cx in [0.3, 1.2] {
        for (k, p) in circle(cx).iter().enumerate() {
            let (x, y) = at(p.0, p.1);
            r.touch(if k == 0 { Phase::Down } else { Phase::Move }, x, y);
        }
        let (x, y) = at(cx, 0.5);
        r.touch(Phase::Up, x, y);
    }
    assert_eq!(r.strokes(), 2);
    let z = zigzag(0.2, 0.42, 0.2, 0.16);
    for (k, p) in z.iter().enumerate() {
        let (x, y) = at(p.0, p.1);
        r.touch(if k == 0 { Phase::Down } else { Phase::Move }, x, y);
    }
    let (x, y) = at(z[z.len() - 1].0, z[z.len() - 1].1);
    r.touch(Phase::Up, x, y);
    assert_eq!(r.strokes(), 1, "only the crossed circle goes");
}
