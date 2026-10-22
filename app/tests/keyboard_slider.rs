//! v1.4.4 Keyboard size: Auto checkbox + stepped slider (S14.4).

use sigma_core::settings::{Change, Orientation, UI_SCALES};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Phase, Refresh};
use sigma_render::i18n::{self, Lang};
use sigma_render::options::CATEGORIES;
use sigma_render::SettingsPage;
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-kbslider-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn kb_page(app: &mut App) {
    app.open_options();
    let i = CATEGORIES
        .iter()
        .position(|c| c.0 == "Keyboard & input")
        .unwrap();
    app.settings_page = SettingsPage::Category(i);
    app.settings_sub = 0;
}

fn at(app: &App, name: &str) -> (i32, i32) {
    app.control_centre(name)
        .unwrap_or_else(|| panic!("no control {name}"))
}

fn tap(app: &mut App, name: &str) {
    let (x, y) = at(app, name);
    app.handle(Event::Tap { x, y });
}

fn touch(app: &mut App, phase: Phase, x: i32, y: i32, t: u64) {
    app.handle(Event::Touch { phase, x, y, t });
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
fn auto_on_disables_the_slider_and_shows_the_auto_size() {
    let mut app = App::new(1440, 1920, fresh("auto"));
    kb_page(&mut app);
    assert_eq!(app.settings.kb_size, 0);
    // Taps on the disabled slider do nothing.
    tap(&mut app, "Keyboard size/100%");
    assert_eq!(app.settings.kb_size, 0);
    let (x, y) = at(&app, "Keyboard size/100%");
    touch(&mut app, Phase::Down, x, y, 10);
    touch(&mut app, Phase::Up, x, y, 60);
    assert_eq!(app.settings.kb_size, 0);
    // Left / Right do nothing either.
    tap(&mut app, "Keyboard size");
    app.handle(Event::Key(HwKey::Left));
    assert_eq!(app.settings.kb_size, 0);
}

#[test]
fn turning_auto_off_keeps_the_size_then_the_slider_sets_it() {
    let mut app = App::new(1440, 1920, fresh("off"));
    set_ui(&mut app, 125);
    kb_page(&mut app);
    tap(&mut app, "Auto keyboard size/Off");
    assert_eq!(app.settings.kb_size, 125, "keeps Auto's 125 %");
    tap(&mut app, "Keyboard size/150%");
    assert_eq!(app.settings.kb_size, 150);
    // Tap anywhere snaps to the nearest dot.
    let (x0, y) = at(&app, "Keyboard size/100%");
    let (x1, _) = at(&app, "Keyboard size/125%");
    app.handle(Event::Tap {
        x: x0 + (x1 - x0) / 3,
        y,
    });
    assert_eq!(app.settings.kb_size, 100);
    // Drag: down on 100 %, move past 125 %, lift near 150 %.
    let (x2, _) = at(&app, "Keyboard size/150%");
    touch(&mut app, Phase::Down, x0, y, 100);
    touch(&mut app, Phase::Move, x1 + 5, y + 200, 140);
    assert_eq!(
        app.settings.kb_size, 125,
        "snaps while dragging, off the row too"
    );
    touch(&mut app, Phase::Up, x2 - 10, y, 180);
    assert_eq!(app.settings.kb_size, 150);
    // Arrows with focus.
    app.handle(Event::Key(HwKey::Left));
    assert_eq!(app.settings.kb_size, 125);
    app.handle(Event::Key(HwKey::Left));
    app.handle(Event::Key(HwKey::Left));
    assert_eq!(app.settings.kb_size, 100, "clamps");
    app.handle(Event::Key(HwKey::Right));
    assert_eq!(app.settings.kb_size, 125);
    // Focus goes when you tap elsewhere.
    tap(&mut app, "Keyboard/Compact");
    app.handle(Event::Key(HwKey::Right));
    assert_eq!(app.settings.kb_size, 125);
    // Auto back on.
    tap(&mut app, "Auto keyboard size/On");
    assert_eq!(app.settings.kb_size, 0);
    assert_eq!(app.settings.keyboard_scale(), 125);
}

#[test]
fn keyboard_page_fits_every_language_orientation_and_ui_scale() {
    for l in [Lang::En, Lang::Fr, Lang::Pt, Lang::PseudoLong] {
        let mut dev = DesktopDevice::new(1440, 1920);
        let mut app = App::new(1440, 1920, fresh(&format!("fit{l:?}")));
        app.set_language(l);
        for &ui in &UI_SCALES {
            set_ui(&mut app, ui);
            for auto in [true, false] {
                app.change(Change::KbAuto(auto));
                for o in [Orientation::Portrait, Orientation::Landscape] {
                    app.change(Change::Orientation(o));
                    kb_page(&mut app);
                    let tag = format!("{l:?} {ui}% auto={auto} {o:?}");
                    let mut seen = false;
                    loop {
                        app.render(&mut dev, Refresh::Full).unwrap();
                        let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
                        let min = app.min_target();
                        assert!(min >= 44);
                        for (n, r) in app.touch_targets() {
                            assert!(r.w >= min && r.h >= min, "{tag}: {n} {r:?}");
                            assert!(
                                r.x >= 0 && r.y >= 0 && r.x + r.w <= w && r.y + r.h <= h,
                                "{tag}: {n} {r:?}"
                            );
                        }
                        if let Some(r) = app
                            .touch_targets()
                            .into_iter()
                            .find(|(n, _)| n == "Keyboard size")
                        {
                            seen = true;
                            // Pure black and white inside the slider.
                            let cw = app.canvas.width as usize;
                            let px =
                                |x: i32, y: i32| app.canvas.pixels[y as usize * cw + x as usize];
                            for y in r.1.y..r.1.y + r.1.h {
                                for x in r.1.x..r.1.x + r.1.w {
                                    let p = px(x, y);
                                    assert!(p == 0 || p == 0xFF, "{tag}: grey {p}");
                                }
                            }
                            for s in ["100%", "125%", "150%"] {
                                assert!(
                                    app.control_centre(&format!("Keyboard size/{s}")).is_some(),
                                    "{tag}: {s}"
                                );
                            }
                        }
                        let before = app.settings_sub;
                        app.handle(Event::PageTurn { forward: true });
                        if app.settings_sub == before {
                            break;
                        }
                    }
                    assert!(seen, "{tag}: slider not on the page");
                }
            }
        }
    }
    i18n::set_lang(Lang::En);
}
