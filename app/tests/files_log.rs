//! What Files writes to the log (v1.17): opening the Trash, new notes and
//! folders, rename, move, delete, restore and empty, with the failures at
//! WARNING. The log is global to the process, so this is one test.

use sigma_core::log::{Level, FILE_NAME};
use sigma_core::NotePath;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: std::path::PathBuf,
}

impl Rig {
    fn send(&mut self, e: Event) {
        if let Some(r) = self.app.handle(e) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }
    fn tap_on(&mut self, label: &str) {
        let (x, y) = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no control {label}"));
        self.send(Event::Tap { x, y });
    }
    fn log(&self) -> String {
        std::fs::read_to_string(self.dir.join("log").join(FILE_NAME)).unwrap_or_default()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        sigma_core::log::configure(false, Level::Info, None);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn files_logs_what_it_does() {
    let d = std::env::temp_dir().join(format!("sigma-files-log-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let notes = d.join("notes");
    std::fs::create_dir_all(notes.join("drafts")).unwrap();
    for (p, body) in [("keep.md", "kept"), ("a.md", "first"), ("drafts/b.md", "x")] {
        std::fs::write(notes.join(p), body).unwrap();
    }
    let mut app = App::new(1440, 1920, notes.clone());
    app.open(&NotePath::parse("keep.md").unwrap());
    let mut dev = DesktopDevice::new(1440, 1920);
    app.render(&mut dev, Refresh::Full).unwrap();
    let mut r = Rig { app, dev, dir: d };
    sigma_core::log::configure(true, Level::Debug, Some(r.dir.join("log")));

    // A new folder, named at once.
    r.app.open_files();
    r.tap_on("New folder");
    r.send(Event::Key(HwKey::Char('x')));
    r.send(Event::Key(HwKey::Return));
    let log = r.log();
    assert!(
        log.contains("INFO     files: new folder new folder"),
        "{log}"
    );
    assert!(
        log.contains("INFO     files: rename new folder -> new folderx"),
        "{log}"
    );

    // Move a note into drafts.
    r.tap_on("More:a.md");
    r.tap_on("Move");
    r.tap_on("drafts");
    assert!(
        r.log().contains("INFO     files: move a.md -> drafts/a.md"),
        "{}",
        r.log()
    );

    // Delete, open the Trash, restore over a name that was taken again.
    r.tap_on("drafts");
    r.tap_on("More:a.md");
    r.tap_on("Delete");
    r.tap_on("Delete");
    std::fs::write(notes.join("drafts/a.md"), "new").unwrap();
    r.tap_on("Up");
    r.tap_on("Trash");
    r.tap_on("a.md");
    r.tap_on("Restore");
    let log = r.log();
    assert!(
        log.contains("INFO     files: delete drafts/a.md -> .trash/"),
        "{log}"
    );
    assert!(log.contains("DEBUG    ui: open trash"), "{log}");
    assert!(
        log.contains(
            "INFO     files: restore drafts/a.md as drafts/a (restored).md (the name was taken)"
        ),
        "{log}"
    );

    // Delete again and restore to its own path.
    r.app.open_files();
    r.tap_on("drafts");
    r.tap_on("More:a (restored).md");
    r.tap_on("Delete");
    r.tap_on("Delete");
    r.tap_on("Up");
    r.tap_on("Trash");
    r.tap_on("a (restored).md");
    r.tap_on("Restore");
    assert!(
        r.log()
            .contains("INFO     files: restore drafts/a (restored).md\n"),
        "{}",
        r.log()
    );

    // A failure is a warning: the file is gone when Delete is confirmed.
    r.tap_on("drafts");
    r.tap_on("More:b.md");
    r.tap_on("Delete");
    std::fs::remove_file(notes.join("drafts/b.md")).unwrap();
    r.tap_on("Delete");
    assert!(
        r.log().contains("WARNING  files: delete drafts/b.md: "),
        "{}",
        r.log()
    );

    // Empty the trash: two taps, one line.
    r.app.open_files();
    r.tap_on("More:keep.md");
    // keep.md is the open note; deleting it is allowed while others exist.
    r.tap_on("Delete");
    r.tap_on("Delete");
    r.tap_on("Trash");
    r.tap_on("Empty trash");
    assert!(!r.log().contains("empty trash"), "armed only");
    r.tap_on("Empty trash");
    assert!(
        r.log().contains("INFO     files: empty trash (1 items)"),
        "{}",
        r.log()
    );
    // No note text in the log.
    assert!(!r.log().contains("kept"), "{}", r.log());
}
