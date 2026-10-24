//! v1.13: Settings → Enhanced keyboard. Off = the Kobo style board (five
//! rows, letters / symbols / accents, no options on it); on = Compact / Full
//! with no mention of the Kobo board.

use sigma_core::buffer::TabKind;
use sigma_core::keyboard::{Dir, Key, KeyAction, Keyboard, Layer, Style};
use sigma_core::layouts::{KeyLayout, ALL};
use sigma_core::settings::{Change, Orientation, Settings};
use sigma_core::Buffer;
use sigma_scribe::App;

fn kobo(lang: KeyLayout) -> Keyboard {
    Keyboard {
        style: Style::Kobo,
        lang,
        ..Keyboard::default()
    }
}

fn press(k: &mut Keyboard, b: &mut Buffer, key: Key) -> KeyAction {
    k.press(key, b, TabKind::default())
}

fn label_key(k: &Keyboard, label: &str) -> Key {
    k.rows()
        .into_iter()
        .flatten()
        .find(|&key| k.label(key) == label)
        .unwrap_or_else(|| panic!("no key {label:?} on {:?}", k.layer))
}

#[test]
fn every_layer_has_five_rows_ten_units_wide_and_no_options() {
    for lang in ALL.iter().copied().filter(|l| l.has_compact()) {
        for layer in [Layer::Letters, Layer::Symbols, Layer::Accents] {
            for shift in [false, true] {
                let k = Keyboard {
                    layer,
                    shift,
                    ..kobo(lang)
                };
                let rows = k.rows();
                assert_eq!(rows.len(), 5, "{lang:?} {layer:?}");
                for (r, row) in rows.iter().enumerate() {
                    let w: f32 = row
                        .iter()
                        .enumerate()
                        .map(|(c, &key)| k.units(key, r, c))
                        .sum();
                    // A row of fewer letters than ten (Greek, Hebrew) is
                    // centred; none is wider than the board.
                    assert!(
                        w <= 10.01 && (r == 4 || w > 7.9),
                        "{lang:?} {layer:?} row {r}: {w} units"
                    );
                    // Nothing but letters, Shift, Back, layer keys, Space,
                    // ← → and Return.
                    for key in row {
                        assert!(
                            matches!(
                                key,
                                Key::Char(_)
                                    | Key::Shift
                                    | Key::Back
                                    | Key::Layer(_)
                                    | Key::Space
                                    | Key::Arrow(Dir::Left | Dir::Right)
                                    | Key::Return
                            ),
                            "{lang:?} {layer:?}: {key:?} is an option"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn the_enhanced_boards_never_mention_the_kobo_board() {
    for layout in [
        sigma_core::keyboard::Layout::Compact,
        sigma_core::keyboard::Layout::Full,
    ] {
        let k = Keyboard::with_layout(layout);
        assert_eq!(k.style, Style::Enhanced);
        for key in k.rows().into_iter().flatten() {
            assert!(!matches!(key, Key::Layer(_)), "{layout:?}: {key:?}");
            assert!(!k.label(key).contains("ÉÀÇ") && !k.label(key).contains("#&+"));
        }
    }
}

#[test]
fn the_layers_type_what_they_show() {
    let mut k = kobo(KeyLayout::Us);
    let mut b = Buffer::new("");
    // Letters, one-shot Shift, digits and "@" unaffected by Shift.
    let h = label_key(&k, "h");
    press(&mut k, &mut b, h);
    press(&mut k, &mut b, Key::Shift);
    let up_i = label_key(&k, "I");
    press(&mut k, &mut b, up_i);
    press(&mut k, &mut b, Key::Shift);
    let five = label_key(&k, "5");
    press(&mut k, &mut b, five);
    let at = label_key(&k, "@");
    press(&mut k, &mut b, at);
    assert_eq!(b.text(), "hI5@");
    // Symbols.
    press(&mut k, &mut b, Key::Layer(Layer::Symbols));
    assert_eq!(k.layer, Layer::Symbols);
    for l in ["€", "¿", "«"] {
        let key = label_key(&k, l);
        press(&mut k, &mut b, key);
    }
    assert_eq!(b.text(), "hI5@€¿«");
    // Accents: lower case, then Shift gives upper case for one key.
    press(&mut k, &mut b, Key::Layer(Layer::Accents));
    let e = label_key(&k, "é");
    press(&mut k, &mut b, e);
    press(&mut k, &mut b, Key::Shift);
    let up_n = label_key(&k, "Ñ");
    press(&mut k, &mut b, up_n);
    let n = label_key(&k, "ñ");
    press(&mut k, &mut b, n);
    assert!(b.text().ends_with("éÑñ"), "{}", b.text());
    // Back to letters; Back, arrows, Return act on the note.
    press(&mut k, &mut b, Key::Layer(Layer::Letters));
    assert_eq!(k.layer, Layer::Letters);
    press(&mut k, &mut b, Key::Back);
    press(&mut k, &mut b, Key::Arrow(Dir::Left));
    press(&mut k, &mut b, Key::Return);
    assert!(b.text().contains('\n'));
}

#[test]
fn the_bottom_row_offers_the_two_other_layers() {
    let mut k = kobo(KeyLayout::Us);
    let bottom = |k: &Keyboard| {
        k.rows()[4]
            .iter()
            .filter_map(|key| match key {
                Key::Layer(l) => Some(*l),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(bottom(&k), [Layer::Symbols, Layer::Accents]);
    k.layer = Layer::Symbols;
    assert_eq!(bottom(&k), [Layer::Letters, Layer::Accents]);
    k.layer = Layer::Accents;
    assert_eq!(bottom(&k), [Layer::Symbols, Layer::Letters]);
    assert_eq!(k.label(Key::Layer(Layer::Symbols)), "#&+");
    assert_eq!(k.label(Key::Layer(Layer::Accents)), "ÉÀÇ");
    assert_eq!(k.label(Key::Layer(Layer::Letters)), "ABC");
}

#[test]
fn the_setting_chooses_the_board_and_layouts_that_do_not_fit_keep_full() {
    let s = Settings::default();
    assert!(s.enhanced_keyboard, "enhanced is the default");
    assert!(!s.kobo_keyboard());
    let mut s = s;
    s.apply(Change::EnhancedKeyboard(false));
    assert!(s.kobo_keyboard());
    s.experimental = true;
    s.kb_layout = KeyLayout::Ja;
    assert!(!s.kobo_keyboard(), "Japanese has no room on the Kobo board");
    let again = Settings::parse(&s.serialize());
    assert!(!again.enhanced_keyboard);
    assert!(Settings::parse("keyboard=compact\n").enhanced_keyboard);
}

#[test]
fn the_app_switches_boards_and_the_geometry_holds_everywhere() {
    for o in [Orientation::Portrait, Orientation::Landscape] {
        for scale in [100, 150, 200] {
            let dir = std::env::temp_dir().join(format!("sigma-kobo-kb-{o:?}-{scale}"));
            let _ = std::fs::remove_dir_all(&dir);
            let mut app = App::new(1440, 1920, &dir);
            app.change(Change::Orientation(o));
            while app.settings.ui_scale > scale {
                app.change(Change::UiScale(-1));
            }
            let enhanced_rows = app.keyboard.rows().len();
            app.change(Change::EnhancedKeyboard(false));
            assert_eq!(app.keyboard.style, Style::Kobo);
            let l = app.screen.layout(true, &app.keyboard);
            let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
            assert!(l.kb_buttons.is_empty(), "no bar on the Kobo board ({o:?})");
            assert_eq!(l.keys.iter().filter(|(_, k)| *k == Key::Return).count(), 1);
            let cap = h * if w > h { 45 } else { 42 } / 100;
            assert!(
                l.keyboard.h <= cap,
                "{o:?} {scale}%: {} > {cap}",
                l.keyboard.h
            );
            for (r, key) in &l.keys {
                assert!(r.w >= 22 && r.h >= 44, "{key:?} {}x{}", r.w, r.h);
                assert!(r.x >= 0 && r.x + r.w <= w, "{key:?} off the width");
                assert!(r.y >= l.keyboard.y && r.y + r.h <= l.keyboard.y + l.keyboard.h);
            }
            for (i, (a, ka)) in l.keys.iter().enumerate() {
                for (b, kb) in l.keys.iter().skip(i + 1) {
                    let hit =
                        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
                    assert!(!hit, "{ka:?} overlaps {kb:?} ({o:?} {scale}%)");
                }
            }
            // Back to the enhanced board: the saved Compact / Full choice.
            app.change(Change::EnhancedKeyboard(true));
            assert_eq!(app.keyboard.style, Style::Enhanced);
            assert_eq!(app.keyboard.rows().len(), enhanced_rows);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
