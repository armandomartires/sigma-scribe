//! v1.17: Display → Warmth, the warm-to-cool slider of the Forma's front
//! light. The slider is a request to the mixer, but a saved one: the stock
//! reader does not read the mixer back, so the step is kept in the settings
//! file and applied at start.

use sigma_core::settings::{Change, WARMTH_MAX};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Phase};
use sigma_render::options::{page_items, Item, Kind, SettingsPage};
use sigma_scribe::App;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-warmth-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn warmth_row(app: &mut App) -> Option<(usize, usize)> {
    app.open_options();
    app.settings_page = SettingsPage::Category(1);
    let m = app.options_model();
    page_items(&m).into_iter().find_map(|i| match i {
        Item::Row(r) if r.label == "Warmth" => match r.kind {
            Kind::Slider { stops, value, .. } => Some((stops.len(), value)),
            _ => None,
        },
        _ => None,
    })
}

#[test]
fn the_scales_agree() {
    assert_eq!(WARMTH_MAX, sigma_device::light::WARMTH_MAX);
}

#[test]
fn the_slider_is_shown_only_when_the_device_has_a_mixer() {
    let d = dir("shown");
    let mut app = App::new(1440, 1920, d.clone());
    let mut dev = DesktopDevice::new(1440, 1920);
    app.sync_warmth(&mut dev);
    assert_eq!(warmth_row(&mut app), None, "no mixer, no row");
    let mut app = App::new(1440, 1920, d.join("b"));
    dev.warmth = Some(4);
    app.sync_warmth(&mut dev);
    assert_eq!(
        warmth_row(&mut app),
        Some((usize::from(WARMTH_MAX) + 1, 4)),
        "eleven stops, at the step the device reads"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_slider_sets_the_mixer_and_saves_the_step() {
    let d = dir("set");
    let mut app = App::new(1440, 1920, d.clone());
    let mut dev = DesktopDevice::new(1440, 1920);
    dev.warmth = Some(2);
    app.sync_warmth(&mut dev);
    assert_eq!(app.settings.warmth, None, "nothing chosen yet");
    assert_eq!(
        dev.warmth,
        Some(2),
        "nothing chosen: the mixer is left alone"
    );
    app.change(Change::Warmth(9));
    assert_eq!(dev.warmth, Some(2), "not sent until the loop syncs");
    app.sync_warmth(&mut dev);
    assert_eq!(dev.warmth, Some(9));
    assert!(app.settings.serialize().contains("warmth=9\n"));
    assert_eq!(warmth_row(&mut app).map(|r| r.1), Some(9));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_saved_step_is_applied_at_start() {
    let d = dir("start");
    {
        let mut app = App::new(1440, 1920, d.clone());
        app.change(Change::Warmth(7));
    }
    let mut app = App::new(1440, 1920, d.clone());
    let mut dev = DesktopDevice::new(1440, 1920);
    dev.warmth = Some(0);
    app.sync_warmth(&mut dev);
    assert_eq!(dev.warmth, Some(7), "the mixer follows the saved step");
    assert_eq!(warmth_row(&mut app).map(|r| r.1), Some(7));
    // A device with no mixer takes no write.
    let mut app = App::new(1440, 1920, d.clone());
    let mut none = DesktopDevice::new(1440, 1920);
    app.sync_warmth(&mut none);
    assert_eq!(none.warmth, None);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_mixer_that_cannot_be_read_still_works() {
    let d = dir("blind");
    let mut app = App::new(1440, 1920, d.clone());
    let mut dev = DesktopDevice::new(1440, 1920);
    dev.warmth = Some(5);
    dev.warmth_readable = false;
    app.sync_warmth(&mut dev);
    assert_eq!(
        warmth_row(&mut app).map(|r| r.1),
        Some(0),
        "unknown: the cool end, until the reader moves it"
    );
    assert_eq!(dev.warmth, Some(5), "and nothing is written");
    app.change(Change::Warmth(3));
    app.sync_warmth(&mut dev);
    assert_eq!(dev.warmth, Some(3));
    let _ = std::fs::remove_dir_all(&d);
}

/// Like the Backlight slider, dragging acts once per step, and a first touch
/// on the step the slider happens to show still counts when nothing is saved.
#[test]
fn warmth_drag_acts_once_per_step() {
    let d = dir("drag");
    let mut app = App::new(1440, 1920, d.clone());
    let mut dev = DesktopDevice::new(1440, 1920);
    dev.warmth = Some(4);
    app.sync_warmth(&mut dev);
    app.open_options();
    app.settings_page = SettingsPage::Category(1);
    let (x, y) = app.control_centre("Warmth/60 %").expect("the 60 % stop");
    let mut t = 0u64;
    let mut go = |app: &mut App, phase: Phase, dx: i32| {
        t += 30;
        app.handle(Event::Touch {
            phase,
            x: x + dx,
            y,
            t,
        })
    };
    assert!(go(&mut app, Phase::Down, 0).is_some(), "the step is set");
    assert_eq!(app.settings.warmth, Some(6));
    for dx in [1, 2, 3, 2, 1] {
        assert!(
            go(&mut app, Phase::Move, dx).is_none(),
            "the same step again does nothing"
        );
    }
    assert!(go(&mut app, Phase::Up, 0).is_none());
    app.sync_warmth(&mut dev);
    assert_eq!(dev.warmth, Some(6));
    let _ = std::fs::remove_dir_all(&d);
}
