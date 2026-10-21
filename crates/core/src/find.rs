//! Find and replace in the open note (brief §3 "Find and replace", S6).
//!
//! Matches are byte ranges on `char` boundaries, left to right and not
//! overlapping. Case-insensitive matching compares characters one by one
//! with their simple lowercase, so a match always covers exactly as many
//! characters as the query.

/// Every match of `query` in `text`.
#[must_use]
pub fn matches(text: &str, query: &str, case: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if query.is_empty() {
        return out;
    }
    let q: Vec<char> = query.chars().collect();
    let mut i = 0;
    while i < text.len() {
        if let Some(end) = match_at(text, i, &q, case) {
            out.push((i, end));
            i = end;
        } else {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    out
}

fn fold(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

fn match_at(text: &str, at: usize, q: &[char], case: bool) -> Option<usize> {
    let mut it = text[at..].char_indices();
    for &qc in q {
        let (_, tc) = it.next()?;
        let same = if case { tc == qc } else { fold(tc) == fold(qc) };
        if !same {
            return None;
        }
    }
    Some(it.next().map_or(text.len(), |(o, _)| at + o))
}

/// The match to show for a caret/selection at `from`: the first match that
/// starts at or after `from`, wrapping to the first.
#[must_use]
pub fn at_or_after(ms: &[(usize, usize)], from: usize) -> Option<usize> {
    if ms.is_empty() {
        return None;
    }
    Some(ms.iter().position(|m| m.0 >= from).unwrap_or(0))
}

/// Next match after the one at `cur` (or after `pos`), wrapping.
#[must_use]
pub fn next(ms: &[(usize, usize)], pos: usize) -> Option<usize> {
    if ms.is_empty() {
        return None;
    }
    Some(ms.iter().position(|m| m.0 >= pos).unwrap_or(0))
}

/// Match before `pos` (a match that starts before it), wrapping to the last.
#[must_use]
pub fn prev(ms: &[(usize, usize)], pos: usize) -> Option<usize> {
    if ms.is_empty() {
        return None;
    }
    Some(ms.iter().rposition(|m| m.0 < pos).unwrap_or(ms.len() - 1))
}

/// `text` with every match replaced.
#[must_use]
pub fn replace_all(text: &str, ms: &[(usize, usize)], with: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for &(a, b) in ms {
        out.push_str(&text[last..a]);
        out.push_str(with);
        last = b;
    }
    out.push_str(&text[last..]);
    out
}

/// The count label: `Find` (empty query), `None`, or `i of n`
/// (`i` is the selected match; `– of n` when none is selected).
#[must_use]
pub fn count_label(query: &str, ms: &[(usize, usize)], current: Option<usize>) -> String {
    if query.is_empty() {
        "Find".into()
    } else if ms.is_empty() {
        "None".into()
    } else {
        match current {
            Some(i) => format!("{} of {}", i + 1, ms.len()),
            None => format!("\u{2013} of {}", ms.len()),
        }
    }
}

/// Index of the match that is exactly the selection `sel`.
#[must_use]
pub fn selected(ms: &[(usize, usize)], sel: Option<(usize, usize)>) -> Option<usize> {
    let s = sel?;
    ms.iter().position(|&m| m == s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_insensitive_by_default_and_exact_with_aa() {
        let t = "Mara met mara. MARA!";
        assert_eq!(matches(t, "mara", false), vec![(0, 4), (9, 13), (15, 19)]);
        assert_eq!(matches(t, "mara", true), vec![(9, 13)]);
        assert!(matches(t, "", false).is_empty());
        assert!(matches(t, "zebra", false).is_empty());
    }

    #[test]
    fn unicode_and_no_overlap() {
        let t = "Éclair éCLAIR aaaa";
        assert_eq!(matches(t, "éclair", false).len(), 2);
        assert_eq!(matches(t, "aa", false), vec![(16, 18), (18, 20)]);
        for (a, b) in matches("ßß straße", "ß", false) {
            assert!("ßß straße".is_char_boundary(a) && "ßß straße".is_char_boundary(b));
        }
    }

    #[test]
    fn navigation_wraps() {
        let ms = vec![(0, 1), (5, 6), (9, 10)];
        assert_eq!(next(&ms, 6), Some(2));
        assert_eq!(next(&ms, 10), Some(0));
        assert_eq!(prev(&ms, 5), Some(0));
        assert_eq!(prev(&ms, 0), Some(2));
        assert_eq!(at_or_after(&ms, 3), Some(1));
        assert_eq!(next(&[], 0), None);
    }

    #[test]
    fn replace_all_fifty() {
        let t = "cat ".repeat(50);
        let ms = matches(&t, "CAT", false);
        assert_eq!(ms.len(), 50);
        let r = replace_all(&t, &ms, "dog");
        assert_eq!(r, "dog ".repeat(50));
        assert_eq!(replace_all("a-a", &matches("a-a", "a", true), ""), "-");
    }

    #[test]
    fn labels() {
        let ms = vec![(0, 1), (2, 3)];
        assert_eq!(count_label("", &[], None), "Find");
        assert_eq!(count_label("x", &[], None), "None");
        assert_eq!(count_label("x", &ms, Some(1)), "2 of 2");
        assert_eq!(selected(&ms, Some((2, 3))), Some(1));
        assert_eq!(selected(&ms, Some((1, 3))), None);
    }
}
