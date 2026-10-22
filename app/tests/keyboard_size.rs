//! v1.4.3 Keyboard size: decoupled from the UI scale, Auto caps at 150 %.

use sigma_core::settings::{Change, Orientation, Settings, KB_SIZES, UI_SCALES};
use sigma_render::i18n::{self, Lang};
use sigma_render::options::{choices, CATEGORIES};
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-kbsize-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn set_ui(app: &mut App, want: u16) {
    for _ in 0..UI_SCALES.len() {
        app.change(Change::UiScale(-1));
    }
    while app.settings.ui_scale < want {
        app.change(Change::UiScale(1));
    }
}

#[test]
fn default_is_auto_and_old_files_load_as_auto() {
    assert_eq!(Settings::default().kb_size, 0);
    assert_eq!(Settings::default().keyboard_scale(), 150);
    // A 1.4.2 file has no keyboard_size key.
    let old = "keyboard=full\nui_scale=200\nlayout=us\n";
    let s = Settings::parse(old);
    assert_eq!(s.kb_size, 0);
    assert_eq!(s.ui_scale, 200);
    assert_eq!(Settings::parse("keyboard_size=133\n").kb_size, 0);
    assert_eq!(Settings::parse("keyboard_size=x\n").kb_size, 0);
}

#[test]
fn every_size_round_trips() {
    for v in KB_SIZES {
        let mut s = Settings::default();
        s.apply(Change::KbSize(v));
        assert_eq!(Settings::parse(&s.serialize()).kb_size, v);
    }
    assert!(Settings::default()
        .serialize()
        .contains("keyboard_size=auto\n"));
}

#[test]
fn auto_follows_ui_scale_up_to_150() {
    let mut s = Settings::default();
    for (ui, want) in [(100, 100), (125, 125), (150, 150), (175, 150), (200, 150)] {
        s.ui_scale = ui;
        assert_eq!(s.keyboard_scale(), want);
        for v in [100, 125, 150] {
            s.kb_size = v;
            assert_eq!(s.keyboard_scale(), v, "fixed ignores UI scale");
        }
        s.kb_size = 0;
    }
}

#[test]
fn keyboard_at_200_ui_is_no_bigger_than_at_150() {
    let dir = fresh("cap");
    let mut app = App::new(1440, 1920, &dir);
    for o in [Orientation::Portrait, Orientation::Landscape] {
        app.change(Change::Orientation(o));
        set_ui(&mut app, 150);
        let at150 = app.screen.layout(true, &app.keyboard).keyboard.h;
        set_ui(&mut app, 200);
        let l = app.screen.layout(true, &app.keyboard);
        assert_eq!(l.keyboard.h, at150, "{o:?}");
        // The rest of the interface still grows.
        assert!(l.title.h > 0);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn every_size_fits_at_every_ui_scale_language_and_orientation() {
    for lang in [Lang::En, Lang::Fr, Lang::Pt, Lang::PseudoLong] {
        let dir = fresh(&format!("fit{lang:?}"));
        let mut app = App::new(1440, 1920, &dir);
        app.set_language(lang);
        for &ui in &UI_SCALES {
            set_ui(&mut app, ui);
            for v in KB_SIZES {
                app.change(Change::KbSize(v));
                for o in [Orientation::Portrait, Orientation::Landscape] {
                    app.change(Change::Orientation(o));
                    let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
                    let min = app.min_target();
                    assert!(min >= 44);
                    let tag = format!("{lang:?} ui {ui}% kb {v} {o:?}");
                    for (n, r) in app.touch_targets() {
                        assert!(r.w >= min && r.h >= min, "{tag}: {n} {r:?}");
                        assert!(
                            r.x >= 0 && r.y >= 0 && r.x + r.w <= w && r.y + r.h <= h,
                            "{tag}: {n} {r:?} off screen"
                        );
                    }
                    let l = app.screen.layout(true, &app.keyboard);
                    let cap = if w > h { 45 } else { 42 };
                    assert!(l.keyboard.h <= h * cap / 100, "{tag}: kb {}", l.keyboard.h);
                    assert!(l.page.h >= 500, "{tag}: page {}", l.page.h);
                    for &(r, _) in &l.keys {
                        assert!(r.h >= 44 && r.x + r.w <= w, "{tag}: key {r:?}");
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }
    i18n::set_lang(Lang::En);
}

#[test]
fn keyboard_size_is_auto_plus_slider_in_keyboard_and_input() {
    let s = Settings::default();
    // `choices` still lists the values (Auto first) for the slider's stops.
    let labels: Vec<_> = choices(&s, "Keyboard size")
        .into_iter()
        .map(|x| x.0)
        .collect();
    assert_eq!(labels, ["Auto", "100%", "125%", "150%"]);
    let kb = CATEGORIES
        .iter()
        .find(|c| c.0 == "Keyboard & input")
        .unwrap();
    assert!(kb
        .1
        .iter()
        .any(|(_, rows)| rows == &["Auto keyboard size", "Keyboard size"]));
}
