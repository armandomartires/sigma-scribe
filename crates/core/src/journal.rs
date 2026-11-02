//! The file side of the daily journal: one Markdown note per day, named
//! `yyyy-mm-dd.md`, under the library folder `journal/`. Where in that folder
//! a day's note lives depends on the [`JournalLayout`]; [`reorganise`] moves
//! the notes when the layout changes. The heading text comes from the caller
//! (core has no translations).

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};

use crate::date::Date;
use crate::library::{Kind, Library, NotePath};
use crate::settings::Words;

/// The journal folder, under the library root.
pub const JOURNAL: &str = "journal";

/// Optional template inside the journal folder (hidden, so never listed).
const TEMPLATE: &str = ".template.md";

/// How the journal folder is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JournalLayout {
    /// `journal/2026-11-02.md`.
    #[default]
    Flat,
    /// `journal/2026/11/2026-11-02.md`.
    YearMonth,
    /// `journal/2026/W45/2026-11-02.md` (ISO week and week-year).
    YearWeek,
}

impl Words for JournalLayout {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Flat, "flat", "One folder"),
        (Self::YearMonth, "month", "Year and month"),
        (Self::YearWeek, "week", "Year and week"),
    ];
}

/// What [`reorganise`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Notes moved, `(from, to)`, in order. Callers use this to follow a
    /// note they have open.
    pub moved: Vec<(NotePath, NotePath)>,
    /// Notes left where they were because something already sat at the
    /// target: `(from, wanted target)`.
    pub skipped: Vec<(NotePath, NotePath)>,
    /// Notes that could not be moved: `(from, wanted target, why)`.
    pub failed: Vec<(NotePath, NotePath, String)>,
    /// Problems that did not undo a move: an ink folder that did not follow
    /// its note, or links that could not all be rewritten.
    pub problems: Vec<String>,
}

fn journal_root() -> NotePath {
    NotePath::root().join(JOURNAL)
}

/// Where `date`'s note belongs under `layout`.
#[must_use]
pub fn entry_path(date: Date, layout: JournalLayout) -> NotePath {
    let name = format!("{date}.md");
    let root = journal_root();
    match layout {
        JournalLayout::Flat => root.join(&name),
        JournalLayout::YearMonth => root
            .join(&format!("{:04}", date.y))
            .join(&format!("{:02}", date.m))
            .join(&name),
        JournalLayout::YearWeek => {
            let (y, w) = date.iso_week();
            root.join(&format!("{y:04}"))
                .join(&format!("W{w:02}"))
                .join(&name)
        }
    }
}

/// The folders under `journal/` (itself first), parents before children.
fn folders(lib: &Library) -> Vec<NotePath> {
    let mut out = vec![journal_root()];
    let mut i = 0;
    while i < out.len() {
        let here = out[i].clone();
        for e in lib.list(&here).unwrap_or_default() {
            if e.kind == Kind::Folder {
                out.push(e.path);
            }
        }
        i += 1;
    }
    out
}

/// Every note under `journal/`, at any depth, whose file name is a valid
/// `yyyy-mm-dd` plus a note extension (case-insensitive, as the library
/// lists notes: `.MD` counts), wherever it sits. If several files claim a
/// date, the one at [`entry_path`] for `layout` wins when it exists;
/// otherwise the one with the smallest path, whatever its depth. The others
/// are left alone and ignored.
#[must_use]
pub fn entries(lib: &Library, layout: JournalLayout) -> BTreeMap<Date, NotePath> {
    let mut found: BTreeMap<Date, Vec<NotePath>> = BTreeMap::new();
    for f in folders(lib) {
        for e in lib.list(&f).unwrap_or_default() {
            if e.kind != Kind::Note {
                continue;
            }
            let Some((stem, ext)) = e.path.name().rsplit_once('.') else {
                continue;
            };
            if !ext.eq_ignore_ascii_case("md") {
                continue;
            }
            if let Some(d) = Date::parse(stem) {
                found.entry(d).or_default().push(e.path);
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(d, mut v)| {
            let want = entry_path(d, layout);
            let pick = v
                .iter()
                .position(|p| *p == want)
                .or_else(|| (0..v.len()).min_by(|&i, &j| v[i].as_str().cmp(v[j].as_str())))?;
            Some((d, v.swap_remove(pick)))
        })
        .collect()
}

/// The note for `date`, wherever it is under `journal/` (see [`entries`] for
/// which one when there are several).
#[must_use]
pub fn find(lib: &Library, date: Date, layout: JournalLayout) -> Option<NotePath> {
    entries(lib, layout).remove(&date)
}

/// Opens `date`'s note, creating it (and its folders) at [`entry_path`] when
/// there is none anywhere. New notes hold `journal/.template.md` with
/// `{{date}}` replaced by `yyyy-mm-dd` and `{{title}}` by `heading`, or
/// `# {heading}` and a blank line without a template. Returns the path and
/// whether it was created. An existing note is never rewritten: the file is
/// created exclusively, and if something turns up at the target in the
/// meantime that is what is returned.
///
/// # Errors
/// The note cannot be created.
pub fn open_or_create(
    lib: &Library,
    date: Date,
    layout: JournalLayout,
    heading: &str,
) -> io::Result<(NotePath, bool)> {
    if let Some(p) = find(lib, date, layout) {
        return Ok((p, false));
    }
    let p = entry_path(date, layout);
    let text = match fs::read_to_string(lib.fs_path(&journal_root()).join(TEMPLATE)) {
        Ok(t) => t
            .replace("{{date}}", &date.to_string())
            .replace("{{title}}", heading),
        Err(_) => format!("# {heading}\n\n"),
    };
    let path = lib.fs_path(&p);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok((p, false)),
        Err(e) => return Err(e),
    };
    if let Err(e) = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
    {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(e);
    }
    Ok((p, true))
}

/// Moves every dated note to its [`entry_path`] under `layout`, creating
/// folders and rewriting `[[wiki]]` links once for all moves. A note whose
/// target is taken is skipped, and one that fails to move is reported; the
/// others carry on and nothing is overwritten. Afterwards the folders this
/// emptied (the old folders of moved notes and their parents below
/// `journal/`) are removed with `remove_dir`; a folder holding anything,
/// hidden files included, stays, and so does any folder that was already
/// empty. Other files stay where they are.
#[must_use]
pub fn reorganise(lib: &Library, layout: JournalLayout) -> Report {
    let mut report = Report::default();
    let mut moves = Vec::new();
    for (date, from) in entries(lib, layout) {
        let to = entry_path(date, layout);
        if from == to {
            continue;
        }
        if lib.exists(&to) {
            report.skipped.push((from, to));
        } else {
            moves.push((from, to));
        }
    }
    let m = lib.move_many(&moves);
    report.moved = m.done;
    report.failed = m
        .failed
        .into_iter()
        .map(|(f, t, e)| (f, t, e.to_string()))
        .collect();
    for (f, t, e) in m.ink_errors {
        report.problems.push(format!("ink of {f} -> {t}: {e}"));
    }
    if let Some(e) = m.relink_error {
        report.problems.push(format!("links: {e}"));
    }
    // Only folders a moved note left: its parents up to, not including,
    // `journal/`, deepest first.
    let root = journal_root();
    let mut emptied: Vec<NotePath> = Vec::new();
    for (from, _) in &report.moved {
        let mut p = from.parent();
        while p != root && p.within(&root) {
            if !emptied.contains(&p) {
                emptied.push(p.clone());
            }
            p = p.parent();
        }
    }
    emptied.sort_by_key(|p| std::cmp::Reverse(p.as_str().len()));
    for f in emptied {
        let _ = fs::remove_dir(lib.fs_path(&f));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(tag: &str) -> Library {
        let d = std::env::temp_dir().join(format!("sigma-journal-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        Library::new(d)
    }

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::new(y, m, day).unwrap()
    }

    fn put(l: &Library, p: &str, text: &str) {
        let path = l.root().join(p);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn get(l: &Library, p: &str) -> String {
        fs::read_to_string(l.root().join(p)).unwrap()
    }

    #[test]
    fn paths_per_layout() {
        let x = d(2026, 11, 2);
        assert_eq!(
            entry_path(x, JournalLayout::Flat).as_str(),
            "journal/2026-11-02.md"
        );
        assert_eq!(
            entry_path(x, JournalLayout::YearMonth).as_str(),
            "journal/2026/11/2026-11-02.md"
        );
        assert_eq!(
            entry_path(x, JournalLayout::YearWeek).as_str(),
            "journal/2026/W45/2026-11-02.md"
        );
        // ISO week-year: 2025-12-29 is in week 1 of 2026.
        assert_eq!(
            entry_path(d(2025, 12, 29), JournalLayout::YearWeek).as_str(),
            "journal/2026/W01/2025-12-29.md"
        );
        assert_eq!(
            entry_path(d(2027, 1, 1), JournalLayout::YearWeek).as_str(),
            "journal/2026/W53/2027-01-01.md"
        );
    }

    #[test]
    fn words() {
        assert_eq!(JournalLayout::default(), JournalLayout::Flat);
        for l in [
            JournalLayout::Flat,
            JournalLayout::YearMonth,
            JournalLayout::YearWeek,
        ] {
            assert_eq!(JournalLayout::from_word(l.word()), Some(l));
        }
        assert_eq!(JournalLayout::YearMonth.word(), "month");
    }

    #[test]
    fn entries_any_depth_and_only_dates() {
        let l = lib("entries");
        put(&l, "journal/2026-11-02.md", "a");
        put(&l, "journal/2026/W45/2026-11-03.md", "b");
        put(&l, "journal/misc/2026-11-04.md", "c");
        put(&l, "journal/notes.md", "x");
        put(&l, "journal/2026-13-01.md", "x");
        put(&l, "journal/2026-11-05.txt", "x");
        put(&l, "journal/2026-1-05.md", "x");
        put(&l, "journal/.template.md", "x");
        put(&l, "other/2026-11-06.md", "x");
        let e = entries(&l, JournalLayout::Flat);
        let got: Vec<_> = e.iter().map(|(k, v)| (k.to_string(), v.as_str())).collect();
        assert_eq!(
            got,
            vec![
                ("2026-11-02".to_owned(), "journal/2026-11-02.md"),
                ("2026-11-03".to_owned(), "journal/2026/W45/2026-11-03.md"),
                ("2026-11-04".to_owned(), "journal/misc/2026-11-04.md"),
            ]
        );
        assert_eq!(
            find(&l, d(2026, 11, 3), JournalLayout::Flat)
                .unwrap()
                .as_str(),
            "journal/2026/W45/2026-11-03.md"
        );
        assert_eq!(find(&l, d(2026, 11, 5), JournalLayout::Flat), None);
    }

    #[test]
    fn open_or_create_heading_template_existing() {
        let l = lib("open");
        let x = d(2026, 11, 2);
        let (p, made) = open_or_create(&l, x, JournalLayout::YearMonth, "Monday").unwrap();
        assert!(made);
        assert_eq!(p.as_str(), "journal/2026/11/2026-11-02.md");
        assert_eq!(get(&l, p.as_str()), "# Monday\n\n");
        // Existing: found anywhere, not rewritten, layout ignored.
        fs::write(l.fs_path(&p), "mine").unwrap();
        let (q, made) = open_or_create(&l, x, JournalLayout::Flat, "Other").unwrap();
        assert!(!made);
        assert_eq!(q, p);
        assert_eq!(get(&l, p.as_str()), "mine");
        // Template.
        put(
            &l,
            "journal/.template.md",
            "# {{title}} ({{date}})\n\n- {{date}}\n",
        );
        let (t, made) = open_or_create(&l, d(2026, 11, 3), JournalLayout::Flat, "Tuesday").unwrap();
        assert!(made);
        assert_eq!(t.as_str(), "journal/2026-11-03.md");
        assert_eq!(
            get(&l, t.as_str()),
            "# Tuesday (2026-11-03)\n\n- 2026-11-03\n"
        );
    }

    #[test]
    fn reorganise_round_trip() {
        let l = lib("reorg");
        put(&l, "journal/2026-11-02.md", "one");
        put(&l, "journal/2025-12-29.md", "two");
        put(
            &l,
            "linker.md",
            "see [[journal/2026-11-02]] and [[2026-11-02]]\n",
        );
        let link = |l: &Library| get(l, "linker.md");

        let r = reorganise(&l, JournalLayout::YearMonth);
        assert_eq!((r.moved.len(), r.skipped.len(), r.failed.len()), (2, 0, 0));
        assert!(r.moved.contains(&(
            NotePath::parse("journal/2026-11-02.md").unwrap(),
            NotePath::parse("journal/2026/11/2026-11-02.md").unwrap()
        )));
        assert_eq!(get(&l, "journal/2026/11/2026-11-02.md"), "one");
        assert_eq!(get(&l, "journal/2025/12/2025-12-29.md"), "two");
        assert!(!l.root().join("journal/2026-11-02.md").exists());
        assert_eq!(
            link(&l),
            "see [[journal/2026/11/2026-11-02]] and [[2026-11-02]]\n"
        );

        let r = reorganise(&l, JournalLayout::YearWeek);
        assert_eq!(r.moved.len(), 2);
        assert_eq!(get(&l, "journal/2026/W45/2026-11-02.md"), "one");
        assert_eq!(get(&l, "journal/2026/W01/2025-12-29.md"), "two");
        assert!(!l.root().join("journal/2025").exists());
        assert!(!l.root().join("journal/2026/11").exists());
        assert_eq!(
            link(&l),
            "see [[journal/2026/W45/2026-11-02]] and [[2026-11-02]]\n"
        );

        // Already in place: nothing to do.
        assert!(reorganise(&l, JournalLayout::YearWeek).moved.is_empty());

        let r = reorganise(&l, JournalLayout::Flat);
        assert_eq!(r.moved.len(), 2);
        assert_eq!(get(&l, "journal/2026-11-02.md"), "one");
        assert_eq!(get(&l, "journal/2025-12-29.md"), "two");
        assert!(!l.root().join("journal/2026").exists());
        assert!(l.root().join("journal").is_dir());
        assert_eq!(link(&l), "see [[journal/2026-11-02]] and [[2026-11-02]]\n");
    }

    #[test]
    fn reorganise_collision_and_kept_files() {
        let l = lib("collide");
        put(&l, "journal/2026-11-02.md", "flat");
        put(&l, "journal/2026-11-03.md", "flat3");
        // Same date as the flat one, already at the month target.
        put(&l, "journal/2026/11/2026-11-03.md", "deep3");
        put(&l, "journal/2026/readme.md", "keep");
        put(&l, "journal/old/2026-11-04.md", "four");
        let r = reorganise(&l, JournalLayout::YearMonth);
        // The file already at the target wins its date, so nothing is skipped.
        assert_eq!(r.moved.len(), 2);
        assert!(r.skipped.is_empty());
        assert_eq!(get(&l, "journal/2026-11-03.md"), "flat3");
        assert_eq!(get(&l, "journal/2026/11/2026-11-03.md"), "deep3");
        assert_eq!(get(&l, "journal/2026/11/2026-11-02.md"), "flat");
        assert_eq!(get(&l, "journal/2026/11/2026-11-04.md"), "four");
        assert_eq!(get(&l, "journal/2026/readme.md"), "keep");
        assert!(!l.root().join("journal/old").exists());
    }

    #[test]
    fn dotfile_keeps_folder() {
        let l = lib("dot");
        put(&l, "journal/2026/11/2026-11-02.md", "x");
        put(&l, "journal/2026/11/.keep", "");
        let _ = reorganise(&l, JournalLayout::Flat);
        assert!(l.root().join("journal/2026-11-02.md").exists());
        assert!(l.root().join("journal/2026/11/.keep").exists());
    }

    #[test]
    fn uppercase_extension_counts_and_is_never_overwritten() {
        let l = lib("upper");
        put(&l, "journal/2026-11-02.MD", "mine");
        let x = d(2026, 11, 2);
        assert_eq!(
            find(&l, x, JournalLayout::Flat).unwrap().as_str(),
            "journal/2026-11-02.MD"
        );
        let (p, made) = open_or_create(&l, x, JournalLayout::Flat, "H").unwrap();
        assert!(!made);
        assert_eq!(p.as_str(), "journal/2026-11-02.MD");
        assert_eq!(get(&l, "journal/2026-11-02.MD"), "mine");
        // A file that appears at the target after the lookup is kept: create
        // is exclusive.
        put(&l, "journal/2026-11-03.md", "raced");
        let (p, made) = open_or_create(&l, d(2026, 11, 3), JournalLayout::Flat, "H").unwrap();
        assert!(!made);
        assert_eq!(p.as_str(), "journal/2026-11-03.md");
        assert_eq!(get(&l, "journal/2026-11-03.md"), "raced");
    }

    #[test]
    fn exclusive_create_refuses_an_existing_file() {
        let l = lib("excl");
        put(&l, "journal/2026-11-04.md", "keep");
        // Bypass the lookup: what open_or_create does at the target.
        let path = l.root().join("journal/2026-11-04.md");
        let e = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(get(&l, "journal/2026-11-04.md"), "keep");
    }

    #[test]
    fn duplicate_date_prefers_the_layout_target_after_reorganise() {
        let l = lib("dup");
        put(&l, "journal/2026-11-02.md", "top");
        put(&l, "journal/old/2026-11-02.md", "old");
        let x = d(2026, 11, 2);
        // Without a copy at the target: smallest path, whatever the depth.
        assert_eq!(
            find(&l, x, JournalLayout::YearMonth).unwrap().as_str(),
            "journal/2026-11-02.md"
        );
        let r = reorganise(&l, JournalLayout::YearMonth);
        assert_eq!(r.moved.len(), 1);
        assert_eq!(
            find(&l, x, JournalLayout::YearMonth).unwrap().as_str(),
            "journal/2026/11/2026-11-02.md"
        );
        // Stable: reorganising again moves nothing and the same note wins.
        assert!(reorganise(&l, JournalLayout::YearMonth).moved.is_empty());
        assert_eq!(get(&l, "journal/2026/11/2026-11-02.md"), "top");
        assert_eq!(get(&l, "journal/old/2026-11-02.md"), "old");
    }

    #[test]
    fn partial_failure_continues_and_reports_the_moves() {
        let l = lib("partial");
        put(&l, "journal/2026-11-02.md", "a");
        put(&l, "journal/2026-12-02.md", "b");
        put(&l, "journal/2027-01-02.md", "c");
        put(
            &l,
            "linker.md",
            "[[journal/2027-01-02]] [[journal/2026-11-02]]\n",
        );
        // A file where the December folder would be: that move fails, the
        // others still happen and are reported and relinked.
        put(&l, "journal/2026/12", "in the way");
        let r = reorganise(&l, JournalLayout::YearMonth);
        assert_eq!(r.moved.len(), 2);
        assert_eq!(r.failed.len(), 1);
        assert_eq!(r.failed[0].0.as_str(), "journal/2026-12-02.md");
        assert_eq!(get(&l, "journal/2026-12-02.md"), "b");
        assert_eq!(get(&l, "journal/2026/11/2026-11-02.md"), "a");
        assert_eq!(get(&l, "journal/2027/01/2027-01-02.md"), "c");
        assert_eq!(
            get(&l, "linker.md"),
            "[[journal/2027/01/2027-01-02]] [[journal/2026/11/2026-11-02]]\n"
        );
    }

    #[test]
    fn prunes_only_folders_this_emptied() {
        let l = lib("prune");
        put(&l, "journal/old/deep/2026-11-02.md", "x");
        fs::create_dir_all(l.root().join("journal/keep/empty")).unwrap();
        fs::create_dir_all(l.root().join("journal/2026/12")).unwrap();
        let r = reorganise(&l, JournalLayout::Flat);
        assert_eq!(r.moved.len(), 1);
        assert!(!l.root().join("journal/old").exists());
        // Empty before, not emptied by us: stay.
        assert!(l.root().join("journal/keep/empty").is_dir());
        assert!(l.root().join("journal/2026/12").is_dir());
        assert!(l.root().join("journal").is_dir());
    }
}
