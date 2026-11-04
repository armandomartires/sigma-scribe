//! Replace in all notes (S24.1): plan, apply with a snapshot, undo.
//!
//! The flow is three steps, none of which blocks the UI if run on a worker:
//!
//! 1. [`plan`] scans the same notes the search covers ([`Library::all_notes`]:
//!    not `.trash/`, not `.ink/`, no hidden files), without a cap, and returns
//!    every match with its byte range, the lines to show in the review sheet
//!    and a flag for matches inside a `[[wiki link]]`. Matches inside the
//!    `(url)` of a Markdown link or image, which includes the `.ink/…` path of
//!    an ink line, are not matches at all (S24 "Ink"). A [`Selection`] says
//!    which matches are ticked; matches in a wiki link start unticked (S24-D6).
//! 2. [`apply`] re-reads each ticked note, refuses it with
//!    [`Outcome::Changed`] if its matches are no longer the planned ones, then
//!    copies every original to `.trash/replace-<yyyymmdd-hhmmss>/<note path>`
//!    with a manifest, and only then writes the notes, each read once more
//!    just before its write and refused if it is not the copy in the snapshot,
//!    each written with a temp file, fsync and rename
//!    ([`crate::store::write_atomic`]). The open note is never
//!    written: its new text comes back in [`Applied::open_text`] for the app to
//!    put in the buffer as one undo step and save.
//! 3. [`undo`] puts the originals back, but only for notes whose current
//!    content still hashes to what the replace wrote; the others are listed
//!    as skipped and left alone. [`latest_snapshot`] finds the newest
//!    snapshot, if it has not been undone.
//!
//! The snapshot folder name starts with `replace-`, so [`Library::list_trash`],
//! [`Library::trash_len`] and [`Library::restore`] do not see it (their ids are
//! unix seconds); [`Library::empty_trash`] removes it with everything else.
//! Ink sidecars are neither read, copied nor changed.
//!
//! The manifest is `.trash/replace-…/.manifest`: a header line, then one line
//! per note, `<16 hex digits of FNV-1a 64 of the text written><TAB><path>`.

use crate::date::civil_from_days;
use crate::find::{matches, replace_all};
use crate::library::{Library, NotePath, TRASH};
use crate::store::write_atomic;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Prefix of a snapshot folder name under `.trash/`.
const SNAPSHOT_PREFIX: &str = "replace-";
/// The manifest inside a snapshot folder. Note paths cannot start with a
/// dot, so it never clashes with a snapshotted note.
const MANIFEST: &str = ".manifest";
/// Marker written in a snapshot folder once it has been undone.
const UNDONE: &str = ".undone";
/// First line of a manifest.
const HEADER: &str = "sigma-replace-manifest 1";

/// A small FNV-1a (64 bit) hash of `text`, as the manifest records it.
#[must_use]
pub fn hash(text: &str) -> u64 {
    hash_bytes(text.as_bytes())
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// One match of the query in one note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    /// Byte offset of the match start, on a `char` boundary.
    pub from: usize,
    /// Byte offset just past the match.
    pub to: usize,
    /// Whether the match is inside a `[[wiki link]]` (brackets included).
    pub in_link: bool,
}

/// A match as it falls on one preview line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineMatch {
    /// Index into [`NotePlan::matches`], also the index used by [`Selection`].
    pub index: usize,
    /// Start in bytes from the start of the line text.
    pub from: usize,
    /// End in bytes from the start of the line text (clamped to the line if
    /// the query itself spans a line break).
    pub to: usize,
    /// Whether the match is inside a `[[wiki link]]`.
    pub in_link: bool,
}

/// A line of a note that has at least one match, for the review sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewLine {
    /// 1-based line number (lines split on `\n`).
    pub line: usize,
    /// The line without its line ending.
    pub text: String,
    /// The matches that start on this line, in order.
    pub ranges: Vec<LineMatch>,
}

/// What is to be replaced in one note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotePlan {
    /// The note.
    pub path: NotePath,
    /// Every match, ascending and not overlapping, with URL matches left out.
    pub matches: Vec<Match>,
    /// The lines holding the matches.
    pub lines: Vec<PreviewLine>,
    /// Whether the text came from the open note's live buffer, not the disk.
    pub open: bool,
}

/// Everything a replace would change: the result of [`plan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The library root on disk.
    pub root: PathBuf,
    /// The text to find.
    pub query: String,
    /// Whether matching is case-sensitive.
    pub case: bool,
    /// The replacement; empty deletes the matches.
    pub with: String,
    /// The notes with at least one match, in library order.
    pub notes: Vec<NotePlan>,
    /// Notes that could not be read (missing or not UTF-8) and were skipped.
    pub skipped: usize,
    /// Notes read.
    pub scanned: usize,
    /// Wall time of the scan, for the log and `docs/dev/perf.md`.
    pub elapsed_ms: u64,
}

impl Plan {
    /// How many matches there are in all notes.
    #[must_use]
    pub fn total_matches(&self) -> usize {
        self.notes.iter().map(|n| n.matches.len()).sum()
    }

    /// How many of the matches are inside a wiki link.
    #[must_use]
    pub fn link_matches(&self) -> usize {
        self.notes
            .iter()
            .map(|n| n.matches.iter().filter(|m| m.in_link).count())
            .sum()
    }
}

/// Which matches of a [`Plan`] are ticked, by note and match index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    ticked: Vec<Vec<bool>>,
}

impl Selection {
    /// The default review state (S24-D6): every match ticked except those
    /// inside a wiki link.
    #[must_use]
    pub fn defaults(plan: &Plan) -> Self {
        Self {
            ticked: plan
                .notes
                .iter()
                .map(|n| n.matches.iter().map(|m| !m.in_link).collect())
                .collect(),
        }
    }

    /// Every match ticked, link matches included.
    #[must_use]
    pub fn all(plan: &Plan) -> Self {
        Self {
            ticked: plan
                .notes
                .iter()
                .map(|n| vec![true; n.matches.len()])
                .collect(),
        }
    }

    /// Nothing ticked.
    #[must_use]
    pub fn none(plan: &Plan) -> Self {
        Self {
            ticked: plan
                .notes
                .iter()
                .map(|n| vec![false; n.matches.len()])
                .collect(),
        }
    }

    /// Whether match `m` of note `note` (indices into the plan) is ticked.
    #[must_use]
    pub fn is_ticked(&self, note: usize, m: usize) -> bool {
        self.ticked
            .get(note)
            .and_then(|v| v.get(m))
            .copied()
            .unwrap_or(false)
    }

    /// Ticks or unticks one match. Out-of-range indices are ignored.
    pub fn set(&mut self, note: usize, m: usize, on: bool) {
        if let Some(t) = self.ticked.get_mut(note).and_then(|v| v.get_mut(m)) {
            *t = on;
        }
    }

    /// The note's checkbox: on ticks the default set of its matches (all but
    /// the wiki-link ones, which keep their state), off unticks them all.
    /// A note whose matches are all inside wiki links has no default set, so
    /// there the box is the only control for them and on ticks them.
    pub fn set_note(&mut self, plan: &Plan, note: usize, on: bool) {
        let (Some(np), Some(t)) = (plan.notes.get(note), self.ticked.get_mut(note)) else {
            return;
        };
        let links_only = np.matches.iter().all(|m| m.in_link);
        for (m, tick) in np.matches.iter().zip(t.iter_mut()) {
            if !on {
                *tick = false;
            } else if links_only || !m.in_link {
                *tick = true;
            }
        }
    }

    /// How many matches of `note` are ticked.
    #[must_use]
    pub fn ticked_in(&self, note: usize) -> usize {
        self.ticked
            .get(note)
            .map_or(0, |v| v.iter().filter(|&&t| t).count())
    }

    /// How many matches are ticked in all.
    #[must_use]
    pub fn total(&self) -> usize {
        self.ticked.iter().flatten().filter(|&&t| t).count()
    }

    /// How many notes have at least one ticked match.
    #[must_use]
    pub fn notes(&self) -> usize {
        self.ticked.iter().filter(|v| v.contains(&true)).count()
    }
}

/// A byte range `[from, to)`.
type Span = (usize, usize);

/// Byte ranges of `text` that hold Markdown link or image URLs (`(…)` after
/// `](`, parentheses included) and of `[[…]]` wiki links (brackets included).
/// Both lists are ascending. Neither spans a line.
fn zones(text: &str) -> (Vec<Span>, Vec<Span>) {
    let b = text.as_bytes();
    let mut urls = Vec::new();
    let mut links = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match (b[i], b.get(i + 1).copied()) {
            (b']', Some(b'(')) => {
                let mut depth = 0usize;
                let mut j = i + 1;
                let mut end = None;
                while j < b.len() {
                    match b[j] {
                        b'\n' => break,
                        b'\\' => j += 1,
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(j + 1);
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if let Some(e) = end {
                    urls.push((i + 1, e));
                    i = e;
                    continue;
                }
            }
            (b'[', Some(b'[')) => {
                let line_end = text[i..].find('\n').map_or(b.len(), |n| i + n);
                if let Some(n) = text[i + 2..line_end].find("]]") {
                    let e = i + 2 + n + 2;
                    links.push((i, e));
                    i = e;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    (urls, links)
}

/// Whether `[from, to)` overlaps any of the ascending `zones`.
fn overlaps(zones: &[Span], from: usize, to: usize) -> bool {
    let at = zones.partition_point(|z| z.1 <= from);
    zones.get(at).is_some_and(|z| z.0 < to)
}

/// The matches of `query` in `text` that a replace may touch, with the
/// wiki-link flag: URL matches are dropped here.
fn matches_in(text: &str, query: &str, case: bool) -> Vec<Match> {
    let ms = matches(text, query, case);
    if ms.is_empty() {
        return Vec::new();
    }
    let (urls, links) = zones(text);
    ms.into_iter()
        .filter(|&(a, b)| !overlaps(&urls, a, b))
        .map(|(from, to)| Match {
            from,
            to,
            in_link: overlaps(&links, from, to),
        })
        .collect()
}

/// The preview lines of `text` for `ms` (ascending).
fn lines_of(text: &str, ms: &[Match]) -> Vec<PreviewLine> {
    let mut out: Vec<PreviewLine> = Vec::new();
    let mut next = 0;
    let mut start = 0;
    for (i, raw) in text.split('\n').enumerate() {
        if next >= ms.len() {
            break;
        }
        let end = start + raw.len();
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let mut ranges = Vec::new();
        while next < ms.len() && ms[next].from <= end {
            let m = ms[next];
            ranges.push(LineMatch {
                index: next,
                from: (m.from - start).min(line.len()),
                to: (m.to.min(end) - start).min(line.len()),
                in_link: m.in_link,
            });
            next += 1;
        }
        if !ranges.is_empty() {
            out.push(PreviewLine {
                line: i + 1,
                text: line.to_owned(),
                ranges,
            });
        }
        start = end + 1;
    }
    out
}

fn note_plan(path: &NotePath, text: &str, query: &str, case: bool, open: bool) -> Option<NotePlan> {
    let ms = matches_in(text, query, case);
    if ms.is_empty() {
        return None;
    }
    Some(NotePlan {
        path: path.clone(),
        lines: lines_of(text, &ms),
        matches: ms,
        open,
    })
}

/// Scans the whole library for `query`, uncapped, and plans replacing it with
/// `with` (empty deletes). Notes are the same as the search's
/// ([`Library::all_notes`]). For the note named in `open_note` the supplied
/// live text is used instead of the disk. An empty query plans nothing.
#[must_use]
pub fn plan(
    library: &Library,
    query: &str,
    case: bool,
    with: &str,
    open_note: Option<(&NotePath, &str)>,
) -> Plan {
    let started = std::time::Instant::now();
    let mut out = Plan {
        root: library.root().to_path_buf(),
        query: query.to_owned(),
        case,
        with: with.to_owned(),
        notes: Vec::new(),
        skipped: 0,
        scanned: 0,
        elapsed_ms: 0,
    };
    if !query.is_empty() {
        let notes = library.all_notes().unwrap_or_default();
        for note in &notes {
            out.scanned += 1;
            let live = open_note.filter(|(p, _)| *p == note).map(|(_, t)| t);
            let disk;
            let text = if let Some(t) = live {
                t
            } else if let Ok(t) = fs::read_to_string(library.fs_path(note)) {
                disk = t;
                &disk
            } else {
                out.skipped += 1;
                continue;
            };
            if let Some(np) = note_plan(note, text, query, case, live.is_some()) {
                out.notes.push(np);
            }
        }
    }
    out.elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    out
}

/// What happened to one note in [`apply`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Rewritten on disk.
    Written,
    /// The open note: not written, its new text is in [`Applied::open_text`].
    Open,
    /// Refused: its matches no longer are the planned ones (it was edited, or
    /// could not be read again). Nothing was changed; plan again.
    Changed,
    /// The write failed (the old file is intact); the message is the error.
    Failed(String),
}

/// One note's result in [`Applied::results`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteResult {
    /// The note.
    pub path: NotePath,
    /// How many matches were ticked in it.
    pub matches: usize,
    /// What happened.
    pub outcome: Outcome,
}

/// A snapshot made by [`apply`], for [`undo`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The folder name under `.trash/`: `replace-<yyyymmdd-hhmmss>`, with
    /// `-2`, `-3`… if several were made in one second.
    pub id: String,
    /// How many notes the manifest lists.
    pub notes: usize,
}

/// What [`apply`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// The snapshot holding the originals; `None` if nothing was to be
    /// replaced or the snapshot could not be made.
    pub snapshot: Option<Snapshot>,
    /// Matches replaced, in notes written and in the open note.
    pub matches: usize,
    /// Notes written plus the open note if it was replaced.
    pub notes: usize,
    /// One entry per note with a ticked match, in plan order.
    pub results: Vec<NoteResult>,
    /// The open note and its new text, to go into the buffer as one undo
    /// step and be saved by the app (the hash in the manifest is of this text).
    pub open_text: Option<(NotePath, String)>,
    /// Set when the snapshot could not be made: then no note was touched.
    pub error: Option<String>,
    /// Wall time of the apply, for the log.
    pub elapsed_ms: u64,
}

impl Applied {
    /// How many notes were refused because they changed since the plan.
    #[must_use]
    pub fn changed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.outcome == Outcome::Changed)
            .count()
    }

    /// How many notes could not be written.
    #[must_use]
    pub fn failed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, Outcome::Failed(_)))
            .count()
    }
}

/// [`apply_with_progress`] without progress.
#[must_use]
pub fn apply(plan: &Plan, selection: &Selection, open_note: Option<(&NotePath, &str)>) -> Applied {
    apply_with_progress(plan, selection, open_note, |_, _| {})
}

/// A note ready to be written.
struct Prepared {
    index: usize,
    path: NotePath,
    ticked: usize,
    original: String,
    new: String,
    open: bool,
}

/// Replaces the ticked matches of `plan`; see the module docs for the order
/// of events. `open_note` is the open note and its live text: that note is
/// checked against, and snapshotted from, the live text and is not written.
/// `progress(done, total)` is called after each note, `total` being the
/// number of notes with a ticked match, so a worker can report
/// "Replacing… 2 of 3".
#[must_use]
pub fn apply_with_progress(
    plan: &Plan,
    selection: &Selection,
    open_note: Option<(&NotePath, &str)>,
    mut progress: impl FnMut(usize, usize),
) -> Applied {
    let started = std::time::Instant::now();
    let mut out = apply_at(plan, selection, open_note, &mut progress, &stamp_now());
    out.elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    out
}

fn apply_at(
    plan: &Plan,
    selection: &Selection,
    open_note: Option<(&NotePath, &str)>,
    progress: &mut dyn FnMut(usize, usize),
    stamp: &str,
) -> Applied {
    let lib = Library::new(&plan.root);
    let mut out = Applied::default();
    let total = (0..plan.notes.len())
        .filter(|&i| selection.ticked_in(i) > 0)
        .count();
    let mut done = 0;

    // 1. Re-plan each ticked note from its current text and refuse it if the
    // matches moved.
    let mut ready: Vec<Prepared> = Vec::new();
    for (i, np) in plan.notes.iter().enumerate() {
        let ticked = selection.ticked_in(i);
        if ticked == 0 {
            continue;
        }
        let live = open_note.filter(|(p, _)| *p == &np.path).map(|(_, t)| t);
        let text = match live {
            Some(t) => Some(t.to_owned()),
            None => fs::read_to_string(lib.fs_path(&np.path)).ok(),
        };
        let fresh = text
            .as_deref()
            .map(|t| matches_in(t, &plan.query, plan.case));
        let (Some(text), Some(fresh)) = (text, fresh) else {
            out.results.push(NoteResult {
                path: np.path.clone(),
                matches: ticked,
                outcome: Outcome::Changed,
            });
            done += 1;
            progress(done, total);
            continue;
        };
        if fresh != np.matches {
            out.results.push(NoteResult {
                path: np.path.clone(),
                matches: ticked,
                outcome: Outcome::Changed,
            });
            done += 1;
            progress(done, total);
            continue;
        }
        let chosen: Vec<(usize, usize)> = fresh
            .iter()
            .enumerate()
            .filter(|(m, _)| selection.is_ticked(i, *m))
            .map(|(_, m)| (m.from, m.to))
            .collect();
        let new = replace_all(&text, &chosen, &plan.with);
        ready.push(Prepared {
            index: out.results.len(),
            path: np.path.clone(),
            ticked,
            original: text,
            new,
            open: live.is_some(),
        });
        out.results.push(NoteResult {
            path: np.path.clone(),
            matches: ticked,
            outcome: Outcome::Written,
        });
    }
    if ready.is_empty() {
        return out;
    }

    // 2. Snapshot: every original, then the manifest. Until the manifest is
    // there the snapshot does not count, and no note has been touched.
    let id = match write_snapshot(&lib, stamp, &ready) {
        Ok(id) => id,
        Err(e) => {
            for p in &ready {
                out.results[p.index].outcome = Outcome::Failed(e.to_string());
            }
            out.error = Some(format!("snapshot: {e}"));
            return out;
        }
    };
    out.snapshot = Some(Snapshot {
        id,
        notes: ready.len(),
    });

    // 3. Write the notes, one at a time.
    for p in ready {
        if p.open {
            out.open_text = Some((p.path.clone(), p.new));
            out.results[p.index].outcome = Outcome::Open;
            out.notes += 1;
            out.matches += p.ticked;
        } else {
            // The note was read in step 1; the snapshot and the notes before
            // this one took time. Read it again and write only if it still is
            // what the snapshot holds, so an edit made meanwhile (a sync, a
            // device, a second writer) is never overwritten.
            let path = lib.fs_path(&p.path);
            if fs::read_to_string(&path).ok().as_deref() != Some(p.original.as_str()) {
                out.results[p.index].outcome = Outcome::Changed;
            } else {
                match write_atomic(&path, &p.new) {
                    Ok(()) => {
                        out.notes += 1;
                        out.matches += p.ticked;
                    }
                    Err(e) => out.results[p.index].outcome = Outcome::Failed(e.to_string()),
                }
            }
        }
        done += 1;
        progress(done, total);
    }
    // A replace that wrote nothing (every note changed under it) has nothing
    // to undo: drop the snapshot it just made, so that it is not offered.
    if out.notes == 0 {
        if let Some(s) = out.snapshot.take() {
            let _ = fs::remove_dir_all(lib.root().join(TRASH).join(&s.id));
        }
    }
    out
}

/// Writes the originals and the manifest of a new snapshot; returns its id.
fn write_snapshot(lib: &Library, stamp: &str, ready: &[Prepared]) -> io::Result<String> {
    let base = lib.root().join(TRASH);
    let mut id = format!("{SNAPSHOT_PREFIX}{stamp}");
    let mut n = 2;
    while base.join(&id).exists() {
        id = format!("{SNAPSHOT_PREFIX}{stamp}-{n}");
        n += 1;
    }
    let dir = base.join(&id);
    fs::create_dir_all(&dir)?;
    let mut manifest = format!("{HEADER}\n");
    for p in ready {
        write_atomic(&dir.join(p.path.as_str()), &p.original)?;
        manifest.push_str(&format!("{:016x}\t{}\n", hash(&p.new), p.path.as_str()));
    }
    write_atomic(&dir.join(MANIFEST), &manifest)?;
    Ok(id)
}

/// `yyyymmdd-hhmmss` in UTC for the snapshot folder name.
fn stamp_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    stamp_of(secs)
}

fn stamp_of(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let (y, m, d) = civil_from_days(days);
    let rem = secs % 86_400;
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

/// The manifest of snapshot `id` as (hash, note) pairs; `None` if it is
/// missing, has another header or a bad line.
fn read_manifest(lib: &Library, id: &str) -> Option<Vec<(u64, NotePath)>> {
    let text = fs::read_to_string(lib.root().join(TRASH).join(id).join(MANIFEST)).ok()?;
    let mut lines = text.lines();
    if lines.next()? != HEADER {
        return None;
    }
    let mut out = Vec::new();
    for l in lines {
        let (h, p) = l.split_once('\t')?;
        let path = NotePath::parse(p).ok().filter(|p| !p.is_root())?;
        out.push((u64::from_str_radix(h, 16).ok()?, path));
    }
    Some(out)
}

/// The newest snapshot with a complete manifest, if it has not been undone.
/// Only the newest counts: once it is undone there is nothing to offer, the
/// older ones are not offered in its place (their notes have been through
/// the newer replace since). A folder without a complete manifest is a
/// snapshot that failed halfway; no note was touched for it, so it is passed
/// over.
#[must_use]
pub fn latest_snapshot(library: &Library) -> Option<Snapshot> {
    let base = library.root().join(TRASH);
    let names: BTreeSet<String> = fs::read_dir(&base)
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|n| n.starts_with(SNAPSHOT_PREFIX))
        .collect();
    // The stamp sorts by time; a `-2` suffix sorts after the plain name.
    names.into_iter().rev().find_map(|id| {
        let notes = read_manifest(library, &id)?.len();
        if base.join(&id).join(UNDONE).exists() {
            return Some(None);
        }
        Some(Some(Snapshot { id, notes }))
    })?
}

/// Why [`undo`] left a note as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Skip {
    /// Its content is not what the replace wrote: edited since.
    Edited,
    /// The note is gone or cannot be read.
    Missing,
    /// The snapshot copy of the original is gone.
    NoCopy,
    /// It already has the original text (the replace never wrote it).
    Original,
    /// The restore failed (the file is intact); the message is the error.
    Failed(String),
}

/// What [`undo`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Undone {
    /// Notes put back on disk.
    pub restored: Vec<NotePath>,
    /// Notes left alone, with the reason.
    pub skipped: Vec<(NotePath, Skip)>,
    /// The open note and its original text, if it was to be restored: not
    /// written, the app puts it in the buffer as one undo step and saves it.
    pub open_text: Option<(NotePath, String)>,
    /// Set when the snapshot's manifest cannot be read: nothing was done.
    pub error: Option<String>,
    /// Wall time of the undo, for the log.
    pub elapsed_ms: u64,
}

/// [`undo_with_open`] when no note is open.
#[must_use]
pub fn undo(library: &Library, snapshot: &Snapshot) -> Undone {
    undo_with_open(library, snapshot, None)
}

/// Restores every note of `snapshot` whose current content hashes to what the
/// replace wrote. `open_note` is the open note and its live text, compared
/// the same way but not written; its original comes back in
/// [`Undone::open_text`]. The snapshot is marked undone (and no longer
/// returned by [`latest_snapshot`]) unless a restore failed. The copies in
/// the snapshot stay, as in any trash, until it is emptied.
#[must_use]
pub fn undo_with_open(
    library: &Library,
    snapshot: &Snapshot,
    open_note: Option<(&NotePath, &str)>,
) -> Undone {
    let started = std::time::Instant::now();
    let mut out = undo_inner(library, snapshot, open_note);
    out.elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    out
}

fn undo_inner(
    library: &Library,
    snapshot: &Snapshot,
    open_note: Option<(&NotePath, &str)>,
) -> Undone {
    let mut out = Undone::default();
    let Some(entries) = read_manifest(library, &snapshot.id) else {
        out.error = Some(format!("{} has no readable manifest", snapshot.id));
        return out;
    };
    let dir = library.root().join(TRASH).join(&snapshot.id);
    let mut failed = false;
    for (want, path) in entries {
        let live = open_note.filter(|(p, _)| *p == &path).map(|(_, t)| t);
        let current = match live {
            Some(t) => Some(hash(t)),
            None => fs::read(library.fs_path(&path))
                .ok()
                .map(|b| hash_bytes(&b)),
        };
        let Some(current) = current else {
            out.skipped.push((path, Skip::Missing));
            continue;
        };
        let original = fs::read_to_string(dir.join(path.as_str())).ok();
        if current != want {
            let reason = if original.as_deref().map(hash) == Some(current) {
                Skip::Original
            } else {
                Skip::Edited
            };
            out.skipped.push((path, reason));
            continue;
        }
        let Some(original) = original else {
            out.skipped.push((path, Skip::NoCopy));
            continue;
        };
        if live.is_some() {
            out.open_text = Some((path, original));
        } else {
            match write_atomic(&library.fs_path(&path), &original) {
                Ok(()) => out.restored.push(path),
                Err(e) => {
                    failed = true;
                    out.skipped.push((path, Skip::Failed(e.to_string())));
                }
            }
        }
    }
    if !failed {
        let _ = fs::write(dir.join(UNDONE), "");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// A fresh library on disk with the given files.
    fn lib(tag: &str, files: &[(&str, &str)]) -> Library {
        let root = std::env::temp_dir().join(format!("sigma-replace-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (p, body) in files {
            let f = root.join(p);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, body).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        Library::new(root)
    }

    fn np(s: &str) -> NotePath {
        NotePath::parse(s).unwrap()
    }

    fn read(l: &Library, p: &str) -> String {
        fs::read_to_string(l.fs_path(&np(p))).unwrap()
    }

    fn write(l: &Library, p: &str, body: &str) {
        fs::write(l.fs_path(&np(p)), body).unwrap();
    }

    fn files_under(dir: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                files_under(&p, out);
            } else {
                out.push(p.file_name().unwrap().to_string_lossy().into_owned());
            }
        }
    }

    #[test]
    fn ranges_are_byte_ranges_in_multibyte_text() {
        let l = lib("utf8", &[("a.md", "é lamp ü\nÅ Lamp\n")]);
        let p = plan(&l, "lamp", false, "x", None);
        let n = &p.notes[0];
        let text = read(&l, "a.md");
        let got: Vec<&str> = n.matches.iter().map(|m| &text[m.from..m.to]).collect();
        assert_eq!(got, ["lamp", "Lamp"]);
        assert_eq!(n.matches[0].from, 3, "é is two bytes");
        assert_eq!(n.lines.len(), 2);
        assert_eq!(n.lines[0].line, 1);
        assert_eq!(n.lines[0].text, "é lamp ü");
        assert_eq!(n.lines[0].ranges[0].from, 3);
        assert_eq!(n.lines[0].ranges[0].to, 7);
        assert_eq!(n.lines[1].line, 2);
        assert_eq!(
            &n.lines[1].text[n.lines[1].ranges[0].from..n.lines[1].ranges[0].to],
            "Lamp"
        );
        assert_eq!(n.lines[1].ranges[0].index, 1);
        assert_eq!(p.total_matches(), 2);
    }

    #[test]
    fn crlf_lines_are_shown_without_the_return() {
        let l = lib("crlf", &[("a.md", "one\r\ntwo lamp\r\n")]);
        let p = plan(&l, "lamp", true, "", None);
        assert_eq!(p.notes[0].lines[0].text, "two lamp");
        assert_eq!(p.notes[0].lines[0].line, 2);
    }

    #[test]
    fn case_option() {
        let l = lib("case", &[("a.md", "Lamp lamp LAMP")]);
        assert_eq!(plan(&l, "lamp", false, "x", None).total_matches(), 3);
        assert_eq!(plan(&l, "lamp", true, "x", None).total_matches(), 1);
        assert!(plan(&l, "", true, "x", None).notes.is_empty());
    }

    #[test]
    fn matches_in_wiki_links_are_flagged_and_unticked() {
        let l = lib(
            "links",
            &[("a.md", "a lamp [[lamp post]] and [[x|lamp]] lamp")],
        );
        let p = plan(&l, "lamp", true, "lantern", None);
        let flags: Vec<bool> = p.notes[0].matches.iter().map(|m| m.in_link).collect();
        assert_eq!(flags, [false, true, true, false]);
        assert_eq!(p.link_matches(), 2);
        let sel = Selection::defaults(&p);
        assert_eq!(sel.total(), 2);
        assert!(sel.is_ticked(0, 0) && !sel.is_ticked(0, 1) && !sel.is_ticked(0, 2));
        assert_eq!(Selection::all(&p).total(), 4);
        assert_eq!(Selection::none(&p).total(), 0);
        // The preview carries the flag too.
        assert!(p.notes[0].lines[0].ranges[1].in_link);
        let a = apply(&p, &sel, None);
        assert_eq!(a.matches, 2);
        assert_eq!(
            read(&l, "a.md"),
            "a lantern [[lamp post]] and [[x|lamp]] lantern"
        );
    }

    #[test]
    fn ticking_a_link_match_edits_its_text() {
        let l = lib("tick-link", &[("a.md", "see [[lamp]]")]);
        let p = plan(&l, "lamp", true, "lantern", None);
        let mut sel = Selection::defaults(&p);
        assert_eq!(sel.total(), 0);
        sel.set(0, 0, true);
        let _ = apply(&p, &sel, None);
        assert_eq!(read(&l, "a.md"), "see [[lantern]]");
    }

    #[test]
    fn set_note_ticks_defaults_and_unticks_all() {
        let l = lib("setnote", &[("a.md", "lamp [[lamp]] lamp")]);
        let p = plan(&l, "lamp", true, "x", None);
        let mut sel = Selection::none(&p);
        sel.set_note(&p, 0, true);
        assert_eq!(
            [
                sel.is_ticked(0, 0),
                sel.is_ticked(0, 1),
                sel.is_ticked(0, 2)
            ],
            [true, false, true]
        );
        assert_eq!(sel.notes(), 1);
        sel.set_note(&p, 0, false);
        assert_eq!(sel.notes(), 0);
        sel.set(9, 9, true); // ignored
        assert_eq!(sel.total(), 0);
    }

    #[test]
    fn set_note_on_a_link_only_note_ticks_its_links() {
        let l = lib(
            "setnote-links",
            &[
                (
                    "trip.md",
                    "See [[harbour]] and [[old harbour]] for the map.",
                ),
                ("mix.md", "harbour and [[harbour]]"),
            ],
        );
        let p = plan(&l, "harbour", false, "port", None);
        let (trip, mix) = (
            p.notes
                .iter()
                .position(|n| n.path == np("trip.md"))
                .unwrap(),
            p.notes.iter().position(|n| n.path == np("mix.md")).unwrap(),
        );
        let mut sel = Selection::defaults(&p);
        assert_eq!(sel.ticked_in(trip), 0);
        // A note with both kinds still ticks only the plain match.
        sel.set_note(&p, mix, true);
        assert!(sel.is_ticked(mix, 0) && !sel.is_ticked(mix, 1));
        // A link-only note ticks its link matches, and off clears them.
        sel.set_note(&p, trip, true);
        assert_eq!(sel.ticked_in(trip), 2);
        sel.set_note(&p, trip, false);
        assert_eq!(sel.ticked_in(trip), 0);
        sel.set_note(&p, trip, true);
        let a = apply(&p, &sel, None);
        assert!(a.matches >= 2);
        assert_eq!(
            read(&l, "trip.md"),
            "See [[port]] and [[old port]] for the map."
        );
    }

    #[test]
    fn apply_and_undo_report_their_time() {
        let l = lib("timing", &[("a.md", "lamp")]);
        let p = plan(&l, "lamp", true, "x", None);
        let a = apply(&p, &Selection::defaults(&p), None);
        assert!(a.snapshot.is_some());
        assert!(a.elapsed_ms < 60_000);
        let u = undo(&l, a.snapshot.as_ref().unwrap());
        assert_eq!(u.restored.len(), 1);
        assert!(u.elapsed_ms < 60_000);
    }

    #[test]
    fn urls_images_and_ink_paths_are_not_matches() {
        let body = "\
lamp in text\n\
[a lamp](https://lamp.example/lamp_(x) \"lamp\") lamp\n\
![lamp](.ink/note/ink-001.svg)\n\
![](.ink/lamp/ink-002.svg)\n\
[[lamp]] and (lamp) plain parens\n";
        let l = lib("urls", &[("a.md", body)]);
        let p = plan(&l, "lamp", true, "X", None);
        let seen: Vec<(usize, bool)> = p.notes[0]
            .lines
            .iter()
            .flat_map(|ln| ln.ranges.iter().map(move |r| (ln.line, r.in_link)))
            .collect();
        // Line 1; line 2: label "a lamp" and the trailing "lamp" (not the URL
        // or the title); line 3: the alt text only; line 4: nothing; line 5:
        // the link (flagged) and the plain parentheses.
        assert_eq!(
            seen,
            [
                (1, false),
                (2, false),
                (2, false),
                (3, false),
                (5, true),
                (5, false)
            ]
        );
        let sel = Selection::all(&p);
        let _ = apply(&p, &sel, None);
        let after = read(&l, "a.md");
        assert!(after.contains("(https://lamp.example/lamp_(x) \"lamp\")"));
        assert!(after.contains("(.ink/note/ink-001.svg)"));
        assert!(after.contains("![](.ink/lamp/ink-002.svg)"));
        assert!(after.contains("![X](.ink/note/ink-001.svg)"));
        assert!(after.contains("[a X]("));
        assert!(after.contains("(X) plain parens"));
    }

    #[test]
    fn an_unclosed_url_paren_excludes_nothing() {
        let l = lib("unclosed", &[("a.md", "[a](lamp\nlamp")]);
        let p = plan(&l, "lamp", true, "x", None);
        assert_eq!(p.total_matches(), 2);
    }

    #[test]
    fn the_scan_covers_what_search_covers() {
        let l = lib(
            "scope",
            &[
                ("a.md", "lamp"),
                ("sub/b.txt", "lamp"),
                (".hidden.md", "lamp"),
                (".trash/1700000000/c.md", "lamp"),
                (".ink/a/ink-001.svg", "lamp"),
                (".scratchpad.md", "lamp"),
                ("img.png", "lamp"),
            ],
        );
        let p = plan(&l, "lamp", true, "x", None);
        let names: Vec<&str> = p.notes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(names, ["a.md", "sub/b.txt"]);
        assert_eq!(p.scanned, 2);
    }

    #[test]
    fn unreadable_notes_are_counted_and_skipped() {
        let l = lib("utf8bad", &[("a.md", "lamp")]);
        fs::write(l.fs_path(&np("b.md")), [0xff, 0xfe, b'l']).unwrap();
        let p = plan(&l, "lamp", true, "x", None);
        assert_eq!(p.skipped, 1);
        assert_eq!(p.notes.len(), 1);
    }

    #[test]
    fn apply_replaces_ticked_matches_only() {
        let l = lib(
            "apply",
            &[
                ("a.md", "lamp one lamp\n"),
                ("b.md", "lamp\n"),
                ("c.md", "none"),
            ],
        );
        let p = plan(&l, "lamp", true, "lantern", None);
        assert_eq!(p.notes.len(), 2);
        let mut sel = Selection::defaults(&p);
        sel.set(0, 1, false); // second match of a.md
        sel.set_note(&p, 1, false); // all of b.md
        let a = apply(&p, &sel, None);
        assert_eq!((a.matches, a.notes), (1, 1));
        assert_eq!(a.results.len(), 1, "b.md has nothing ticked");
        assert_eq!(a.results[0].outcome, Outcome::Written);
        assert_eq!(read(&l, "a.md"), "lantern one lamp\n");
        assert_eq!(read(&l, "b.md"), "lamp\n");
        assert!(a.error.is_none() && a.open_text.is_none());
    }

    #[test]
    fn empty_replacement_deletes() {
        let l = lib("empty", &[("a.md", "a lamp b lamp c\n")]);
        let p = plan(&l, "lamp ", true, "", None);
        assert_eq!(p.with, "");
        let a = apply(&p, &Selection::defaults(&p), None);
        assert_eq!(a.matches, 2);
        assert_eq!(read(&l, "a.md"), "a b c\n");
        let u = undo(&l, a.snapshot.as_ref().unwrap());
        assert_eq!(u.restored.len(), 1);
        assert_eq!(read(&l, "a.md"), "a lamp b lamp c\n");
    }

    #[test]
    fn nothing_ticked_writes_nothing_and_makes_no_snapshot() {
        let l = lib("noop", &[("a.md", "lamp")]);
        let p = plan(&l, "lamp", true, "x", None);
        let a = apply(&p, &Selection::none(&p), None);
        assert_eq!(a, Applied::default());
        assert!(!l.root().join(TRASH).exists());
        assert!(latest_snapshot(&l).is_none());
    }

    #[test]
    fn writes_are_temp_file_then_rename_and_leave_no_temp_files() {
        let l = lib("atomic", &[("a.md", "lamp\n"), ("d/b.md", "lamp\n")]);
        // A second name for the original file keeps the old inode alive: if
        // the write truncated it in place, this name would change too.
        #[cfg(unix)]
        let keep = l.root().join("keep-old-inode");
        #[cfg(unix)]
        fs::hard_link(l.fs_path(&np("a.md")), &keep).unwrap();
        let p = plan(&l, "lamp", true, "lantern", None);
        let a = apply(&p, &Selection::defaults(&p), None);
        assert_eq!(a.notes, 2);
        #[cfg(unix)]
        {
            assert_eq!(fs::read_to_string(&keep).unwrap(), "lamp\n");
            assert_eq!(read(&l, "a.md"), "lantern\n");
        }
        let mut all = Vec::new();
        files_under(l.root(), &mut all);
        assert!(
            all.iter().all(|f| !f.ends_with(".tmp")),
            "temp files left: {all:?}"
        );
    }

    #[test]
    fn a_note_changed_after_planning_is_refused_and_the_rest_go_ahead() {
        let l = lib("changed", &[("a.md", "lamp\n"), ("b.md", "lamp\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        write(&l, "a.md", "lamp lamp\n"); // a new match appeared
        let a = apply(&p, &Selection::defaults(&p), None);
        assert_eq!(a.changed(), 1);
        assert_eq!(a.results[0].outcome, Outcome::Changed);
        assert_eq!(a.results[1].outcome, Outcome::Written);
        assert_eq!(read(&l, "a.md"), "lamp lamp\n", "refused note untouched");
        assert_eq!(read(&l, "b.md"), "x\n");
        assert_eq!(a.snapshot.as_ref().unwrap().notes, 1);
    }

    #[test]
    fn a_note_whose_matches_moved_or_vanished_is_refused() {
        let l = lib("moved", &[("a.md", "lamp\n"), ("b.md", "lamp\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        write(&l, "a.md", "text before\nlamp\n"); // same count, other place
        fs::remove_file(l.fs_path(&np("b.md"))).unwrap();
        let a = apply(&p, &Selection::defaults(&p), None);
        assert_eq!(a.changed(), 2);
        assert_eq!(a.notes, 0);
        assert!(a.snapshot.is_none());
        assert_eq!(read(&l, "a.md"), "text before\nlamp\n");
    }

    #[test]
    fn edits_elsewhere_in_a_note_do_not_refuse_it() {
        let l = lib("elsewhere", &[("a.md", "lamp\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        write(&l, "a.md", "lamp\nmore text after\n");
        let a = apply(&p, &Selection::defaults(&p), None);
        assert_eq!(a.changed(), 0);
        assert_eq!(read(&l, "a.md"), "x\nmore text after\n");
    }

    #[test]
    fn snapshot_holds_originals_and_a_manifest() {
        let l = lib("snap", &[("a.md", "lamp\n"), ("d/b.md", "a lamp\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        let a = apply_at(
            &p,
            &Selection::defaults(&p),
            None,
            &mut |_, _| {},
            "20261103-101500",
        );
        let s = a.snapshot.unwrap();
        assert_eq!(s.id, "replace-20261103-101500");
        let dir = l.root().join(TRASH).join(&s.id);
        assert_eq!(fs::read_to_string(dir.join("a.md")).unwrap(), "lamp\n");
        assert_eq!(fs::read_to_string(dir.join("d/b.md")).unwrap(), "a lamp\n");
        let m = fs::read_to_string(dir.join(MANIFEST)).unwrap();
        let want = format!(
            "{HEADER}\n{:016x}\ta.md\n{:016x}\td/b.md\n",
            hash("x\n"),
            hash("a x\n")
        );
        assert_eq!(m, want);
        // Same second again: a new folder, not an overwrite.
        write(&l, "a.md", "lamp\n");
        let p = plan(&l, "lamp", true, "y", None);
        let b = apply_at(
            &p,
            &Selection::defaults(&p),
            None,
            &mut |_, _| {},
            "20261103-101500",
        );
        assert_eq!(b.snapshot.unwrap().id, "replace-20261103-101500-2");
        assert_eq!(latest_snapshot(&l).unwrap().id, "replace-20261103-101500-2");
    }

    #[test]
    fn the_stamp_is_utc_yyyymmdd_hhmmss() {
        assert_eq!(stamp_of(0), "19700101-000000");
        assert_eq!(stamp_of(1_762_171_530), "20251103-120530");
        let now = stamp_now();
        assert_eq!(now.len(), 15);
        assert_eq!(&now[8..9], "-");
    }

    #[test]
    fn the_snapshot_is_invisible_to_the_scan_and_to_the_trash_listing() {
        let l = lib("hidden", &[("a.md", "lamp\n"), ("b.md", "other\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        let a = apply(&p, &Selection::defaults(&p), None);
        assert!(a.snapshot.is_some());
        // The copy of the original still has "lamp" in it, and is not found.
        let again = plan(&l, "lamp", true, "x", None);
        assert!(again.notes.is_empty());
        assert_eq!(l.all_notes().unwrap().len(), 2);
        // The Trash screen's source does not list it or count it.
        assert!(l.list_trash().unwrap().is_empty());
        assert_eq!(l.trash_len(), 0);
        // A real deletion beside it is listed alone.
        l.trash(&np("b.md")).unwrap();
        assert_eq!(l.list_trash().unwrap().len(), 1);
        assert_eq!(l.trash_len(), 1);
        assert!(latest_snapshot(&l).is_some());
    }

    #[test]
    fn undo_restores_and_skips_notes_edited_since() {
        let l = lib(
            "undo",
            &[("a.md", "lamp\n"), ("b.md", "lamp\n"), ("c.md", "lamp\n")],
        );
        let p = plan(&l, "lamp", true, "lantern", None);
        let a = apply(&p, &Selection::defaults(&p), None);
        assert_eq!(a.notes, 3);
        write(&l, "b.md", "lantern, edited since\n");
        fs::remove_file(l.fs_path(&np("c.md"))).unwrap();
        let s = latest_snapshot(&l).unwrap();
        assert_eq!(s.notes, 3);
        let u = undo(&l, &s);
        assert_eq!(u.restored, [np("a.md")]);
        assert_eq!(
            u.skipped,
            [(np("b.md"), Skip::Edited), (np("c.md"), Skip::Missing)]
        );
        assert_eq!(read(&l, "a.md"), "lamp\n");
        assert_eq!(read(&l, "b.md"), "lantern, edited since\n");
        assert!(u.error.is_none() && u.open_text.is_none());
        // Undone: not offered again, and a second undo does nothing.
        assert!(latest_snapshot(&l).is_none());
        let again = undo(&l, &s);
        assert!(again.restored.is_empty());
        assert_eq!(again.skipped[0], (np("a.md"), Skip::Original));
    }

    #[test]
    fn undo_survives_a_restart() {
        // Nothing but the folder on disk is needed: a new Library finds it.
        let l = lib("restart", &[("a.md", "lamp\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        let _ = apply(&p, &Selection::defaults(&p), None);
        let fresh = Library::new(l.root());
        let s = latest_snapshot(&fresh).unwrap();
        assert_eq!(undo(&fresh, &s).restored.len(), 1);
        assert_eq!(read(&l, "a.md"), "lamp\n");
    }

    #[test]
    fn a_snapshot_without_a_manifest_is_not_offered_and_cannot_undo() {
        let l = lib("nomanifest", &[("a.md", "lamp\n")]);
        let d = l.root().join(TRASH).join("replace-20260101-000000");
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("a.md"), "x").unwrap();
        assert!(latest_snapshot(&l).is_none());
        let u = undo(
            &l,
            &Snapshot {
                id: "replace-20260101-000000".into(),
                notes: 1,
            },
        );
        assert!(u.error.is_some());
        assert!(u.restored.is_empty());
        assert_eq!(read(&l, "a.md"), "lamp\n");
    }

    #[test]
    fn latest_picks_the_newest_and_skips_undone_ones() {
        let l = lib("latest", &[("a.md", "lamp\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        let s1 = apply_at(
            &p,
            &Selection::defaults(&p),
            None,
            &mut |_, _| {},
            "20261101-000000",
        )
        .snapshot
        .unwrap();
        let p = plan(&l, "x", true, "y", None);
        let s2 = apply_at(
            &p,
            &Selection::defaults(&p),
            None,
            &mut |_, _| {},
            "20261102-000000",
        )
        .snapshot
        .unwrap();
        assert_eq!(latest_snapshot(&l).unwrap(), s2);
        let _ = undo(&l, &s2);
        // Only the newest counts: s1 is not offered in its place (S24 review).
        assert_eq!(latest_snapshot(&l), None);
        assert_ne!(latest_snapshot(&l), Some(s1));
        // A newer, complete snapshot is offered again, and an older one is
        // not offered once that is undone.
        let p = plan(&l, "x", true, "z", None);
        let s3 = apply_at(
            &p,
            &Selection::defaults(&p),
            None,
            &mut |_, _| {},
            "20261103-000000",
        )
        .snapshot
        .unwrap();
        assert_eq!(latest_snapshot(&l).unwrap(), s3);
        let _ = undo(&l, &s3);
        assert_eq!(latest_snapshot(&l), None);
    }

    /// An incomplete (manifest-less) folder newer than a real snapshot is
    /// passed over: no note was touched for it.
    #[test]
    fn a_failed_halfway_snapshot_does_not_hide_the_real_one() {
        let l = lib("halfway", &[("a.md", "lamp\n")]);
        let p = plan(&l, "lamp", true, "x", None);
        let s1 = apply_at(
            &p,
            &Selection::defaults(&p),
            None,
            &mut |_, _| {},
            "20261101-000000",
        )
        .snapshot
        .unwrap();
        let d = l.root().join(TRASH).join("replace-20261102-000000");
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("a.md"), "x").unwrap();
        assert_eq!(latest_snapshot(&l).unwrap(), s1);
    }

    /// A note edited after step 1 (here: while an earlier note is being
    /// written) is read again before its write and left as it is.
    #[test]
    fn a_note_edited_after_the_snapshot_is_not_overwritten() {
        let l = lib(
            "late",
            &[("a.md", "lamp\n"), ("b.md", "lamp\n"), ("c.md", "lamp\n")],
        );
        let p = plan(&l, "lamp", true, "x", None);
        let b = l.fs_path(&np("b.md"));
        let mut edited = false;
        // `progress` runs after each note of step 3: edit b.md after a.md.
        let a = apply_with_progress(&p, &Selection::defaults(&p), None, |_, _| {
            if !edited {
                edited = true;
                fs::write(&b, "lamp, edited on another device\n").unwrap();
            }
        });
        assert_eq!(a.results[0].outcome, Outcome::Written);
        assert_eq!(a.results[1].outcome, Outcome::Changed, "{:?}", a.results);
        assert_eq!(a.results[2].outcome, Outcome::Written);
        assert_eq!(a.changed(), 1);
        assert_eq!((a.notes, a.matches), (2, 2));
        assert_eq!(read(&l, "a.md"), "x\n");
        assert_eq!(read(&l, "b.md"), "lamp, edited on another device\n");
        assert_eq!(read(&l, "c.md"), "x\n");
        // The snapshot is still there and undo leaves b.md alone.
        let s = a.snapshot.unwrap();
        let u = undo(&l, &s);
        assert_eq!(u.restored.len(), 2);
        assert_eq!(read(&l, "b.md"), "lamp, edited on another device\n");
    }

    /// When every ticked note changed after step 1, nothing was written:
    /// the snapshot that step 2 made is dropped, and the one before it is
    /// still the latest.
    #[test]
    fn a_replace_that_wrote_nothing_drops_its_snapshot() {
        let l = lib("nothing-written", &[("a.md", "lamp\n"), ("b.md", "lamp\n")]);
        let p1 = plan(&l, "lamp", true, "lantern", None);
        let first = apply_at(
            &p1,
            &Selection::defaults(&p1),
            None,
            &mut |_, _| {},
            "20261101-000000",
        )
        .snapshot
        .unwrap();
        // Second replace over x.md and y.md. `progress` first runs in step 1,
        // when y.md (changed since the plan) is refused, after x.md was read;
        // it then edits x.md, the only note left to write, so that step 3
        // refuses it too.
        write(&l, "y.md", "wick\n");
        write(&l, "x.md", "wick\n");
        let p2 = plan(&l, "wick", true, "flame", None);
        write(&l, "y.md", "wick wick\n");
        let d = l.fs_path(&np("x.md"));
        let mut edited = false;
        let a = apply_at(
            &p2,
            &Selection::defaults(&p2),
            None,
            &mut |_, _| {
                if !edited {
                    edited = true;
                    fs::write(&d, "wick, edited elsewhere\n").unwrap();
                }
            },
            "20261102-000000",
        );
        assert_eq!(a.results[0].outcome, Outcome::Changed, "{:?}", a.results);
        assert_eq!(a.results[1].outcome, Outcome::Changed, "{:?}", a.results);
        assert_eq!((a.notes, a.matches), (0, 0));
        assert!(a.snapshot.is_none());
        assert!(!l
            .root()
            .join(TRASH)
            .join("replace-20261102-000000")
            .exists());
        assert_eq!(latest_snapshot(&l).unwrap(), first);
        assert_eq!(read(&l, "x.md"), "wick, edited elsewhere\n");
    }

    #[test]
    fn the_open_note_is_planned_from_live_text_and_never_written() {
        let l = lib("open", &[("a.md", "disk lamp\n"), ("b.md", "lamp\n")]);
        let a_path = np("a.md");
        let live = "live lamp lamp, unsaved\n";
        let p = plan(&l, "lamp", true, "lantern", Some((&a_path, live)));
        assert!(p.notes[0].open && !p.notes[1].open);
        assert_eq!(p.notes[0].matches.len(), 2, "from the live text");
        let a = apply(&p, &Selection::defaults(&p), Some((&a_path, live)));
        assert_eq!(a.results[0].outcome, Outcome::Open);
        assert_eq!(a.results[1].outcome, Outcome::Written);
        assert_eq!((a.matches, a.notes), (3, 2));
        assert_eq!(
            a.open_text,
            Some((a_path.clone(), "live lantern lantern, unsaved\n".to_owned()))
        );
        assert_eq!(read(&l, "a.md"), "disk lamp\n", "disk untouched");
        assert_eq!(read(&l, "b.md"), "lantern\n");
        // The snapshot holds the live original, and the manifest the new text.
        let s = a.snapshot.unwrap();
        let dir = l.root().join(TRASH).join(&s.id);
        assert_eq!(fs::read_to_string(dir.join("a.md")).unwrap(), live);
        let m = fs::read_to_string(dir.join(MANIFEST)).unwrap();
        assert!(m.contains(&format!(
            "{:016x}\ta.md",
            hash("live lantern lantern, unsaved\n")
        )));
    }

    #[test]
    fn the_open_note_is_checked_against_its_live_text() {
        let l = lib("open-changed", &[("a.md", "lamp\n")]);
        let a_path = np("a.md");
        let p = plan(&l, "lamp", true, "x", Some((&a_path, "lamp\n")));
        // Typed another match meanwhile.
        let a = apply(&p, &Selection::defaults(&p), Some((&a_path, "lamp lamp\n")));
        assert_eq!(a.results[0].outcome, Outcome::Changed);
        assert!(a.open_text.is_none() && a.snapshot.is_none());
    }

    #[test]
    fn undo_of_the_open_note_returns_its_text_instead_of_writing() {
        let l = lib("open-undo", &[("a.md", "lamp\n"), ("b.md", "lamp\n")]);
        let a_path = np("a.md");
        let p = plan(&l, "lamp", true, "x", Some((&a_path, "lamp\n")));
        let a = apply(&p, &Selection::defaults(&p), Some((&a_path, "lamp\n")));
        // The app puts the new text in the buffer and saves it.
        let (_, new) = a.open_text.clone().unwrap();
        write(&l, "a.md", &new);
        let s = a.snapshot.unwrap();
        let u = undo_with_open(&l, &s, Some((&a_path, "x\n")));
        assert_eq!(u.restored, [np("b.md")]);
        assert_eq!(u.open_text, Some((a_path.clone(), "lamp\n".to_owned())));
        assert_eq!(read(&l, "a.md"), "x\n", "the app restores it");
        // Edited in the buffer since: skipped.
        let l2 = lib("open-undo2", &[("a.md", "lamp\n")]);
        let p = plan(&l2, "lamp", true, "x", None);
        let s = apply(&p, &Selection::defaults(&p), None).snapshot.unwrap();
        let u = undo_with_open(&l2, &s, Some((&a_path, "x, then typed\n")));
        assert_eq!(u.skipped, [(a_path, Skip::Edited)]);
        assert_eq!(read(&l2, "a.md"), "x\n");
    }

    #[test]
    fn progress_counts_the_notes_with_a_ticked_match() {
        let l = lib(
            "progress",
            &[("a.md", "lamp"), ("b.md", "lamp"), ("c.md", "lamp")],
        );
        let p = plan(&l, "lamp", true, "x", None);
        let mut sel = Selection::defaults(&p);
        sel.set_note(&p, 1, false);
        let mut seen = Vec::new();
        let a = apply_with_progress(&p, &sel, None, |d, t| seen.push((d, t)));
        assert_eq!(a.notes, 2);
        assert_eq!(seen, [(1, 2), (2, 2)]);
    }

    #[test]
    fn a_failed_snapshot_touches_no_note() {
        let l = lib("snapfail", &[("a.md", "lamp\n")]);
        // `.trash` is a file, so the snapshot folder cannot be made.
        fs::write(l.root().join(TRASH), "in the way").unwrap();
        let p = plan(&l, "lamp", true, "x", None);
        let a = apply(&p, &Selection::defaults(&p), None);
        assert!(a.error.is_some());
        assert_eq!(a.notes, 0);
        assert!(matches!(a.results[0].outcome, Outcome::Failed(_)));
        assert_eq!(read(&l, "a.md"), "lamp\n");
    }

    #[test]
    fn hash_is_fnv1a_64() {
        assert_eq!(hash(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(hash("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(hash("foobar"), 0x8594_4171_f739_67e8);
    }
}
