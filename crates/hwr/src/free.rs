//! Free writing (v1.5.3, decision 55): ink written anywhere in the open
//! area, no boxes and no baseline. Word mode failed its gate (S15.3), so
//! the free area is read by **splitting the ink into characters at the
//! gaps between them** and reading each with the v1.5.2 network; a wide
//! gap is a space. Size and baseline come from the ink itself.

use crate::{features, Recogniser, Seg, BASELINE};

/// Gap (line units) above which a space is typed.
pub const SPACE_GAP: f32 = 0.38;
/// Pieces closer than this (line units) are one character.
pub const JOIN_GAP: f32 = 0.03;
/// A piece wider than this (line units) is taken for touching letters.
pub const WIDE: f32 = 0.8;

/// What the free area reads: the text and up to 4 alternatives.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reading {
    /// The best reading.
    pub text: String,
    /// Other readings, without duplicates of `text`.
    pub alts: Vec<String>,
    /// For each character of the letter-by-letter reading (`text` before a
    /// lexicon word may replace it): the character read and its ink in box
    /// units, or `None` for a space or a letter of a word read with the
    /// lexicon. What the app learns from when a candidate corrects one
    /// letter ([`learnable`]).
    pub letters: Vec<Option<(char, Vec<Seg>)>>,
}

/// The one letter a correction teaches, if it is clear: `typed` (what the
/// panel typed) and `picked` (the candidate chosen) have the same length
/// and differ at exactly one position, where `typed` has the character
/// read from that ink. Returns the picked character and the ink.
#[must_use]
pub fn learnable<'a>(r: &'a Reading, typed: &str, picked: &str) -> Option<(char, &'a [Seg])> {
    let t: Vec<char> = typed.chars().collect();
    let p: Vec<char> = picked.chars().collect();
    if t.len() != p.len() || t.len() != r.letters.len() {
        return None;
    }
    let mut diff = (0..t.len()).filter(|&i| t[i] != p[i]);
    let i = diff.next()?;
    if diff.next().is_some() {
        return None;
    }
    let (read, ink) = r.letters[i].as_ref()?;
    (*read == t[i] && !p[i].is_whitespace()).then_some((p[i], ink.as_slice()))
}

fn bbox(segs: &[Seg]) -> (f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(a, b) in segs {
        for p in [a, b] {
            x0 = x0.min(p.0);
            y0 = y0.min(p.1);
            x1 = x1.max(p.0);
            y1 = y1.max(p.1);
        }
    }
    (x0, y0, x1, y1)
}

fn median(mut v: Vec<f32>) -> f32 {
    v.sort_by(f32::total_cmp);
    v.get(v.len() / 2).copied().unwrap_or(0.0)
}

/// Groups strokes (each a list of segments) into characters: strokes
/// whose x ranges overlap (or nearly touch) are one character.
#[must_use]
pub fn clusters(strokes: &[Vec<Seg>]) -> Vec<Vec<Seg>> {
    let mut items: Vec<(f32, f32, &Vec<Seg>)> = strokes
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (x0, _, x1, _) = bbox(s);
            (x0, x1, s)
        })
        .collect();
    items.sort_by(|a, b| a.0.total_cmp(&b.0));
    let all: Vec<Seg> = strokes.iter().flatten().copied().collect();
    if all.is_empty() {
        return Vec::new();
    }
    let (_, y0, _, y1) = bbox(&all);
    let tol = (y1 - y0) * 0.02;
    let mut out: Vec<(f32, Vec<Seg>)> = Vec::new();
    for (x0, x1, s) in items {
        match out.last_mut() {
            Some((end, c)) if x0 <= *end + tol => {
                c.extend_from_slice(s);
                *end = end.max(x1);
            }
            _ => out.push((x1, s.clone())),
        }
    }
    out.into_iter().map(|(_, c)| c).collect()
}

/// Line unit and baseline estimated from the characters: most letters
/// sit on the baseline; the median letter is about x-height tall.
fn metrics(cs: &[Vec<Seg>]) -> (f32, f32) {
    let b: Vec<(f32, f32, f32, f32)> = cs.iter().map(|c| bbox(c)).collect();
    let base = median(b.iter().map(|x| x.3).collect());
    let top = median(b.iter().map(|x| x.1).collect());
    let all_top = b.iter().map(|x| x.1).fold(f32::MAX, f32::min);
    let h = (base - top).max((base - all_top) * 0.6).max(1e-3);
    // x-height ≈ 0.4 of the line unit, a cap/ascender ≈ 0.56 (S15.2 boxes).
    let unit = if cs.len() == 1 { h / 0.5 } else { h / 0.45 };
    (unit, base)
}

/// Reads free ink (any units, y down). `allow` masks the characters
/// that may be read (123 key, language key); `None` allows all.
#[must_use]
pub fn read(rec: &Recogniser, strokes: &[Vec<Seg>], allow: Option<&[bool]>) -> Reading {
    read_lex(rec, strokes, allow, None, WIDE)
}

/// [`read`], and a word whose ink has a piece too wide for one letter
/// (`wide`, line units: touching letters) is read with the lexicon (word
/// mode) when given; `wide` 0 reads every word with the lexicon.
#[must_use]
pub fn read_lex(
    rec: &Recogniser,
    strokes: &[Vec<Seg>],
    allow: Option<&[bool]>,
    lex: Option<&crate::word::Lexicon>,
    wide: f32,
) -> Reading {
    let mut cs = clusters(strokes);
    if cs.is_empty() {
        return Reading::default();
    }
    let (unit, base) = metrics(&cs);
    // Join pieces of one character that do not quite overlap.
    let mut merged: Vec<Vec<Seg>> = Vec::new();
    for c in cs.drain(..) {
        if let Some(last) = merged.last_mut() {
            let (_, _, lx1, _) = bbox(last);
            let (x0, _, _, _) = bbox(&c);
            if (x0 - lx1) / unit < JOIN_GAP {
                last.extend(c);
                continue;
            }
        }
        merged.push(c);
    }
    // Words (runs between spaces) with a too-wide piece go to the lexicon.
    let mut word_of = vec![0usize; merged.len()];
    let mut px1: Option<f32> = None;
    let mut wi = 0;
    for (i, c) in merged.iter().enumerate() {
        let (x0, _, x1, _) = bbox(c);
        if px1.is_some_and(|p| (x0 - p) / unit > SPACE_GAP) {
            wi += 1;
        }
        word_of[i] = wi;
        px1 = Some(x1);
    }
    let mut lex_word: Vec<Option<String>> = vec![None; wi + 1];
    let mut lex_more: Vec<(usize, String)> = Vec::new();
    if let Some(lex) = lex {
        for (w, slot) in lex_word.iter_mut().enumerate() {
            let idx: Vec<usize> = (0..merged.len()).filter(|&i| word_of[i] == w).collect();
            let wide = idx.iter().any(|&i| {
                let (x0, _, x1, _) = bbox(&merged[i]);
                (x1 - x0) / unit > wide
            });
            if !wide {
                continue;
            }
            let segs: Vec<Seg> = idx
                .iter()
                .flat_map(|&i| merged[i].iter())
                .map(|&(a, b)| {
                    let m = |p: (f32, f32)| (p.0 / unit, BASELINE + (p.1 - base) / unit);
                    (m(a), m(b))
                })
                .collect();
            let mut got = crate::word::recognise(rec, &segs, lex, 5).into_iter();
            *slot = got.next().map(|x| x.0);
            lex_more.extend(got.map(|x| (w, x.0)));
        }
    }
    let mut chars: Vec<Vec<(char, f32)>> = Vec::new();
    let mut inks: Vec<Vec<Seg>> = Vec::new();
    let mut spaces = Vec::new();
    let mut prev_x1: Option<f32> = None;
    for c in &merged {
        let (x0, _, x1, _) = bbox(c);
        spaces.push(prev_x1.is_some_and(|p| (x0 - p) / unit > SPACE_GAP));
        prev_x1 = Some(x1);
        let cx = (x0 + x1) / 2.0;
        let boxed: Vec<Seg> = c
            .iter()
            .map(|&(a, b)| {
                let m = |p: (f32, f32)| ((p.0 - cx) / unit + 0.5, BASELINE + (p.1 - base) / unit);
                (m(a), m(b))
            })
            .collect();
        let mut p = rec.probs(&features(&boxed));
        inks.push(boxed);
        if let Some(m) = allow {
            for (pi, ok) in p.iter_mut().zip(m) {
                if !ok {
                    *pi = 0.0;
                }
            }
        }
        let mut idx: Vec<usize> = (0..p.len()).collect();
        idx.sort_by(|&a, &b| p[b].total_cmp(&p[a]));
        chars.push(
            idx.iter()
                .take(2)
                .map(|&i| (rec.model.chars[i], p[i]))
                .collect(),
        );
    }
    let build = |swap: Option<usize>, over: Option<(usize, &str)>| -> String {
        let mut s = String::new();
        for (i, c) in chars.iter().enumerate() {
            if spaces[i] {
                s.push(' ');
            }
            if let Some(w) = &lex_word[word_of[i]] {
                // The whole word once, at its first piece.
                if i == 0 || word_of[i - 1] != word_of[i] {
                    match over {
                        Some((ow, t)) if ow == word_of[i] => s.push_str(t),
                        _ => s.push_str(w),
                    }
                }
                continue;
            }
            let k = usize::from(swap == Some(i) && c.len() > 1);
            s.push(c[k].0);
        }
        s
    };
    let text = build(None, None);
    // The same walk as `build(None, None)`, keeping each letter's ink.
    let mut letters = Vec::new();
    for (i, c) in chars.iter().enumerate() {
        if spaces[i] {
            letters.push(None);
        }
        if let Some(w) = &lex_word[word_of[i]] {
            if i == 0 || word_of[i - 1] != word_of[i] {
                letters.extend(w.chars().map(|_| None));
            }
            continue;
        }
        letters.push(Some((c[0].0, inks[i].clone())));
    }
    // Alternatives: the 4 least sure characters, each at its runner-up.
    let mut unsure: Vec<usize> = (0..chars.len()).filter(|&i| chars[i].len() > 1).collect();
    unsure.sort_by(|&a, &b| {
        (chars[a][0].1 - chars[a][1].1).total_cmp(&(chars[b][0].1 - chars[b][1].1))
    });
    let mut alts: Vec<String> = unsure
        .into_iter()
        .take(4)
        .map(|i| build(Some(i), None))
        .collect();
    // Runner-up lexicon words, the last word's first.
    lex_more.sort_by_key(|x| std::cmp::Reverse(x.0));
    alts.extend(lex_more.iter().map(|(w, t)| build(None, Some((*w, t)))));
    let mut text = text;
    // A lexicon word among the alternatives beats a non-word reading.
    if let Some(lex) = lex {
        let known = |t: &str| {
            t.split(' ')
                .all(|w| lex.words.iter().any(|(x, _)| *x == w.to_lowercase()))
        };
        if !known(&text) {
            if let Some(i) = alts.iter().position(|a| known(a)) {
                std::mem::swap(&mut text, &mut alts[i]);
            }
        }
    }
    let mut seen = vec![text.clone()];
    alts.retain(|a| {
        let new = !seen.contains(a);
        seen.push(a.clone());
        new
    });
    alts.truncate(4);
    Reading {
        text,
        alts,
        letters,
    }
}

/// Characters allowed by a language filter (v1.5.3 globe key): `None`
/// all; otherwise ASCII plus that language's accented letters.
#[must_use]
pub fn allow_lang(rec: &Recogniser, lang: Option<&str>, digits: bool) -> Option<Vec<bool>> {
    if lang.is_none() && !digits {
        return None;
    }
    let fr = "àâçéèêëîïôùûüœæÀÂÇÉÈÊËÎÏÔÙÛÜŒÆ«»€";
    let pt = "àâçéêíóôõúãáÀÂÇÉÊÍÓÔÕÚÃÁ€";
    Some(
        rec.model
            .chars
            .iter()
            .map(|&c| {
                if digits {
                    return c.is_ascii_digit() || ".,:;-+=/%*#()€".contains(c);
                }
                c.is_ascii()
                    || match lang {
                        Some("fr") => fr.contains(c),
                        Some("pt") => pt.contains(c),
                        _ => false,
                    }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_a_one_letter_correction_of_read_ink_is_learnable() {
        let ink = vec![((0.4, 0.3), (0.4, 0.7))];
        let r = Reading {
            text: "ab c".into(),
            alts: Vec::new(),
            letters: vec![
                Some(('a', ink.clone())),
                Some(('b', ink.clone())),
                None,
                Some(('c', ink.clone())),
            ],
        };
        assert_eq!(learnable(&r, "ab c", "ab e").map(|x| x.0), Some('e'));
        assert_eq!(learnable(&r, "ab c", "ob c").map(|x| x.0), Some('o'));
        // Two letters, a different length, a space, or text that does not
        // match what was read: nothing.
        assert!(learnable(&r, "ab c", "ob e").is_none());
        assert!(learnable(&r, "ab c", "abc").is_none());
        assert!(learnable(&r, "ab c", "ab  ").is_none());
        assert!(learnable(&r, "xb c", "ab c").is_none());
        // A letter of a lexicon word (no ink of its own): nothing.
        assert!(learnable(&r, "ab c", "abxc").is_none());
    }

    use super::*;
    use crate::Model;

    fn ring(cx: f32, cy: f32, r: f32) -> Vec<Seg> {
        let p: Vec<(f32, f32)> = (0..=24)
            .map(|k| {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                (cx + r * a.cos(), cy + r * 0.95 * a.sin())
            })
            .collect();
        p.windows(2).map(|w| (w[0], w[1])).collect()
    }

    fn bar(x: f32, y0: f32, y1: f32) -> Vec<Seg> {
        vec![
            ((x, y0), (x, (y0 + y1) / 2.0)),
            ((x, (y0 + y1) / 2.0), (x, y1)),
        ]
    }

    #[test]
    fn splits_at_gaps_reads_letters_and_spaces_at_any_size() {
        let r = Recogniser::new(Model::default_model());
        for k in [40.0f32, 120.0] {
            // "lo lo" in pixels: ascender bar, x-height ring, wide gap.
            let base = 300.0;
            let strokes = vec![
                bar(10.0, base - 0.58 * k, base),
                ring(10.0 + 0.4 * k, base - 0.18 * k, 0.18 * k),
                bar(10.0 + 1.5 * k, base - 0.58 * k, base),
                ring(10.0 + 1.9 * k, base - 0.18 * k, 0.18 * k),
            ];
            let got = read(&r, &strokes, None);
            assert_eq!(got.text.chars().filter(|&c| c == ' ').count(), 1, "{got:?}");
            assert_eq!(got.text.chars().count(), 5, "{got:?}");
            assert!(got.alts.len() <= 4);
        }
        let only = allow_lang(&r, None, true).unwrap();
        let got = read(&r, &[ring(50.0, 50.0, 20.0)], Some(&only));
        assert!(got.text.chars().all(|c| !c.is_alphabetic()), "{got:?}");
        assert!(read(&r, &[], None).text.is_empty());
    }
}
