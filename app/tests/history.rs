//! v1.0.2 Back navigation: history stack, Back arrow, Esc / Alt+Left.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Mods, Refresh};
use sigma_scribe::{App, HISTORY_CAP};

struct Rig {
    app: App,
    dev: DesktopDevice,
}

impl Rig {
    fn new(tag: &str, w: u32, h: u32) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-hist-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(w, h, d.clone());
        let long: String = (0..400).map(|i| format!("line {i}\n")).collect();
        for (n, body) in [
            ("a.md", format!("# A\n\nGo to [[b]].\n\n{long}")),
            ("b.md", "# B\n\nOn to [[c]].".to_owned()),
            ("c.md", "# C\n\nBack to [[a]].".to_owned()),
        ] {
            std::fs::write(d.join(n), body).unwrap();
        }
        app.open(&sigma_core::NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(w, h);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev }
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(k) = r {
            self.app.render(&mut self.dev, k).unwrap();
        }
        r
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.app.control_centre(label).expect(label);
        self.send(Event::Tap { x, y })
    }
}

#[test]
fn links_push_and_back_restores_note_mode_and_position() {
    let mut r = Rig::new("links", 1440, 1920);
    assert!(
        r.app.control_centre("Back").is_none(),
        "no Back while empty"
    );
    // Write mode, caret deep in a.md.
    let at = r.app.buffer.text().find("line 300").unwrap();
    r.app.buffer.set_caret(at);
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    r.app.enter_preview();
    let page = r.app.preview.as_ref().unwrap().page;
    r.app.exit_preview();
    r.app.follow_link("b");
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert_eq!(r.app.current.as_str(), "b.md");
    assert!(r.app.preview.is_some());
    r.app.follow_link("c");
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert_eq!(r.app.history.len(), 2);
    assert!(r.app.control_centre("Back").is_some());
    // Back button: to b.md in preview.
    assert_eq!(r.tap_on("Back"), Some(Refresh::Full));
    assert_eq!(r.app.current.as_str(), "b.md");
    assert!(r.app.preview.is_some());
    // Esc: to a.md in write mode at the old caret.
    r.send(Event::Key(HwKey::Escape));
    assert_eq!(r.app.current.as_str(), "a.md");
    assert!(r.app.preview.is_none());
    assert_eq!(r.app.buffer.caret(), at);
    assert!(r.app.history.is_empty());
    assert!(r.app.control_centre("Back").is_none());
    let _ = page;
}

#[test]
fn alt_left_goes_back_and_history_is_capped() {
    let mut r = Rig::new("cap", 1440, 1920);
    for i in 0..(HISTORY_CAP + 10) {
        r.app.follow_link(if i % 2 == 0 { "b" } else { "c" });
    }
    assert_eq!(r.app.history.len(), HISTORY_CAP);
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    let was = r.app.current.clone();
    r.send(Event::Chord {
        key: HwKey::Left,
        mods: Mods {
            alt: true,
            ..Mods::default()
        },
    });
    assert_ne!(r.app.current, was);
    assert_eq!(r.app.history.len(), HISTORY_CAP - 1);
}

#[test]
fn files_and_contents_push() {
    let mut r = Rig::new("files", 1440, 1920);
    r.tap_on("Files");
    r.tap_on("b.md");
    assert_eq!(r.app.current.as_str(), "b.md");
    assert_eq!(r.app.history.len(), 1);
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "a.md");
}

#[test]
fn title_bar_fits_with_back_in_both_orientations() {
    for land in [false, true] {
        let mut r = Rig::new(&format!("fit{land}"), 1440, 1920);
        if land {
            r.app.change(sigma_core::settings::Change::Orientation(
                sigma_core::settings::Orientation::Landscape,
            ));
        }
        let w = r.app.canvas.width;
        r.app.follow_link("b");
        r.app.render(&mut r.dev, Refresh::Full).unwrap();
        let l = r.app.screen.layout(false, &r.app.keyboard);
        let s = r.app.screen.scale().px(44.0);
        assert!(l.back_button.w >= s && l.back_button.h >= s);
        assert!(l.back_button.x + l.back_button.w < l.files_button.x);
        assert!(l.name_rect.x + l.name_rect.w <= l.back_button.x);
        assert!(
            l.name_rect.w >= r.app.screen.page_scale().px(200.0),
            "{land}: {:?}",
            l.name_rect
        );
        assert!(l.options_button.x + l.options_button.w <= w as i32);
    }
}

/// Taps the `[[…]]` link whose target is `target` on the shown preview
/// page; returns how it was drawn.
fn tap_link(r: &mut Rig, target: &str) -> Option<sigma_render::preview::Underline> {
    let p = r.app.preview.as_ref().expect("in preview");
    let piece = p.doc.lines[p.doc.pages[p.page].clone()]
        .iter()
        .flat_map(|l| &l.pieces)
        .find(|pc| pc.target.as_deref() == Some(target))
        .unwrap_or_else(|| panic!("no link to {target}"))
        .clone();
    let (x, y) = r
        .app
        .screen
        .preview_link_point(&r.app.keyboard, &p.doc, p.page, piece.text.trim())
        .unwrap();
    r.send(Event::Tap { x, y });
    piece.underline
}

fn note_files(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    v.sort();
    v
}

#[test]
fn heading_link_opens_the_note_and_creates_nothing() {
    use sigma_render::preview::Underline;
    let mut r = Rig::new("heading", 1440, 1920);
    let root = r.app.library.root().to_path_buf();
    std::fs::write(root.join("chapter-01.md"), "# Chapter\n\n## Intro\n\nText.").unwrap();
    std::fs::write(
        root.join("index.md"),
        "# Index\n\nSee [[chapter-01#Intro]] and [[#Top]].\n",
    )
    .unwrap();
    r.app
        .open(&sigma_core::NotePath::parse("index.md").unwrap());
    r.app.enter_preview();
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    let before = note_files(&root);
    // Both draw as existing notes, not as missing (dashed) links.
    assert_eq!(tap_link(&mut r, "chapter-01#Intro"), Some(Underline::Solid));
    assert_eq!(r.app.current.as_str(), "chapter-01.md");
    assert!(r.app.preview.is_some());
    assert_eq!(note_files(&root), before, "no note#heading.md created");
    // Back, then the bare heading: stays on this note.
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "index.md");
    assert_eq!(tap_link(&mut r, "#Top"), Some(Underline::Solid));
    assert_eq!(r.app.current.as_str(), "index.md");
    assert!(r.app.preview.is_some());
    assert_eq!(note_files(&root), before, "no #Top.md created");
    // A missing note with a heading is created without the heading.
    r.app.follow_link("fresh#Part");
    assert_eq!(r.app.current.as_str(), "fresh.md");
    assert!(root.join("fresh.md").exists());
    assert!(!root.join("fresh#Part.md").exists());
}

/// Opens a.md, then b.md, then c.md from links, so Back goes c → b → a.
fn a_b_c(tag: &str) -> (Rig, std::path::PathBuf) {
    let mut r = Rig::new(tag, 1440, 1920);
    let root = r.app.library.root().to_path_buf();
    r.app.follow_link("b");
    r.app.follow_link("c");
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert_eq!(r.app.history.len(), 2);
    (r, root)
}

/// Types `s` into the open note and saves it, as the user would.
fn type_and_save(r: &mut Rig, s: &str) {
    r.app.exit_preview();
    for c in s.chars() {
        r.send(Event::Key(HwKey::Char(c)));
    }
    r.app.enter_preview(); // saves
}

#[test]
fn back_after_rename_finds_the_renamed_note() {
    let (mut r, root) = a_b_c("rename");
    r.app.open_files();
    r.tap_on("More:a.md");
    r.tap_on("Rename");
    r.send(Event::Key(HwKey::End));
    for _ in 0..".md".len() {
        r.send(Event::Key(HwKey::Backspace));
    }
    r.send(Event::Key(HwKey::Char('2')));
    r.send(Event::Key(HwKey::Return));
    assert!(root.join("a2.md").exists() && !root.join("a.md").exists());
    r.app.files = None;
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "b.md");
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "a2.md");
    assert!(
        r.app.buffer.text().starts_with("# A"),
        "the note, not an empty one"
    );
    type_and_save(&mut r, "x");
    assert!(!root.join("a.md").exists(), "no phantom a.md");
}

#[test]
fn back_after_delete_skips_the_deleted_note() {
    let (mut r, root) = a_b_c("delete");
    r.app.open_files();
    r.tap_on("More:b.md");
    r.tap_on("Delete");
    r.tap_on("Delete");
    assert!(!root.join("b.md").exists());
    assert_eq!(r.app.history.len(), 1, "b.md pruned");
    r.app.files = None;
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "a.md");
    assert!(r.app.history.is_empty());
    assert!(r.app.control_centre("Back").is_none(), "Back slot gone");
    type_and_save(&mut r, "x");
    assert!(!root.join("b.md").exists(), "no phantom b.md");
    // The open note deleted: Back never lands on it either.
    let (mut r, root) = a_b_c("delete-open");
    r.app.follow_link("a");
    r.app.open_files();
    r.tap_on("More:c.md");
    r.tap_on("Delete");
    r.tap_on("Delete");
    r.app.files = None;
    while r.app.go_back().is_some() {
        assert_ne!(r.app.current.as_str(), "c.md");
    }
    type_and_save(&mut r, "x");
    assert!(!root.join("c.md").exists(), "no phantom c.md");
}

#[test]
fn back_after_move_opens_the_note_in_its_folder() {
    let (mut r, root) = a_b_c("move");
    std::fs::create_dir_all(root.join("folder")).unwrap();
    r.app.open_files();
    r.tap_on("More:a.md");
    r.tap_on("Move");
    r.tap_on("folder");
    assert!(root.join("folder/a.md").exists() && !root.join("a.md").exists());
    r.app.files = None;
    r.tap_on("Back");
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "folder/a.md");
    assert!(r.app.buffer.text().starts_with("# A"));
    type_and_save(&mut r, "x");
    assert!(!root.join("a.md").exists(), "no phantom a.md");
    // A folder moved with the note inside: the entry follows too.
    let (mut r, root) = a_b_c("move-folder");
    std::fs::create_dir_all(root.join("box")).unwrap();
    std::fs::create_dir_all(root.join("folder")).unwrap();
    std::fs::rename(root.join("a.md"), root.join("folder/a.md")).unwrap();
    r.app.history[0].note = sigma_core::NotePath::parse("folder/a.md").unwrap();
    r.app.open_files();
    r.tap_on("More:folder");
    r.tap_on("Move");
    r.tap_on("box");
    assert!(root.join("box/folder/a.md").exists());
    r.app.files = None;
    r.tap_on("Back");
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "box/folder/a.md");
}
