//! v1.2 orientation: four rotations plus Auto (sensor stubbed off on Kobo).

use sigma_core::settings::{Change, Orientation, Rotation, Settings};
use sigma_device::desktop::DesktopDevice;
use sigma_device::gsensor::Tilt;
use sigma_device::{Device, Event, Phase, Refresh};
use sigma_scribe::{App, TILT_STABLE_MS, TILT_TYPING_PAUSE_MS};

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-orient-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn run(app: &mut App, dev: &mut DesktopDevice, e: Event) {
    if let Some(r) = app.handle(e) {
        app.render(dev, r).unwrap();
    }
}

fn ticks(app: &mut App, dev: &mut DesktopDevice, from: u64, to: u64) {
    let mut t = from;
    while t <= to {
        run(app, dev, Event::Tick { t });
        t += 50;
    }
}

#[test]
fn auto_is_the_default_and_old_files_migrate() {
    let s = Settings::default();
    assert_eq!(
        (s.orientation, s.rotation),
        (Orientation::Auto, Rotation::R0)
    );
    // A 1.1 file always wrote orientation=portrait: that becomes Auto.
    let old = Settings::parse("orientation=portrait\n");
    assert_eq!(old.orientation, Orientation::Auto);
    let land = Settings::parse("orientation=landscape\n");
    assert_eq!(
        (land.orientation, land.rotation),
        (Orientation::Landscape, Rotation::R90)
    );
    // A 1.2 file keeps a manual portrait and Auto's last rotation.
    let mut m = Settings::default();
    m.apply(Change::Orientation(Orientation::Portrait));
    m.apply(Change::UpsideDown(true));
    let back = Settings::parse(&m.serialize());
    assert_eq!(
        (back.orientation, back.rotation),
        (Orientation::Portrait, Rotation::R180)
    );
    let mut a = Settings::default();
    a.apply(Change::Rotate(Rotation::R270));
    let back = Settings::parse(&a.serialize());
    assert_eq!(
        (back.orientation, back.rotation),
        (Orientation::Auto, Rotation::R270)
    );
}

#[test]
fn manual_choices_give_all_four_rotations() {
    let mut s = Settings::default();
    for (o, up, want) in [
        (Orientation::Portrait, false, Rotation::R0),
        (Orientation::Landscape, false, Rotation::R90),
        (Orientation::Portrait, true, Rotation::R180),
        (Orientation::Landscape, true, Rotation::R270),
    ] {
        s.apply(Change::Orientation(o));
        s.apply(Change::UpsideDown(up));
        assert_eq!(s.rotation, want);
    }
    // Back to Auto keeps the last rotation; a manual sensor event is ignored.
    s.apply(Change::Orientation(Orientation::Auto));
    assert_eq!(s.rotation, Rotation::R270);
    s.apply(Change::Orientation(Orientation::Portrait));
    s.apply(Change::UpsideDown(false));
    s.apply(Change::Rotate(Rotation::R90));
    assert_eq!(s.rotation, Rotation::R0, "Rotate only acts in Auto");
}

#[test]
fn every_rotation_maps_panel_and_touch_exactly() {
    let dir = fresh("map");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.render(&mut dev, Refresh::Full).unwrap();
    for (o, up) in [
        (Orientation::Portrait, false),
        (Orientation::Landscape, false),
        (Orientation::Portrait, true),
        (Orientation::Landscape, true),
    ] {
        app.change(Change::Orientation(o));
        let r = app.change(Change::UpsideDown(up));
        app.render(&mut dev, r).unwrap();
        let rot = app.rotation();
        let (lw, lh) = (app.canvas.width as i32, app.canvas.height as i32);
        assert_eq!(rot.is_landscape(), lw > lh);
        assert_eq!(app.view(&dev.panel).pixels, app.canvas.pixels, "{rot:?}");
        let (x, y) = app.key_centre("k").unwrap();
        let (px, py) = match rot {
            Rotation::R0 => (x, y),
            Rotation::R90 => (lh - 1 - y, x),
            Rotation::R180 => (lw - 1 - x, lh - 1 - y),
            Rotation::R270 => (y, lw - 1 - x),
        };
        assert_eq!(
            app.to_logical(Event::Tap { x: px, y: py }),
            Event::Tap { x, y },
            "{rot:?}"
        );
        let before = app.buffer.text().len();
        let n = dev.refreshes.len();
        let e = app.to_logical(Event::Tap { x: px, y: py });
        run(&mut app, &mut dev, e);
        assert_eq!(app.buffer.text().len(), before + 1, "{rot:?}: typed k");
        for (r, _) in &dev.refreshes[n..] {
            assert!(r.w >= 16 && r.h >= 16, "{rot:?} {r:?}");
            assert!(r.x >= 0 && r.y >= 0 && r.x + r.w <= 1440 && r.y + r.h <= 1920);
        }
        assert_eq!(
            app.view(&dev.panel).pixels,
            app.canvas.pixels,
            "{rot:?} partial"
        );
        // Upside down, the page buttons swap.
        let turned = app.to_logical(Event::PageTurn { forward: true });
        assert_eq!(
            turned,
            Event::PageTurn {
                forward: !rot.is_inverted()
            }
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn auto_without_a_sensor_keeps_the_last_rotation() {
    let dir = fresh("nosensor");
    let mut dev = DesktopDevice::new(1440, 1920);
    assert!(!dev.has_gsensor());
    let mut app = App::new(1440, 1920, &dir);
    app.change(Change::Orientation(Orientation::Landscape));
    app.change(Change::Orientation(Orientation::Auto));
    assert_eq!(app.orientation_status(), "Auto (no sensor) — Landscape");
    run(
        &mut app,
        &mut dev,
        Event::Tilt {
            tilt: Tilt::Upright,
            t: 100,
        },
    );
    ticks(&mut app, &mut dev, 100, 3000);
    assert_eq!(app.rotation(), Rotation::R90);
    const { assert!(!sigma_device::gsensor::ENABLED) };
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn auto_applies_a_stable_reading_with_one_full_refresh() {
    let dir = fresh("auto");
    let mut dev = DesktopDevice::new(1440, 1920);
    dev.gsensor = true;
    let mut app = App::new(1440, 1920, &dir);
    app.set_gsensor(true);
    app.render(&mut dev, Refresh::Full).unwrap();
    assert_eq!(app.orientation_status(), "Auto — Portrait");
    // A brief wobble is ignored.
    run(
        &mut app,
        &mut dev,
        Event::Tilt {
            tilt: Tilt::Clockwise,
            t: 1000,
        },
    );
    ticks(&mut app, &mut dev, 1000, 1000 + TILT_STABLE_MS - 100);
    run(
        &mut app,
        &mut dev,
        Event::Tilt {
            tilt: Tilt::Upright,
            t: 1500,
        },
    );
    ticks(&mut app, &mut dev, 1500, 3000);
    assert_eq!(app.rotation(), Rotation::R0);
    // A held turn applies once, after the stable time, with a Full refresh.
    let n = dev.refreshes.len();
    run(
        &mut app,
        &mut dev,
        Event::Tilt {
            tilt: Tilt::UpsideDown,
            t: 4000,
        },
    );
    ticks(&mut app, &mut dev, 4000, 4000 + TILT_STABLE_MS - 50);
    assert_eq!(app.rotation(), Rotation::R0);
    ticks(&mut app, &mut dev, 4000 + TILT_STABLE_MS, 6000);
    assert_eq!(app.rotation(), Rotation::R180);
    let fulls = dev.refreshes[n..]
        .iter()
        .filter(|(_, k)| *k == Refresh::Full)
        .count();
    assert_eq!(fulls, 1);
    assert_eq!(app.view(&dev.panel).pixels, app.canvas.pixels);
    // Remembered across a restart.
    drop(app);
    let app = App::new(1440, 1920, &dir);
    assert_eq!(app.rotation(), Rotation::R180);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn auto_waits_for_the_finger_and_for_typing_to_stop() {
    let dir = fresh("pause");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.set_gsensor(true);
    app.render(&mut dev, Refresh::Full).unwrap();
    // Finger down on the page: no turn while it rests.
    run(
        &mut app,
        &mut dev,
        Event::Touch {
            phase: Phase::Down,
            x: 700,
            y: 400,
            t: 1000,
        },
    );
    run(
        &mut app,
        &mut dev,
        Event::Tilt {
            tilt: Tilt::Clockwise,
            t: 1000,
        },
    );
    ticks(&mut app, &mut dev, 1000, 3000);
    assert_eq!(app.rotation(), Rotation::R0, "finger down");
    run(
        &mut app,
        &mut dev,
        Event::Touch {
            phase: Phase::Up,
            x: 700,
            y: 400,
            t: 3000,
        },
    );
    // Typing keeps it paused for a second.
    run(
        &mut app,
        &mut dev,
        Event::Key(sigma_device::HwKey::Char('a')),
    );
    ticks(&mut app, &mut dev, 3000, 3000 + TILT_TYPING_PAUSE_MS - 100);
    assert_eq!(app.rotation(), Rotation::R0, "just typed");
    ticks(&mut app, &mut dev, 3000 + TILT_TYPING_PAUSE_MS, 5000);
    assert_eq!(app.rotation(), Rotation::R90);
    // A manual choice turns Auto off and drops readings.
    app.change(Change::Orientation(Orientation::Portrait));
    run(
        &mut app,
        &mut dev,
        Event::Tilt {
            tilt: Tilt::CounterClockwise,
            t: 6000,
        },
    );
    ticks(&mut app, &mut dev, 6000, 8000);
    assert_eq!(app.rotation(), Rotation::R0);
    let _ = std::fs::remove_dir_all(dir);
}
