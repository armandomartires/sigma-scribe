//! S22.1 Search across all notes: the Find bar's All notes scope, the
//! results list, a row opens the note at its line, Back returns to the
//! list, and the list follows edits.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_render::Scope;
use sigma_scribe::App;

/// A word no seed note has, so the counts below are the fixture's.
const WORD: &str = "zebrafox";

struct Rig {
    app: App,
    dev: DesktopDevice,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-search-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d.clone());
        for (n, body) in [
            ("a.md", "# A\n\nThe zebrafox on the bridge.\n".to_owned()),
            (
                "b.md",
                "# B\n\nTwo zebrafoxes, one Zebrafox.\nAnother zebrafox here.\n".to_owned(),
            ),
            ("c.md", "# C\n\nNothing to find.\n".to_owned()),
        ] {
            std::fs::write(d.join(n), body).unwrap();
        }
        app.open(&sigma_core::NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev }
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(k) = r {
            self.app.render(&mut self.dev, k).unwrap();
        }
        if self.app.settle() {
            self.app.render(&mut self.dev, Refresh::Partial).unwrap();
        }
        r
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.app.control_centre(label).expect(label);
        self.send(Event::Tap { x, y })
    }
    /// Types on the USB keyboard and waits for the search.
    fn type_hw(&mut self, s: &str) {
        for c in s.chars() {
            self.send(Event::Key(HwKey::Char(c)));
        }
    }
    /// Opens Find in All notes and searches `q`.
    fn search(&mut self, q: &str) {
        self.tap_on("Find");
        self.tap_on("All notes");
        self.type_hw(q);
        assert_eq!(self.app.find_scope(), Scope::All);
    }
    fn rows(&self) -> usize {
        self.app.find.as_ref().map_or(0, |f| f.rows.len())
    }
    fn first(&self) -> usize {
        self.app.find.as_ref().map_or(0, |f| f.first)
    }
}

#[test]
fn all_notes_lists_the_hits_and_a_row_opens_the_note_at_its_line() {
    let mut r = Rig::new("list");
    r.tap_on("Find");
    assert_eq!(r.app.find_scope(), Scope::Note);
    assert_eq!(r.app.find_label(), "Find");
    r.tap_on("All notes");
    assert!(
        r.app.find_label().starts_with("Type to search"),
        "{}",
        r.app.find_label()
    );
    let caret = r.app.buffer.caret();
    r.type_hw(WORD);
    // 1 in a.md, 3 in b.md (case off), none in c.md; b's first line has two.
    assert_eq!(r.rows(), 4);
    assert!(
        r.app.find_label().starts_with("4 matches in 2 notes"),
        "{}",
        r.app.find_label()
    );
    // The open buffer was left alone: no selection, caret still at the top.
    assert!(r.app.buffer.selection().is_none());
    assert_eq!(r.app.buffer.caret(), caret);
    assert_eq!(r.app.current.as_str(), "a.md");
    // Every control of the list is a real target.
    let min = r.app.min_target();
    for (n, rect) in r.app.touch_targets() {
        assert!(rect.w >= min && rect.h >= min, "{n} {rect:?}");
    }
    // Rows: path, line and the hit inside the line.
    let f = r.app.find.as_ref().unwrap();
    assert_eq!(f.rows[0].path, "a.md");
    assert_eq!(f.rows[0].line, 3);
    assert_eq!(&f.rows[0].text[f.rows[0].hit.0..f.rows[0].hit.1], WORD);
    assert_eq!((f.rows[1].path.as_str(), f.rows[1].line), ("b.md", 3));
    assert_eq!((f.rows[2].path.as_str(), f.rows[2].line), ("b.md", 3));
    assert_eq!(
        &f.rows[2].text[f.rows[2].hit.0..f.rows[2].hit.1],
        "Zebrafox"
    );
    assert_eq!((f.rows[3].path.as_str(), f.rows[3].line), ("b.md", 4));
    // The single Replace key is inert in All notes (S24-D5); All opens the
    // review sheet (S24.1), and Cancel brings the same list back.
    assert_eq!(r.tap_on("Replace"), None);
    assert!(!r.app.review_shown());
    r.tap_on("All");
    assert!(r.app.review_shown(), "All opens the review sheet");
    assert_eq!(r.tap_on("ReviewCancel"), Some(Refresh::Full));
    assert!(!r.app.review_shown());
    assert_eq!(r.rows(), 4);
    // The third row: b.md, caret on "Zebrafox", bar closed, Back armed.
    assert_eq!(r.tap_on("Result:2"), Some(Refresh::Full));
    assert_eq!(r.app.current.as_str(), "b.md");
    assert!(r.app.find.is_none());
    let text = r.app.buffer.text().to_owned();
    assert_eq!(r.app.buffer.caret(), text.find("Zebrafox").unwrap());
    assert_eq!(r.app.history.len(), 1);
    assert!(r.app.history[0].search.is_some());
    // Back: the list again, same query, in the note it was opened from.
    assert_eq!(r.tap_on("Back"), Some(Refresh::Full));
    assert_eq!(r.app.current.as_str(), "a.md");
    let f = r.app.find.as_ref().expect("bar reopened");
    assert_eq!(f.scope, Scope::All);
    assert_eq!(f.query.text(), WORD);
    assert_eq!(r.rows(), 4);
    assert!(r.app.history.is_empty());
    // Esc closes the bar and the list.
    r.send(Event::Key(HwKey::Escape));
    assert!(r.app.find.is_none());
    assert!(!r.app.search_list_shown());
}

#[test]
fn back_returns_to_the_same_page() {
    let mut r = Rig::new("page");
    let root = r.app.library.root().to_path_buf();
    let many: String = (0..40).map(|i| format!("{WORD} {i}\n")).collect();
    std::fs::write(root.join("many.md"), many).unwrap();
    r.search(WORD);
    assert_eq!(r.rows(), 44);
    assert_eq!(r.first(), 0);
    // Page buttons turn the list, not the hidden note.
    assert_eq!(
        r.send(Event::PageTurn { forward: true }),
        Some(Refresh::Partial)
    );
    assert_eq!(
        r.send(Event::PageTurn { forward: true }),
        Some(Refresh::Partial)
    );
    let page = r.first();
    assert!(page > 0);
    assert_eq!(r.app.screen.scroll, 0, "the note did not scroll");
    assert!(
        r.app.find_label().contains("page 3 of"),
        "{}",
        r.app.find_label()
    );
    // ‹ then › on the list's own arrows.
    r.tap_on("SearchPrev");
    assert!(r.first() < page);
    r.tap_on("SearchNext");
    assert_eq!(r.first(), page);
    // Open the first row of this page; Back lands on the same page.
    let at = r.first();
    r.tap_on(&format!("Result:{at}"));
    assert_eq!(r.app.current.as_str(), "many.md");
    assert!(r.app.buffer.caret() > 0);
    r.tap_on("Back");
    assert_eq!(r.app.find_scope(), Scope::All);
    assert_eq!(r.first(), page);
    assert_eq!(r.rows(), 44);
}

#[test]
fn editing_a_note_changes_the_results() {
    let mut r = Rig::new("edit");
    r.search(WORD);
    assert_eq!(r.rows(), 4);
    // Open the hit in a.md and type the word again after it.
    r.tap_on("Result:0");
    assert_eq!(r.app.current.as_str(), "a.md");
    r.send(Event::Key(HwKey::End));
    r.type_hw(&format!(" {WORD}"));
    // Back re-runs the query over the saved notes.
    r.tap_on("Back");
    assert_eq!(r.app.find_scope(), Scope::All);
    assert_eq!(r.rows(), 5);
    assert!(
        r.app.find_label().starts_with("5 matches in 2 notes"),
        "{}",
        r.app.find_label()
    );
    // Typing in the query narrows it; Aa narrows it further.
    r.type_hw("es");
    assert_eq!(r.rows(), 1);
    r.tap_on("Aa");
    assert_eq!(r.rows(), 1);
    r.send(Event::Key(HwKey::Backspace));
    r.send(Event::Key(HwKey::Backspace));
    assert_eq!(r.rows(), 4, "Aa drops the capitalised one");
    // Back to This note: the match after the caret is selected on the page.
    r.app.buffer.set_caret(0);
    r.tap_on("This note");
    assert_eq!(r.app.find_scope(), Scope::Note);
    assert!(r.app.buffer.selection().is_some());
    assert_eq!(r.app.find_label(), "1 of 2");
}

#[test]
fn up_down_move_the_selection_and_enter_opens_it() {
    let mut r = Rig::new("keys");
    r.search(WORD);
    assert_eq!(r.rows(), 4);
    assert_eq!(r.app.find.as_ref().unwrap().selected, None);
    r.send(Event::Key(HwKey::Down));
    assert_eq!(r.app.find.as_ref().unwrap().selected, Some(0));
    r.send(Event::Key(HwKey::Down));
    r.send(Event::Key(HwKey::Down));
    r.send(Event::Key(HwKey::Up));
    assert_eq!(r.app.find.as_ref().unwrap().selected, Some(1));
    // Enter opens the selected row: b.md at its first hit.
    r.send(Event::Key(HwKey::Return));
    assert_eq!(r.app.current.as_str(), "b.md");
    assert!(r.app.find.is_none());
    let text = r.app.buffer.text().to_owned();
    assert_eq!(r.app.buffer.caret(), text.find(WORD).unwrap());
    // Enter with no selection opens the first row.
    r.tap_on("Back");
    r.send(Event::Key(HwKey::Return));
    assert_eq!(r.app.current.as_str(), "a.md");
    assert_eq!(r.app.history.len(), 1);
}

#[test]
fn an_empty_query_and_no_matches_say_so() {
    let mut r = Rig::new("empty");
    r.tap_on("Find");
    r.tap_on("All notes");
    assert_eq!(r.rows(), 0);
    assert!(
        r.app.find_label().starts_with("Type to search"),
        "{}",
        r.app.find_label()
    );
    r.type_hw("qqqqq");
    assert_eq!(r.rows(), 0);
    assert_eq!(r.app.find_label(), "No matches");
    // Enter and the page buttons do nothing on an empty list.
    assert_eq!(r.send(Event::Key(HwKey::Return)), None);
    assert_eq!(r.send(Event::PageTurn { forward: true }), None);
    assert!(r.app.find.is_some());
}
