//! v1.13.1: the UI scale slider moves its thumb and shows the value while the
//! finger is down, but the scale only changes when the finger lifts.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Phase, Refresh};
use sigma_render::SettingsPage;
use sigma_scribe::App;

fn rig(tag: &str) -> (App, DesktopDevice, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("sigma-uisl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = App::new(1440, 1920, dir.clone());
    let mut dev = DesktopDevice::new(1440, 1920);
    app.open_options();
    app.settings_page = SettingsPage::Category(1);
    app.render(&mut dev, Refresh::Full).unwrap();
    (app, dev, dir)
}

fn touch(app: &mut App, dev: &mut DesktopDevice, t: &mut u64, phase: Phase, x: i32, y: i32) {
    *t += 30;
    if let Some(r) = app.handle(Event::Touch { phase, x, y, t: *t }) {
        app.render(dev, r).unwrap();
    }
}

#[test]
fn ui_scale_changes_on_release_not_while_dragging() {
    let (mut app, mut dev, dir) = rig("drag");
    assert_eq!(app.settings.ui_scale, 200);
    let (x0, y0) = app
        .control_centre("UI scale/200 %")
        .expect("the 200 % stop");
    let (x1, _) = app
        .control_centre("UI scale/100 %")
        .expect("the 100 % stop");
    let mut t = 0;
    touch(&mut app, &mut dev, &mut t, Phase::Down, x0, y0);
    for k in 1..=8 {
        touch(
            &mut app,
            &mut dev,
            &mut t,
            Phase::Move,
            x0 + (x1 - x0) * k / 8,
            y0,
        );
        assert_eq!(
            app.settings.ui_scale, 200,
            "unchanged while the finger is down"
        );
    }
    assert_eq!(
        app.options_model().ui_scale_pending,
        Some(100),
        "the thumb and value follow the finger"
    );
    touch(&mut app, &mut dev, &mut t, Phase::Up, x1, y0);
    assert_eq!(app.settings.ui_scale, 100, "applied on release");
    assert_eq!(app.options_model().ui_scale_pending, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_tap_on_a_stop_applies_on_release() {
    let (mut app, mut dev, dir) = rig("tap");
    let (x, y) = app
        .control_centre("UI scale/150 %")
        .expect("the 150 % stop");
    let mut t = 0;
    touch(&mut app, &mut dev, &mut t, Phase::Down, x, y);
    assert_eq!(app.settings.ui_scale, 200, "not yet");
    touch(&mut app, &mut dev, &mut t, Phase::Up, x, y);
    assert_eq!(app.settings.ui_scale, 150);
    let _ = std::fs::remove_dir_all(&dir);
}
