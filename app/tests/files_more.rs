//! More in Files (v1.17): the Rename / Move / Delete row shows for any row,
//! including the last one that fits on the page, in both orientations.

use sigma_core::settings::{Change, Orientation};
use sigma_core::NotePath;
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-more-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for i in 0..30 {
            std::fs::write(d.join(format!("n{i:02}.md")), "x").unwrap();
        }
        let mut app = App::new(1440, 1920, d.clone());
        app.open(&NotePath::parse("n00.md").unwrap());
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
    /// The names of the rows shown, from the controls on screen.
    fn shown(&self) -> Vec<String> {
        (0..30)
            .map(|i| format!("n{i:02}.md"))
            .filter(|n| self.has(&format!("More:{n}")))
            .collect()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn every_shown_row_opens_its_actions(r: &mut Rig) {
    r.app.open_files();
    let shown = r.shown();
    assert!(shown.len() >= 5, "{shown:?}");
    // Last first: the row that used to lose its actions.
    for name in shown.iter().rev() {
        // Each from the top of the list, which More may have scrolled.
        r.app.open_files();
        r.tap_on(&format!("More:{name}"));
        for c in ["Rename", "Move", "Delete"] {
            assert!(r.has(c), "More on {name}: no {c}");
        }
        assert!(r.has(&format!("More:{name}")), "the row stays in view");
        r.tap_on(&format!("More:{name}"));
        assert!(!r.has("Rename"), "closing More on {name}");
    }
}

#[test]
fn more_on_every_row_of_a_page_shows_its_actions_in_portrait() {
    let mut r = Rig::new("portrait");
    every_shown_row_opens_its_actions(&mut r);
}

#[test]
fn more_on_every_row_of_a_page_shows_its_actions_in_landscape() {
    let mut r = Rig::new("landscape");
    r.app.change(Change::Orientation(Orientation::Landscape));
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    every_shown_row_opens_its_actions(&mut r);
}

#[test]
fn more_and_the_trash_button_fit_together() {
    let mut r = Rig::new("trash-button");
    r.app.open_files();
    r.tap_on("More:n00.md");
    // The Trash button needs something in the trash first.
    r.tap_on("Delete");
    r.tap_on("Delete");
    assert!(r.has("Trash"));
    let last = r.shown().pop().unwrap();
    r.tap_on(&format!("More:{last}"));
    assert!(r.has("Delete"), "actions with three bottom buttons");
    assert!(r.has("Trash"));
}

#[test]
fn turning_the_page_closes_the_open_row() {
    let mut r = Rig::new("page");
    r.app.open_files();
    r.tap_on("More:n02.md");
    assert!(r.has("Delete"));
    r.send(Event::PageTurn { forward: true });
    assert!(
        !r.has("Delete"),
        "the row left the page; so did its actions"
    );
    assert!(!r.shown().contains(&"n02.md".to_owned()));
    r.send(Event::PageTurn { forward: false });
    // Back on the first page nothing is open.
    assert!(r.shown().contains(&"n02.md".to_owned()));
    assert!(!r.has("Delete"));
}
