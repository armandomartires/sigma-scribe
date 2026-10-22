//! Word mode trial (v1.5.3, sprint S15.3): a whole word written on one
//! line, read by over-segmentation + the character network + a lexicon.
//!
//! Units: "line units", the line height is 1 and the baseline is at
//! [`crate::BASELINE`], so a character hypothesis moved into a unit box
//! looks like a Letters box.

#![allow(clippy::needless_range_loop)] // DP tables index by span.

use crate::{features, Recogniser, Seg};

/// Longest run of pieces one character may take.
const MAX_SPAN: usize = 6;
/// Widest character hypothesis (line units).
const MAX_W: f32 = 1.1;
const COL: f32 = 0.02;
const MAX_CUTS: usize = 64;
/// No two neighbouring cuts further apart than this (touching letters
/// with no density dip still get cut candidates).
const FILL: f32 = 0.12;

/// The built-in common words (own lists, EN/FR/PT).
#[must_use]
pub fn builtin_words() -> Vec<String> {
    let mut out = Vec::new();
    for t in [
        include_str!("../../../assets/hwr/words-en.txt"),
        include_str!("../../../assets/hwr/words-fr.txt"),
        include_str!("../../../assets/hwr/words-pt.txt"),
    ] {
        for l in t.lines().filter(|l| !l.starts_with('#')) {
            out.extend(l.split_whitespace().map(str::to_owned));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// A lexicon: words as indices into the model's characters (words with
/// a character outside the charset are skipped).
#[derive(Clone, Debug, Default)]
pub struct Lexicon {
    pub words: Vec<(String, Vec<usize>)>,
}

impl Lexicon {
    #[must_use]
    pub fn new(rec: &Recogniser, words: &[String]) -> Self {
        let mut seen = std::collections::HashSet::new();
        let words = words
            .iter()
            .filter(|w| seen.insert(w.to_lowercase()))
            .filter_map(|w| {
                let w = w.to_lowercase();
                let ix: Option<Vec<usize>> = w
                    .chars()
                    .map(|c| rec.model.chars.iter().position(|&m| m == c))
                    .collect();
                Some((w, ix?))
            })
            .filter(|(_, ix)| !ix.is_empty() && ix.len() <= 20)
            .collect();
        Self { words }
    }

    /// Words of the notes (letters only, 2–20 characters), most frequent
    /// first, at most `cap`.
    #[must_use]
    pub fn words_in(text: &str, cap: usize) -> Vec<String> {
        let mut n = std::collections::HashMap::<String, usize>::new();
        for w in text.split(|c: char| !c.is_alphabetic()) {
            let c = w.chars().count();
            if (2..=20).contains(&c) {
                *n.entry(w.to_lowercase()).or_default() += 1;
            }
        }
        let mut v: Vec<(String, usize)> = n.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.into_iter().take(cap).map(|(w, _)| w).collect()
    }
}

/// Cut positions along x: gaps and ink-density minima, sorted, first and
/// last at the ink's ends.
#[must_use]
pub fn cuts(segs: &[Seg]) -> Vec<f32> {
    let mut xs = Vec::new();
    for &(a, b) in segs {
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let n = (len / 0.01).ceil().max(1.0) as usize;
        for i in 0..=n {
            xs.push(a.0 + (b.0 - a.0) * i as f32 / n as f32);
        }
    }
    if xs.is_empty() {
        return Vec::new();
    }
    let x0 = xs.iter().copied().fold(f32::MAX, f32::min);
    let x1 = xs.iter().copied().fold(f32::MIN, f32::max);
    let ncol = (((x1 - x0) / COL).ceil() as usize).max(1) + 1;
    let mut h = vec![0f32; ncol];
    for x in &xs {
        h[(((x - x0) / COL) as usize).min(ncol - 1)] += 1.0;
    }
    let sm: Vec<f32> = (0..ncol)
        .map(|i| {
            let lo = i.saturating_sub(1);
            let hi = (i + 1).min(ncol - 1);
            (lo..=hi).map(|j| h[j]).sum::<f32>() / (hi - lo + 1) as f32
        })
        .collect();
    let mut nz: Vec<f32> = sm.iter().copied().filter(|&v| v > 0.0).collect();
    nz.sort_by(f32::total_cmp);
    let med = nz.get(nz.len() / 2).copied().unwrap_or(1.0);
    // (position, strength: lower is a better cut)
    let mut cand: Vec<(f32, f32)> = Vec::new();
    let mut i = 0;
    while i < ncol {
        if h[i] == 0.0 {
            let s = i;
            while i < ncol && h[i] == 0.0 {
                i += 1;
            }
            cand.push((x0 + (s + i) as f32 / 2.0 * COL, -1.0));
            continue;
        }
        let l = if i > 0 { sm[i - 1] } else { f32::MAX };
        let r = if i + 1 < ncol { sm[i + 1] } else { f32::MAX };
        if sm[i] <= l && sm[i] <= r && sm[i] < med * 0.8 {
            cand.push((x0 + (i as f32 + 0.5) * COL, sm[i] / med));
        }
        i += 1;
    }
    cand.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut out = vec![x0 - 0.001, x1 + 0.001];
    for (x, _) in cand {
        if out.len() >= MAX_CUTS {
            break;
        }
        if out.iter().all(|&o| (o - x).abs() >= 0.05) {
            out.push(x);
        }
    }
    out.sort_by(f32::total_cmp);
    let mut filled = Vec::with_capacity(out.len() * 2);
    for w in out.windows(2) {
        filled.push(w[0]);
        let n = ((w[1] - w[0]) / FILL).floor() as usize;
        for i in 1..=n {
            let x = w[0] + (w[1] - w[0]) * i as f32 / (n + 1) as f32;
            if filled.len() < MAX_CUTS * 2 {
                filled.push(x);
            }
        }
    }
    filled.push(*out.last().unwrap_or(&x1));
    filled
}

/// Log-probabilities of every hypothesis `[k0, k1)` (k1 - k0 ≤ MAX_SPAN),
/// `None` when too wide or empty.
type Hyp = Vec<Vec<Option<Vec<f32>>>>;

fn hypotheses(rec: &Recogniser, segs: &[Seg], cuts: &[f32]) -> Hyp {
    let k = cuts.len();
    let mut out: Hyp = vec![vec![None; MAX_SPAN + 1]; k];
    for a in 0..k {
        for span in 1..=MAX_SPAN {
            let b = a + span;
            if b >= k {
                break;
            }
            let (xa, xb) = (cuts[a], cuts[b]);
            if xb - xa > MAX_W {
                break;
            }
            let mid = (xa + xb) / 2.0;
            let part: Vec<Seg> = segs
                .iter()
                .filter(|(p, q)| {
                    let m = (p.0 + q.0) / 2.0;
                    m >= xa && m < xb
                })
                .map(|&(p, q)| ((p.0 - mid + 0.5, p.1), (q.0 - mid + 0.5, q.1)))
                .collect();
            if part.is_empty() {
                continue;
            }
            let p = rec.probs(&features(&part));
            out[a][span] = Some(p.iter().map(|v| v.max(1e-6).ln()).collect());
        }
    }
    out
}

/// The `n` best readings of a word, best first, with a score (total
/// log-probability + a bonus per letter; higher is better). Lexicon
/// words first; the free (open-vocabulary) reading last.
#[must_use]
pub fn recognise(rec: &Recogniser, segs: &[Seg], lex: &Lexicon, n: usize) -> Vec<(String, f32)> {
    recognise_with(rec, segs, lex, n, LETTER_BONUS)
}

/// [`recognise`] with a given per-letter bonus (tuning).
#[must_use]
pub fn recognise_with(
    rec: &Recogniser,
    segs: &[Seg],
    lex: &Lexicon,
    n: usize,
    bonus: f32,
) -> Vec<(String, f32)> {
    let cuts = cuts(segs);
    if cuts.len() < 2 {
        return Vec::new();
    }
    let hyp = hypotheses(rec, segs, &cuts);
    let k = cuts.len() - 1; // last cut index
    let upper: Vec<Option<usize>> = rec
        .model
        .chars
        .iter()
        .map(|c| {
            let u = c.to_uppercase().next()?;
            rec.model.chars.iter().position(|&m| m == u && m != *c)
        })
        .collect();
    let mut scored: Vec<(String, f32)> = Vec::new();
    let mut dp = Vec::new();
    for (w, ix) in &lex.words {
        let l = ix.len();
        if l > k || l * MAX_SPAN < k {
            continue;
        }
        // dp[j][c]: best total for the first j letters ending at cut c.
        dp.clear();
        dp.resize((l + 1) * (k + 1), f32::MIN);
        dp[0] = 0.0;
        let mut cap = false;
        for j in 1..=l {
            for c in j..=k {
                let mut best = f32::MIN;
                for span in 1..=MAX_SPAN.min(c) {
                    let a = c - span;
                    let prev = dp[(j - 1) * (k + 1) + a];
                    if prev == f32::MIN {
                        continue;
                    }
                    if let Some(lp) = &hyp[a][span] {
                        let mut v = lp[ix[j - 1]];
                        if j == 1 {
                            if let Some(u) = upper[ix[0]] {
                                if lp[u] > v + CAP_MARGIN && a == 0 {
                                    v = lp[u];
                                    cap = true;
                                }
                            }
                        }
                        best = best.max(prev + v);
                    }
                }
                dp[j * (k + 1) + c] = best;
            }
        }
        let tot = dp[l * (k + 1) + k];
        if tot > f32::MIN {
            let mut s = w.clone();
            if cap {
                let mut cs = s.chars();
                if let Some(f) = cs.next() {
                    s = f.to_uppercase().chain(cs).collect();
                }
            }
            scored.push((s, tot + bonus * l as f32));
        }
    }
    // Open vocabulary: best character per hypothesis, Viterbi over cuts.
    let mut best = vec![(f32::MIN, 0usize, 0usize, 0usize); k + 1]; // (total, len, from, char)
    best[0] = (0.0, 0, 0, 0);
    for c in 1..=k {
        for span in 1..=MAX_SPAN.min(c) {
            let a = c - span;
            if best[a].0 == f32::MIN {
                continue;
            }
            if let Some(lp) = &hyp[a][span] {
                let (ci, v) = lp
                    .iter()
                    .enumerate()
                    .max_by(|x, y| x.1.total_cmp(y.1))
                    .map(|(i, v)| (i, *v))
                    .unwrap_or((0, f32::MIN));
                let t = best[a].0 + v + OPEN_BONUS;
                if t > best[c].0 {
                    best[c] = (t, best[a].1 + 1, a, ci);
                }
            }
        }
    }
    let mut open_r = None;
    if best[k].0 > f32::MIN {
        let mut s = Vec::new();
        let mut c = k;
        while c > 0 {
            s.push(rec.model.chars[best[c].3]);
            c = best[c].2;
        }
        s.reverse();
        let open: String = s.into_iter().collect();
        // The free reading only follows the lexicon words (or stands
        // alone when the lexicon has nothing that fits).
        if !scored
            .iter()
            .any(|(w, _)| w.to_lowercase() == open.to_lowercase())
        {
            open_r = Some(open);
        }
    }
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(n.saturating_sub(1).max(1));
    if let Some(o) = open_r {
        if scored.len() < n {
            scored.push((o, f32::MIN));
        }
    }
    scored.truncate(n);
    scored
}

/// Added per letter to a lexicon word's total log-probability, so longer
/// readings that explain the ink letter by letter are not out-scored by
/// short words of a few wide hypotheses (tuned on the training fonts).
pub const LETTER_BONUS: f32 = 8.0;
/// Letter bonus of the free reading (lower: it has no lexicon to keep it
/// from cutting the ink into many small letters).
pub const OPEN_BONUS: f32 = 0.0;
/// A capital first letter needs this much more log-probability (≈ 3×).
pub const CAP_MARGIN: f32 = 1.1;

/// Levenshtein distance in characters.
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Model;

    #[test]
    fn builtin_lexicon_and_edit_distance() {
        let w = builtin_words();
        assert!(w.len() > 500, "{}", w.len());
        assert!(w.iter().any(|x| x == "coração") && w.iter().any(|x| x == "fenêtre"));
        let r = Recogniser::new(Model::default_model());
        let l = Lexicon::new(&r, &w);
        assert!(l.words.len() > 500);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(
            Lexicon::words_in("Le chat, le chien; a b le", 2),
            vec!["le".to_string(), "chat".to_string()]
        );
    }

    #[test]
    fn cuts_split_at_gaps() {
        let segs = vec![((0.0, 0.4), (0.1, 0.7)), ((0.5, 0.4), (0.6, 0.7))];
        let c = cuts(&segs);
        assert!(c.len() >= 3);
        assert!(c.iter().any(|&x| x > 0.15 && x < 0.45), "{c:?}");
    }
}
