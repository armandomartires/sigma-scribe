//! Search across every note of the library (S22, "Find bar: All notes").
//!
//! [`search_notes`] runs [`crate::find::matches`] on the whole text of each
//! note and turns each byte range into a [`SearchHit`] with the note, the
//! 1-based line and the absolute byte offset, so a hit can become the caret
//! when the note is opened. [`Searcher`] runs the same search on a worker
//! thread, shaped like [`crate::outline::Indexer`]: requests are numbered
//! and only the newest one's answer is ever handed back, so typing a new
//! query while a slow search runs never shows stale results.
//!
//! The worker reads notes from disk, so the app must save the open note
//! before each request for its latest text to be searched.

use crate::find::matches;
use crate::library::{Library, NotePath};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// One match of the query in one note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    /// The note the hit is in.
    pub note: NotePath,
    /// 1-based line of the hit (lines split on `\n`).
    pub line: usize,
    /// Absolute byte offset of the hit in the note's text, on a `char`
    /// boundary; usable directly as the caret when the note is opened.
    pub byte: usize,
    /// Length of the hit in bytes.
    pub len: usize,
    /// The content of that line, without the line ending.
    pub text: String,
}

/// What [`search_notes`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchResult {
    /// The hits, in library order and then text order within a note.
    pub hits: Vec<SearchHit>,
    /// How many notes have at least one hit.
    pub notes_with_hits: usize,
    /// Whether the search stopped at the cap, so there may be more hits.
    pub capped: bool,
    /// Notes that could not be read (missing or not UTF-8) and were skipped.
    pub skipped: usize,
}

/// Searches `query` in `notes` under `root`, in the order given (normally
/// [`Library::all_notes`] order), stopping once `cap` hits are collected.
///
/// An empty query finds nothing. `case` is whether matching is
/// case-sensitive. Notes that cannot be read, or are not UTF-8, are skipped
/// and counted in [`SearchResult::skipped`].
#[must_use]
pub fn search_notes(
    root: &Path,
    notes: &[NotePath],
    query: &str,
    case: bool,
    cap: usize,
) -> SearchResult {
    search_unless(root, notes, query, case, cap, || false).unwrap_or_default()
}

/// [`search_notes`], giving up (with `None`) as soon as `abandon` says so.
/// `abandon` is asked once per note, so a long search can yield to a newer
/// request without finishing.
fn search_unless(
    root: &Path,
    notes: &[NotePath],
    query: &str,
    case: bool,
    cap: usize,
    mut abandon: impl FnMut() -> bool,
) -> Option<SearchResult> {
    let mut out = SearchResult::default();
    if query.is_empty() || cap == 0 {
        return Some(out);
    }
    let lib = Library::new(root);
    for note in notes {
        if abandon() {
            return None;
        }
        let Ok(text) = std::fs::read_to_string(lib.fs_path(note)) else {
            out.skipped += 1;
            continue;
        };
        let ms = matches(&text, query, case);
        if ms.is_empty() {
            continue;
        }
        out.notes_with_hits += 1;
        let room = cap - out.hits.len();
        if ms.len() > room {
            out.capped = true;
        }
        out.hits
            .extend(hits_of(note, &text, &ms[..ms.len().min(room)]));
        if out.hits.len() >= cap {
            // Any note after this one may hold more hits.
            out.capped = out.capped || notes.last() != Some(note);
            break;
        }
    }
    Some(out)
}

/// Turns ascending byte ranges of `text` into hits with their line.
fn hits_of(note: &NotePath, text: &str, ms: &[(usize, usize)]) -> Vec<SearchHit> {
    let mut out = Vec::with_capacity(ms.len());
    let mut ms = ms.iter().peekable();
    // Walk the lines once; `start` is the byte offset of the current line.
    let mut start = 0;
    for (i, raw) in text.split('\n').enumerate() {
        let end = start + raw.len();
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        while let Some(&&(from, to)) = ms.peek() {
            if from > end {
                break;
            }
            out.push(SearchHit {
                note: note.clone(),
                line: i + 1,
                byte: from,
                len: to - from,
                text: line.to_owned(),
            });
            ms.next();
        }
        start = end + 1;
    }
    out
}

/// One search to run on the [`Searcher`] worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRequest {
    /// The library root on disk.
    pub root: PathBuf,
    /// The notes to search, in order; `None` lists the whole library with
    /// [`Library::all_notes`] on the worker.
    pub notes: Option<Vec<NotePath>>,
    /// The text to find.
    pub query: String,
    /// Whether matching is case-sensitive.
    pub case: bool,
    /// Stop after this many hits.
    pub cap: usize,
}

/// Searches the library on a worker thread. Requests are numbered; only
/// the newest answer is kept, and a search that is still running when a
/// newer request arrives is abandoned, so a slow search never overwrites
/// (or delays) a newer one.
pub struct Searcher {
    tx: Sender<(u64, SearchRequest)>,
    rx: Receiver<(u64, SearchResult)>,
    asked: u64,
    got: u64,
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher {
    /// Starts the worker thread. No search runs until [`Searcher::request`].
    ///
    /// # Panics
    /// If the OS cannot create the thread (from [`std::thread::spawn`]).
    #[must_use]
    pub fn new() -> Self {
        let (tx, jobs) = channel::<(u64, SearchRequest)>();
        let (done, rx) = channel();
        thread::spawn(move || {
            let mut next: Option<(u64, SearchRequest)> = None;
            loop {
                let (mut id, mut req) = match next.take() {
                    Some(job) => job,
                    None => match jobs.recv() {
                        Ok(job) => job,
                        Err(_) => break,
                    },
                };
                // Coalesce a burst of requests into the newest one.
                while let Ok(newer) = jobs.try_recv() {
                    (id, req) = newer;
                }
                let notes = match req.notes.take() {
                    Some(n) => n,
                    None => Library::new(&req.root).all_notes().unwrap_or_default(),
                };
                let result = search_unless(
                    &req.root,
                    &notes,
                    &req.query,
                    req.case,
                    req.cap,
                    || match jobs.try_recv() {
                        Ok(newer) => {
                            next = Some(newer);
                            true
                        }
                        Err(_) => false,
                    },
                );
                if let Some(result) = result {
                    if done.send((id, result)).is_err() {
                        break;
                    }
                }
            }
        });
        Self {
            tx,
            rx,
            asked: 0,
            got: 0,
        }
    }

    /// Asks for a search. Requests that pile up before the worker gets to
    /// them are merged into the newest one, and a running search is
    /// abandoned for a newer request.
    pub fn request(&mut self, req: SearchRequest) {
        self.asked += 1;
        let _ = self.tx.send((self.asked, req));
    }

    /// Whether the newest request has not been answered yet.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.got < self.asked
    }

    /// The newest finished search since the last call, without blocking.
    pub fn poll(&mut self) -> Option<SearchResult> {
        let mut best = None;
        while let Ok((n, result)) = self.rx.try_recv() {
            if n > self.got {
                self.got = n;
                best = Some(result);
            }
        }
        best
    }

    /// Blocks until the newest request is answered (tests, emulator).
    pub fn wait(&mut self) -> Option<SearchResult> {
        let mut best = self.poll();
        while self.pending() {
            match self.rx.recv() {
                Ok((n, result)) if n > self.got => {
                    self.got = n;
                    best = Some(result);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A fresh library on disk with the given files (path, bytes).
    fn lib(tag: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("sigma-search-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (p, body) in files {
            let f = root.join(p);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, body).unwrap();
        }
        root
    }

    fn three(tag: &str) -> (PathBuf, Vec<NotePath>) {
        let root = lib(
            tag,
            &[
                ("a.md", b"alpha\nbeta\n" as &[u8]),
                ("b.md", b"nothing here\n"),
                ("c/d.md", b"one\ntwo\nthree Beta beta\n"),
            ],
        );
        let notes = Library::new(&root).all_notes().unwrap();
        (root, notes)
    }

    fn brief(r: &SearchResult) -> Vec<(&str, usize, usize)> {
        r.hits
            .iter()
            .map(|h| (h.note.as_str(), h.line, h.byte))
            .collect()
    }

    #[test]
    fn hits_across_notes_in_library_order() {
        let (root, notes) = three("order");
        assert_eq!(
            notes.iter().map(NotePath::as_str).collect::<Vec<_>>(),
            ["a.md", "b.md", "c/d.md"]
        );
        let r = search_notes(&root, &notes, "beta", false, 100);
        assert_eq!(
            brief(&r),
            [("a.md", 2, 6), ("c/d.md", 3, 14), ("c/d.md", 3, 19)]
        );
        assert_eq!(r.notes_with_hits, 2);
        assert!(!r.capped);
        assert_eq!(r.skipped, 0);
        assert!(search_notes(&root, &notes, "", false, 100).hits.is_empty());
    }

    #[test]
    fn line_byte_len_and_text_of_a_third_line_hit() {
        let (root, notes) = three("third");
        let r = search_notes(&root, &notes, "three", true, 100);
        assert_eq!(r.hits.len(), 1);
        let h = &r.hits[0];
        assert_eq!(h.note.as_str(), "c/d.md");
        assert_eq!(h.line, 3);
        assert_eq!(h.byte, 8);
        assert_eq!(h.len, 5);
        assert_eq!(h.text, "three Beta beta");
        // CRLF: the line text has no `\r`, the offset is still absolute.
        let root = lib("crlf", &[("n.md", b"x\r\nyy\r\nzzz needle\r\n")]);
        let notes = Library::new(&root).all_notes().unwrap();
        let r = search_notes(&root, &notes, "needle", true, 100);
        assert_eq!(brief(&r), [("n.md", 3, 11)]);
        assert_eq!(r.hits[0].text, "zzz needle");
    }

    #[test]
    fn cap_is_honoured() {
        let (root, notes) = three("cap");
        let r = search_notes(&root, &notes, "beta", false, 2);
        assert_eq!(brief(&r), [("a.md", 2, 6), ("c/d.md", 3, 14)]);
        assert!(r.capped);
        assert_eq!(r.notes_with_hits, 2);
        // Exactly at the cap on the last note: nothing more could follow.
        let r = search_notes(&root, &notes, "beta", false, 3);
        assert_eq!(r.hits.len(), 3);
        assert!(!r.capped);
        // Exactly at the cap with notes still to search: may be more.
        let r = search_notes(&root, &notes, "alpha", false, 1);
        assert_eq!(r.hits.len(), 1);
        assert!(r.capped);
        assert!(search_notes(&root, &notes, "beta", false, 0)
            .hits
            .is_empty());
    }

    #[test]
    fn case_flag() {
        let (root, notes) = three("case");
        assert_eq!(
            search_notes(&root, &notes, "beta", false, 100).hits.len(),
            3
        );
        assert_eq!(search_notes(&root, &notes, "beta", true, 100).hits.len(), 2);
        assert_eq!(search_notes(&root, &notes, "Beta", true, 100).hits.len(), 1);
    }

    #[test]
    fn trash_is_not_searched() {
        let root = lib(
            "trash",
            &[
                ("keep.md", b"needle" as &[u8]),
                (".trash/1/gone.md", b"needle"),
                (".trash/1/.origin", b"gone.md"),
                (".ink/keep.md.ink", b"needle"),
            ],
        );
        let notes = Library::new(&root).all_notes().unwrap();
        let r = search_notes(&root, &notes, "needle", true, 100);
        assert_eq!(brief(&r), [("keep.md", 1, 0)]);
    }

    #[test]
    fn unreadable_notes_are_skipped_and_counted() {
        let root = lib(
            "skip",
            &[
                ("bad.md", b"\xff\xfe needle" as &[u8]),
                ("good.md", b"needle"),
            ],
        );
        let mut notes = Library::new(&root).all_notes().unwrap();
        notes.push(NotePath::parse("missing.md").unwrap());
        let r = search_notes(&root, &notes, "needle", true, 100);
        assert_eq!(brief(&r), [("good.md", 1, 0)]);
        assert_eq!(r.skipped, 2);
        assert_eq!(r.notes_with_hits, 1);
    }

    #[test]
    fn searcher_newest_request_wins() {
        // A big library for the slow first request, a tiny one for the second.
        let big: Vec<(String, Vec<u8>)> = (0..400)
            .map(|i| (format!("n{i}.md"), b"slow needle\n".repeat(50)))
            .collect();
        let big_files: Vec<(&str, &[u8])> = big
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_slice()))
            .collect();
        let slow = lib("slow", &big_files);
        let fast = lib("fast", &[("one.md", b"fast needle" as &[u8])]);

        let mut s = Searcher::new();
        assert!(!s.pending());
        assert_eq!(s.poll(), None);
        s.request(SearchRequest {
            root: slow.clone(),
            notes: None,
            query: "needle".into(),
            case: true,
            cap: 100_000,
        });
        s.request(SearchRequest {
            root: fast.clone(),
            notes: None,
            query: "needle".into(),
            case: true,
            cap: 100,
        });
        assert!(s.pending());
        let r = s.wait().expect("the newest request is answered");
        assert_eq!(brief(&r), [("one.md", 1, 5)]);
        assert_eq!(r.hits[0].text, "fast needle");
        assert!(!s.pending());
        assert_eq!(s.poll(), None, "the first result never surfaces");

        // A request with an explicit note list and poll after wait.
        let notes = Library::new(&slow).all_notes().unwrap();
        s.request(SearchRequest {
            root: slow,
            notes: Some(notes[..2].to_vec()),
            query: "needle".into(),
            case: true,
            cap: 10,
        });
        let r = s.wait().unwrap();
        assert_eq!(r.hits.len(), 10);
        assert!(r.capped);
        assert_eq!(r.notes_with_hits, 1);
    }
}
