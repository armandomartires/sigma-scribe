//! Soft wrapping by measured width.
//!
//! The brief insists on measured text ("do not guess line length"). The
//! measure is a trait so the device can supply real glyph advances while
//! tests and the monospace Cobalt grid use [`MonoMeasure`].

/// Width of a run of text in the front end's unit (pixels or grid cells).
pub trait TextMeasure {
    fn width(&self, text: &str) -> u32;

    /// x of the caret at byte `at` of a row, from the row's left end. A
    /// right-to-left row (v1.8) is laid out by its glyphs, not by the
    /// width of the text before the caret.
    fn caret_x(&self, row: &str, at: usize) -> u32 {
        self.width(&row[..at])
    }

    /// Byte of a right-to-left row nearest to `x`; None: use widths.
    fn index_at(&self, _row: &str, _x: u32) -> Option<usize> {
        None
    }

    /// Does the row read right to left (it is drawn flush right)?
    fn rtl(&self, _row: &str) -> bool {
        false
    }
}

/// Every char is one cell wide; tabs expand to `tab` cells.
#[derive(Clone, Copy, Debug)]
pub struct MonoMeasure {
    pub tab: u32,
}

impl Default for MonoMeasure {
    fn default() -> Self {
        Self { tab: 4 }
    }
}

impl TextMeasure for MonoMeasure {
    fn width(&self, text: &str) -> u32 {
        text.chars()
            .map(|c| if c == '\t' { self.tab } else { 1 })
            .sum()
    }
}

/// One drawn row: a byte range of the source text (without the `\n`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualLine {
    pub start: usize,
    pub end: usize,
}

/// Wraps `text` into rows no wider than `max` units. Hard line breaks are
/// kept, words are not split unless a single word is wider than a row, and
/// trailing spaces stay on the row they follow (so the caret after a space
/// has a home). Always returns at least one row.
#[must_use]
pub fn wrap(text: &str, max: u32, measure: &dyn TextMeasure) -> Vec<VisualLine> {
    let max = max.max(1);
    let mut rows = Vec::new();
    let mut line_start = 0;
    for logical in text.split('\n') {
        wrap_logical(logical, line_start, max, measure, &mut rows);
        line_start += logical.len() + 1;
    }
    rows
}

fn wrap_logical(
    line: &str,
    base: usize,
    max: u32,
    m: &dyn TextMeasure,
    rows: &mut Vec<VisualLine>,
) {
    if line.is_empty() {
        rows.push(VisualLine {
            start: base,
            end: base,
        });
        return;
    }
    let mut row_start = 0;
    while row_start < line.len() {
        let rest = &line[row_start..];
        if m.width(rest) <= max {
            rows.push(VisualLine {
                start: base + row_start,
                end: base + line.len(),
            });
            return;
        }
        // Longest prefix that fits.
        // (Binary search: shaped text is costly to measure.)
        let bounds = cluster_ends(rest);
        let (mut lo, mut hi) = (0, bounds.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if m.width(&rest[..bounds[mid]]) > max {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        // Never between a letter and its marks, nor where Thai forbids it.
        let mut k = lo;
        while k > 0 && !may_break(rest, bounds[k - 1]) {
            k -= 1;
        }
        if k == 0 && lo > 0 {
            k = lo;
        }
        let mut fit = if k == 0 { 0 } else { bounds[k - 1] };
        if fit == 0 {
            fit = bounds.first().copied().unwrap_or(rest.len());
        }
        // Prefer to break after the last space inside (or right after) the fit.
        let mut cut = rest[..fit].rfind(' ').map(|i| i + 1);
        if rest[fit..].starts_with(' ') {
            let spaces = rest[fit..].len() - rest[fit..].trim_start_matches(' ').len();
            cut = Some(fit + spaces);
        }
        let cut = cut.filter(|&c| c > 0).unwrap_or(fit);
        rows.push(VisualLine {
            start: base + row_start,
            end: base + row_start + cut,
        });
        row_start += cut;
    }
}

/// Byte offsets just after each grapheme cluster of `s` (just after each
/// char when `s` is ASCII, the common case).
fn cluster_ends(s: &str) -> Vec<usize> {
    use unicode_segmentation::UnicodeSegmentation;
    if s.is_ascii() {
        return (1..=s.len()).collect();
    }
    s.grapheme_indices(true).map(|(i, g)| i + g.len()).collect()
}

/// Thai has no spaces between words: a row may end between letters, but
/// not after a leading vowel (เ แ โ ไ ใ) nor before a following vowel or
/// sign (ะ า ำ ๆ ฯ).
fn may_break(s: &str, at: usize) -> bool {
    let before = s[..at].chars().next_back();
    let after = s[at..].chars().next();
    let thai = |c: Option<char>| c.is_some_and(|c| ('\u{0E00}'..='\u{0E7F}').contains(&c));
    if !thai(before) && !thai(after) {
        return true;
    }
    !matches!(before, Some('เ' | 'แ' | 'โ' | 'ไ' | 'ใ'))
        && !matches!(after, Some('ะ' | 'า' | 'ำ' | 'ๆ' | 'ฯ'))
}

/// Row and column (in measure units) of a caret byte offset.
#[must_use]
pub fn caret_cell(
    text: &str,
    rows: &[VisualLine],
    caret: usize,
    m: &dyn TextMeasure,
) -> (usize, u32) {
    let mut found = 0;
    for (i, r) in rows.iter().enumerate() {
        if caret >= r.start && caret <= r.end {
            found = i;
            // A caret exactly at a soft break belongs to the next row,
            // unless the next row starts a new logical line.
            if caret == r.end && rows.get(i + 1).is_some_and(|n| n.start == r.end) {
                continue;
            }
            break;
        }
    }
    let r = rows[found];
    (found, m.caret_x(&text[r.start..r.end], caret - r.start))
}

/// Byte offset nearest to `col` units into `row` (tap or up/down by row).
#[must_use]
pub fn offset_at(text: &str, row: VisualLine, col: u32, m: &dyn TextMeasure) -> usize {
    let s = &text[row.start..row.end];
    if let Some(i) = m.index_at(s, col) {
        return row.start + i;
    }
    let mut best = row.start;
    for (i, c) in s.char_indices() {
        let after = i + c.len_utf8();
        if m.width(&s[..after]) > col {
            return best;
        }
        best = row.start + after;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str, max: u32) -> Vec<&str> {
        wrap(text, max, &MonoMeasure::default())
            .iter()
            .map(|r| &text[r.start..r.end])
            .collect()
    }

    #[test]
    fn keeps_hard_breaks_and_empty_lines() {
        assert_eq!(rows("a\n\nb", 10), vec!["a", "", "b"]);
        assert_eq!(rows("", 10), vec![""]);
    }

    #[test]
    fn wraps_at_spaces() {
        assert_eq!(
            rows("the quick brown fox", 10),
            vec!["the quick ", "brown fox"]
        );
    }

    #[test]
    fn splits_overlong_words() {
        assert_eq!(rows("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn measured_not_counted() {
        struct Wide;
        impl TextMeasure for Wide {
            fn width(&self, t: &str) -> u32 {
                t.chars().map(|c| if c == 'm' { 3 } else { 1 }).sum()
            }
        }
        let t = "mm aa";
        let r = wrap(t, 6, &Wide);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn caret_positions() {
        let t = "the quick brown fox";
        let m = MonoMeasure::default();
        let r = wrap(t, 10, &m);
        assert_eq!(caret_cell(t, &r, 0, &m), (0, 0));
        assert_eq!(caret_cell(t, &r, 10, &m), (1, 0)); // at soft break → next row
        assert_eq!(caret_cell(t, &r, t.len(), &m), (1, 9));
        let t2 = "ab\ncd";
        let r2 = wrap(t2, 10, &m);
        assert_eq!(caret_cell(t2, &r2, 2, &m), (0, 2)); // end of hard line stays
        assert_eq!(offset_at(t, r[1], 3, &m), 13);
        assert_eq!(offset_at(t, r[1], 99, &m), t.len());
    }

    #[test]
    fn every_byte_is_covered() {
        let t = "lorem ipsum dolor sit amet, consectetur\nadipiscing elit sed do";
        let r = wrap(t, 7, &MonoMeasure::default());
        let joined: String = r
            .iter()
            .map(|x| &t[x.start..x.end])
            .collect::<Vec<_>>()
            .join("");
        assert_eq!(joined, t.replace('\n', ""));
    }
}

#[cfg(test)]
mod cluster_tests {
    use super::*;

    fn rows(text: &str, max: u32) -> Vec<&str> {
        wrap(text, max, &MonoMeasure::default())
            .iter()
            .map(|r| &text[r.start..r.end])
            .collect()
    }

    #[test]
    fn a_row_never_ends_inside_a_letter_and_its_marks() {
        // Each of क्षि and की is one cluster; width 1 per char.
        let t = "कीकीकीकी";
        for r in rows(t, 3) {
            assert!(r.chars().count() % 2 == 0, "{r:?}");
        }
        let hindi = "क्षिक्षिक्षि";
        for r in rows(hindi, 5) {
            assert!(r.chars().count() % 4 == 0, "{r:?}");
        }
    }

    #[test]
    fn thai_breaks_between_syllables_not_after_a_leading_vowel() {
        for r in rows("เกเกเกเกเกเก", 3) {
            assert!(!r.ends_with('เ'), "{r:?}");
        }
        for r in rows("กำกำกำกำ", 3) {
            assert!(!r.starts_with('ำ'), "{r:?}");
        }
    }
}
