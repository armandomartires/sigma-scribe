//! v1.4.2 Settings redesign: Kobo-style pages, checkboxes, choice lists,
//! paging, back/Esc, and 1.4.0 settings carried over unchanged.

use sigma_core::settings::{Orientation, Settings};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_render::i18n::{self, Lang};
use sigma_render::options::{category_of, CATEGORIES};
use sigma_render::SettingsPage;
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-set-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn tap(app: &mut App, name: &str) {
    let (x, y) = app
        .control_centre(name)
        .unwrap_or_else(|| panic!("no control {name}"));
    app.handle(Event::Tap { x, y });
}

#[test]
fn aa_opens_the_root_and_back_esc_walk_up() {
    let mut app = App::new(1440, 1920, fresh("nav"));
    tap(&mut app, "Options");
    assert!(app.options && app.settings_page == SettingsPage::Root);
    tap(&mut app, "Display");
    assert_eq!(app.settings_page, SettingsPage::Category(1));
    tap(&mut app, "Orientation");
    assert_eq!(app.settings_page, SettingsPage::Choice(1, "Orientation"));
    tap(&mut app, "Landscape");
    assert_eq!(app.settings.orientation, Orientation::Landscape);
    assert_eq!(
        app.settings_page,
        SettingsPage::Category(1),
        "a pick goes back"
    );
    tap(&mut app, "Upside down/On");
    assert!(app.settings.upside_down, "checkbox toggles");
    tap(&mut app, "Upside down/On");
    assert!(app.settings.upside_down, "already on: no-op");
    tap(&mut app, "Upside down/Off");
    assert!(!app.settings.upside_down);
    app.handle(Event::Key(HwKey::Escape));
    assert_eq!(app.settings_page, SettingsPage::Root);
    app.handle(Event::Key(HwKey::Escape));
    assert!(!app.options, "Esc at the root closes");
    tap(&mut app, "Options");
    assert_eq!(app.settings_page, SettingsPage::Root, "reopens at the root");
    tap(&mut app, "Help & about");
    tap(&mut app, "App info");
    assert!(!app.options && app.help.is_some());
}

#[test]
fn every_page_fits_at_200_percent_in_each_language_and_orientation() {
    for l in [Lang::En, Lang::Fr, Lang::Pt, Lang::PseudoLong] {
        let mut dev = DesktopDevice::new(1440, 1920);
        let mut app = App::new(1440, 1920, fresh(&format!("fit{l:?}")));
        app.set_language(l);
        for o in [Orientation::Portrait, Orientation::Landscape] {
            app.change(sigma_core::settings::Change::Orientation(o));
            app.open_options();
            let mut pages = vec![SettingsPage::Root];
            pages.extend((0..CATEGORIES.len()).map(SettingsPage::Category));
            for r in ["Orientation", "Layout", "Keyboard tone", "Tab width"] {
                pages.push(SettingsPage::Choice(category_of(r).unwrap(), r));
            }
            for p in pages {
                app.settings_page = p;
                app.settings_sub = 0;
                loop {
                    app.render(&mut dev, Refresh::Full).unwrap();
                    let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
                    let min = app.min_target();
                    for (n, r) in app.touch_targets() {
                        assert!(r.w >= min && r.h >= min, "{l:?} {o:?} {p:?}: {n} {r:?}");
                        assert!(
                            r.x >= 0 && r.y >= 0 && r.x + r.w <= w && r.y + r.h <= h,
                            "{l:?} {o:?} {p:?}: {n}"
                        );
                    }
                    let before = app.settings_sub;
                    app.handle(Event::PageTurn { forward: true });
                    if app.settings_sub == before {
                        break;
                    }
                }
            }
        }
    }
    i18n::set_lang(Lang::En);
}

#[test]
fn settings_from_1_4_0_carry_over_unchanged() {
    let dir = fresh("migrate");
    std::fs::create_dir_all(&dir).unwrap();
    let old = "keyboard=compact\npaper=night\ntype=atkinson\nline=off\nsize=22\ntab=tab\ntab_width=8\nkey_tone=dark\ndrawer=left\nusb=on\nscroll=natural\norientation=landscape\nupside_down=on\nrotation=270\nui_scale=150\nlayout=fr\n";
    std::fs::write(dir.join(".sigma-settings"), old).unwrap();
    let want = Settings::parse(old);
    let app = App::new(1440, 1920, &dir);
    assert_eq!(app.settings, want, "same keys, same values");
    assert_eq!(want.paper, sigma_core::settings::Paper::Night);
    assert!(want.usb && want.upside_down && want.natural_scroll);
    assert_eq!(want.ui_scale, 150);
    let _ = std::fs::remove_dir_all(dir);
}
