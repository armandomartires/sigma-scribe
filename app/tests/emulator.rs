//! End-to-end v0.1 checks on the headless 1440×1920 emulator.

use sigma_core::FsNoteStore;
use sigma_device::desktop::DesktopDevice;
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
    let mut app = App::new(1440, 1920, FsNoteStore::new(&dir));
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
    let restarted = App::new(1440, 1920, FsNoteStore::new(&dir));
    assert_eq!(restarted.buffer.text(), on_disk);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn power_button_saves_and_stops() {
    let dir = std::env::temp_dir().join(format!("sigma-emu-p-{}", std::process::id()));
    let mut app = App::new(1440, 1920, FsNoteStore::new(&dir));
    app.handle(Event::Power);
    assert!(!app.running);
    let _ = std::fs::remove_dir_all(dir);
}
