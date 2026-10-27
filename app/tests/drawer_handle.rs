//! Drawer width by dragging the grip on the drawer's inner edge (S7.5): the
//! stop nearest the finger applies when it lifts, the setting is saved, and
//! a touch that did not start on the grip leaves the width alone.

use sigma_core::settings::{DRAWER_WIDTH_DEFAULT, DRAWER_WIDTH_STOPS};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Phase, Refresh};
use sigma_render::Pane;
use sigma_scribe::App;

fn rig(tag: &str) -> (App, DesktopDevice, std::path::PathBuf) {
    let d = std::env::temp_dir().join(format!("sigma-dh-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let mut app = App::new(1440, 1920, d.clone());
    app.set_builtin_graph(true);
    app.set_builtin_panes(true);
    let mut dev = DesktopDevice::new(1440, 1920);
    app.render(&mut dev, Refresh::Full).unwrap();
    app.toggle_pane(Pane::Contents);
    app.render(&mut dev, Refresh::Partial).unwrap();
    (app, dev, d)
}

fn touch(app: &mut App, dev: &mut DesktopDevice, phase: Phase, x: i32, y: i32, t: u64) {
    if let Some(r) = app.handle(Event::Touch { phase, x, y, t }) {
        app.render(dev, r).unwrap();
    }
}

fn drag(app: &mut App, dev: &mut DesktopDevice, from: (i32, i32), to_x: i32) {
    touch(app, dev, Phase::Down, from.0, from.1, 0);
    let steps = 6;
    for i in 1..=steps {
        let x = from.0 + (to_x - from.0) * i / steps;
        touch(app, dev, Phase::Move, x, from.1, 20 * i as u64);
    }
    touch(app, dev, Phase::Up, to_x, from.1, 200);
}

#[test]
fn dragging_the_grip_picks_the_nearest_width_and_saves_it() {
    let (mut app, mut dev, dir) = rig("drag");
    assert_eq!(app.settings.drawer_width, DRAWER_WIDTH_DEFAULT);
    let before = app.drawer_width_px();
    let grip = app
        .control_centre("DrawerHandle")
        .expect("grip while the drawer is open");
    // The drawer is on the right by default: dragging left widens it, right narrows it.
    let right = app.page_rect().x + app.page_rect().w - 10;
    drag(&mut app, &mut dev, grip, right);
    assert_eq!(app.settings.drawer_width, 0, "dragged to the narrowest");
    let narrow = app.drawer_width_px();
    assert!(narrow < before && narrow > 0, "{narrow} < {before}");
    let saved = std::fs::read_to_string(dir.join(".sigma-settings")).unwrap();
    assert!(saved.contains("drawer_width=0"), "{saved}");
    // And back out to the widest.
    let grip = app.control_centre("DrawerHandle").unwrap();
    drag(&mut app, &mut dev, grip, 0);
    assert_eq!(app.settings.drawer_width, DRAWER_WIDTH_STOPS - 1);
}

#[test]
fn a_touch_off_the_grip_or_without_a_drawer_does_nothing() {
    let (mut app, mut dev, _d) = rig("off");
    let (x, y) = app.control_centre("DrawerHandle").unwrap();
    // Well below the grip, still on the drawer edge: not the grip.
    drag(&mut app, &mut dev, (x, y + 400), x - 300);
    assert_eq!(app.settings.drawer_width, DRAWER_WIDTH_DEFAULT);
    // With every pane closed there is no grip to catch.
    app.toggle_pane(Pane::Contents);
    app.render(&mut dev, Refresh::Partial).unwrap();
    assert!(app.control_centre("DrawerHandle").is_none());
}

#[test]
fn the_grip_is_a_big_enough_target() {
    let (app, _dev, _d) = rig("target");
    let min = app.min_target();
    let (_, r) = app
        .touch_targets()
        .into_iter()
        .find(|(n, _)| n == "drawer handle")
        .expect("listed");
    assert!(r.w >= min && r.h >= min, "{}x{} vs {min}", r.w, r.h);
}
