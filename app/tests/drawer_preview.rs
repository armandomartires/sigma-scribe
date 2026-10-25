//! v1.0.3 (S10.3-D1): the drawer stays open in preview — layout beside the
//! drawer, Contents jumps, to-do toggles, scratch typing; never on sheets.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-dp-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d.clone());
        app.set_builtin_graph(true); // the Graph icon is part of the sheet check
        app.set_builtin_panes(true);
        let mut body = String::from("# Top\n\n- [ ] first task\n- [x] done task\n\n");
        for i in 0..60 {
            body.push_str(&format!("Paragraph {i} with enough words in it to wrap onto a second line when the drawer narrows the preview page beside it, and a third. More filler text here so that each paragraph is long enough to need several rows at either width, with more.\n\n"));
        }
        body.push_str("## Far heading\n\nEnd.\n");
        std::fs::write(d.join("n.md"), &body).unwrap();
        app.open(&sigma_core::NotePath::parse("n.md").unwrap());
        app.buffer.set_caret(0);
        app.screen.scroll = 0;
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev, dir: d }
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.app.control_centre(label).expect(label);
        self.send(Event::Tap { x, y })
    }
    fn settle(&mut self) {
        self.app.settle();
        self.app.render(&mut self.dev, Refresh::Partial).unwrap();
    }
}

#[test]
fn drawer_stays_in_preview_and_page_is_beside_it() {
    let mut r = Rig::new("vis");
    r.tap_on("Contents");
    assert!(r.app.drawer_visible());
    r.tap_on("Preview");
    assert!(r.app.preview.is_some());
    assert!(r.app.drawer_visible(), "drawer stays open in preview");
    let narrow = r.app.preview.as_ref().unwrap().doc.clone();
    // Closing the pane in preview re-lays the page at full width.
    r.tap_on("Contents");
    assert!(!r.app.drawer_visible());
    let wide = r.app.preview.as_ref().unwrap().doc.clone();
    assert!(
        narrow.width < wide.width,
        "{} vs {}",
        narrow.width,
        wide.width
    );
    assert!(narrow.lines.len() > wide.lines.len(), "narrow wraps more");
    assert!(r.app.screen.preview_drawer.is_none());
    // Reopen in preview; a sheet hides it.
    r.tap_on("Contents");
    assert!(r.app.drawer_visible());
    r.tap_on("Graph");
    assert!(!r.app.drawer_visible(), "never over a sheet");
}

#[test]
fn contents_in_preview_jumps_to_the_heading_page() {
    let mut r = Rig::new("jump");
    r.tap_on("Contents");
    r.tap_on("Preview");
    assert_eq!(r.app.preview.as_ref().unwrap().page, 0);
    r.tap_on("Heading:Far heading");
    let p = r.app.preview.as_ref().unwrap();
    assert!(p.page > 0, "jumped to a later page");
    assert_eq!(p.page, p.doc.pages.len() - 1);
    assert!(r.app.preview.is_some(), "still in preview");
    // Back returns to the first page.
    r.send(Event::Key(HwKey::Escape));
    assert_eq!(r.app.preview.as_ref().map(|p| p.page), Some(0));
}

#[test]
fn todo_toggles_in_preview_writes_and_rerenders() {
    let mut r = Rig::new("todo");
    r.tap_on("To-dos");
    r.settle();
    r.tap_on("Preview");
    let before = r.app.preview.as_ref().unwrap().doc.clone();
    r.tap_on("TodoBox:first task");
    assert!(r.app.buffer.text().contains("- [x] first task"));
    let disk = std::fs::read_to_string(r.dir.join("n.md")).unwrap();
    assert!(disk.contains("- [x] first task"), "saved to disk");
    let after = &r.app.preview.as_ref().unwrap().doc;
    assert_ne!(&before, after, "preview re-rendered");
    assert!(r.app.preview.is_some());
    // And back.
    r.tap_on("TodoBox:first task");
    assert!(r.app.buffer.text().contains("- [ ] first task"));
}

#[test]
fn scratch_types_in_preview_note_stays_read_only() {
    let mut r = Rig::new("scratch");
    r.tap_on("Scratch");
    r.tap_on("Preview");
    let note = r.app.buffer.text().to_owned();
    r.tap_on("ScratchPane");
    for c in "hi".chars() {
        r.send(Event::Key(HwKey::Char(c)));
    }
    assert!(r.app.scratch.text().ends_with("hi"));
    assert_eq!(r.app.buffer.text(), note, "note untouched");
    // Escape drops scratch focus; then keys do nothing to the note.
    r.send(Event::Key(HwKey::Escape));
    r.send(Event::Key(HwKey::Char('z')));
    assert_eq!(r.app.buffer.text(), note);
    assert!(!r.app.scratch.text().ends_with('z'));
}
