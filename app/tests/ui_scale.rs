//! v1.1 UI scale: 200 % default, every step fits and keeps 44 px targets.

use sigma_core::settings::{Change, Orientation, Settings, UI_SCALES};
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-uiscale-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn default_is_200_and_steps_clamp() {
    let mut s = Settings::default();
    assert_eq!(s.ui_scale, 200);
    s.apply(Change::UiScale(1));
    assert_eq!(s.ui_scale, 200, "clamped at the top");
    for want in [175, 150, 125, 100, 100] {
        s.apply(Change::UiScale(-1));
        assert_eq!(s.ui_scale, want);
    }
}

#[test]
fn ui_scale_round_trips_and_rejects_junk() {
    let mut s = Settings::default();
    s.apply(Change::UiScale(-2));
    let back = Settings::parse(&s.serialize());
    assert_eq!(back.ui_scale, 150);
    assert_eq!(Settings::parse("ui_scale=133\n").ui_scale, 200);
    assert_eq!(Settings::parse("ui_scale=x\n").ui_scale, 200);
}

#[test]
fn every_scale_fits_in_both_orientations() {
    let dir = fresh("fit");
    let mut app = App::new(1440, 1920, &dir);
    app.set_builtin_panes(true);
    for _ in 0..UI_SCALES.len() {
        app.change(Change::UiScale(-1));
    }
    for (i, &p) in UI_SCALES.iter().enumerate() {
        if i > 0 {
            app.change(Change::UiScale(1));
        }
        for o in [Orientation::Portrait, Orientation::Landscape] {
            app.change(Change::Orientation(o));
            let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
            let min = app.min_target();
            for (n, r) in app.touch_targets() {
                assert!(r.w >= min && r.h >= min, "{p}% {o:?}: {n} {r:?}");
                assert!(
                    r.x >= 0 && r.y >= 0 && r.x + r.w <= w && r.y + r.h <= h,
                    "{p}% {o:?}: {n} {r:?} off screen"
                );
            }
            let l = app.screen.layout(true, &app.keyboard);
            let cap = if w > h { 45 } else { 42 };
            assert!(
                l.keyboard.h <= h * cap / 100,
                "{p}% {o:?}: keyboard {}",
                l.keyboard.h
            );
            assert!(l.page.h >= 500, "{p}% {o:?}: page {}", l.page.h);
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn ui_scale_grows_chrome_but_not_the_note_text() {
    let dir = fresh("grow");
    let mut app = App::new(1440, 1920, &dir);
    app.set_builtin_panes(true);
    let big = app.screen.layout(true, &app.keyboard);
    let body_big = app.screen.page_scale();
    for _ in 0..4 {
        app.change(Change::UiScale(-1));
    }
    let small = app.screen.layout(true, &app.keyboard);
    assert!(big.files_button.w > small.files_button.w);
    assert!(big.title.h > small.title.h);
    assert_eq!(body_big.0, app.screen.page_scale().0);
    let _ = std::fs::remove_dir_all(dir);
}
