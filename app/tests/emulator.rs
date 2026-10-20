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
    for l in ["Enter", "Shift", "O", "k", "Space", "1"] {
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
fn keyboard_defaults_to_full_and_compact_choice_is_remembered() {
    use sigma_core::keyboard::Layout;
    let dir = fresh("layout");
    let mut app = App::new(1440, 1920, &dir);
    assert_eq!(
        app.keyboard.layout,
        Layout::Full,
        "default with no settings file"
    );
    assert!(!dir.join(".sigma-settings").exists());
    assert!(app.page_rect().h >= 1300, "page {}", app.page_rect().h);
    let (x, y) = app.key_centre("Compact").unwrap();
    app.handle(Event::Tap { x, y });
    assert_eq!(app.keyboard.layout, Layout::Compact);
    drop(app);
    let mut app = App::new(1440, 1920, &dir);
    assert_eq!(
        app.keyboard.layout,
        Layout::Compact,
        "stored Compact is kept"
    );
    assert!(std::fs::read_to_string(dir.join(".sigma-settings"))
        .unwrap()
        .contains("keyboard=compact"));
    assert!(!app
        .library
        .list(&sigma_core::NotePath::root())
        .unwrap()
        .iter()
        .any(|e| e.path.name().starts_with('.')));
    let (x, y) = app.key_centre("Full").unwrap();
    app.handle(Event::Tap { x, y });
    drop(app);
    assert_eq!(App::new(1440, 1920, &dir).keyboard.layout, Layout::Full);
}

#[test]
fn touch_typing_still_types_every_tap_including_fast_doubles_and_holds() {
    use sigma_device::Phase;
    let dir = fresh("touchtype");
    let mut app = App::new(1440, 1920, &dir);
    let start = app.buffer.text().len();
    let (x, y) = app.key_centre("a").unwrap();
    let mut t = 1000;
    let touch = |app: &mut App, phase, dx: i32, t: u64| {
        app.handle(Event::Touch {
            phase,
            x: x + dx,
            y,
            t,
        });
    };
    // Two quick taps on the same key = "aa" (double-tap only selects on the page).
    for _ in 0..2 {
        touch(&mut app, Phase::Down, 0, t);
        touch(&mut app, Phase::Up, 0, t + 60);
        t += 120;
    }
    // A held key types once; a sloppy slide types where the finger lifts.
    touch(&mut app, Phase::Down, 0, t);
    for k in 1..12 {
        app.handle(Event::Tick { t: t + k * 50 });
    }
    touch(&mut app, Phase::Up, 0, t + 700);
    t += 900;
    touch(&mut app, Phase::Down, 0, t);
    touch(&mut app, Phase::Move, 35, t + 40);
    touch(&mut app, Phase::Up, 35, t + 80);
    assert_eq!(&app.buffer.text()[start..], "aaaa");
    assert!(app.sel_ui.is_none());
}

fn tap_control(app: &mut App, dev: &mut DesktopDevice, label: &str) {
    let (x, y) = app
        .control_centre(label)
        .unwrap_or_else(|| panic!("no control {label}"));
    if let Some(r) = app.handle(Event::Tap { x, y }) {
        app.render(dev, r).unwrap();
    }
}

fn step(app: &mut App, dev: &mut DesktopDevice, e: Event) -> Option<Refresh> {
    let r = app.handle(e);
    if let Some(r) = r {
        app.render(dev, r).unwrap();
    }
    r
}

#[test]
fn preview_toggle_is_read_only_and_uses_a_full_refresh() {
    let dir = fresh("preview");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.render(&mut dev, Refresh::Full).unwrap();
    let before = app.buffer.text().to_owned();
    let caret = app.buffer.caret();
    let n = dev.refreshes.len();
    tap_control(&mut app, &mut dev, "Eye");
    assert!(app.preview.is_some());
    assert_eq!(dev.refreshes[n..], [(dev.panel.bounds(), Refresh::Full)]);
    // Keys, taps on the page, holds and page turns past the end change nothing.
    for e in [
        Event::Key(HwKey::Char('z')),
        Event::Key(HwKey::Return),
        Event::Key(HwKey::Backspace),
        Event::Tap { x: 700, y: 400 },
        Event::Tap { x: 700, y: 1880 },
        Event::PageTurn { forward: true },
        Event::PageTurn { forward: false },
    ] {
        assert_eq!(step(&mut app, &mut dev, e), None, "{e:?}");
    }
    assert_eq!(app.buffer.text(), before);
    assert!(app.buffer.selection().is_none());
    // Holding on the page does not select in preview.
    step(
        &mut app,
        &mut dev,
        Event::Touch {
            phase: sigma_device::Phase::Down,
            x: 300,
            y: 160,
            t: 10_000,
        },
    );
    step(&mut app, &mut dev, Event::Tick { t: 10_700 });
    step(
        &mut app,
        &mut dev,
        Event::Touch {
            phase: sigma_device::Phase::Up,
            x: 300,
            y: 160,
            t: 10_800,
        },
    );
    assert!(app.buffer.selection().is_none() && app.sel_ui.is_none());
    assert!(app.preview.is_some());
    let n = dev.refreshes.len();
    tap_control(&mut app, &mut dev, "Pencil");
    assert!(app.preview.is_none());
    assert_eq!(dev.refreshes[n..], [(dev.panel.bounds(), Refresh::Full)]);
    assert_eq!(app.buffer.caret(), caret, "caret unchanged");
    assert_eq!(
        std::fs::read_to_string(dir.join("field-notes.md")).unwrap(),
        before
    );
    // Write mode works as before.
    tap(&mut app, &mut dev, "k");
    assert!(app.buffer.text().ends_with('k'));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn page_turns_page_the_preview_with_partial_refreshes() {
    let dir = fresh("pages");
    std::fs::create_dir_all(&dir).unwrap();
    let long: String = (0..300)
        .map(|i| format!("## Part {i}\n\nSome *text* for part {i}.\n\n"))
        .collect();
    std::fs::write(dir.join("long.md"), &long).unwrap();
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.open(&sigma_core::NotePath::parse("long.md").unwrap());
    app.buffer.set_caret(0);
    app.render(&mut dev, Refresh::Full).unwrap();
    tap_control(&mut app, &mut dev, "Eye");
    let pages = app.preview.as_ref().unwrap().doc.pages.len();
    assert!(pages > 5, "{pages}");
    assert_eq!(app.preview.as_ref().unwrap().page, 0);
    let n = dev.refreshes.len();
    for i in 1..pages {
        assert_eq!(
            step(&mut app, &mut dev, Event::PageTurn { forward: true }),
            Some(Refresh::Partial)
        );
        assert_eq!(app.preview.as_ref().unwrap().page, i);
    }
    // At the last page Next does nothing; Prev goes back.
    assert_eq!(
        step(&mut app, &mut dev, Event::PageTurn { forward: true }),
        None
    );
    step(&mut app, &mut dev, Event::PageTurn { forward: false });
    assert_eq!(app.preview.as_ref().unwrap().page, pages - 2);
    assert!(dev.refreshes[n..]
        .iter()
        .all(|(r, k)| *k == Refresh::Partial && *r != dev.panel.bounds()));
    // The write position is kept: back in write mode, then into preview from
    // the end of the note lands on the last page.
    step(&mut app, &mut dev, Event::Key(HwKey::Escape));
    assert!(app.preview.is_none());
    app.buffer.set_caret(app.buffer.text().len());
    app.render(&mut dev, Refresh::Fast).unwrap();
    tap_control(&mut app, &mut dev, "Eye");
    let p = app.preview.as_ref().unwrap();
    assert!(
        p.page + 1 >= p.doc.pages.len() - 1,
        "{} of {}",
        p.page,
        p.doc.pages.len()
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn files_from_preview_leaves_preview() {
    let dir = fresh("pfiles");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.render(&mut dev, Refresh::Full).unwrap();
    tap_control(&mut app, &mut dev, "Eye");
    tap_control(&mut app, &mut dev, "Files");
    assert!(app.preview.is_none() && app.files.is_some());
    std::fs::remove_dir_all(dir).unwrap();
}
