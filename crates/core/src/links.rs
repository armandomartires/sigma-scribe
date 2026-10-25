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

/// Rewrites `[[…]]` links in `text` whose target resolved (among
/// `before`, the notes before a rename / move) to a note in `moved`, to
/// that note's new name (v1.5.2). Keeps the link's form: a path stays a
/// path, a bare name a bare name, an `.md` / `.txt` ending and a
/// `#heading` part stay, an `|alias` stays as written. Links in fenced
/// code and inline code are left alone. `None` when nothing changed.
#[must_use]
pub fn rewrite(text: &str, before: &[NotePath], moved: &[(NotePath, NotePath)]) -> Option<String> {
    let stem = |s: &str| s.rsplit_once('.').map_or(s, |(a, _)| a).to_owned();
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut fence = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
            out.push_str(line);
            continue;
        }
        if fence {
            out.push_str(line);
            continue;
        }
        let mut rest = line;
        let mut code = false;
        while !rest.is_empty() {
            if rest.starts_with('`') {
                code = !code;
                out.push('`');
                rest = &rest[1..];
                continue;
            }
            if !code && rest.starts_with("[[") {
                if let Some(end) = rest[2..].find("]]") {
                    let inner = &rest[2..2 + end];
                    let (target, alias) = match inner.split_once('|') {
                        Some((t, a)) => (t, Some(a)),
                        None => (inner, None),
                    };
                    let (name, frag) = match target.split_once('#') {
                        Some((n, f)) => (n, Some(f)),
                        None => (target, None),
                    };
                    let hit = resolve(name, before)
                        .and_then(|p| moved.iter().find(|(o, _)| o == p))
                        .map(|(_, n)| n);
                    if let Some(new) = hit {
                        let trimmed = name.trim();
                        let lower = trimmed.to_lowercase();
                        let ext = [".md", ".txt"]
                            .into_iter()
                            .find(|e| lower.ends_with(e))
                            .map_or(String::new(), |e| {
                                trimmed[trimmed.len() - e.len()..].to_owned()
                            });
                        let base = if trimmed.contains('/') {
                            stem(new.as_str())
                        } else {
                            stem(new.name())
                        };
                        out.push_str("[[");
                        out.push_str(&base);
                        out.push_str(&ext);
                        if let Some(f) = frag {
                            out.push('#');
                            out.push_str(f);
                        }
                        if let Some(a) = alias {
                            out.push('|');
                            out.push_str(a);
                        }
                        out.push_str("]]");
                        changed = true;
                        rest = &rest[end + 4..];
                        continue;
                    }
                    out.push_str(&rest[..end + 4]);
                    rest = &rest[end + 4..];
                    continue;
                }
            }
            let c = rest.chars().next().unwrap_or(' ');
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    (changed && out != text).then_some(out)
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
    /// The target as written in the note.
    pub target: String,
    /// Index into [`Graph::notes`] of the note it reaches; `None` when no
    /// note matches (a broken link).
    pub to: Option<usize>,
}

/// Every note and its outgoing links.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    /// Every note, in library order.
    pub notes: Vec<NotePath>,
    /// Outgoing links of each note, by the note's index in `notes`. Links
    /// from a note to itself are left out.
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

    /// Builds the graph from notes and their texts (`texts[i]` is the text
    /// of `notes[i]`), without touching the disk.
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

    /// Position of `p` in [`Graph::notes`].
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
    fn rewrite_keeps_form_alias_heading_and_skips_code() {
        let before = vec![p("drafts/old.md"), p("other.md"), p("b/old.md")];
        let moved = vec![(p("drafts/old.md"), p("drafts/new one.md"))];
        let t = "a [[old]] b [[Old|my alias]] c [[drafts/old#Intro|x]] d [[old.md]]\n\
                 `[[old]]` [[other]] [[b/old]]\n```\n[[old]]\n```\n";
        let got = rewrite(t, &before, &moved).unwrap();
        assert_eq!(
            got,
            "a [[new one]] b [[new one|my alias]] c [[drafts/new one#Intro|x]] d [[new one.md]]\n\
             `[[old]]` [[other]] [[b/old]]\n```\n[[old]]\n```\n"
        );
        assert_eq!(rewrite("[[other]] é [[x]]", &before, &moved), None);
        // Folder move: path links follow, bare names keep the name.
        let moved = vec![(p("other.md"), p("archive/other.md"))];
        assert_eq!(
            rewrite("[[other]]", &before, &moved),
            None,
            "bare name unchanged"
        );
        assert_eq!(
            rewrite(
                "[[archive]] [[./x]] [[other]] [[OTHER.md]]",
                &before,
                &moved
            )
            .as_deref(),
            Some("[[archive]] [[./x]] [[other]] [[other.md]]")
        );
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
