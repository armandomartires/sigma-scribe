//! S24.1 Replace in all notes: the All key in All notes plans the replace
//! and opens a review sheet; Apply rewrites the ticked notes on a worker, the
//! open note through its buffer as one undo step; Undo last replace puts the
//! notes back; Cancel, Esc and Back return to the results list.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Refresh};
use sigma_render::Scope;
use sigma_scribe::{App, Notice};
use std::path::PathBuf;

const WORD: &str = "zebrafox";

const A: &str = "# A\n\nThe zebrafox on the bridge.\n";
const B: &str = "# B\n\nTwo zebrafoxes, one Zebrafox.\nAnother zebrafox here.\n";
const C: &str = "# C\n\nNothing to find.\n";
const D: &str = "# D\n\nA zebrafox at dawn.\nAnd a zebrafox at dusk.\n";

struct Rig {
    app: App,
    dev: DesktopDevice,
    root: PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-replace-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d.clone());
        for (n, body) in [("a.md", A), ("b.md", B), ("c.md", C), ("d.md", D)] {
            std::fs::write(d.join(n), body).unwrap();
        }
        app.open(&sigma_core::NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev, root: d }
    }
    /// Sends an event, draws, then waits for the workers and draws again.
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(k) = r {
            self.app.render(&mut self.dev, k).unwrap();
        }
        if self.app.settle() {
            self.app.render(&mut self.dev, Refresh::Full).unwrap();
        }
        r
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.app.control_centre(label).expect(label);
        self.send(Event::Tap { x, y })
    }
    fn type_hw(&mut self, s: &str) {
        for c in s.chars() {
            self.send(Event::Key(HwKey::Char(c)));
        }
    }
    fn disk(&self, name: &str) -> String {
        std::fs::read_to_string(self.root.join(name)).unwrap()
    }
    /// Find in All notes with the query typed, and `with` in the Replace field.
    fn search_with(&mut self, q: &str, with: &str) {
        self.tap_on("Find");
        self.tap_on("All notes");
        self.type_hw(q);
        self.tap_on("ReplaceField");
        self.type_hw(with);
        assert_eq!(self.app.find_scope(), Scope::All);
    }
    fn snapshots(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.root.join(".trash"))
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                    .filter(|n| n.starts_with("replace-"))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }
}

/// The Replace field takes text in All notes, the single Replace key stays
/// inert, and typing changes nothing on disk.
#[test]
fn the_replace_field_works_in_all_notes_and_writes_nothing() {
    let mut r = Rig::new("field");
    r.search_with(WORD, "lynx");
    assert_eq!(r.app.find.as_ref().unwrap().replace.text(), "lynx");
    assert_eq!(r.tap_on("Replace"), None);
    assert!(!r.app.review_shown());
    assert_eq!(r.disk("b.md"), B);
    assert_eq!(r.app.buffer.text(), A);
    // Tab moves between the two fields now.
    r.send(Event::Key(HwKey::Tab));
    r.type_hw("x");
    assert_eq!(
        r.app.find.as_ref().unwrap().query.text(),
        format!("{WORD}x")
    );
}

/// Open the sheet, untick a note, apply: the other notes change on disk, the
/// unticked one does not, the open note is changed through its buffer.
#[test]
fn applying_changes_the_ticked_notes_only() {
    let mut r = Rig::new("apply");
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    assert!(r.app.review_shown());
    // The keyboard is gone while the sheet is up.
    assert!(r.app.key_centre("q").is_none());
    {
        let rv = r.app.find.as_ref().unwrap().review.as_ref().unwrap();
        let names: Vec<_> = rv.plan.notes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(names, ["a.md", "b.md", "d.md"]);
        assert_eq!(rv.plan.total_matches(), 1 + 3 + 2);
        assert_eq!(rv.selection.total(), 6);
    }
    // Every control of the sheet is a real target.
    let min = r.app.min_target();
    for (n, rect) in r.app.touch_targets() {
        assert!(rect.w >= min && rect.h >= min, "{n} {rect:?}");
    }
    // Untick b.md: its row is a toggle.
    r.tap_on("ReviewNote:b.md");
    {
        let rv = r.app.find.as_ref().unwrap().review.as_ref().unwrap();
        assert_eq!(rv.selection.ticked_in(1), 0);
        assert_eq!(rv.selection.total(), 3);
        assert_eq!(rv.selection.notes(), 2);
    }
    // Apply: the sheet shows progress at once, and nothing answers a tap
    // until the worker is done.
    let (x, y) = r.app.control_centre("ReviewApply").unwrap();
    assert_eq!(
        r.app.handle(Event::Tap { x, y }),
        Some(Refresh::Partial),
        "Apply starts the worker"
    );
    assert!(r.app.replace_busy());
    assert_eq!(
        r.app
            .find
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .progress,
        Some((0, 2))
    );
    let (cx, cy) = r.app.control_centre("ReviewCancel").unwrap();
    assert_eq!(r.app.handle(Event::Tap { x: cx, y: cy }), None);
    assert_eq!(r.app.handle(Event::Key(HwKey::Escape)), None);
    assert!(r.app.settle(), "the answer redraws the screen");
    assert!(!r.app.replace_busy());
    // Disk: d.md rewritten, b.md untouched, c.md untouched.
    assert_eq!(r.disk("d.md"), D.replace(WORD, "lynx"));
    assert_eq!(r.disk("b.md"), B);
    assert_eq!(r.disk("c.md"), C);
    // The open note went through the buffer, and was saved byte for byte.
    assert_eq!(r.app.buffer.text(), A.replace(WORD, "lynx"));
    assert_eq!(r.disk("a.md"), r.app.buffer.text());
    // The sheet closed, the band says what happened and offers Undo.
    let f = r.app.find.as_ref().unwrap();
    assert!(f.review.is_none());
    assert_eq!(
        f.notice,
        Some(Notice::Replaced {
            matches: 3,
            notes: 2,
            changed: 0,
            failed: 0
        })
    );
    assert!(f.snapshot.is_some());
    assert_eq!(r.snapshots().len(), 1);
    assert!(r.app.control_centre("ReplaceUndo").is_some());
    // The results list follows: only b.md still has the word.
    assert_eq!(f.rows.len(), 3);
    assert!(f.rows.iter().all(|row| row.path == "b.md"));
    // The open note changed in one undo step.
    assert!(r.app.buffer.undo());
    assert_eq!(r.app.buffer.text(), A);
    assert!(!r.app.buffer.undo(), "one step, not several");
}

/// Undo last replace puts every note back, the open one through its buffer,
/// and the button goes away; a note edited since is left alone.
#[test]
fn undo_last_replace_restores_the_notes_and_hides() {
    let mut r = Rig::new("undo");
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    r.tap_on("ReviewApply");
    assert_eq!(r.app.buffer.text(), A.replace(WORD, "lynx"));
    assert_eq!(
        r.disk("b.md"),
        B.replace(WORD, "lynx").replace("Zebrafox", "lynx")
    );
    // Someone edits d.md after the replace.
    let edited = format!("{}more\n", r.disk("d.md"));
    std::fs::write(r.root.join("d.md"), &edited).unwrap();
    r.tap_on("ReplaceUndo");
    assert_eq!(r.app.buffer.text(), A);
    assert_eq!(r.disk("a.md"), A);
    assert_eq!(r.disk("b.md"), B);
    assert_eq!(r.disk("d.md"), edited, "an edited note is left alone");
    let f = r.app.find.as_ref().unwrap();
    assert_eq!(f.notice, Some(Notice::Restored { notes: 2, left: 1 }));
    assert!(f.snapshot.is_none());
    assert!(r.app.control_centre("ReplaceUndo").is_none(), "hidden");
    // The results see the words again: a.md's one and b.md's three; d.md
    // keeps its replaced text.
    assert_eq!(f.rows.len(), 4);
}

/// The same bar after a restart: the newest snapshot still can be undone.
#[test]
fn the_snapshot_survives_a_restart() {
    let mut r = Rig::new("restart");
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    r.tap_on("ReviewApply");
    let root = r.root.clone();
    drop(r);
    let mut app = App::new(1440, 1920, root.clone());
    app.open(&sigma_core::NotePath::parse("c.md").unwrap());
    let mut dev = DesktopDevice::new(1440, 1920);
    app.render(&mut dev, Refresh::Full).unwrap();
    let mut r = Rig {
        app,
        dev,
        root: root.clone(),
    };
    r.tap_on("Find");
    r.tap_on("All notes");
    let (x, y) = r
        .app
        .control_centre("ReplaceUndo")
        .expect("Undo last replace");
    r.send(Event::Tap { x, y });
    assert_eq!(r.disk("a.md"), A);
    assert_eq!(r.disk("b.md"), B);
    assert_eq!(r.disk("d.md"), D);
}

/// Cancel, Esc and Back close the sheet and leave the results list with its
/// query, scope and page; nothing was written.
#[test]
fn cancel_returns_to_the_results() {
    let mut r = Rig::new("cancel");
    let many: String = (0..40).map(|i| format!("{WORD} {i}\n")).collect();
    std::fs::write(r.root.join("many.md"), many).unwrap();
    r.search_with(WORD, "lynx");
    let rows = r.app.find.as_ref().unwrap().rows.len();
    assert!(rows > 40);
    r.send(Event::PageTurn { forward: true });
    let first = r.app.find.as_ref().unwrap().first;
    assert!(first > 0);
    for how in ["button", "esc", "back"] {
        r.tap_on("All");
        assert!(r.app.review_shown(), "{how}");
        match how {
            "button" => assert_eq!(r.tap_on("ReviewCancel"), Some(Refresh::Full)),
            "esc" => assert_eq!(r.send(Event::Key(HwKey::Escape)), Some(Refresh::Full)),
            _ => assert_eq!(r.tap_on("Back"), Some(Refresh::Full)),
        }
        assert!(!r.app.review_shown(), "{how}");
        let f = r.app.find.as_ref().expect("the bar stays open");
        assert_eq!(f.scope, Scope::All, "{how}");
        assert_eq!(f.query.text(), WORD, "{how}");
        assert_eq!(f.replace.text(), "lynx", "{how}");
        assert_eq!(f.rows.len(), rows, "{how}");
        assert_eq!(f.first, first, "{how}");
        assert!(r.app.history.is_empty(), "{how}: Back left history alone");
        // The keyboard is back.
        assert!(r.app.key_centre("q").is_some(), "{how}");
    }
    assert_eq!(r.disk("b.md"), B);
    assert_eq!(r.app.buffer.text(), A);
    assert!(r.snapshots().is_empty());
}

/// The sheet pages with the page keys, and Apply with nothing ticked does
/// nothing.
#[test]
fn the_sheet_pages_and_apply_needs_a_tick() {
    let mut r = Rig::new("pages");
    let many: String = (0..60).map(|i| format!("{WORD} {i}\n")).collect();
    std::fs::write(r.root.join("many.md"), many).unwrap();
    r.search_with(WORD, "");
    r.tap_on("All");
    assert!(r.app.review_shown());
    let l = r.app.review_layout_now().unwrap();
    assert!(l.pages > 1, "{l:?}");
    assert_eq!(
        r.send(Event::PageTurn { forward: true }),
        Some(Refresh::Partial)
    );
    assert_eq!(r.app.review_layout_now().unwrap().index, 1);
    r.tap_on("ReviewPrev");
    assert_eq!(r.app.review_layout_now().unwrap().index, 0);
    // Nothing ticked: Apply does nothing and the sheet stays.
    {
        let rv = r.app.find.as_mut().unwrap().review.as_mut().unwrap();
        rv.selection = sigma_core::replace::Selection::none(&rv.plan);
    }
    assert_eq!(r.tap_on("ReviewApply"), None);
    assert!(r.app.review_shown());
    assert!(!r.app.replace_busy());
    assert_eq!(r.disk("b.md"), B);
    // An empty replacement deletes: the header says so.
    let h = r.app.screen.review_header(
        &r.app.find.as_ref().unwrap().review.as_ref().unwrap().plan,
        10_000.0,
    );
    assert!(h.starts_with("Delete \u{201c}zebrafox\u{201d}"), "{h}");
}

/// The open note is reloaded (Sync) or edited while the apply runs: the
/// worker's text for it is stale, so the newer text stays in the buffer and
/// the note counts as changed since the review. The other notes still go.
#[test]
fn a_buffer_changed_while_the_apply_runs_is_not_overwritten() {
    let mut r = Rig::new("race-apply");
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    let (x, y) = r.app.control_centre("ReviewApply").unwrap();
    assert_eq!(r.app.handle(Event::Tap { x, y }), Some(Refresh::Partial));
    assert!(r.app.replace_busy());
    // Whatever the worker has done by now, the buffer moves on after the job
    // has its text, as a sync reload does.
    let newer = "# A\n\nEdited elsewhere; zebrafox stays.\n";
    r.app.buffer.replace_text(newer.to_owned(), 0);
    assert!(r.app.settle());
    assert_eq!(r.app.buffer.text(), newer, "the newer text survives");
    // The worker never wrote the open note; autosave may have saved the
    // newer buffer, but the replacement is nowhere in it.
    assert!(!r.disk("a.md").contains("lynx"));
    assert_eq!(r.disk("d.md"), D.replace(WORD, "lynx"), "the rest went");
    let f = r.app.find.as_ref().unwrap();
    assert_eq!(
        f.notice,
        Some(Notice::Replaced {
            matches: 5,
            notes: 2,
            changed: 1,
            failed: 0
        })
    );
}

/// The same for Undo last replace: a reload during the undo wins.
#[test]
fn a_buffer_changed_while_the_undo_runs_is_not_overwritten() {
    let mut r = Rig::new("race-undo");
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    r.tap_on("ReviewApply");
    assert_eq!(r.app.buffer.text(), A.replace(WORD, "lynx"));
    let (x, y) = r.app.control_centre("ReplaceUndo").unwrap();
    r.app.handle(Event::Tap { x, y });
    assert!(r.app.replace_busy());
    let newer = "# A\n\nEdited elsewhere.\n";
    r.app.buffer.replace_text(newer.to_owned(), 0);
    assert!(r.app.settle());
    assert_eq!(r.app.buffer.text(), newer);
    assert_eq!(r.disk("b.md"), B, "the other notes were put back");
    let f = r.app.find.as_ref().unwrap();
    assert_eq!(f.notice, Some(Notice::Restored { notes: 2, left: 1 }));
}

/// Ticking a line ticks its plain matches and never a wiki-link match on it
/// (S24-D6); applying leaves the link as it was.
#[test]
fn a_line_tick_leaves_wiki_link_matches_unticked() {
    let mut r = Rig::new("linkline");
    std::fs::write(
        r.root.join("e.md"),
        format!("{WORD} and [[{WORD}]] together\n"),
    )
    .unwrap();
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    let ei = {
        let rv = r.app.find.as_ref().unwrap().review.as_ref().unwrap();
        let ei = rv
            .plan
            .notes
            .iter()
            .position(|n| n.path.as_str() == "e.md")
            .unwrap();
        assert_eq!(rv.plan.notes[ei].matches.len(), 2);
        assert!(!rv.selection.is_ticked(ei, 1), "the link starts unticked");
        ei
    };
    let ticked = |r: &Rig| {
        let rv = r.app.find.as_ref().unwrap().review.as_ref().unwrap();
        (rv.selection.is_ticked(ei, 0), rv.selection.is_ticked(ei, 1))
    };
    assert_eq!(ticked(&r), (true, false));
    r.tap_on("ReviewLine:e.md:1");
    assert_eq!(ticked(&r), (false, false));
    r.tap_on("ReviewLine:e.md:1");
    assert_eq!(ticked(&r), (true, false), "the link is still unticked");
    r.tap_on("ReviewApply");
    assert_eq!(r.disk("e.md"), format!("lynx and [[{WORD}]] together\n"));
}

/// A note whose matches are all inside wiki links: its row ticks them (the
/// only control for them besides a line tick), Apply changes the link text,
/// and a second tap unticks them again. A note with both kinds keeps its
/// links unticked.
#[test]
fn a_note_row_ticks_a_link_only_note() {
    let mut r = Rig::new("linknote");
    std::fs::write(
        r.root.join("trip.md"),
        format!("See [[{WORD}]] and [[old {WORD}]] for the map.\n"),
    )
    .unwrap();
    r.search_with(WORD, "port");
    r.tap_on("All");
    let ticked = |r: &Rig| {
        let rv = r.app.find.as_ref().unwrap().review.as_ref().unwrap();
        let ti = rv
            .plan
            .notes
            .iter()
            .position(|n| n.path.as_str() == "trip.md")
            .unwrap();
        rv.selection.ticked_in(ti)
    };
    assert_eq!(ticked(&r), 0, "link matches start unticked");
    r.tap_on("ReviewNote:trip.md");
    assert_eq!(ticked(&r), 2, "the note row ticks its link matches");
    r.tap_on("ReviewNote:trip.md");
    assert_eq!(ticked(&r), 0, "and unticks them again");
    r.tap_on("ReviewNote:trip.md");
    r.tap_on("ReviewApply");
    assert_eq!(
        r.disk("trip.md"),
        "See [[port]] and [[old port]] for the map.\n"
    );
}

/// The band's Undo is for the snapshot of the replace it reports, never an
/// older one: a replace that wrote nothing offers no Undo.
#[test]
fn the_band_undo_is_never_an_older_snapshot() {
    let mut r = Rig::new("band");
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    r.tap_on("ReviewNote:b.md"); // b.md stays out of the first replace
    r.tap_on("ReviewApply");
    assert_eq!(r.snapshots().len(), 1);
    let first = r.app.find.as_ref().unwrap().snapshot.clone().unwrap();
    // Second replace: only b.md, which is edited under the review.
    r.tap_on("All");
    assert!(r.app.review_shown());
    std::fs::write(r.root.join("b.md"), format!("moved\n{B}")).unwrap();
    r.tap_on("ReviewApply");
    let f = r.app.find.as_ref().unwrap();
    assert_eq!(
        f.notice,
        Some(Notice::Replaced {
            matches: 0,
            notes: 0,
            changed: 1,
            failed: 0
        })
    );
    assert_ne!(f.snapshot.as_ref(), Some(&first), "not the older one");
    assert!(f.snapshot.is_none());
    assert!(r.app.control_centre("ReplaceUndo").is_none());
    assert_eq!(r.disk("b.md"), format!("moved\n{B}"));
}

/// After the newest snapshot is undone, the bar offers no older one, also
/// after a restart.
#[test]
fn undo_last_replace_is_not_offered_again_for_an_older_snapshot() {
    let mut r = Rig::new("older");
    r.search_with(WORD, "lynx");
    r.tap_on("All");
    r.tap_on("ReviewNote:b.md");
    r.tap_on("ReviewApply"); // snapshot 1: a.md, d.md
                             // Snapshots are named by the second: make the next one sort later.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    r.tap_on("All"); // b.md only
    r.tap_on("ReviewApply"); // snapshot 2: b.md
    assert_eq!(r.snapshots().len(), 2);
    r.tap_on("ReplaceUndo"); // undoes snapshot 2
    assert_eq!(r.disk("b.md"), B);
    assert!(r.app.control_centre("ReplaceUndo").is_none());
    let root = r.root.clone();
    drop(r);
    let mut app = App::new(1440, 1920, root);
    app.open(&sigma_core::NotePath::parse("c.md").unwrap());
    let mut dev = DesktopDevice::new(1440, 1920);
    app.render(&mut dev, Refresh::Full).unwrap();
    let mut r = Rig {
        app,
        dev,
        root: PathBuf::new(),
    };
    r.tap_on("Find");
    r.tap_on("All notes");
    assert!(
        r.app.control_centre("ReplaceUndo").is_none(),
        "snapshot 1 is older: not offered"
    );
}
