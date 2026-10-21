//! v0.7 hardening: touch targets, rotation, unreadable notes, power loss.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_scribe::App;

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-hard-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn tap_on(app: &mut App, label: &str) {
    let (x, y) = app
        .control_centre(label)
        .unwrap_or_else(|| panic!("no control {label}"));
    app.handle(Event::Tap { x, y });
    app.settle();
}

/// Every control on screen is at least 44 px (scaled) both ways and lies
/// inside the screen.
fn audit(app: &App, what: &str, bad: &mut Vec<String>) {
    let min = app.min_target();
    let (w, h) = (app.canvas.width as i32, app.canvas.height as i32);
    let targets = app.touch_targets();
    assert!(!targets.is_empty(), "{what}: no targets");
    for (n, r) in targets {
        if r.w < min || r.h < min {
            bad.push(format!("{what}: {n} {}x{} < {min}", r.w, r.h));
        }
        if r.x < 0 || r.y < 0 || r.x + r.w > w || r.y + r.h > h {
            bad.push(format!("{what}: {n} {r:?} off screen {w}x{h}"));
        }
    }
}

fn walk(app: &mut App, tag: &str, bad: &mut Vec<String>) {
    audit(app, &format!("{tag} write"), bad);
    for p in ["Contents", "To-dos", "Scratch"] {
        tap_on(app, p);
    }
    audit(app, &format!("{tag} drawer"), bad);
    for p in ["Contents", "To-dos", "Scratch"] {
        tap_on(app, p);
    }
    tap_on(app, "Find");
    audit(app, &format!("{tag} find"), bad);
    tap_on(app, "FindClose");
    tap_on(app, "Preview");
    audit(app, &format!("{tag} preview"), bad);
    tap_on(app, "Pencil");
    tap_on(app, "Files");
    audit(app, &format!("{tag} files"), bad);
    tap_on(app, "More:field-notes.md");
    audit(app, &format!("{tag} files more"), bad);
    tap_on(app, "Close");
    tap_on(app, "Graph");
    audit(app, &format!("{tag} graph"), bad);
    tap_on(app, "Close");
    tap_on(app, "Help");
    audit(app, &format!("{tag} help"), bad);
    tap_on(app, "Markdown reference");
    audit(app, &format!("{tag} help page"), bad);
    tap_on(app, "Close");
    tap_on(app, "Options");
    audit(app, &format!("{tag} options"), bad);
    tap_on(app, "Close");
}

#[test]
fn every_target_is_44px_and_on_screen_in_both_orientations() {
    let dir = fresh("targets");
    let mut app = App::new(1440, 1920, &dir);
    let mut bad = Vec::new();
    walk(&mut app, "portrait", &mut bad);
    for keys in ["Split", "Full", "Centre"] {
        tap_on(&mut app, "Options");
        tap_on(&mut app, "Orientation/Landscape");
        tap_on(&mut app, "Close");
        tap_on(&mut app, keys);
        walk(&mut app, &format!("landscape {keys}"), &mut bad);
    }
    assert!(
        bad.is_empty(),
        "{} small or misplaced targets:\n{}",
        bad.len(),
        bad.join("\n")
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unreadable_note_is_never_overwritten() {
    let dir = fresh("binary");
    std::fs::create_dir_all(&dir).unwrap();
    let junk: Vec<u8> = vec![0xff, 0xfe, b'a', 0x80, 0x00, b'z'];
    std::fs::write(dir.join("old.txt"), &junk).unwrap();
    let mut app = App::new(1440, 1920, &dir);
    app.open(&sigma_core::NotePath::parse("old.txt").unwrap());
    assert!(app.read_only);
    assert_eq!(app.autosave.state.label(), "not saved");
    app.handle(Event::Key(sigma_device::HwKey::Char('x')));
    app.handle(Event::Power);
    assert_eq!(std::fs::read(dir.join("old.txt")).unwrap(), junk);
    // Another note is writable again.
    app.open(&sigma_core::NotePath::parse("field-notes.md").unwrap());
    assert!(!app.read_only);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn orientation_round_trip_keeps_text_caret_and_regions() {
    let dir = fresh("rotate");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    let _ = app.render(&mut dev, Refresh::Full);
    let (text, caret) = (app.buffer.text().to_owned(), app.buffer.caret());
    for o in ["Landscape", "Portrait", "Landscape", "Portrait"] {
        tap_on(&mut app, "Options");
        tap_on(&mut app, &format!("Orientation/{o}"));
        tap_on(&mut app, "Close");
        let _ = app.render(&mut dev, Refresh::Full);
        tap_on(&mut app, "Find");
        let _ = app.render(&mut dev, Refresh::Full);
        tap_on(&mut app, "FindClose");
        let _ = app.render(&mut dev, Refresh::Full);
    }
    assert_eq!(app.buffer.text(), text);
    assert_eq!(app.buffer.caret(), caret);
    assert_eq!((app.canvas.width, app.canvas.height), (1440, 1920));
    for (r, _) in &dev.refreshes {
        assert!(
            r.w >= 16
                && r.h >= 16
                && r.x >= 0
                && r.y >= 0
                && r.x + r.w <= 1440
                && r.y + r.h <= 1920,
            "{r:?}"
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Power loss during a save: a child process saves two different full
/// versions of a note in a loop and is killed (SIGKILL) at random moments.
/// The note on disk must always be one complete version, never a mix or a
/// truncation.
#[test]
fn killed_during_save_leaves_a_whole_note() {
    if let Ok(path) = std::env::var("SIGMA_KILL_CHILD") {
        let a = "a".repeat(200_000);
        let b = "b".repeat(150_000);
        let mut i = 0u64;
        loop {
            let body = if i % 2 == 0 { &a } else { &b };
            sigma_core::store::write_atomic(std::path::Path::new(&path), body).unwrap();
            i += 1;
        }
    }
    let dir = fresh("kill");
    std::fs::create_dir_all(&dir).unwrap();
    let note = dir.join("note.md");
    std::fs::write(&note, "a".repeat(200_000)).unwrap();
    let exe = std::env::current_exe().unwrap();
    for round in 0..10u64 {
        let mut child = std::process::Command::new(&exe)
            .args([
                "--exact",
                "killed_during_save_leaves_a_whole_note",
                "--nocapture",
            ])
            .env("SIGMA_KILL_CHILD", &note)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30 + round * 17));
        child.kill().unwrap();
        let _ = child.wait();
        let body = std::fs::read_to_string(&note).unwrap();
        let whole = body == "a".repeat(200_000) || body == "b".repeat(150_000);
        assert!(whole, "round {round}: {} bytes, mixed or cut", body.len());
        // The app still opens it and the library does not list the temp file.
        let app = App::new(1440, 1920, &dir);
        assert!(app
            .library
            .all_notes()
            .unwrap()
            .iter()
            .all(|n| !n.as_str().ends_with(".tmp")));
    }
    let _ = std::fs::remove_dir_all(dir);
}
