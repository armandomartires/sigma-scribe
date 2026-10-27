//! v1.12: Settings → Experimental features. Asian and right-to-left
//! layouts, Word mode and Fast ink show and work only while it is on; it is
//! off by default and on for anyone who already used one of them.

use sigma_core::layouts::KeyLayout;
use sigma_core::settings::{Change, Settings};
use sigma_render::options::{display_row, page_items, Item, Kind, OptionsModel, SettingsPage};

/// A row label as `&'static str` (a test may leak the odd owned one).
fn st(l: std::borrow::Cow<'static, str>) -> &'static str {
    match l {
        std::borrow::Cow::Borrowed(s) => s,
        std::borrow::Cow::Owned(s) => Box::leak(s.into_boxed_str()),
    }
}

fn rows(s: &Settings, page: SettingsPage) -> Vec<&'static str> {
    let kb = sigma_core::keyboard::Keyboard::default();
    let m = OptionsModel {
        settings: s,
        theme: sigma_render::theme::Theme::DAY,
        keyboard: &kb,
        orientation_status: "Auto",
        page,
        sub: 0,
        language: "English",
        version: "test",
        focus: None,
        hw_samples: 0,
        light: None,
        ui_scale_pending: None,
        plugins: &[],
        repository: &[],
        repo_note: "",
    };
    page_items(&m)
        .into_iter()
        .filter_map(|i| match i {
            Item::Row(r) => Some(st(r.label)),
            _ => None,
        })
        .collect()
}

#[test]
fn it_is_off_by_default_and_hides_what_it_guards() {
    let s = Settings::default();
    assert!(!s.experimental);
    let layouts = s.layouts();
    assert!(layouts.contains(&KeyLayout::Us) && layouts.contains(&KeyLayout::ViTelex));
    for l in [
        KeyLayout::Ja,
        KeyLayout::ZhHans,
        KeyLayout::ZhHant,
        KeyLayout::Ko,
        KeyLayout::Ar,
        KeyLayout::Fa,
        KeyLayout::Ur,
        KeyLayout::He,
        KeyLayout::Deva,
        KeyLayout::Bn,
        KeyLayout::Ta,
        KeyLayout::Th,
    ] {
        assert!(!layouts.contains(&l), "{l:?} is experimental");
    }
    // The Handwriting rows for Fast ink and Word mode are not shown.
    let kb = rows(&s, SettingsPage::Category(2));
    assert!(!kb.contains(&"Fast ink (A2)") && !kb.contains(&"Word mode (experimental)"));
    assert!(kb.contains(&"Letter boxes"));
    // The switch itself is on its own page.
    assert_eq!(
        rows(&s, SettingsPage::Category(5)),
        ["Experimental features"]
    );
    // Word mode and Fast ink do nothing while it is off.
    let w = Settings {
        ink_words: true,
        ink_a2: true,
        ..s
    };
    assert!(!w.word_mode() && !w.fast_ink());
}

#[test]
fn turning_it_on_shows_everything_and_off_puts_a_layout_back() {
    let mut s = Settings::default();
    s.apply(Change::Experimental(true));
    assert_eq!(s.layouts().len(), sigma_core::layouts::ALL.len());
    let kb = rows(&s, SettingsPage::Category(2));
    assert!(kb.contains(&"Fast ink (A2)") && kb.contains(&"Word mode (experimental)"));
    s.kb_layout = KeyLayout::Ja;
    s.apply(Change::Experimental(false));
    assert_eq!(
        s.kb_layout,
        KeyLayout::Us,
        "an experimental layout goes back to US"
    );
    // Stepping through the layouts skips the hidden ones.
    for _ in 0..40 {
        s.apply(Change::KbLayout(1));
        assert!(!s.kb_layout.experimental());
    }
}

#[test]
fn old_settings_files_keep_what_they_used_and_the_choice_is_saved() {
    // No experimental= line, but a Japanese layout: the switch starts on.
    let old = Settings::parse("keyboard=compact\nlayout=ja\n");
    assert!(old.experimental);
    assert_eq!(old.kb_layout, KeyLayout::Ja);
    let old = Settings::parse("keyboard=compact\nink_words=on\n");
    assert!(old.experimental);
    // Nothing experimental in use: it starts off.
    assert!(!Settings::parse("keyboard=compact\nlayout=fr\n").experimental);
    // An explicit off wins over a stray experimental layout.
    let off = Settings::parse("experimental=off\nlayout=ko\n");
    assert!(!off.experimental);
    assert_eq!(off.kb_layout, KeyLayout::Us);
    // Round trip.
    let on = Settings {
        experimental: true,
        ..Settings::default()
    };
    assert!(Settings::parse(&on.serialize()).experimental);
    assert!(!Settings::parse(&Settings::default().serialize()).experimental);
}

#[test]
fn the_switch_is_a_checkbox_row() {
    let s = Settings::default();
    let kb = sigma_core::keyboard::Keyboard::default();
    let m = OptionsModel {
        settings: &s,
        theme: sigma_render::theme::Theme::DAY,
        keyboard: &kb,
        orientation_status: "Auto",
        page: SettingsPage::Category(5),
        sub: 0,
        language: "English",
        version: "test",
        focus: None,
        hw_samples: 0,
        light: None,
        ui_scale_pending: None,
        plugins: &[],
        repository: &[],
        repo_note: "",
    };
    let row = display_row(&m, "Experimental features").unwrap();
    assert!(matches!(row.kind, Kind::Check { on: false, .. }));
}

// ---- v1.12: Display → Backlight -------------------------------------

use sigma_core::settings::BACKLIGHT_MAX;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Device, Event, Phase, Refresh};
use sigma_scribe::App;

fn display_rows(light: Option<u8>) -> Vec<(&'static str, Option<usize>)> {
    let s = Settings::default();
    let kb = sigma_core::keyboard::Keyboard::default();
    let m = OptionsModel {
        settings: &s,
        theme: sigma_render::theme::Theme::DAY,
        keyboard: &kb,
        orientation_status: "Auto",
        page: SettingsPage::Category(1),
        sub: 0,
        language: "English",
        version: "test",
        focus: None,
        hw_samples: 0,
        light,
        ui_scale_pending: None,
        plugins: &[],
        repository: &[],
        repo_note: "",
    };
    page_items(&m)
        .into_iter()
        .filter_map(|i| match i {
            Item::Row(r) => Some((
                st(r.label.clone()),
                match r.kind {
                    Kind::Slider { stops, value, .. } if r.label == "Backlight" => {
                        assert_eq!(stops.len(), usize::from(BACKLIGHT_MAX) + 1, "nine stops");
                        Some(value)
                    }
                    _ => None,
                },
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn the_backlight_slider_is_shown_only_when_the_device_has_a_light() {
    let none = display_rows(None);
    assert!(none.iter().all(|(l, _)| *l != "Backlight"));
    let some = display_rows(Some(3));
    assert!(some.contains(&("Backlight", Some(3))));
}

#[test]
fn the_slider_sets_the_light_and_the_app_sends_it_to_the_device() {
    let dir = std::env::temp_dir().join(format!("sigma-light-app-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = App::new(1440, 1920, dir.clone());
    let mut dev = DesktopDevice::new(1440, 1920);
    dev.light = Some(2);
    app.sync_light(&mut dev);
    assert!(
        app.options_model().light == Some(2),
        "reads the light at start"
    );
    app.change(Change::Backlight(7));
    assert_eq!(app.options_model().light, Some(7));
    assert_eq!(dev.light, Some(2), "not sent until the loop syncs");
    app.sync_light(&mut dev);
    assert_eq!(dev.light, Some(7));
    // It is not a saved setting: nothing of it reaches the settings file.
    assert!(!app.settings.serialize().contains("backlight"));
    // A device without a light shows no row and takes no writes.
    let mut app2 = App::new(1440, 1920, dir.join("b"));
    let mut dev2 = DesktopDevice::new(1440, 1920);
    app2.sync_light(&mut dev2);
    assert_eq!(app2.options_model().light, None);
    assert!(dev2.light_step().is_none());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = (Event::Quit, Phase::Down, Refresh::Full);
}

/// v1.13.1: dragging the Backlight slider acts once per step, not on every
/// move event (the Forma's log showed the same step up to five times in a
/// row, each one a sysfs write).
#[test]
fn backlight_drag_acts_once_per_step() {
    use sigma_device::{Event, Phase};
    let dir = std::env::temp_dir().join(format!("sigma-light-once-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = sigma_scribe::App::new(1440, 1920, dir.clone());
    let mut dev = sigma_device::desktop::DesktopDevice::new(1440, 1920);
    dev.light = Some(2);
    app.sync_light(&mut dev);
    app.open_options();
    app.settings_page = SettingsPage::Category(1);
    let (x, y) = app.control_centre("Backlight/62 %").expect("the 62 % stop");
    let mut t = 0u64;
    let mut go = |app: &mut sigma_scribe::App, phase: Phase, dx: i32| {
        t += 30;
        app.handle(Event::Touch {
            phase,
            x: x + dx,
            y,
            t,
        })
    };
    assert!(go(&mut app, Phase::Down, 0).is_some(), "the step is set");
    assert_eq!(app.options_model().light, Some(5));
    for dx in [1, 2, 3, 2, 1] {
        assert!(
            go(&mut app, Phase::Move, dx).is_none(),
            "the same step again does nothing"
        );
    }
    assert!(go(&mut app, Phase::Up, 0).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
