//! The side drawer's data (brief §5): headings of a note (Contents), every
//! Markdown checkbox in the library (To-dos), the line rewrite that checks a
//! box, and a background indexer so scanning the library never blocks input.
//!
//! Lines are 0-based logical lines (split on `\n`). Fenced code is skipped:
//! a `# comment` or `- [ ]` inside ``` is not a heading or a to-do.

use crate::library::{Library, NotePath, SCRATCH};
use crate::markdown::item_of;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// A Markdown heading of a note, as found by [`headings`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    /// Number of `#`s, 1 to 6.
    pub level: u8,
    /// The heading text, without the hashes (leading or closing).
    pub text: String,
    /// 0-based logical line it is on.
    pub line: usize,
}

/// A checkbox list item of a note, as found by [`todos`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Todo {
    /// 0-based logical line it is on.
    pub line: usize,
    /// Whether the box is checked.
    pub done: bool,
    /// The item text after the box.
    pub text: String,
}

/// A to-do and the note it lives in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoItem {
    /// The note the to-do is in.
    pub note: NotePath,
    /// The to-do itself.
    pub todo: Todo,
}

/// To-dos pane filter: the button cycles All → Open → Done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    /// Every to-do.
    #[default]
    All,
    /// Unchecked to-dos only.
    Open,
    /// Checked to-dos only.
    Done,
}

impl Filter {
    /// The filter the button moves to next.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Open,
            Self::Open => Self::Done,
            Self::Done => Self::All,
        }
    }

    /// The button label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Open => "Open",
            Self::Done => "Done",
        }
    }

    /// Whether a to-do with this `done` state is shown.
    #[must_use]
    pub fn keeps(self, done: bool) -> bool {
        match self {
            Self::All => true,
            Self::Open => !done,
            Self::Done => done,
        }
    }
}

/// Lines outside fenced code blocks, with their numbers.
fn prose_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut fence = false;
    text.split('\n').enumerate().filter(move |(_, l)| {
        let t = l.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
            return false;
        }
        !fence
    })
}

/// `#`…`######` headings (a space after the hashes), in order.
#[must_use]
pub fn headings(text: &str) -> Vec<Heading> {
    prose_lines(text)
        .filter_map(|(line, l)| {
            let t = l.trim_start();
            let level = t.bytes().take_while(|&b| b == b'#').count();
            let rest = &t[level..];
            ((1..=6).contains(&level) && (rest.is_empty() || rest.starts_with(' '))).then(|| {
                Heading {
                    level: level as u8,
                    text: rest.trim().trim_end_matches('#').trim_end().to_owned(),
                    line,
                }
            })
        })
        .collect()
}

/// List items with a `[ ]` / `[x]` box.
#[must_use]
pub fn todos(text: &str) -> Vec<Todo> {
    prose_lines(text)
        .filter_map(|(line, l)| {
            let (_, task, rest) = item_of(l.trim_start())?;
            task.map(|done| Todo {
                line,
                done,
                text: rest.trim().to_owned(),
            })
        })
        .collect()
}

/// Byte range of logical line `line`, if it exists.
#[must_use]
pub fn line_range(text: &str, line: usize) -> Option<(usize, usize)> {
    let mut start = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i == line {
            return Some((start, start + l.len()));
        }
        start += l.len() + 1;
    }
    None
}

/// Byte range of the box (`[ ]` / `[x]`) on `line`, and whether it is done.
#[must_use]
pub fn box_at(text: &str, line: usize) -> Option<(usize, bool)> {
    let (a, b) = line_range(text, line)?;
    let l = &text[a..b];
    let t = l.trim_start();
    let (_, task, _) = item_of(t)?;
    let done = task?;
    let indent = l.len() - t.len();
    let at = t.find('[')?;
    Some((a + indent + at, done))
}

/// The text with the box on `line` flipped; `None` if that line is not a
/// to-do (the file changed under us: nothing is written). Everything else,
/// including line endings, stays byte-identical.
#[must_use]
pub fn toggle(text: &str, line: usize) -> Option<String> {
    let (at, done) = box_at(text, line)?;
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..at]);
    out.push_str(if done { "[ ]" } else { "[x]" });
    out.push_str(&text[at + 3..]);
    Some(out)
}

/// Every to-do in every note, then the scratch pad (brief: its to-dos still
/// appear), in library order. Unreadable files are skipped.
#[must_use]
pub fn scan(lib: &Library) -> Vec<TodoItem> {
    let mut notes = lib.all_notes().unwrap_or_default();
    if let Ok(s) = NotePath::parse(SCRATCH) {
        notes.push(s);
    }
    let mut out = Vec::new();
    for n in notes {
        let Ok(body) = std::fs::read_to_string(lib.fs_path(&n)) else {
            continue;
        };
        out.extend(todos(&body).into_iter().map(|todo| TodoItem {
            note: n.clone(),
            todo,
        }));
    }
    out
}

/// Scans the library on a worker thread. Requests are numbered; only the
/// newest answer is kept, so a slow scan never overwrites a newer one.
pub struct Indexer {
    tx: Sender<(u64, PathBuf)>,
    rx: Receiver<(u64, Vec<TodoItem>)>,
    asked: u64,
    got: u64,
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}

impl Indexer {
    /// Starts the worker thread. No scan runs until [`Indexer::request`].
    ///
    /// # Panics
    /// If the OS cannot create the thread (from [`std::thread::spawn`]).
    #[must_use]
    pub fn new() -> Self {
        let (tx, jobs) = channel::<(u64, PathBuf)>();
        let (done, rx) = channel();
        thread::spawn(move || {
            while let Ok(mut job) = jobs.recv() {
                // Coalesce a burst of requests into one scan.
                while let Ok(newer) = jobs.try_recv() {
                    job = newer;
                }
                let items = scan(&Library::new(&job.1));
                if done.send((job.0, items)).is_err() {
                    break;
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

    /// Asks for a scan of the library at `root`. Requests that pile up
    /// before the worker gets to them are merged into the newest one.
    pub fn request(&mut self, root: PathBuf) {
        self.asked += 1;
        let _ = self.tx.send((self.asked, root));
    }

    /// Whether the newest request has not been answered yet.
    #[must_use]
    pub fn pending(&self) -> bool {
        self.got < self.asked
    }

    /// The newest finished scan since the last call, without blocking.
    pub fn poll(&mut self) -> Option<Vec<TodoItem>> {
        let mut best = None;
        while let Ok((n, items)) = self.rx.try_recv() {
            if n > self.got {
                self.got = n;
                best = Some(items);
            }
        }
        best
    }

    /// Blocks until the newest request is answered (tests, emulator).
    pub fn wait(&mut self) -> Option<Vec<TodoItem>> {
        let mut best = self.poll();
        while self.pending() {
            match self.rx.recv() {
                Ok((n, items)) if n > self.got => {
                    self.got = n;
                    best = Some(items);
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

    const NOTE: &str = "# Title\n\nintro\n## Part two ##\n- [ ] open one\n  - [x] nested done\n* [X] star done\n1. [ ] numbered\n- plain item\n```\n# not a heading\n- [ ] not a todo\n```\n#nospace\n### Third";

    #[test]
    fn headings_skip_code_and_need_a_space() {
        let h = headings(NOTE);
        let got: Vec<_> = h
            .iter()
            .map(|h| (h.level, h.text.as_str(), h.line))
            .collect();
        assert_eq!(got, [(1, "Title", 0), (2, "Part two", 3), (3, "Third", 14)]);
        assert!(headings("no headings\njust text").is_empty());
    }

    #[test]
    fn todos_find_every_box_style() {
        let t = todos(NOTE);
        let got: Vec<_> = t
            .iter()
            .map(|t| (t.line, t.done, t.text.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (4, false, "open one"),
                (5, true, "nested done"),
                (6, true, "star done"),
                (7, false, "numbered"),
            ]
        );
    }

    #[test]
    fn toggle_rewrites_only_that_box() {
        let crlf = "a\r\n- [ ] x\r\n  - [X] y\r\n";
        let once = toggle(crlf, 1).unwrap();
        assert_eq!(once, "a\r\n- [x] x\r\n  - [X] y\r\n");
        assert_eq!(toggle(&once, 2).unwrap(), "a\r\n- [x] x\r\n  - [ ] y\r\n");
        assert_eq!(toggle(&toggle(NOTE, 4).unwrap(), 4).unwrap(), NOTE);
        assert_eq!(toggle(NOTE, 8), None, "plain item");
        assert_eq!(toggle(NOTE, 13), None, "#nospace");
        assert_eq!(toggle(NOTE, 99), None);
    }

    #[test]
    fn filter_cycles() {
        let f = Filter::default();
        assert_eq!(
            (f.next(), f.next().next(), f.next().next().next()),
            (Filter::Open, Filter::Done, Filter::All)
        );
        assert!(Filter::Open.keeps(false) && !Filter::Open.keeps(true));
        assert!(Filter::Done.keeps(true) && Filter::All.keeps(false));
    }

    #[test]
    fn scan_includes_scratch_and_indexer_returns_newest() {
        let root = std::env::temp_dir().join(format!("sigma-outline-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("drafts")).unwrap();
        std::fs::write(root.join("a.md"), "- [ ] alpha\n").unwrap();
        std::fs::write(root.join("drafts/b.md"), "- [x] beta\n").unwrap();
        std::fs::write(root.join(SCRATCH), "- [ ] from scratch\n").unwrap();
        let items = scan(&Library::new(&root));
        let names: Vec<_> = items
            .iter()
            .map(|i| (i.note.as_str(), i.todo.text.as_str()))
            .collect();
        assert!(names.contains(&("a.md", "alpha")));
        assert!(names.contains(&("drafts/b.md", "beta")));
        assert_eq!(names.last(), Some(&(SCRATCH, "from scratch")));
        let mut ix = Indexer::new();
        ix.request(root.clone());
        std::fs::write(root.join("a.md"), "- [ ] alpha\n- [ ] gamma\n").unwrap();
        ix.request(root.clone());
        let got = ix.wait().unwrap();
        assert!(!ix.pending());
        assert_eq!(got.len(), 4);
        std::fs::remove_dir_all(root).unwrap();
    }
}
