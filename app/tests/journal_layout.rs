//! Settings -> Journal: changing the folder layout reorganises the journal,
//! and the open note follows its file. Offset and week start only save.

use sigma_core::date::WeekStart;
use sigma_core::journal::JournalLayout;
use sigma_core::{Change, NotePath};
use sigma_scribe::App;

fn path(s: &str) -> NotePath {
    NotePath::parse(s).unwrap()
}

#[test]
fn changing_the_layout_moves_notes_and_the_open_note_follows() {
    let d = std::env::temp_dir().join(format!("sigma-journal-layout-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let notes = d.join("notes");
    std::fs::create_dir_all(notes.join("journal")).unwrap();
    std::fs::write(notes.join("journal/2026-11-02.md"), "# Monday\n").unwrap();
    std::fs::write(notes.join("journal/2026-03-15.md"), "# March\n").unwrap();
    std::fs::write(notes.join("other.md"), "see [[2026-11-02]]\n").unwrap();

    let mut app = App::new(1440, 1920, notes.clone());
    app.open(&path("journal/2026-11-02.md"));
    app.change(Change::JournalLayout(JournalLayout::YearMonth));

    assert_eq!(app.current, path("journal/2026/11/2026-11-02.md"));
    assert!(notes.join("journal/2026/11/2026-11-02.md").exists());
    assert!(notes.join("journal/2026/03/2026-03-15.md").exists());
    assert!(!notes.join("journal/2026-11-02.md").exists());
    assert_eq!(app.journal_note, "Journal: 2 moved");
    assert_eq!(app.buffer.text(), "# Monday\n");
    // A link by bare name still finds the note.
    assert_eq!(
        std::fs::read_to_string(notes.join("other.md")).unwrap(),
        "see [[2026-11-02]]\n"
    );

    // Back to one folder, and the empty year and month folders go.
    app.change(Change::JournalLayout(JournalLayout::Flat));
    assert_eq!(app.current, path("journal/2026-11-02.md"));
    assert!(!notes.join("journal/2026").exists());

    // The other two options move nothing.
    app.change(Change::UtcOffset(2));
    app.change(Change::WeekStart(WeekStart::Sunday));
    assert_eq!(app.settings.utc_offset, 2);
    assert_eq!(app.current, path("journal/2026-11-02.md"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_summary_shows_under_folder_layout_and_the_files_list_follows() {
    let d = std::env::temp_dir().join(format!("sigma-journal-note-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let notes = d.join("notes");
    std::fs::create_dir_all(notes.join("journal")).unwrap();
    std::fs::write(notes.join("journal/2026-11-02.md"), "# Monday\n").unwrap();
    let mut app = App::new(1440, 1920, notes.clone());
    app.open_files();
    app.open_options();
    // Nothing to say before a change.
    let m = app.options_model();
    let row = sigma_render::options::display_row(&m, "Folder layout").unwrap();
    assert!(row.note.is_none());
    app.change(Change::JournalLayout(JournalLayout::YearMonth));
    let m = app.options_model();
    assert_eq!(m.journal_note, "Journal: 1 moved");
    let row = sigma_render::options::display_row(&m, "Folder layout").unwrap();
    assert_eq!(row.note.as_deref(), Some("Journal: 1 moved"));
    // The Files sheet underneath lists the new place, not the old one.
    let f = app.files.as_ref().unwrap();
    assert_eq!(f.message.as_deref(), Some("Journal: 1 moved"));
    // Closing Options forgets it; opening it again starts clean.
    app.open_options();
    assert!(app.options_model().journal_note.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn back_after_a_reorganise_opens_the_moved_note_and_leaves_no_ghost() {
    let d = std::env::temp_dir().join(format!("sigma-journal-back-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let notes = d.join("notes");
    std::fs::create_dir_all(notes.join("journal")).unwrap();
    std::fs::write(notes.join("journal/2026-11-02.md"), "# Monday\n").unwrap();
    std::fs::write(notes.join("journal/2026-11-03.md"), "# Tuesday\n").unwrap();
    std::fs::write(notes.join("plain.md"), "plain\n").unwrap();
    let mut app = App::new(1440, 1920, notes.clone());
    app.open(&path("journal/2026-11-02.md"));
    app.push_history();
    app.open(&path("plain.md"));
    app.push_history();
    app.open(&path("journal/2026-11-03.md"));
    app.change(Change::JournalLayout(JournalLayout::YearMonth));
    assert_eq!(app.current, path("journal/2026/11/2026-11-03.md"));

    app.go_back();
    assert_eq!(app.current, path("plain.md"));
    app.go_back();
    assert_eq!(app.current, path("journal/2026/11/2026-11-02.md"));
    assert_eq!(app.buffer.text(), "# Monday\n");
    assert!(!notes.join("journal/2026-11-02.md").exists(), "no ghost");
    assert!(!notes.join("journal/2026-11-03.md").exists(), "no ghost");

    // Choosing the layout already in force does nothing and says nothing.
    app.journal_note.clear();
    app.change(Change::JournalLayout(JournalLayout::YearMonth));
    assert!(app.journal_note.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}
