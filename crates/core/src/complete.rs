//! Wiki-link autocomplete (S23): the link being typed at the caret, and the
//! notes or headings to suggest for it.
//!
//! Matching folds case and the common Latin accents (S23-D2): names that
//! start with the query come first, then names that contain it, each in
//! the order given. There is no fuzzy matching.

use crate::library::NotePath;
use crate::markdown::code_span;
use std::collections::HashMap;

/// One chip of the suggestion bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    /// What the chip shows.
    pub label: String,
    /// What replaces the query when the chip is picked.
    pub insert: String,
}

/// The wiki link being typed: given the caret's line up to the caret, the
/// byte offset where the query starts (just after the last open `[[`) and
/// the query itself, possibly empty.
///
/// `None` outside an open `[[`: once `]]` has closed it, after a `|` (the
/// label is not completed), and inside inline code. A backtick run with no
/// closing run before the caret counts as code being typed. Fenced code is
/// the caller's to rule out, as it needs the lines above.
#[must_use]
pub fn link_query(before_caret: &str) -> Option<(usize, &str)> {
    let mut open: Option<usize> = None;
    let mut i = 0;
    while i < before_caret.len() {
        let rest = &before_caret[i..];
        if rest.starts_with('`') {
            let (t, e) = code_span(rest)?;
            i += 2 * t + e;
        } else if rest.starts_with("[[") {
            open = Some(i + 2);
            i += 2;
        } else if rest.starts_with("]]") {
            open = None;
            i += 2;
        } else {
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    let at = open?;
    let q = &before_caret[at..];
    (!q.contains('|')).then_some((at, q))
}

/// `c` lowercased, without an accent.
fn fold_char(c: char) -> char {
    if c.is_ascii() {
        return c.to_ascii_lowercase();
    }
    let c = crate::find::fold(c);
    if let Some((_, base)) = crate::layouts::decompose(c) {
        return base;
    }
    match c {
        'ç' => 'c',
        'ş' | 'ș' => 's',
        'ţ' | 'ț' => 't',
        'ą' => 'a',
        'ę' => 'e',
        'ł' => 'l',
        'ø' => 'o',
        'ő' => 'o',
        'ű' => 'u',
        'ğ' => 'g',
        c => c,
    }
}

/// `s` lowercased, without accents or combining marks.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| !('\u{300}'..='\u{36F}').contains(c))
        .map(fold_char)
        .collect()
}

fn stem(s: &str) -> &str {
    s.rsplit_once('.').map_or(s, |(a, _)| a)
}

/// Where `key` matches the folded query: 0 a prefix, 1 inside it.
fn rank(key: &str, q: &str) -> Option<u8> {
    if key.starts_with(q) {
        Some(0)
    } else if key.contains(q) {
        Some(1)
    } else {
        None
    }
}

/// Up to `cap` notes for the query `query` (the text after `[[`, without a
/// `#heading` part), matched on the file name without its extension and on
/// the path. Prefix hits come before contains hits, each in library order;
/// an empty query lists `recent` first (most recent first), then the rest.
/// A chip shows and inserts the name, or the path without extension when
/// another note has the same name.
#[must_use]
pub fn suggest_notes(
    query: &str,
    notes: &[NotePath],
    recent: &[NotePath],
    cap: usize,
) -> Vec<Suggestion> {
    let q = fold(query.trim());
    let mut names: HashMap<String, usize> = HashMap::new();
    for p in notes {
        *names.entry(fold(stem(p.name()))).or_default() += 1;
    }
    let chip = |p: &NotePath| {
        let name = stem(p.name());
        let text = if names.get(&fold(name)).copied().unwrap_or(0) > 1 {
            stem(p.as_str())
        } else {
            name
        };
        Suggestion {
            label: text.to_owned(),
            insert: text.to_owned(),
        }
    };
    let picked: Vec<&NotePath> = if q.is_empty() {
        let mut v: Vec<&NotePath> = recent.iter().filter(|p| notes.contains(p)).collect();
        v.dedup();
        v.extend(notes.iter().filter(|p| !recent.contains(p)));
        v
    } else {
        let mut hits: Vec<(u8, &NotePath)> = notes
            .iter()
            .filter_map(|p| {
                let by_name = rank(&fold(stem(p.name())), &q);
                let by_path = rank(&fold(stem(p.as_str())), &q);
                by_name.into_iter().chain(by_path).min().map(|r| (r, p))
            })
            .collect();
        hits.sort_by_key(|h| h.0);
        hits.into_iter().map(|h| h.1).collect()
    };
    picked.into_iter().take(cap).map(chip).collect()
}

/// Up to `cap` headings of `text` (a note's contents, see
/// [`crate::outline::headings`]) for `query`, the text after `#`; matched
/// like [`suggest_notes`], in the note's order. Empty and repeated
/// headings are left out.
#[must_use]
pub fn suggest_headings(text: &str, query: &str, cap: usize) -> Vec<Suggestion> {
    let q = fold(query.trim());
    let mut seen: Vec<String> = Vec::new();
    let mut hits: Vec<(u8, String)> = Vec::new();
    for h in crate::outline::headings(text) {
        if h.text.is_empty() || seen.contains(&h.text) {
            continue;
        }
        seen.push(h.text.clone());
        if let Some(r) = rank(&fold(&h.text), &q) {
            hits.push((r, h.text));
        }
    }
    hits.sort_by_key(|h| h.0);
    hits.into_iter()
        .take(cap)
        .map(|(_, t)| Suggestion {
            label: t.clone(),
            insert: t,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> NotePath {
        NotePath::parse(s).unwrap()
    }

    fn labels(v: &[Suggestion]) -> Vec<&str> {
        v.iter().map(|s| s.label.as_str()).collect()
    }

    #[test]
    fn link_query_open_closed_and_label() {
        assert_eq!(link_query("see [["), Some((6, "")));
        assert_eq!(link_query("see [[chap"), Some((6, "chap")));
        assert_eq!(link_query("[[chapter-01#In"), Some((2, "chapter-01#In")));
        assert_eq!(link_query("see [[chap]]"), None);
        assert_eq!(link_query("see [[chap]] and"), None);
        assert_eq!(link_query("[[chap|lab"), None);
        assert_eq!(link_query("no link here"), None);
        assert_eq!(link_query("one [ bracket"), None);
        assert_eq!(link_query(""), None);
    }

    #[test]
    fn link_query_two_links_on_a_line() {
        assert_eq!(link_query("[[a]] then [[b"), Some((13, "b")));
        assert_eq!(link_query("[[a|x]] [["), Some((10, "")));
        assert_eq!(link_query("[[a]] [[b]]"), None);
        // A second `[[` before the first closed starts over.
        assert_eq!(link_query("[[a [[b"), Some((6, "b")));
        // Non-ASCII before the link keeps byte offsets right.
        let s = "é [[ção";
        let (at, q) = link_query(s).unwrap();
        assert_eq!((&s[at..], q), ("ção", "ção"));
    }

    #[test]
    fn link_query_skips_code_spans() {
        assert_eq!(link_query("`[[a`"), None);
        assert_eq!(link_query("`[[a` [[b"), Some((8, "b")));
        assert_eq!(link_query("``a`[[b`` [[c"), Some((12, "c")));
        // Code still being typed: the span has not closed yet.
        assert_eq!(link_query("x `code [[a"), None);
        assert_eq!(link_query("[[a `x"), None);
    }

    #[test]
    fn suggest_prefix_before_contains() {
        let notes = vec![
            p("my-chapter.md"),
            p("chapter-01.md"),
            p("drafts/outline.md"),
            p("chapter-02.txt"),
        ];
        let s = suggest_notes("chap", &notes, &[], 10);
        assert_eq!(labels(&s), ["chapter-01", "chapter-02", "my-chapter"]);
        assert_eq!(s[0].insert, "chapter-01");
        // Matched on the path too.
        assert_eq!(
            labels(&suggest_notes("drafts/o", &notes, &[], 10)),
            ["outline"]
        );
        assert_eq!(
            labels(&suggest_notes("zzz", &notes, &[], 10)),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn suggest_folds_case_and_accents() {
        let notes = vec![p("Café notes.md"), p("Ação.md"), p("cafeteria.md")];
        assert_eq!(
            labels(&suggest_notes("cafe", &notes, &[], 10)),
            ["Café notes", "cafeteria"]
        );
        assert_eq!(
            labels(&suggest_notes("CAFÉ", &notes, &[], 10)),
            ["Café notes", "cafeteria"]
        );
        assert_eq!(labels(&suggest_notes("acao", &notes, &[], 10)), ["Ação"]);
        // A decomposed accent in the query folds the same way.
        assert_eq!(
            labels(&suggest_notes("Ac\u{327}a\u{303}o", &notes, &[], 10)),
            ["Ação"]
        );
    }

    #[test]
    fn suggest_shared_name_gives_the_path() {
        let notes = vec![
            p("drafts/chapter-01.md"),
            p("final/Chapter-01.md"),
            p("other.md"),
        ];
        let s = suggest_notes("chap", &notes, &[], 10);
        assert_eq!(labels(&s), ["drafts/chapter-01", "final/Chapter-01"]);
        assert_eq!(s[1].insert, "final/Chapter-01");
        assert_eq!(suggest_notes("oth", &notes, &[], 10)[0].insert, "other");
    }

    #[test]
    fn suggest_cap_and_recent_first_on_empty_query() {
        let notes = vec![p("a.md"), p("b.md"), p("c.md"), p("d.md")];
        let recent = vec![p("c.md"), p("gone.md"), p("a.md")];
        assert_eq!(
            labels(&suggest_notes("", &notes, &recent, 10)),
            ["c", "a", "b", "d"]
        );
        assert_eq!(
            labels(&suggest_notes(" ", &notes, &recent, 3)),
            ["c", "a", "b"]
        );
        assert_eq!(suggest_notes("", &notes, &recent, 0), Vec::new());
        // With a query, library order rules, not recency.
        let notes = vec![p("ab.md"), p("ac.md")];
        assert_eq!(
            labels(&suggest_notes("a", &notes, &[p("ac.md")], 1)),
            ["ab"]
        );
    }

    #[test]
    fn suggest_headings_filters_like_notes() {
        let text = "# Intro\n\nbody\n## Méthodes\n```\n# not a heading\n```\n### An intro aside\n#\n# Intro\n";
        assert_eq!(
            labels(&suggest_headings(text, "", 10)),
            ["Intro", "Méthodes", "An intro aside"]
        );
        assert_eq!(
            labels(&suggest_headings(text, "intro", 10)),
            ["Intro", "An intro aside"]
        );
        assert_eq!(labels(&suggest_headings(text, "meth", 10)), ["Méthodes"]);
        assert_eq!(labels(&suggest_headings(text, "", 1)), ["Intro"]);
        assert_eq!(suggest_headings(text, "not", 10), Vec::new());
    }

    /// Times the name-only walk and one suggestion call on 2000 notes in 40
    /// folders, as on a large library (`docs/dev/perf.md`):
    /// `cargo test -p sigma-core --release -- --ignored --nocapture suggest_timing`.
    #[test]
    #[ignore = "measurement"]
    fn suggest_timing() {
        use crate::library::Library;
        use std::time::Instant;
        let d = std::env::temp_dir().join(format!("sigma-suggest-timing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for f in 0..40 {
            let dir = d.join(format!("folder-{f:02}"));
            std::fs::create_dir_all(&dir).unwrap();
            for n in 0..50 {
                let body = format!("# Note {n}\n\n{}", "Some words here. ".repeat(200));
                std::fs::write(dir.join(format!("chapter-{f:02}-{n:02}.md")), body).unwrap();
            }
        }
        let lib = Library::new(&d);
        let t = Instant::now();
        let all = lib.all_notes().unwrap();
        let t_all = t.elapsed();
        let t = Instant::now();
        let notes = lib.note_paths().unwrap();
        let t_paths = t.elapsed();
        assert_eq!(notes.len(), 2000);
        assert_eq!(all.len(), 2000);
        let t = Instant::now();
        let mut n = 0;
        for q in ["", "c", "ch", "cha", "chap", "chapter-1", "07"] {
            n += suggest_notes(q, &notes, &notes[..5], 12).len();
        }
        let t_suggest = t.elapsed() / 7;
        assert!(n > 0);
        println!(
            "suggest_timing: 2000 notes: all_notes {t_all:?}, note_paths {t_paths:?}, suggest_notes {t_suggest:?} per call"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
