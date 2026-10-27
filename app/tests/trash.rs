//! The Trash in Files (v1.17): Delete moves a note to the trash, a Trash
//! button shows while it holds something, the Trash lists what was deleted,
//! Restore puts a note back (under a free name if its path is taken) and
//! Empty trash takes two taps.

use sigma_core::settings::{Change, Orientation};
use sigma_core::NotePath;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_render::i18n::{self, Lang};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-trash-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("drafts")).unwrap();
        for (p, body) in [
            ("keep.md", "kept"),
            ("a.md", "first"),
            ("drafts/b.md", "second"),
        ] {
            std::fs::write(d.join(p), body).unwrap();
        }
        let mut app = App::new(1440, 1920, d.clone());
        app.open(&NotePath::parse("keep.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev, dir: d }
    }
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
    fn has(&self, label: &str) -> bool {
        self.app.control_centre(label).is_some()
    }
    fn delete(&mut self, row: &str) {
        self.tap_on(&format!("More:{row}"));
        self.tap_on("Delete");
        self.tap_on("Delete");
    }
    fn notes(&self) -> Vec<String> {
        self.app
            .library
            .all_notes()
            .unwrap()
            .iter()
            .map(|p| p.as_str().to_owned())
            .collect()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn trash_button_shows_only_while_the_trash_holds_something() {
    let mut r = Rig::new("button");
    r.app.open_files();
    assert!(!r.has("Trash"), "empty trash: no button");
    r.delete("a.md");
    assert!(r.has("Trash"));
    assert!(!r.notes().contains(&"a.md".to_owned()));
    r.tap_on("Trash");
    assert!(r.has("Restore") && r.has("Empty trash") && !r.has("New note"));
    r.tap_on("Up");
    assert!(r.has("New note") && r.has("Trash"));
}

#[test]
fn restore_needs_a_selected_note_and_puts_it_back() {
    let mut r = Rig::new("restore");
    r.app.open_files();
    r.delete("a.md");
    r.tap_on("Trash");
    r.tap_on("Restore");
    assert!(!r.notes().contains(&"a.md".to_owned()), "nothing selected");
    r.tap_on("a.md");
    r.tap_on("Restore");
    assert_eq!(
        std::fs::read_to_string(r.dir.join("a.md")).unwrap(),
        "first"
    );
    assert!(r.notes().contains(&"a.md".to_owned()));
    // The trash is empty: back in Files, without the button, and the note
    // is in the list.
    assert!(!r.has("Trash") && r.has("a.md"));
    assert_eq!(r.app.library.trash_len(), 0);
}

#[test]
fn restore_under_a_free_name_when_the_path_is_taken() {
    let mut r = Rig::new("clash");
    r.app.open_files();
    r.delete("a.md");
    std::fs::write(r.dir.join("a.md"), "new one").unwrap();
    r.tap_on("Up");
    r.app.open_files();
    r.tap_on("Trash");
    r.tap_on("a.md");
    r.tap_on("Restore");
    assert_eq!(
        std::fs::read_to_string(r.dir.join("a.md")).unwrap(),
        "new one"
    );
    assert_eq!(
        std::fs::read_to_string(r.dir.join("a (restored).md")).unwrap(),
        "first"
    );
}

#[test]
fn a_note_in_a_deleted_folder_comes_back_with_its_folder() {
    let mut r = Rig::new("folder");
    r.app.open_files();
    r.delete("drafts");
    assert!(!r.dir.join("drafts").exists());
    r.tap_on("Trash");
    r.tap_on("drafts");
    r.tap_on("Restore");
    assert_eq!(
        std::fs::read_to_string(r.dir.join("drafts/b.md")).unwrap(),
        "second"
    );
}

#[test]
fn empty_trash_takes_two_taps_and_any_other_tap_disarms_it() {
    let mut r = Rig::new("empty");
    r.app.open_files();
    r.delete("a.md");
    r.tap_on("Trash");
    r.tap_on("Empty trash");
    assert_eq!(r.app.library.trash_len(), 1, "one tap deletes nothing");
    r.tap_on("a.md");
    r.tap_on("Empty trash");
    assert_eq!(r.app.library.trash_len(), 1, "a tap between disarms it");
    r.tap_on("Empty trash");
    assert_eq!(r.app.library.trash_len(), 0);
    assert!(!r.dir.join(".trash").exists());
    assert!(!r.has("Trash"), "back in Files with nothing to show");
    assert!(r.notes().contains(&"keep.md".to_owned()));
}

#[test]
fn the_buttons_fit_in_every_language_and_orientation() {
    for l in [Lang::En, Lang::Fr, Lang::De, Lang::Pt, Lang::PseudoLong] {
        for o in [Orientation::Portrait, Orientation::Landscape] {
            let mut r = Rig::new(&format!("fit-{l:?}-{o:?}"));
            r.app.set_language(l);
            r.app.change(Change::Orientation(o));
            r.app.open_files();
            r.delete("a.md");
            for trash in [false, true] {
                if trash {
                    r.tap_on("Trash");
                    r.tap_on("a.md");
                }
                r.app.render(&mut r.dev, Refresh::Full).unwrap();
                let (w, h) = (r.app.canvas.width as i32, r.app.canvas.height as i32);
                let min = r.app.min_target();
                for (n, t) in r.app.touch_targets() {
                    assert!(t.w >= min && t.h >= min, "{l:?} {o:?}: {n} {t:?}");
                    assert!(
                        t.x >= 0 && t.y >= 0 && t.x + t.w <= w && t.y + t.h <= h,
                        "{l:?} {o:?}: {n} {t:?} off screen"
                    );
                }
            }
        }
    }
    i18n::set_lang(Lang::En);
}
