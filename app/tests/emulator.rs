//! End-to-end v0.1 + v0.2 checks on the headless 1440×1920 emulator.

use sigma_device::desktop::DesktopDevice;
use sigma_device::HwKey;
use sigma_device::{Event, Refresh};
use sigma_scribe::App;

fn tap(app: &mut App, dev: &mut DesktopDevice, label: &str) {
    let (x, y) = app
        .key_centre(label)
        .unwrap_or_else(|| panic!("no key {label}"));
    if let Some(r) = app.handle(Event::Tap { x, y }) {
        app.render(dev, r).unwrap();
    }
}

#[test]
fn typing_on_osk_autosaves_to_a_plain_md_file_and_survives_restart() {
    let dir = std::env::temp_dir().join(format!("sigma-emu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.render(&mut dev, Refresh::Full).unwrap();
    assert!(
        dir.join("field-notes.md").exists(),
        "seed written on first run"
    );
    for l in ["Return", "Shift", "O", "k", "Space", "123", "1"] {
        tap(&mut app, &mut dev, l);
    }
    assert!(
        app.buffer.text().ends_with("\nOk 1"),
        "{:?}",
        app.buffer.text()
    );
    assert_eq!(app.autosave.state.label(), "saved");
    let on_disk = std::fs::read_to_string(dir.join("field-notes.md")).unwrap();
    assert!(on_disk.ends_with("Ok 1"));
    // Keystrokes use fast partial refreshes, never a full flash.
    assert!(dev
        .refreshes
        .iter()
        .skip(1)
        .all(|(_, r)| *r == Refresh::Fast));
    let restarted = App::new(1440, 1920, &dir);
    assert_eq!(restarted.buffer.text(), on_disk);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn power_button_saves_and_stops() {
    let dir = std::env::temp_dir().join(format!("sigma-emu-p-{}", std::process::id()));
    let mut app = App::new(1440, 1920, &dir);
    app.handle(Event::Power);
    assert!(!app.running);
    let _ = std::fs::remove_dir_all(dir);
}

fn fresh(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-app-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn tap_on(app: &mut App, label: &str) {
    let (x, y) = app
        .control_centre(label)
        .unwrap_or_else(|| panic!("no control {label}"));
    app.handle(Event::Tap { x, y });
}

#[test]
fn seeds_library_and_restores_last_opened_note() {
    let dir = fresh("restore");
    let mut app = App::new(1440, 1920, &dir);
    assert_eq!(app.current.as_str(), "field-notes.md");
    assert!(
        dir.join("drafts/chapter-01.md").exists()
            && dir.join("scratch.txt").exists()
            && dir.join(".scratchpad.md").exists()
    );
    tap_on(&mut app, "Files");
    tap_on(&mut app, "drafts");
    tap_on(&mut app, "chapter-01.md");
    assert_eq!(app.current.as_str(), "drafts/chapter-01.md");
    assert!(app.buffer.text().starts_with("# Chapter one"));
    drop(app);
    let app = App::new(1440, 1920, &dir);
    assert_eq!(
        app.current.as_str(),
        "drafts/chapter-01.md",
        "last-opened restored"
    );
    std::fs::remove_file(dir.join("drafts/chapter-01.md")).unwrap();
    let app2 = App::new(1440, 1920, &dir);
    assert_eq!(
        app2.current.as_str(),
        "field-notes.md",
        "falls back when the note is gone"
    );
    drop(app);
}

#[test]
fn deleting_the_open_note_needs_two_taps_and_opens_another() {
    let dir = fresh("delete");
    let mut app = App::new(1440, 1920, &dir);
    tap_on(&mut app, "Files");
    tap_on(&mut app, "More:field-notes.md");
    tap_on(&mut app, "Delete");
    assert!(dir.join("field-notes.md").exists(), "first tap only arms");
    tap_on(&mut app, "Delete");
    assert!(!dir.join("field-notes.md").exists());
    assert_ne!(app.current.as_str(), "field-notes.md");
    app.handle(Event::Quit);
    assert!(
        !dir.join("field-notes.md").exists(),
        "deleted note is not re-saved on quit"
    );
}

#[test]
fn rename_from_title_bar_keeps_edits_and_cancel_changes_nothing() {
    let dir = fresh("rename");
    let mut app = App::new(1440, 1920, &dir);
    app.handle(Event::Key(HwKey::Char('Z')));
    tap_on(&mut app, "Title");
    assert!(app.is_renaming());
    for _ in 0..20 {
        app.handle(Event::Key(HwKey::Backspace));
    }
    for c in "log".chars() {
        app.handle(Event::Key(HwKey::Char(c)));
    }
    tap_on(&mut app, "Cancel");
    assert_eq!(app.current.as_str(), "field-notes.md");
    tap_on(&mut app, "Title");
    for _ in 0..20 {
        app.handle(Event::Key(HwKey::Backspace));
    }
    for c in "log/../x".chars() {
        app.handle(Event::Key(HwKey::Char(c)));
    }
    app.handle(Event::Key(HwKey::Return));
    assert_eq!(app.current.as_str(), "log x.md");
    assert!(!dir.join("field-notes.md").exists());
    assert!(std::fs::read_to_string(dir.join("log x.md"))
        .unwrap()
        .contains('Z'));
}

#[test]
fn keyboard_layout_choice_is_remembered() {
    let dir = fresh("layout");
    let mut app = App::new(1440, 1920, &dir);
    assert_eq!(app.keyboard.layout, sigma_core::keyboard::Layout::Compact);
    let compact_page = app.page_rect().h;
    let (x, y) = app.key_centre("Full").unwrap();
    app.handle(Event::Tap { x, y });
    assert_eq!(app.keyboard.layout, sigma_core::keyboard::Layout::Full);
    assert!(
        app.page_rect().h >= 1300,
        "page {} (compact {compact_page})",
        app.page_rect().h
    );
    drop(app);
    let app = App::new(1440, 1920, &dir);
    assert_eq!(
        app.keyboard.layout,
        sigma_core::keyboard::Layout::Full,
        "restored"
    );
    assert!(std::fs::read_to_string(dir.join(".sigma-settings"))
        .unwrap()
        .contains("keyboard=full"));
    assert!(!app
        .library
        .list(&sigma_core::NotePath::root())
        .unwrap()
        .iter()
        .any(|e| e.path.name().starts_with('.')));
}
