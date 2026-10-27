//! Drawer width by dragging the grip on the drawer's inner edge (S7.5): the
//! stop nearest the finger applies when it lifts, the setting is saved, and
//! a touch that did not start on the grip leaves the width alone. v1.16:
//! a double tap on the grip or a drag to the screen edge closes the
//! drawer; a tab at the edge brings the same panes back.

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
    // The drawer is on the right by default: dragging left widens it, right
    // narrows it. Near the narrowest drawer's edge, not the screen's (the
    // screen edge closes it, v1.16).
    let right = app.page_rect().x + app.page_rect().w - before * 35 / 100;
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

fn tap_at(app: &mut App, dev: &mut DesktopDevice, (x, y): (i32, i32), t: u64) {
    touch(app, dev, Phase::Down, x, y, t);
    touch(app, dev, Phase::Up, x, y, t + 60);
}

#[test]
fn a_double_tap_on_the_grip_closes_the_drawer_and_the_tab_brings_it_back() {
    let (mut app, mut dev, _d) = rig("double");
    app.toggle_pane(Pane::Scratch);
    assert!(app.scratch_focus);
    let grip = app.control_centre("DrawerHandle").unwrap();
    assert!(
        app.control_centre("DrawerTab").is_none(),
        "no tab while open"
    );
    tap_at(&mut app, &mut dev, grip, 1000);
    assert!(app.drawer_visible(), "one tap does nothing");
    tap_at(&mut app, &mut dev, grip, 1200);
    assert!(!app.drawer_visible(), "two close it");
    assert_eq!(app.drawer, [false; 3]);
    assert!(!app.scratch_focus, "the keyboard is the page's");
    assert_eq!(app.settings.drawer_width, DRAWER_WIDTH_DEFAULT);
    let tab = app
        .control_centre("DrawerTab")
        .expect("the tab at the edge");
    let page = app.page_rect();
    assert!(tab.0 > page.x + page.w * 8 / 10, "on the right");
    let edge = page.x + page.w - app.drawer_width_px();
    assert!(tab.0 > edge, "where the drawer's outer edge was");
    tap_at(&mut app, &mut dev, tab, 3000);
    assert_eq!(app.drawer, [true, false, true], "the same panes");
    assert!(app.scratch_focus, "and the same focus");
    assert!(app.control_centre("DrawerTab").is_none());
}

#[test]
fn two_slow_taps_on_the_grip_do_not_close_it() {
    let (mut app, mut dev, _d) = rig("slow");
    let grip = app.control_centre("DrawerHandle").unwrap();
    tap_at(&mut app, &mut dev, grip, 1000);
    tap_at(&mut app, &mut dev, grip, 2000);
    assert!(app.drawer_visible());
}

#[test]
fn dragging_the_grip_to_the_edge_closes_and_dragging_the_tab_in_reopens_at_that_width() {
    let (mut app, mut dev, _d) = rig("edge");
    let page = app.page_rect();
    let grip = app.control_centre("DrawerHandle").unwrap();
    drag(&mut app, &mut dev, grip, page.x + page.w - 5);
    assert!(!app.drawer_visible(), "dragged off the screen edge");
    assert_eq!(
        app.settings.drawer_width, DRAWER_WIDTH_DEFAULT,
        "width kept"
    );
    // The tab dragged to the middle: open again, at the stop nearest the finger.
    let tab = app.control_centre("DrawerTab").unwrap();
    drag(&mut app, &mut dev, tab, page.x + page.w * 3 / 4);
    assert_eq!(app.drawer, [true, false, false]);
    assert!(app.settings.drawer_width < DRAWER_WIDTH_DEFAULT);
    // A tab drag that stays at the edge leaves the drawer closed.
    let grip = app.control_centre("DrawerHandle").unwrap();
    drag(&mut app, &mut dev, grip, page.x + page.w - 5);
    let tab = app.control_centre("DrawerTab").unwrap();
    drag(&mut app, &mut dev, tab, page.x + page.w - 2);
    assert!(!app.drawer_visible());
}

#[test]
fn the_tab_is_not_the_scroll_bar_and_a_tap_reopens() {
    let (mut app, mut dev, _d) = rig("bar");
    app.buffer.set_caret(0);
    let grip = app.control_centre("DrawerHandle").unwrap();
    tap_at(&mut app, &mut dev, grip, 1000);
    tap_at(&mut app, &mut dev, grip, 1100);
    let scroll = app.screen.scroll;
    let (x, y) = app.control_centre("DrawerTab").unwrap();
    // A script tap or a mouse click arrives as a Tap.
    if let Some(r) = app.handle(Event::Tap { x, y }) {
        app.render(&mut dev, r).unwrap();
    }
    assert!(app.drawer_visible());
    assert_eq!(app.screen.scroll, scroll, "the note did not scroll");
}

#[test]
fn ticking_a_pane_by_hand_forgets_the_closed_panes_and_the_tab() {
    let (mut app, mut dev, _d) = rig("forget");
    let grip = app.control_centre("DrawerHandle").unwrap();
    tap_at(&mut app, &mut dev, grip, 1000);
    tap_at(&mut app, &mut dev, grip, 1100);
    app.toggle_pane(Pane::Todos);
    app.toggle_pane(Pane::Todos);
    app.render(&mut dev, Refresh::Partial).unwrap();
    assert!(!app.drawer_visible());
    assert!(
        app.control_centre("DrawerTab").is_none(),
        "nothing to reopen"
    );
}

#[test]
fn the_tab_is_a_big_enough_target_and_not_shown_over_sheets() {
    let (mut app, mut dev, _d) = rig("tabtarget");
    let grip = app.control_centre("DrawerHandle").unwrap();
    tap_at(&mut app, &mut dev, grip, 1000);
    tap_at(&mut app, &mut dev, grip, 1100);
    let min = app.min_target();
    let (_, r) = app
        .touch_targets()
        .into_iter()
        .find(|(n, _)| n == "drawer tab")
        .expect("listed");
    assert!(r.w >= min && r.h >= min, "{}x{} vs {min}", r.w, r.h);
    app.options = true;
    assert!(app.control_centre("DrawerTab").is_none());
}
