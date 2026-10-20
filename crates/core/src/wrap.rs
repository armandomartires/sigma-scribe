//! Soft wrapping by measured width.
//!
//! The brief insists on measured text ("do not guess line length"). The
//! measure is a trait so the device can supply real glyph advances while
//! tests and the monospace Cobalt grid use [`MonoMeasure`].

/// Width of a run of text in the front end's unit (pixels or grid cells).
pub trait TextMeasure {
    fn width(&self, text: &str) -> u32;
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
        let mut fit = 0;
        for (i, c) in rest.char_indices() {
            let next = i + c.len_utf8();
            if m.width(&rest[..next]) > max {
                break;
            }
            fit = next;
        }
        if fit == 0 {
            fit = rest.chars().next().map_or(rest.len(), char::len_utf8);
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
    (found, m.width(&text[r.start..caret]))
}

/// Byte offset nearest to `col` units into `row` (tap or up/down by row).
#[must_use]
pub fn offset_at(text: &str, row: VisualLine, col: u32, m: &dyn TextMeasure) -> usize {
    let s = &text[row.start..row.end];
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
