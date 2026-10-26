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
                let (px, py) = (b.x + (x * b.w as f32) as i32, b.y + (y * b.h as f32) as i32);
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
    r.app.change(Change::InkBoxes(true));
    r.tap("Write");
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
    assert_eq!(cands[0], "o");
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
    r.app.change(Change::InkBoxes(true));
    r.tap("Write");
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

/// Box-unit strokes placed at `x` (area heights) in the free area, 0.4 high.
fn at(x: f32, s: Vec<Vec<(f32, f32)>>) -> Vec<Vec<(f32, f32)>> {
    s.into_iter()
        .map(|st| {
            st.into_iter()
                .map(|(a, b)| (x + a * 0.4, 0.2 + b * 0.4))
                .collect()
        })
        .collect()
}

#[test]
fn free_area_is_the_default_and_reads_printed_letters() {
    let mut r = Rig::new("free");
    r.tap("Write");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Free);
    let a = r.app.letter_box(0).expect("area");
    // Free strokes: x in area heights (like the emulator's ink-line).
    let mut sent = Vec::new();
    for (i, s) in [circle(), bar(), circle()].into_iter().enumerate() {
        for st in at(0.1 + 0.16 * i as f32, s) {
            for (k, &(x, y)) in st.iter().enumerate() {
                let (px, py) = (a.x + (x * a.h as f32) as i32, a.y + (y * a.h as f32) as i32);
                let ph = if k == 0 { Phase::Down } else { Phase::Move };
                sent.extend(r.touch(ph, px, py));
            }
            let &(x, y) = st.last().unwrap();
            let (px, py) = (a.x + (x * a.h as f32) as i32, a.y + (y * a.h as f32) as i32);
            sent.extend(r.touch(Phase::Up, px, py));
        }
    }
    assert!(
        sent.iter().all(|&s| s == Refresh::Ink),
        "free ink is DU: {sent:?}"
    );
    assert_eq!(r.app.buffer.text(), "");
    r.tick(1300);
    let got = r.app.buffer.text();
    assert_eq!(got.chars().count(), 4, "three letters and a space: {got:?}");
    assert!(got.ends_with(' '));
    assert!(!r.app.ink.as_ref().unwrap().cands.is_empty());
    // The keys type directly; 123 and Draw switch.
    r.tap(",");
    assert!(r.app.buffer.text().ends_with(','));
    r.tap("123");
    assert!(r.app.ink.as_ref().unwrap().digits);
    r.tap("Draw");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Draw);
    r.tap("Letters");
    assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Free);
}

#[test]
fn stray_reports_outside_the_box_do_not_draw_lines_to_its_corners() {
    let mut r = Rig::new("stray");
    r.app.change(Change::InkBoxes(true));
    r.tap("Write");
    let b = r.app.letter_box(0).expect("box");
    let (cx, cy) = (b.x + b.w / 2, b.y + b.h / 3);
    r.touch(Phase::Down, cx, cy);
    // Bogus reports far outside the box (panel corner, above the panel).
    r.touch(Phase::Move, 0, 0);
    r.touch(Phase::Move, 1439, 0);
    for k in 1..=6 {
        r.touch(Phase::Move, cx, cy + k * 8);
    }
    r.touch(Phase::Up, cx, cy + 48);
    let ink = r.app.ink.as_ref().unwrap();
    let stroke = ink.ink.strokes.last().expect("one stroke");
    assert!(
        stroke
            .points
            .iter()
            .all(|p| p.x == cx && (cy..=cy + 48).contains(&p.y)),
        "points: {:?}",
        stroke.points
    );
}

fn learned(root: &std::path::Path) -> Vec<sigma_hwr::Sample> {
    std::fs::read_to_string(sigma_scribe::learned_path(root))
        .map(|t| sigma_hwr::samples_from_text(&t))
        .unwrap_or_default()
}

#[test]
fn a_letter_fixed_with_a_candidate_is_learned_once_and_capped() {
    let mut r = Rig::new("learn");
    let root = r.app.library.root().to_path_buf();
    r.app.change(Change::InkBoxes(true));
    r.tap("Write");
    r.write(0, &circle());
    r.tick(LETTER_MS + 100);
    let cands = r.app.ink.as_ref().unwrap().cands.clone();
    // Reading accepted as typed: nothing learned.
    assert!(learned(&root).is_empty());
    r.tap("Cand1");
    let got = learned(&root);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].ch.to_string(), cands[1]);
    // Picking again for the same reading replaces what it taught.
    r.tap("Cand2");
    let got = learned(&root);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].ch.to_string(), cands[2]);
    // The same correction many times keeps only the newest few.
    for i in 0..sigma_scribe::LEARN_PER_CHAR + 2 {
        r.write(i % 2, &circle());
        r.tick(LETTER_MS + 100);
        r.tap("Cand2");
    }
    let got = learned(&root);
    let mut per = std::collections::HashMap::new();
    for s in &got {
        *per.entry(s.ch).or_insert(0usize) += 1;
    }
    assert!(
        per.values().all(|&n| n <= sigma_scribe::LEARN_PER_CHAR),
        "{per:?}"
    );
    // The recogniser is fitted again on them, in the background.
    r.app.hwr.poll(true);
    assert_eq!(r.app.hwr.samples, got.len());
    // Settings → Delete samples removes the learned ones too.
    r.app.change(Change::DeleteSamples);
    assert!(!sigma_scribe::learned_path(&root).exists());
}

#[test]
fn learning_off_keeps_corrections_out_of_the_samples() {
    let mut r = Rig::new("nolearn");
    let root = r.app.library.root().to_path_buf();
    r.app.change(Change::InkBoxes(true));
    r.app.change(Change::InkLearn(false));
    r.tap("Write");
    r.write(0, &circle());
    r.tick(LETTER_MS + 100);
    r.tap("Cand1");
    assert!(!sigma_scribe::learned_path(&root).exists());
}

#[test]
fn a_free_area_letter_fixed_with_a_candidate_is_learned() {
    let mut r = Rig::new("learnfree");
    let root = r.app.library.root().to_path_buf();
    r.tap("Write");
    let a = r.app.letter_box(0).expect("area");
    for (i, s) in [circle(), bar(), circle()].into_iter().enumerate() {
        for st in at(0.1 + 0.16 * i as f32, s) {
            for (k, &(x, y)) in st.iter().enumerate() {
                let (px, py) = (a.x + (x * a.h as f32) as i32, a.y + (y * a.h as f32) as i32);
                r.touch(if k == 0 { Phase::Down } else { Phase::Move }, px, py);
            }
            let &(x, y) = st.last().unwrap();
            let (px, py) = (a.x + (x * a.h as f32) as i32, a.y + (y * a.h as f32) as i32);
            r.touch(Phase::Up, px, py);
        }
    }
    r.tick(1300);
    let cands = r.app.ink.as_ref().unwrap().cands.clone();
    let typed: Vec<char> = cands[0].chars().collect();
    // An alternative that changes one letter of the reading.
    let (k, ch) = cands
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(k, c)| {
            let c: Vec<char> = c.chars().collect();
            let d: Vec<usize> = (0..c.len())
                .filter(|&i| c.len() == typed.len() && c[i] != typed[i])
                .collect();
            (d.len() == 1).then(|| (k, c[d[0]]))
        })
        .expect("a one-letter alternative");
    r.tap(&format!("Cand{k}"));
    let got = learned(&root);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].ch, ch);
}
