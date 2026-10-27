//! Notes that change on disk while the app has them open (Syncthing
//! delivering a newer version): the app takes the new text in when the
//! reader has no unsaved edit, and never writes over it when they do.
//! `docs/sync.md` §9.

use sigma_device::{Event, HwKey};
use sigma_scribe::App;
use std::path::{Path, PathBuf};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-syncnotes-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn app_on(notes: &Path) -> App {
    let mut app = App::new(1440, 1920, notes);
    app.open(&sigma_core::NotePath::parse("a.md").unwrap());
    app
}

fn conflicts(notes: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(notes)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".conflict-")
        })
        .collect();
    v.sort();
    v
}

#[test]
fn a_note_changed_on_disk_is_taken_in_while_sync_is_on() {
    let d = dir("reload");
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    let mut app = app_on(&d);
    assert_eq!(app.buffer.text(), "first\n");
    app.settings.sync = true;
    // Nothing changed: nothing to draw.
    assert!(app.handle(Event::Tick { t: 10 }).is_none());
    std::fs::write(d.join("a.md"), "second, from another device\n").unwrap();
    // Within the second, the watcher has not looked again.
    assert!(app.handle(Event::Tick { t: 500 }).is_none());
    assert!(
        app.handle(Event::Tick { t: 1100 }).is_some(),
        "a redraw is asked for"
    );
    assert_eq!(app.buffer.text(), "second, from another device\n");
    // It counts as saved: nothing is written back, nothing conflicts.
    assert!(!app.autosave.dirty(app.buffer.revision()));
    assert!(conflicts(&d).is_empty());
    assert_eq!(
        std::fs::read_to_string(d.join("a.md")).unwrap(),
        "second, from another device\n"
    );
}

#[test]
fn nothing_is_watched_while_sync_is_off() {
    let d = dir("off");
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    let mut app = app_on(&d);
    std::fs::write(d.join("a.md"), "changed\n").unwrap();
    assert!(app.handle(Event::Tick { t: 5000 }).is_none());
    assert_eq!(app.buffer.text(), "first\n");
}

#[test]
fn typing_after_a_change_on_disk_does_not_overwrite_it() {
    let d = dir("typing");
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    let mut app = app_on(&d);
    // Syncthing delivers a newer version; the app has not looked yet.
    std::fs::write(d.join("a.md"), "delivered by sync, much longer\n").unwrap();
    app.handle(Event::Key(HwKey::Char('x')));
    // The delivered text is on disk and in the buffer.
    assert_eq!(
        std::fs::read_to_string(d.join("a.md")).unwrap(),
        "delivered by sync, much longer\n"
    );
    assert_eq!(app.buffer.text(), "delivered by sync, much longer\n");
    // What the reader had is kept beside it, with the keystroke in it.
    let kept = conflicts(&d);
    assert_eq!(kept.len(), 1, "{kept:?}");
    let name = kept[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("a.conflict-") && name.ends_with(".md"),
        "{name}"
    );
    let text = std::fs::read_to_string(&kept[0]).unwrap();
    assert!(text.contains('x') && text.contains("first"), "{text:?}");
    // And the next keystroke is an ordinary save.
    app.handle(Event::Key(HwKey::Char('y')));
    assert!(std::fs::read_to_string(d.join("a.md"))
        .unwrap()
        .contains('y'));
    assert_eq!(conflicts(&d).len(), 1);
}

#[test]
fn ordinary_typing_leaves_no_conflict_files() {
    let d = dir("plain");
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    let mut app = app_on(&d);
    for c in "hello".chars() {
        app.handle(Event::Key(HwKey::Char(c)));
    }
    assert!(std::fs::read_to_string(d.join("a.md"))
        .unwrap()
        .contains("hello"));
    assert!(conflicts(&d).is_empty());
}

#[test]
fn a_change_with_the_same_text_is_not_a_conflict() {
    let d = dir("same");
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    let mut app = app_on(&d);
    // Touched, same text (a sync that rewrote it identically).
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    app.handle(Event::Key(HwKey::Char('x')));
    assert!(conflicts(&d).is_empty());
    assert!(std::fs::read_to_string(d.join("a.md"))
        .unwrap()
        .contains('x'));
}

#[test]
fn a_note_deleted_on_disk_is_written_again_by_the_next_edit() {
    let d = dir("deleted");
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    std::fs::write(d.join("b.md"), "other\n").unwrap();
    let mut app = app_on(&d);
    std::fs::remove_file(d.join("a.md")).unwrap();
    app.handle(Event::Key(HwKey::Char('x')));
    assert!(std::fs::read_to_string(d.join("a.md"))
        .unwrap()
        .contains("first"));
    assert!(conflicts(&d).is_empty());
}

#[test]
fn the_scratch_pad_is_guarded_too() {
    let d = dir("scratch");
    std::fs::write(d.join("a.md"), "first\n").unwrap();
    std::fs::write(d.join(".scratchpad.md"), "mine\n").unwrap();
    let mut app = app_on(&d);
    app.settings.sync = true;
    std::fs::write(d.join(".scratchpad.md"), "theirs, from the computer\n").unwrap();
    assert!(app.handle(Event::Tick { t: 2000 }).is_some());
    assert_eq!(app.scratch.text(), "theirs, from the computer\n");
    assert!(conflicts(&d).is_empty());
}
