//! v1.11.2: caret and page keys (Home, End, PgUp, PgDn, Ins) in the Write
//! panel, Insert / overwrite typing, and Undo that takes back typed text.

use sigma_core::settings::{Change, Orientation};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Phase, Refresh};
use sigma_render::ink::{InkButton, InkMode};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let (w, h) = (1440, 1920);
        let dir = std::env::temp_dir().join(format!("sigma-wnav-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(w, h, dir.clone());
        app.buffer.select_all();
        app.buffer.backspace();
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
    fn touch(&mut self, phase: Phase, x: i32, y: i32) {
        self.t += 20;
        let t = self.t;
        self.send(Event::Touch { phase, x, y, t });
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
    fn put(&mut self, text: &str) {
        self.app.buffer.insert(text);
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn the_free_panel_has_the_nav_keys_and_undo_within_reach() {
    for o in [Orientation::Portrait, Orientation::Landscape] {
        for scale in [100, 150, 200] {
            let mut r = Rig::new("layout");
            r.app.change(Change::Orientation(o));
            while r.app.settings.ui_scale > scale {
                r.app.change(Change::UiScale(-1));
            }
            r.app.open_ink();
            assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Free);
            let (w, h) = (r.app.canvas.width as i32, r.app.canvas.height as i32);
            let l = r.app.panel_layout().unwrap();
            let at = format!("{o:?} {scale}%");
            for b in [
                InkButton::Home,
                InkButton::End,
                InkButton::PgUp,
                InkButton::PgDn,
                InkButton::Ins,
                InkButton::Undo,
            ] {
                let (rect, _) = l
                    .buttons
                    .iter()
                    .find(|(_, k)| *k == b)
                    .unwrap_or_else(|| panic!("{b:?} missing at {at}"));
                assert!(
                    rect.w >= 44 && rect.h >= 44,
                    "{b:?} {}x{} at {at}",
                    rect.w,
                    rect.h
                );
                assert!(
                    rect.x >= 0 && rect.x + rect.w <= w,
                    "{b:?} off the width at {at}"
                );
                assert!(
                    rect.y >= l.panel.y && rect.y + rect.h <= l.panel.y + l.panel.h,
                    "{b:?} off the panel at {at}"
                );
                assert!(
                    rect.y + rect.h <= l.area.y,
                    "{b:?} runs into the writing area at {at}"
                );
            }
            assert!(
                l.area.w >= w * 55 / 100 && l.area.h >= h / 5,
                "the writing area stays usable: {}x{} at {at}",
                l.area.w,
                l.area.h
            );
            for (i, (a, ka)) in l.buttons.iter().enumerate() {
                for (b, kb) in l.buttons.iter().skip(i + 1) {
                    let hit =
                        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
                    assert!(!hit, "{ka:?} overlaps {kb:?} at {at}");
                }
            }
        }
    }
}

#[test]
fn home_and_end_move_the_caret_on_the_line_and_page_keys_turn_pages() {
    let mut r = Rig::new("nav");
    r.put("first line\nsecond line here");
    r.tap("Write");
    r.tap("Home");
    assert_eq!(r.app.buffer.caret(), "first line\n".len());
    r.tap("End");
    assert_eq!(r.app.buffer.caret(), r.app.buffer.text().len());
    // A long note: PgUp / PgDn move the caret by a screen.
    let long = (0..400)
        .map(|i| format!("line {i} of a long note"))
        .collect::<Vec<_>>()
        .join("\n");
    r.app.buffer.select_all();
    r.app.buffer.insert(&long);
    r.app.buffer.set_caret(0);
    r.tap("PgDn");
    let down = r.app.buffer.caret();
    assert!(down > 0, "PgDn moves the caret down");
    r.tap("PgDn");
    assert!(r.app.buffer.caret() > down);
    r.tap("PgUp");
    assert!(r.app.buffer.caret() < r.app.buffer.text().len());
    // The panel stays open and the typed text is untouched.
    assert!(r.app.ink.is_some());
    assert_eq!(r.app.buffer.text(), long);
}

#[test]
fn ins_switches_to_overwrite_and_typed_characters_replace_the_next_one() {
    let mut r = Rig::new("ins");
    r.put("abcdef");
    r.app.buffer.set_caret(1);
    r.tap("Write");
    r.tap("Ins");
    assert!(r.app.buffer.overwrite());
    r.tap(",");
    r.tap(".");
    assert_eq!(r.app.buffer.text(), "a,.def");
    r.tap("Ins");
    assert!(!r.app.buffer.overwrite());
    r.tap("!");
    assert_eq!(r.app.buffer.text(), "a,.!def");
}

#[test]
fn the_usb_insert_key_toggles_and_composing_layouts_still_insert() {
    let mut r = Rig::new("usb");
    r.put("abcd");
    r.app.buffer.set_caret(1);
    r.send(Event::Key(HwKey::Insert));
    assert!(r.app.buffer.overwrite());
    r.send(Event::Key(HwKey::Char('X')));
    assert_eq!(r.app.buffer.text(), "aXcd");
    // The status bar says so.
    assert!(
        r.app.screen.overwrite || {
            r.app.render(&mut r.dev, Refresh::Full).unwrap();
            r.app.screen.overwrite
        }
    );
    // Korean composes: its letters are inserted, not overwritten.
    r.app.settings.kb_layout = sigma_core::layouts::KeyLayout::Ko;
    r.app.keyboard.lang = sigma_core::layouts::KeyLayout::Ko;
    r.app.buffer.set_caret(1);
    let before = r.app.buffer.text().chars().count();
    r.send(Event::Key(HwKey::Char('r')));
    assert_eq!(r.app.buffer.text().chars().count(), before + 1);
    r.send(Event::Key(HwKey::Insert));
    assert!(!r.app.buffer.overwrite());
}

#[test]
fn undo_takes_back_the_last_stroke_then_the_text_typed_by_the_panel() {
    let mut r = Rig::new("undo");
    r.put("start ");
    r.tap("Write");
    r.tap(",");
    r.tap("?");
    assert_eq!(r.app.buffer.text(), "start ,?");
    // Text first: each tap above is a punctuation step.
    r.tap("Undo");
    assert_eq!(r.app.buffer.text(), "start ,");
    r.tap("Undo");
    assert_eq!(r.app.buffer.text(), "start ");
    // A pending stroke goes before any text.
    let a = r.app.letter_box(0).unwrap();
    let (x, y) = (a.x + a.w / 4, a.y + a.h / 2);
    r.touch(Phase::Down, x, y);
    r.touch(Phase::Move, x + 80, y + 10);
    r.touch(Phase::Up, x + 80, y + 10);
    assert_eq!(r.app.ink.as_ref().unwrap().ink.strokes.len(), 1);
    r.tap("Undo");
    assert_eq!(r.app.ink.as_ref().unwrap().ink.strokes.len(), 0);
    assert_eq!(r.app.buffer.text(), "start ");
    // The pending read does not fire for the undone stroke.
    r.tick(2000);
    assert_eq!(r.app.buffer.text(), "start ");
}

#[test]
fn the_letters_panel_has_a_second_row_with_the_nav_keys() {
    for o in [Orientation::Portrait, Orientation::Landscape] {
        let mut r = Rig::new("letters");
        r.app.change(Change::Orientation(o));
        r.app.change(Change::InkBoxes(true));
        r.app.open_ink();
        assert_eq!(r.app.ink.as_ref().unwrap().mode, InkMode::Letters);
        let l = r.app.panel_layout().unwrap();
        for b in [
            InkButton::Home,
            InkButton::End,
            InkButton::PgUp,
            InkButton::PgDn,
            InkButton::Ins,
            InkButton::Undo,
        ] {
            let (rect, _) = l
                .buttons
                .iter()
                .find(|(_, k)| *k == b)
                .unwrap_or_else(|| panic!("{b:?} missing in {o:?}"));
            assert!(rect.w >= 44 && rect.h >= 44);
            assert!(
                rect.x + rect.w <= r.app.canvas.width as i32,
                "{b:?} off the width in {o:?}"
            );
            assert!(
                rect.y + rect.h <= l.area.y,
                "{b:?} runs into the boxes in {o:?}"
            );
        }
        assert!(
            l.boxes.iter().all(|b| b.w >= 100 && b.h >= 100),
            "boxes stay usable in {o:?}"
        );
    }
}

/// v1.13: the title bar keeps working while a panel is open: Options
/// closes the panel and opens Settings, in the Write panel and in boxes.
#[test]
fn the_title_bar_stays_live_with_the_panel_open() {
    for boxes in [false, true] {
        let mut r = Rig::new(if boxes { "bar-boxes" } else { "bar-free" });
        r.app.change(Change::InkBoxes(boxes));
        let (x, y) = r.app.control_centre("Options").expect("Options");
        r.app.open_ink();
        assert!(r.app.panel_layout().is_some());
        r.touch(Phase::Down, x, y);
        r.touch(Phase::Up, x, y);
        assert!(
            r.app.panel_layout().is_none(),
            "panel closed, boxes={boxes}"
        );
        assert!(r.app.options, "Settings open, boxes={boxes}");
    }
}

/// v1.13: letter boxes: Words on the left, Train and Done on the right,
/// the edit keys centred between them.
#[test]
fn letter_boxes_put_words_left_train_done_right_keys_centre() {
    let mut r = Rig::new("boxes-layout");
    r.app.change(Change::InkBoxes(true));
    r.app.open_ink();
    let l = r.app.panel_layout().unwrap();
    let at = |b: InkButton| l.buttons.iter().find(|(_, k)| *k == b).unwrap().0;
    let (mode, done, train) = (
        at(InkButton::Mode),
        at(InkButton::Cancel),
        at(InkButton::Train),
    );
    assert!(mode.x < 100 && done.x + done.w > l.panel.w - 100);
    assert!(train.x > l.panel.w / 2 && done.x > train.x);
    let keys = [
        InkButton::Undo,
        InkButton::Redo,
        InkButton::Back,
        InkButton::Enter,
        InkButton::Space,
    ];
    let mid: Vec<_> = keys.iter().map(|b| at(*b)).collect();
    assert!(
        mid.windows(2).all(|p| p[0].x < p[1].x),
        "Undo, Redo, Erase, Return order"
    );
    let (lo, hi) = (mid[0].x, mid[4].x + mid[4].w);
    assert!(lo > mode.x + mode.w && hi < train.x);
    assert!(((lo + hi) / 2 - l.panel.w / 2).abs() < 80, "centred");
}
