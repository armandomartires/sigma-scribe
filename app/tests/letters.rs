//! v1.5.2 letters (plan §12.5): one character per box, read after a pause
//! or when the next box starts; candidates replace the last letter;
//! Train saves the user's samples locally; Settings deletes them.

use sigma_core::settings::Change;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Phase, Refresh};
use sigma_render::ink::InkMode;
use sigma_scribe::{samples_path, App, LETTER_MS};

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sigma-letters-{tag}-{}", std::process::id()));
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
    /// Strokes in box units into box `i`.
    fn write(&mut self, i: usize, strokes: &[Vec<(f32, f32)>]) -> Vec<Refresh> {
        let b = self.app.letter_box(i).expect("box");
        let mut out = Vec::new();
        for s in strokes {
            for (k, &(x, y)) in s.iter().enumerate() {
                let (px, py) = (
                    b.x + (x * b.w as f32) as i32,
                    b.y + (y * b.h as f32) as i32,
                );
                let ph = if k == 0 { Phase::Down } else { Phase::Move };
                out.extend(self.touch(ph, px, py));
            }
            let &(x, y) = s.last().unwrap();
            out.extend(self.touch(
                Phase::Up,
                b.x + (x * b.w as f32) as i32,
                b.y + (y * b.h as f32) as i32,
            ));
        }
        out
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn circle() -> Vec<Vec<(f32, f32)>> {
    vec![(0..=24)
        .map(|k| {
            let a = k as f32 / 24.0 * std::f32::consts::TAU;
            (0.5 + 0.18 * a.cos(), 0.52 + 0.17 * a.sin())
        })
        .collect()]
}

fn bar() -> Vec<Vec<(f32, f32)>> {
    vec![(0..=12)
        .map(|k| (0.5, 0.12 + 0.58 * k as f32 / 12.0))
        .collect()]
}

#[test]
fn boxes_read_after_a_pause_or_the_next_box_and_ink_is_du() {
    let mut r = Rig::new("read");
    r.tap("Write");
    r.tap("Letters");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Letters);
    let sent = r.write(0, &circle());
    assert!(
        sent.iter().all(|&s| s == Refresh::Ink),
        "live letter ink is DU: {sent:?}"
    );
    // Not read before the pause…
    r.tick(LETTER_MS - 100);
    assert_eq!(r.app.buffer.text(), "");
    r.tick(200);
    assert_eq!(r.app.buffer.text(), "o");
    // …or as soon as another box starts.
    r.write(1, &bar());
    r.write(2, &circle());
    assert_eq!(r.app.buffer.text(), "ol");
    r.tick(LETTER_MS + 100);
    assert_eq!(r.app.buffer.text(), "olo");
    let cands = r.app.ink.as_ref().unwrap().cands.clone();
    assert_eq!(cands.len(), 5);
    assert_eq!(cands[0], 'o');
    // A candidate replaces the last letter; Space, Erase, Return type.
    r.tap("Cand1");
    assert_eq!(r.app.buffer.text(), format!("ol{}", cands[1]));
    r.tap("Erase");
    r.tap("Space");
    r.tap("Return");
    assert_eq!(r.app.buffer.text(), "ol \n");
    r.tap("Done");
    assert!(r.app.ink.is_none());
}

#[test]
fn train_saves_samples_locally_and_settings_deletes_them() {
    let mut r = Rig::new("train");
    let root = r.app.library.root().to_path_buf();
    r.tap("Write");
    r.tap("Letters");
    r.tap("Train");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Train(0));
    // Save with an empty box does nothing.
    r.tap("Save");
    assert!(!samples_path(&root).exists());
    r.write(0, &circle());
    r.tap("Save");
    r.tap("Skip");
    r.write(0, &bar());
    r.tap("Save");
    let text = std::fs::read_to_string(samples_path(&root)).unwrap();
    let s = sigma_hwr::samples_from_text(&text);
    assert_eq!(s.iter().map(|x| x.ch).collect::<String>(), "ac");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Train(3));
    r.tap("Done");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Letters);
    // The notes list never shows the samples folder.
    assert!(r
        .app
        .library
        .list(&sigma_core::NotePath::root())
        .unwrap()
        .iter()
        .all(|e| !e.path.as_str().contains("handwriting")));
    r.app.change(Change::DeleteSamples);
    assert!(!samples_path(&root).exists());
    assert!(!root.join(".handwriting").exists());
}
