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
    // v1.1: 200 % UI scale, keyboard capped at 42 %.
    assert!(app.page_rect().h >= 800, "page {}", app.page_rect().h);
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

mod v04 {
    use super::*;
    use sigma_core::settings::{Orientation, Paper};
    use sigma_core::Change;

    fn fresh(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sigma-v04-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn night_is_a_full_refresh_and_settings_survive_restart() {
        let dir = fresh("persist");
        let mut dev = DesktopDevice::new(1440, 1920);
        let mut app = App::new(1440, 1920, &dir);
        app.render(&mut dev, Refresh::Full).unwrap();
        assert_eq!(app.change(Change::Paper(Paper::Night)), Refresh::Full);
        assert_eq!(app.change(Change::LineOn(true)), Refresh::Partial);
        app.change(Change::LineWords(2));
        app.change(Change::TabSpaces(false));
        app.change(Change::Usb(true));
        let before = app.settings;
        drop(app);
        let app = App::new(1440, 1920, &dir);
        assert_eq!(app.settings, before);
        assert_eq!(sigma_scribe::words_per_row(&app), 14);
        let file = std::fs::read_to_string(dir.join(".sigma-settings")).unwrap();
        assert!(file.contains("paper=night\n") && file.contains("tab=tab\n"));
    }

    #[test]
    fn old_v021_settings_file_still_loads() {
        let dir = fresh("old");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".sigma-settings"), "keyboard=compact\n").unwrap();
        let app = App::new(1440, 1920, &dir);
        assert_eq!(app.keyboard.layout, sigma_core::keyboard::Layout::Compact);
        assert_eq!(app.settings.paper, Paper::Day);
    }

    #[test]
    fn landscape_rotates_onto_the_portrait_panel_with_safe_regions() {
        let dir = fresh("rot");
        let mut dev = DesktopDevice::new(1440, 1920);
        let mut app = App::new(1440, 1920, &dir);
        app.render(&mut dev, Refresh::Full).unwrap();
        let r = app.change(Change::Orientation(Orientation::Landscape));
        assert_eq!(r, Refresh::Full);
        assert_eq!((app.canvas.width, app.canvas.height), (1920, 1440));
        app.render(&mut dev, r).unwrap();
        assert_eq!(
            dev.refreshes.last().unwrap().0,
            sigma_render::Rect::new(0, 0, 1440, 1920)
        );
        // The panel, turned back, is exactly what the app drew.
        assert_eq!(app.view(&dev.panel).pixels, app.canvas.pixels);
        // A device tap lands on the logical key under it.
        let (x, y) = app.key_centre("k").unwrap();
        let phys = Event::Tap { x: 1439 - y, y: x };
        assert_eq!(app.to_logical(phys), Event::Tap { x, y });
        let n = dev.refreshes.len();
        if let Some(r) = app.handle(app.to_logical(phys)) {
            app.render(&mut dev, r).unwrap();
        }
        assert!(app.buffer.text().ends_with('k'));
        for (r, _) in &dev.refreshes[n..] {
            assert!(r.w >= 16 && r.h >= 16, "{r:?}");
            assert!(r.x >= 0 && r.y >= 0 && r.x + r.w <= 1440 && r.y + r.h <= 1920);
        }
        assert_eq!(app.view(&dev.panel).pixels, app.canvas.pixels);
    }

    #[test]
    fn tab_key_follows_options() {
        let dir = fresh("tab");
        let mut app = App::new(1440, 1920, &dir);
        app.change(Change::TabWidth(2));
        app.handle(Event::Key(HwKey::Tab));
        assert!(app.buffer.text().ends_with("  "));
        app.change(Change::TabSpaces(false));
        app.handle(Event::Key(HwKey::Tab));
        assert!(app.buffer.text().ends_with("  \t"));
    }
}

fn dstep(app: &mut App, dev: &mut DesktopDevice, label: &str) {
    tap_on(app, label);
    let _ = app.render(dev, Refresh::Partial);
    app.settle();
}

#[test]
fn checking_a_todo_in_another_note_rewrites_only_that_line() {
    let dir = fresh("todo");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    let _ = app.render(&mut dev, Refresh::Full);
    let before = std::fs::read_to_string(dir.join("drafts/chapter-01.md")).unwrap();
    dstep(&mut app, &mut dev, "To-dos");
    dstep(&mut app, &mut dev, "TodoBox:Name the conductor");
    let after = std::fs::read_to_string(dir.join("drafts/chapter-01.md")).unwrap();
    assert_eq!(
        after,
        before.replace("- [ ] Name the conductor", "- [x] Name the conductor")
    );
    dstep(&mut app, &mut dev, "TodoBox:Name the conductor");
    assert_eq!(
        std::fs::read_to_string(dir.join("drafts/chapter-01.md")).unwrap(),
        before
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn scratch_persists_and_is_not_a_listed_note() {
    let dir = fresh("scratch");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    let _ = app.render(&mut dev, Refresh::Full);
    dstep(&mut app, &mut dev, "Scratch");
    dstep(&mut app, &mut dev, "ScratchPane");
    for c in ["a", "b", "c"] {
        tap(&mut app, &mut dev, c);
    }
    assert!(app.scratch.text().contains("abc"));
    assert!(!app.buffer.text().contains("abc"));
    app.handle(Event::Power);
    let restarted = App::new(1440, 1920, &dir);
    assert!(restarted.scratch.text().contains("abc"));
    // The scratch pad never shows up as a note, so it cannot be deleted.
    let notes = restarted.library.all_notes().unwrap();
    assert!(notes.iter().all(|n| !n.as_str().contains("scratchpad")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn drawer_refresh_regions_are_at_least_16px() {
    let dir = fresh("regions");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    let _ = app.render(&mut dev, Refresh::Full);
    for l in [
        "Contents",
        "To-dos",
        "Scratch",
        "ScratchPane",
        "Filter",
        "Contents",
    ] {
        dstep(&mut app, &mut dev, l);
    }
    tap(&mut app, &mut dev, "x");
    assert!(app.scratch.text().contains('x'));
    assert!(dev.refreshes.len() > 3);
    for (r, _) in &dev.refreshes {
        assert!(r.w >= 16 && r.h >= 16, "region {r:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

fn ktap(app: &mut App, dev: &mut DesktopDevice, label: &str) {
    tap(app, dev, label);
}

#[test]
fn replace_all_fifty_is_one_undo_step_and_saved() {
    let dir = fresh("find");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cats.md"), "cat ".repeat(50)).unwrap();
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    app.open(&sigma_core::NotePath::parse("cats.md").unwrap());
    let _ = app.render(&mut dev, Refresh::Full);
    dstep(&mut app, &mut dev, "Find");
    for c in ["c", "a", "t"] {
        ktap(&mut app, &mut dev, c);
    }
    assert_eq!(app.find_label(), "1 of 50");
    assert_eq!(
        app.buffer.text(),
        "cat ".repeat(50),
        "typing went to the field"
    );
    dstep(&mut app, &mut dev, "Aa");
    assert_eq!(app.find_label(), "1 of 50", "all lower case: Aa keeps them");
    dstep(&mut app, &mut dev, "Aa");
    dstep(&mut app, &mut dev, "ReplaceField");
    for c in ["d", "o", "g"] {
        ktap(&mut app, &mut dev, c);
    }
    dstep(&mut app, &mut dev, "All");
    assert_eq!(app.buffer.text(), "dog ".repeat(50));
    assert_eq!(app.find_label(), "None");
    assert_eq!(
        std::fs::read_to_string(dir.join("cats.md")).unwrap(),
        "dog ".repeat(50)
    );
    assert!(app.buffer.undo());
    assert_eq!(app.buffer.text(), "cat ".repeat(50));
    for (r, _) in &dev.refreshes {
        assert!(r.w >= 16 && r.h >= 16, "region {r:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn missing_wiki_link_creates_the_note_and_backlinks_follow() {
    let dir = fresh("wiki");
    let mut dev = DesktopDevice::new(1440, 1920);
    let mut app = App::new(1440, 1920, &dir);
    let _ = app.render(&mut dev, Refresh::Full);
    // field-notes links to chapter-01 and chapter-01 links back.
    let g = app.link_graph();
    let fnp = sigma_core::NotePath::parse("field-notes.md").unwrap();
    let ch = sigma_core::NotePath::parse("drafts/chapter-01.md").unwrap();
    assert_eq!(g.backlinks(&fnp), vec![ch.clone()]);
    assert_eq!(g.backlinks(&ch), vec![fnp.clone()]);
    assert!(g.notes.iter().all(|n| !n.as_str().contains("scratchpad")));
    app.open(&ch);
    let r = app.follow_link("Mara's parcel");
    assert_eq!(r, Some(Refresh::Full));
    assert_eq!(app.current.as_str(), "drafts/Mara's parcel.md");
    assert!(dir.join("drafts/Mara's parcel.md").exists());
    // An existing note opens in preview instead.
    app.follow_link("FIELD-NOTES");
    assert_eq!(app.current, fnp);
    assert!(app.preview.is_some());
    let _ = std::fs::remove_dir_all(dir);
}
