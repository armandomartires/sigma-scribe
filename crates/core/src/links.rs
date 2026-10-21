//! Wiki links (brief §4, §6 Graph, S6): `[[note]]` and `[[note|label]]`
//! targets, resolution to notes, backlinks, and the link graph.
//!
//! Targets come from the Markdown parser, so links inside code are not
//! links. A target names a note by its file name or its path from the
//! library root, with or without the extension, ignoring case. The scratch
//! file is never a node.

use crate::library::{clean_note_name, Library, NotePath};

/// Link targets in `text`, in order, without repeats (case-insensitive).
#[must_use]
pub fn targets(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for b in crate::markdown::parse(text) {
        for r in &b.runs {
            if r.style.link == Some(crate::markdown::LinkKind::Wiki) {
                if let Some(t) = r.target.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
                    if !out.iter().any(|o| o.eq_ignore_ascii_case(t)) {
                        out.push(t.to_owned());
                    }
                }
            }
        }
    }
    out
}

fn keys(p: &NotePath) -> [String; 2] {
    let stem = |s: &str| s.rsplit_once('.').map_or(s, |(a, _)| a).to_lowercase();
    [stem(p.as_str()), stem(p.name())]
}

/// The note a target names. A full path wins over a bare name; among
/// equal names the first in library order wins.
#[must_use]
pub fn resolve<'a>(target: &str, notes: &'a [NotePath]) -> Option<&'a NotePath> {
    let t = target.trim().to_lowercase();
    let t = t
        .strip_suffix(".md")
        .or_else(|| t.strip_suffix(".txt"))
        .unwrap_or(&t);
    notes
        .iter()
        .find(|p| keys(p)[0] == t)
        .or_else(|| notes.iter().find(|p| keys(p)[1] == t))
}

/// Where tapping a link to a missing note creates it: in the named folder
/// if the target has a path and that folder exists, else beside the note
/// that links to it. The name is cleaned like any new note.
#[must_use]
pub fn new_note_place(lib: &Library, from: &NotePath, target: &str) -> (NotePath, String) {
    let t = target.trim();
    let (dir, name) = t.rsplit_once('/').unwrap_or(("", t));
    let folder = NotePath::parse(dir)
        .ok()
        .filter(|f| !dir.is_empty() && lib.fs_path(f).is_dir())
        .unwrap_or_else(|| from.parent());
    (folder, clean_note_name(name, ".md"))
}

/// One outgoing link: its target text and the note it reaches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub target: String,
    pub to: Option<usize>,
}

/// Every note and its outgoing links.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    pub notes: Vec<NotePath>,
    pub out: Vec<Vec<Link>>,
}

impl Graph {
    /// Reads every note; `live` overrides one note's text (the open buffer).
    #[must_use]
    pub fn build(lib: &Library, live: Option<(&NotePath, &str)>) -> Self {
        let notes = lib.all_notes().unwrap_or_default();
        let texts: Vec<String> = notes
            .iter()
            .map(|p| match live {
                Some((lp, t)) if lp == p => t.to_owned(),
                _ => std::fs::read_to_string(lib.fs_path(p)).unwrap_or_default(),
            })
            .collect();
        Self::from_texts(notes, &texts)
    }

    #[must_use]
    pub fn from_texts(notes: Vec<NotePath>, texts: &[String]) -> Self {
        let out = texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                targets(t)
                    .into_iter()
                    .map(|target| {
                        let to = resolve(&target, &notes)
                            .and_then(|p| notes.iter().position(|n| n == p))
                            .filter(|&j| j != i);
                        Link { target, to }
                    })
                    // A note linking to itself is not an edge.
                    .filter(|l| l.to.is_some() || resolve(&l.target, &notes).is_none())
                    .collect()
            })
            .collect();
        Self { notes, out }
    }

    #[must_use]
    pub fn index(&self, p: &NotePath) -> Option<usize> {
        self.notes.iter().position(|n| n == p)
    }

    /// Notes that link to `p`, in library order.
    #[must_use]
    pub fn backlinks(&self, p: &NotePath) -> Vec<NotePath> {
        let Some(i) = self.index(p) else {
            return Vec::new();
        };
        self.out
            .iter()
            .enumerate()
            .filter(|(_, ls)| ls.iter().any(|l| l.to == Some(i)))
            .map(|(j, _)| self.notes[j].clone())
            .collect()
    }

    /// Number of links that reach a note.
    #[must_use]
    pub fn edges(&self) -> usize {
        self.out.iter().flatten().filter(|l| l.to.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> NotePath {
        NotePath::parse(s).unwrap()
    }

    #[test]
    fn targets_skip_code_and_dedupe() {
        let t = "See [[A]] and [[b|Bee]], [[a]] again.\n\n```\n[[code]]\n```\n`[[inline]]`";
        assert_eq!(targets(t), vec!["A".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn resolve_by_name_or_path_ignoring_case_and_extension() {
        let notes = vec![
            p("field-notes.md"),
            p("drafts/chapter-01.md"),
            p("x/notes.txt"),
        ];
        assert_eq!(resolve("Chapter-01", &notes), Some(&notes[1]));
        assert_eq!(resolve("drafts/chapter-01.md", &notes), Some(&notes[1]));
        assert_eq!(resolve("notes", &notes), Some(&notes[2]));
        assert_eq!(resolve("missing", &notes), None);
    }

    #[test]
    fn graph_and_backlinks() {
        let notes = vec![p("a.md"), p("b.md"), p("c.md")];
        let texts = vec![
            "[[b]] [[c]] [[nowhere]] [[a]]".to_owned(),
            "[[c]]".to_owned(),
            String::new(),
        ];
        let g = Graph::from_texts(notes.clone(), &texts);
        assert_eq!(
            g.backlinks(&notes[2]),
            vec![notes[0].clone(), notes[1].clone()]
        );
        assert_eq!(g.backlinks(&notes[0]), Vec::<NotePath>::new());
        assert_eq!(g.edges(), 3);
        assert_eq!(g.out[0].len(), 3, "self link dropped, missing kept");
        assert_eq!(g.out[0][2].to, None);
    }

    #[test]
    fn new_note_place_uses_folder_when_it_exists() {
        let dir = std::env::temp_dir().join(format!("sigma-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("drafts")).unwrap();
        let lib = Library::new(&dir);
        let from = p("drafts/chapter-01.md");
        assert_eq!(
            new_note_place(&lib, &from, "The Parcel"),
            (p("drafts"), "The Parcel.md".to_owned())
        );
        let root = p("field-notes.md");
        assert_eq!(new_note_place(&lib, &root, "drafts/x").0, p("drafts"));
        assert_eq!(new_note_place(&lib, &root, "nope/x").0, NotePath::root());
        let _ = std::fs::remove_dir_all(dir);
    }
}
