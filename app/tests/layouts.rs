//! v1.3 Latin layouts: on-screen Full board, Compact letters, USB, dead keys.

use sigma_core::keyboard::{Key, Layout};
use sigma_core::layouts::{KeyLayout, ALL};
use sigma_core::settings::{Change, Orientation, Settings};
use sigma_device::usbkbd::{raw_for_char_in, KeyMapper};
use sigma_device::{Event, HwKey};
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-layouts-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn app_with(tag: &str, l: KeyLayout) -> (App, std::path::PathBuf) {
    let dir = fresh(tag);
    let mut app = App::new(1440, 1920, &dir);
    while app.settings.kb_layout != l {
        app.change(Change::KbLayout(1));
    }
    app.buffer = sigma_core::Buffer::new("");
    (app, dir)
}

fn tap(app: &mut App, label: &str) {
    let (x, y) = app
        .key_centre(label)
        .unwrap_or_else(|| panic!("no key {label:?}"));
    app.handle(Event::Tap { x, y });
}

#[test]
fn setting_steps_wraps_and_round_trips() {
    let mut s = Settings::default();
    assert_eq!(s.kb_layout, KeyLayout::Us);
    s.apply(Change::KbLayout(-1));
    assert_eq!(s.kb_layout, ALL[ALL.len() - 1], "wraps");
    for l in ALL {
        s.kb_layout = l;
        assert_eq!(Settings::parse(&s.serialize()).kb_layout, l);
    }
    assert_eq!(Settings::parse("layout=xx\n").kb_layout, KeyLayout::Us);
}

#[test]
fn every_layout_fits_and_hits_in_both_orientations() {
    for l in ALL {
        for kb in [Layout::Full, Layout::Compact] {
            for o in [Orientation::Portrait, Orientation::Landscape] {
                let (mut app, dir) = app_with(&format!("fit{l:?}{kb:?}{o:?}"), l);
                app.change(Change::Keyboard(kb));
                app.change(Change::Orientation(o));
                let lay = app.screen.layout(true, &app.keyboard);
                let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
                let min = app.min_target();
                for (i, (a, ka)) in lay.keys.iter().enumerate() {
                    assert!(a.w >= min && a.h >= min, "{l:?} {kb:?} {o:?} {ka:?} {a:?}");
                    assert!(a.x >= 0 && a.y >= 0 && a.x + a.w <= w && a.y + a.h <= h);
                    for (b, _) in &lay.keys[i + 1..] {
                        assert!(
                            a.x + a.w <= b.x
                                || b.x + b.w <= a.x
                                || a.y + a.h <= b.y
                                || b.y + b.h <= a.y,
                            "{l:?} overlap {a:?} {b:?}"
                        );
                    }
                }
                // Full rows still add up to the grid width.
                if kb == Layout::Full {
                    for (r, row) in app.keyboard.rows().iter().enumerate() {
                        let u: f32 = row
                            .iter()
                            .enumerate()
                            .map(|(c, k)| app.keyboard.units(*k, r, c))
                            .sum();
                        assert!((u - 16.0).abs() < 0.01, "{l:?} row {r}: {u}");
                    }
                }
                let _ = std::fs::remove_dir_all(dir);
            }
        }
    }
}

#[test]
fn azerty_and_qwertz_letters_sit_in_place_on_screen() {
    let (app, dir) = app_with("pos", KeyLayout::Fr);
    let rows = app.keyboard.rows();
    assert_eq!(rows[1][1], Key::Char('a'), "AZERTY top row");
    assert_eq!(rows[3][1], Key::Char('<'), "ISO key");
    assert_eq!(rows[0][3], Key::Char('é'));
    let _ = std::fs::remove_dir_all(dir);
    let (mut app, dir) = app_with("posde", KeyLayout::De);
    assert_eq!(app.keyboard.rows()[1][6], Key::Char('z'));
    app.change(Change::Keyboard(Layout::Compact));
    assert_eq!(app.keyboard.rows()[0][5], Key::Char('z'), "Compact QWERTZ");
    assert_eq!(app.keyboard.rows()[2][1], Key::Char('y'));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn on_screen_dead_keys_shift_and_altgr() {
    let (mut app, dir) = app_with("osk", KeyLayout::Fr);
    // ^ then e → ê; é is its own key; Shift+é → 2.
    tap(&mut app, "^");
    assert_eq!(app.buffer.text(), "", "dead key waits");
    tap(&mut app, "e");
    tap(&mut app, "é");
    tap(&mut app, "Shift");
    tap(&mut app, "2");
    // Alt is AltGr on characters: Alt + e → €.
    tap(&mut app, "Alt");
    tap(&mut app, "€");
    // A dead key before Space writes the accent.
    tap(&mut app, "^");
    tap(&mut app, "Space");
    assert_eq!(app.buffer.text(), "êé2€^");
    let _ = std::fs::remove_dir_all(dir);
    let (mut app, dir) = app_with("oskde", KeyLayout::De);
    tap(&mut app, "ß");
    tap(&mut app, "Alt");
    tap(&mut app, "@");
    tap(&mut app, "´");
    tap(&mut app, "Enter");
    assert_eq!(app.buffer.text(), "ß@´\n", "Enter flushes the accent");
    let _ = std::fs::remove_dir_all(dir);
}

fn usb_type(app: &mut App, l: KeyLayout, text: &str) {
    let mut m = KeyMapper::with_layout(l);
    for c in text.chars() {
        let raws = raw_for_char_in(l, c);
        assert!(!raws.is_empty(), "{l:?} cannot type {c:?}");
        for r in raws {
            if let Some(e) = m.feed(r) {
                app.handle(e);
            }
        }
    }
}

#[test]
fn usb_types_every_layout_including_dead_keys() {
    for (l, text) in [
        (KeyLayout::Us, "Hello, world! [x] {y}"),
        (KeyLayout::Uk, "£5 \"quoted\" @home #1"),
        (KeyLayout::Fr, "Où êtes-vous ? à l'été, 5 € ! ç"),
        (KeyLayout::De, "Grüße aus Köln: {ä} @ 3 €"),
        (KeyLayout::Es, "¿Qué tal? ¡Señor Muñoz! ü"),
        (KeyLayout::Pt, "Coração, ação, à, você, ü"),
        (KeyLayout::It, "Perché è già più così? @#"),
        (KeyLayout::UsIntl, "Café, naïve, São Paulo, ñ ß"),
        (KeyLayout::Be, "Où êtes-vous ? @ 5 € ç"),
        (KeyLayout::Dk, "Rødgrød med fløde, Århus @ é"),
        (KeyLayout::No, "Blåbærsyltetøy, Ærø @ é"),
        (KeyLayout::Se, "Hyvää yötä, Åsa! @ é"),
        (KeyLayout::Pl, "Zażółć gęślą jaźń, ŁÓDŹ Ąę"),
        (KeyLayout::Cz, "Příliš žluťoučký kůň úpěl ódy"),
        (KeyLayout::Ro, "Știință, țară, mâine, în ăsta"),
        (KeyLayout::Tr, "Iğdır, İstanbul, şöyle çok güzel"),
        (KeyLayout::Latam, "¿Qué tal? ¡Señor! @ ü"),
        (KeyLayout::Br, "Coração, ação, você, à, ü / ?"),
    ] {
        let (mut app, dir) = app_with(&format!("usb{l:?}"), l);
        usb_type(&mut app, l, text);
        assert_eq!(app.buffer.text(), text, "{l:?}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn usb_ctrl_shortcuts_keep_their_letter() {
    let (mut app, dir) = app_with("ctrl", KeyLayout::Fr);
    usb_type(&mut app, KeyLayout::Fr, "abc");
    let mut m = KeyMapper::with_layout(KeyLayout::Fr);
    // The AZERTY Z key is evdev 17 (W on US). Ctrl+it undoes.
    let ev = |code, value| sigma_device::evdev::RawEvent {
        kind: sigma_device::evdev::EV_KEY,
        code,
        value,
    };
    let mut out = Vec::new();
    for r in [ev(29, 1), ev(17, 1), ev(17, 0), ev(29, 0)] {
        out.extend(m.feed(r));
    }
    assert!(
        matches!(
            out[0],
            Event::Chord {
                key: HwKey::Char('z'),
                ..
            }
        ),
        "{out:?}"
    );
    for e in out {
        app.handle(e);
    }
    assert_ne!(app.buffer.text(), "abc", "Ctrl+Z undid typing");
    let _ = std::fs::remove_dir_all(dir);
}
