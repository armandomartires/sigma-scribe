//! Synthetic ink and training for the default handwriting model
//! (v1.5.2). Nothing here ships in the device binary.
//!
//! Ink: each glyph of each OFL font in `fonts/` is rasterised into a box
//! (baseline at [`sigma_hwr::BASELINE`]), thinned to a one-pixel skeleton
//! (Zhang–Suen) and turned into segments; every sample then gets a random
//! affine jitter (size, slant, rotation, aspect, offset) and a smooth
//! elastic wobble. Training: plain SGD with momentum on a 406→H→128 net,
//! then int8 quantisation. Everything is seeded, so a run is repeatable.

// Dense maths: index loops read better than iterator chains here.
#![allow(clippy::needless_range_loop)]

use sigma_hwr::{features, Layer, Model, Seg, BASELINE, CHARSET, DIM};
use sigma_render::typeface::FontFile;
use std::path::{Path, PathBuf};

/// Fonts used for training; [`HELD_OUT_FONTS`] are never trained on.
pub const TRAIN_FONTS: &[&str] = &[
    "Caveat.ttf",
    "PatrickHand-Regular.ttf",
    "Kalam-Regular.ttf",
    "IndieFlower-Regular.ttf",
    "ShadowsIntoLight.ttf",
    "ArchitectsDaughter-Regular.ttf",
    "GochiHand-Regular.ttf",
    "ComicNeue-Regular.ttf",
    "Sniglet-Regular.ttf",
    "AtkinsonHyperlegible-Regular.ttf",
];
/// Unseen fonts: the closest stand-in for a new hand.
pub const HELD_OUT_FONTS: &[&str] = &["Handlee-Regular.ttf", "Delius-Regular.ttf"];

/// Seeded `SplitMix64`.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Where the fonts are (this crate's `fonts/`, plus the app's own).
#[must_use]
pub fn font_path(name: &str) -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = here.join("fonts").join(name);
    if p.exists() {
        p
    } else {
        here.join("../../assets/fonts").join(name)
    }
}

/// # Panics
/// If the font is missing or does not parse.
#[must_use]
pub fn load_font(name: &str) -> FontFile {
    let b = std::fs::read(font_path(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    FontFile::from_owned(b, 0).unwrap_or_else(|| panic!("{name}: not a font"))
}

const RES: usize = 96;

/// The skeleton of a glyph in box units, or `None` if the font lacks it.
#[must_use]
pub fn skeleton(font: &FontFile, ch: char) -> Option<Vec<Seg>> {
    if font.glyph(ch) == 0 {
        return None;
    }
    let px = RES as f32 * 0.8;
    let m = font.rasterize_char(ch, px);
    if m.width == 0 || m.height == 0 {
        return None;
    }
    let base = BASELINE * RES as f32;
    let top = (base + m.top as f32).round() as i32;
    let left = ((RES as i32 - m.width as i32) / 2).max(0);
    let pad = 2usize;
    let (w, h) = (m.width + 2 * pad, m.height + 2 * pad);
    let mut on = vec![false; w * h];
    for y in 0..m.height {
        for x in 0..m.width {
            on[(y + pad) * w + x + pad] = m.coverage[y * m.width + x] >= 110;
        }
    }
    thin(&mut on, w, h);
    let mut segs = Vec::new();
    let to_box = |x: usize, y: usize| {
        (
            (left as f32 + x as f32 - pad as f32 + 0.5) / RES as f32,
            (top as f32 + y as f32 - pad as f32 + 0.5) / RES as f32,
        )
    };
    for y in 0..h {
        for x in 0..w {
            if !on[y * w + x] {
                continue;
            }
            let mut any = false;
            for (dx, dy) in [(1i32, 0i32), (1, 1), (0, 1), (-1, 1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0
                    && (nx as usize) < w
                    && (ny as usize) < h
                    && on[ny as usize * w + nx as usize]
                {
                    segs.push((to_box(x, y), to_box(nx as usize, ny as usize)));
                    any = true;
                }
            }
            if !any {
                let p = to_box(x, y);
                segs.push((p, (p.0 + 0.01, p.1)));
            }
        }
    }
    (!segs.is_empty()).then_some(segs)
}

/// Zhang–Suen thinning in place.
fn thin(on: &mut [bool], w: usize, h: usize) {
    loop {
        let mut changed = false;
        for pass in 0..2 {
            let mut kill = Vec::new();
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    if !on[y * w + x] {
                        continue;
                    }
                    let p = |dx: i32, dy: i32| {
                        on[(y as i32 + dy) as usize * w + (x as i32 + dx) as usize]
                    };
                    let n = [
                        p(0, -1),
                        p(1, -1),
                        p(1, 0),
                        p(1, 1),
                        p(0, 1),
                        p(-1, 1),
                        p(-1, 0),
                        p(-1, -1),
                    ];
                    let b = n.iter().filter(|&&v| v).count();
                    if !(2..=6).contains(&b) {
                        continue;
                    }
                    let a = (0..8).filter(|&i| !n[i] && n[(i + 1) % 8]).count();
                    if a != 1 {
                        continue;
                    }
                    let ok = if pass == 0 {
                        !(n[0] && n[2] && n[4]) && !(n[2] && n[4] && n[6])
                    } else {
                        !(n[0] && n[2] && n[6]) && !(n[0] && n[4] && n[6])
                    };
                    if ok {
                        kill.push(y * w + x);
                    }
                }
            }
            changed |= !kill.is_empty();
            for i in kill {
                on[i] = false;
            }
        }
        if !changed {
            break;
        }
    }
}

/// A jittered copy of a skeleton: a writer's size, slant, tilt, aspect and
/// offset, plus a smooth wobble.
#[must_use]
pub fn jitter(segs: &[Seg], rng: &mut Rng) -> Vec<Seg> {
    let s = rng.range(0.7, 1.2);
    let sx = s * rng.range(0.8, 1.2);
    let sy = s * rng.range(0.85, 1.15);
    let rot = rng.range(-0.14, 0.14);
    let shear = rng.range(-0.35, 0.25);
    let (tx, ty) = (rng.range(-0.08, 0.08), rng.range(-0.06, 0.06));
    let (c, sn) = (rot.cos(), rot.sin());
    let mut waves = [(0.0f32, 0.0f32, 0.0f32, 0.0f32); 4];
    for w in &mut waves {
        *w = (
            rng.range(2.0, 7.0),
            rng.range(0.0, 6.3),
            rng.range(0.0, 6.3),
            rng.range(0.0, 0.018),
        );
    }
    let (ax, ay) = (0.5, BASELINE);
    let map = |(x, y): (f32, f32)| {
        let (mut u, v) = (x - ax, y - ay);
        u += -shear * v;
        let (u2, v2) = (u * sx, v * sy);
        let (u3, v3) = (c * u2 - sn * v2, sn * u2 + c * v2);
        let (mut px, mut py) = (u3 + ax + tx, v3 + ay + ty);
        for (i, (f, p1, p2, a)) in waves.iter().enumerate() {
            if i % 2 == 0 {
                px += a * (f * y + p1).sin();
            } else {
                py += a * (f * x + p2).sin();
            }
        }
        (px, py)
    };
    segs.iter().map(|&(a, b)| (map(a), map(b))).collect()
}

/// Labelled feature vectors.
pub type Set = Vec<(Vec<f32>, usize)>;

/// `per` jittered samples of every character in every font (fonts missing
/// a glyph are skipped for it).
#[must_use]
pub fn synth_set(fonts: &[&str], per: usize, seed: u64) -> Set {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for name in fonts {
        let f = load_font(name);
        for (ci, ch) in CHARSET.chars().enumerate() {
            let Some(sk) = skeleton(&f, ch) else {
                continue;
            };
            for _ in 0..per {
                out.push((features(&jitter(&sk, &mut rng)), ci));
            }
        }
    }
    out
}

/// Float network for training.
pub struct Net {
    pub h: usize,
    pub c: usize,
    pub w1: Vec<f32>,
    pub b1: Vec<f32>,
    pub w2: Vec<f32>,
    pub b2: Vec<f32>,
}

impl Net {
    #[must_use]
    pub fn new(h: usize, c: usize, rng: &mut Rng) -> Self {
        let k1 = (2.0 / DIM as f32).sqrt() * 1.7;
        let k2 = (2.0 / h as f32).sqrt() * 1.7;
        Self {
            h,
            c,
            w1: (0..h * DIM).map(|_| rng.range(-k1, k1)).collect(),
            b1: vec![0.0; h],
            w2: (0..c * h).map(|_| rng.range(-k2, k2)).collect(),
            b2: vec![0.0; c],
        }
    }

    /// Trains with SGD + momentum; returns the final epoch's mean loss.
    pub fn train(&mut self, set: &Set, epochs: usize, rng: &mut Rng) -> f32 {
        let (h, c) = (self.h, self.c);
        let mut vw1 = vec![0.0f32; self.w1.len()];
        let mut vb1 = vec![0.0f32; h];
        let mut vw2 = vec![0.0f32; self.w2.len()];
        let mut vb2 = vec![0.0f32; c];
        let mut order: Vec<usize> = (0..set.len()).collect();
        let mut hid = vec![0.0f32; h];
        let mut out = vec![0.0f32; c];
        let mut dh = vec![0.0f32; h];
        let mut loss = 0.0;
        for ep in 0..epochs {
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i + 1));
            }
            let lr = 0.02 * 0.5f32.powf(ep as f32 / (epochs as f32 / 3.0));
            let mom = 0.9;
            loss = 0.0;
            for &i in &order {
                let (x, y) = (&set[i].0, set[i].1);
                for j in 0..h {
                    let row = &self.w1[j * DIM..(j + 1) * DIM];
                    let a: f32 = row.iter().zip(x).map(|(w, v)| w * v).sum::<f32>() + self.b1[j];
                    hid[j] = a.max(0.0);
                }
                for k in 0..c {
                    let row = &self.w2[k * h..(k + 1) * h];
                    out[k] = row.iter().zip(&hid).map(|(w, v)| w * v).sum::<f32>() + self.b2[k];
                }
                sigma_hwr::softmax(&mut out);
                loss -= out[y].max(1e-9).ln();
                out[y] -= 1.0;
                dh.iter_mut().for_each(|d| *d = 0.0);
                for k in 0..c {
                    let g = out[k];
                    if g.abs() < 1e-6 {
                        continue;
                    }
                    let row = &mut self.w2[k * h..(k + 1) * h];
                    let vrow = &mut vw2[k * h..(k + 1) * h];
                    for j in 0..h {
                        dh[j] += g * row[j];
                        vrow[j] = mom * vrow[j] - lr * g * hid[j];
                        row[j] += vrow[j];
                    }
                    vb2[k] = mom * vb2[k] - lr * g;
                    self.b2[k] += vb2[k];
                }
                for j in 0..h {
                    if hid[j] <= 0.0 {
                        continue;
                    }
                    let g = dh[j];
                    let row = &mut self.w1[j * DIM..(j + 1) * DIM];
                    let vrow = &mut vw1[j * DIM..(j + 1) * DIM];
                    for (i2, xv) in x.iter().enumerate() {
                        vrow[i2] = mom * vrow[i2] - lr * g * xv;
                        row[i2] += vrow[i2];
                    }
                    vb1[j] = mom * vb1[j] - lr * g;
                    self.b1[j] += vb1[j];
                }
            }
            loss /= set.len() as f32;
            eprintln!("epoch {ep}: loss {loss:.4} lr {lr:.4}");
        }
        loss
    }

    #[must_use]
    pub fn model(&self) -> Model {
        let mut chars: Vec<char> = CHARSET.chars().collect();
        if self.c > chars.len() {
            chars.push(sigma_hwr::NOT_A_LETTER);
        }
        Model {
            chars,
            hidden: Layer::quantise(self.h, DIM, &self.w1, &self.b1),
            out: Layer::quantise(self.c, self.h, &self.w2, &self.b2),
        }
    }
}

/// Top-1, top-5 and case-folded top-1 accuracy (`o`/`O`, `c`/`C`… count
/// as right when the letter matches; the candidate row fixes case).
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub n: usize,
    pub top1: f32,
    pub top5: f32,
    pub fold1: f32,
}

#[must_use]
pub fn fold(c: char) -> char {
    match c {
        '0' => 'o',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

#[must_use]
pub fn score(r: &sigma_hwr::Recogniser, set: &Set) -> Score {
    let chars: Vec<char> = CHARSET.chars().collect();
    let (mut t1, mut t5, mut f1) = (0usize, 0usize, 0usize);
    for (f, y) in set {
        let top = r.rank(f, 5);
        if top[0].0 == chars[*y] {
            t1 += 1;
        }
        if top.iter().any(|(c, _)| *c == chars[*y]) {
            t5 += 1;
        }
        if fold(top[0].0) == fold(chars[*y]) {
            f1 += 1;
        }
    }
    let n = set.len().max(1) as f32;
    Score {
        n: set.len(),
        top1: t1 as f32 / n,
        top5: t5 as f32 / n,
        fold1: f1 as f32 / n,
    }
}

/// Enrolment stand-in: `per` jittered samples of every character of an
/// unseen font, as [`sigma_hwr::Sample`]s (segments as two-point strokes).
#[must_use]
pub fn enrol_samples(font: &str, per: usize, seed: u64) -> Vec<sigma_hwr::Sample> {
    let mut rng = Rng::new(seed);
    let f = load_font(font);
    let mut out = Vec::new();
    for ch in CHARSET.chars() {
        let Some(sk) = skeleton(&f, ch) else {
            continue;
        };
        for _ in 0..per {
            let strokes = jitter(&sk, &mut rng)
                .into_iter()
                .map(|(a, b)| vec![a, b])
                .collect();
            out.push(sigma_hwr::Sample { ch, strokes });
        }
    }
    out
}

/// Typical ink width of each character of [`CHARSET`] in the training
/// fonts, as (character, mean of ln width, spread), widths in line units.
/// Characters no training font has are left out. Written to
/// `assets/hwr/widths.txt` by `hwr-train widths`; the word decoder's
/// width prior.
#[must_use]
pub fn letter_widths() -> Vec<(char, f32, f32)> {
    let fonts: Vec<FontFile> = TRAIN_FONTS.iter().map(|f| load_font(f)).collect();
    CHARSET
        .chars()
        .filter_map(|c| {
            let ws: Vec<f32> = fonts
                .iter()
                .filter_map(|f| skeleton(f, c))
                .map(|sk| {
                    let (lo, hi) = sk.iter().fold((f32::MAX, f32::MIN), |(lo, hi), (a, b)| {
                        (lo.min(a.0).min(b.0), hi.max(a.0).max(b.0))
                    });
                    (hi - lo).max(0.02).ln()
                })
                .collect();
            if ws.is_empty() {
                return None;
            }
            let m = ws.iter().sum::<f32>() / ws.len() as f32;
            let v = ws.iter().map(|w| (w - m).powi(2)).sum::<f32>() / ws.len() as f32;
            Some((c, m, v.sqrt()))
        })
        .collect()
}

/// A synthetic written word (v1.5.3): each letter's skeleton, given a
/// small jitter of its own, placed after the previous one with a random
/// gap (sometimes touching or overlapping), then one jitter for the
/// whole word. Line units (height 1, baseline at `BASELINE`). `None` if
/// the font lacks a letter.
#[must_use]
pub fn synth_word(font: &FontFile, word: &str, rng: &mut Rng) -> Option<Vec<Seg>> {
    synth_word_labelled(font, word, rng).map(|(segs, _)| segs)
}

/// [`synth_word`] with, for each segment, the position in `word` of the
/// letter it belongs to. Uses `rng` exactly as [`synth_word`] does.
#[must_use]
pub fn synth_word_labelled(
    font: &FontFile,
    word: &str,
    rng: &mut Rng,
) -> Option<(Vec<Seg>, Vec<usize>)> {
    let mut out = Vec::new();
    let mut labels = Vec::new();
    let mut pen = 0.0f32;
    let gap_base = rng.range(-0.01, 0.08);
    for (li, ch) in word.chars().enumerate() {
        let sk = skeleton(font, ch)?;
        let mut r2 = Rng::new(rng.next_u64());
        let sk = mild(&sk, &mut r2);
        let x0 = sk
            .iter()
            .flat_map(|(a, b)| [a.0, b.0])
            .fold(f32::MAX, f32::min);
        let x1 = sk
            .iter()
            .flat_map(|(a, b)| [a.0, b.0])
            .fold(f32::MIN, f32::max);
        let dx = pen - x0;
        out.extend(sk.iter().map(|&(a, b)| ((a.0 + dx, a.1), (b.0 + dx, b.1))));
        labels.resize(out.len(), li);
        pen += (x1 - x0) + gap_base + rng.range(-0.03, 0.05);
    }
    // Whole-word jitter around the word's middle.
    let mid = pen / 2.0;
    let centred: Vec<Seg> = out
        .iter()
        .map(|&(a, b)| ((a.0 - mid + 0.5, a.1), (b.0 - mid + 0.5, b.1)))
        .collect();
    let j = jitter(&centred, rng);
    Some((j, labels))
}

/// A piece counts as one letter when that letter is at least this share
/// of the piece's ink and the piece holds at least this share of the
/// letter's ink.
const PIECE_ONE: f32 = 0.85;
/// A piece counts as "not a letter" when its main letter is below this
/// share of its ink, or it holds below this share of that letter. Pieces
/// in between are left out (neither label is clearly right).
const PIECE_NONE: f32 = 0.6;

/// Pieces of synthetic letter strings, cut as the word decoder cuts
/// words ([`sigma_hwr::word::piece_indices`]): a piece that is one letter
/// is labelled with it (the letter seen with its neighbours' edges), a
/// piece that is part of a letter or several letters with `not_letter`.
/// `per` strings of 2–8 random letters per font (not lexicon words, so
/// the word figures stay unseen); at most `max_none` "not a letter"
/// pieces per font, picked at random.
#[must_use]
pub fn piece_set(fonts: &[&str], per: usize, max_none: usize, seed: u64, not_letter: usize) -> Set {
    use sigma_hwr::word::{cuts, piece_indices};
    let letters: Vec<char> = CHARSET.chars().filter(|c| c.is_alphabetic()).collect();
    let lower: Vec<char> = letters
        .iter()
        .copied()
        .filter(|c| c.is_lowercase())
        .collect();
    let index = |c: char| CHARSET.chars().position(|m| m == c);
    let len = |s: &Seg| ((s.1 .0 - s.0 .0).powi(2) + (s.1 .1 - s.0 .1).powi(2)).sqrt();
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for name in fonts {
        let f = load_font(name);
        let mut none = Vec::new();
        let mut done = 0;
        while done < per {
            let n = 2 + rng.below(7);
            let word: String = (0..n)
                .map(|i| {
                    if i == 0 && rng.below(5) == 0 {
                        letters[rng.below(letters.len())]
                    } else {
                        lower[rng.below(lower.len())]
                    }
                })
                .collect();
            let Some((segs, labels)) = synth_word_labelled(&f, &word, &mut rng) else {
                continue;
            };
            done += 1;
            let chars: Vec<char> = word.chars().collect();
            let mut total = vec![0.0f32; chars.len()];
            for (s, &l) in segs.iter().zip(&labels) {
                total[l] += len(s);
            }
            let cut = cuts(&segs);
            for (a, span, ix) in piece_indices(&segs, &cut) {
                let mut mine = vec![0.0f32; chars.len()];
                for &i in &ix {
                    mine[labels[i]] += len(&segs[i]);
                }
                let ink: f32 = mine.iter().sum();
                let (l, own) = mine
                    .iter()
                    .enumerate()
                    .max_by(|x, y| x.1.total_cmp(y.1))
                    .map_or((0, 0.0), |(l, v)| (l, *v));
                if ink <= 0.0 || total[l] <= 0.0 {
                    continue;
                }
                let (share, cover) = (own / ink, own / total[l]);
                let mid = (cut[a] + cut[a + span]) / 2.0;
                let part: Vec<Seg> = ix
                    .iter()
                    .map(|&i| {
                        let (p, q) = segs[i];
                        ((p.0 - mid + 0.5, p.1), (q.0 - mid + 0.5, q.1))
                    })
                    .collect();
                if share >= PIECE_ONE && cover >= PIECE_ONE {
                    if let Some(ci) = index(chars[l]) {
                        out.push((features(&part), ci));
                    }
                } else if share < PIECE_NONE || cover < PIECE_NONE {
                    none.push(features(&part));
                }
            }
        }
        for _ in 0..max_none.min(none.len()) {
            let i = rng.below(none.len());
            out.push((none.swap_remove(i), not_letter));
        }
    }
    out
}

/// Small per-letter jitter (size ±8 %, slant ±0.08, shift ±0.03).
fn mild(segs: &[Seg], rng: &mut Rng) -> Vec<Seg> {
    let s = rng.range(0.92, 1.08);
    let sh = rng.range(-0.08, 0.08);
    let ty = rng.range(-0.03, 0.03);
    let map = |(x, y): (f32, f32)| {
        let v = y - BASELINE;
        (0.5 + (x - 0.5) * s - sh * v, BASELINE + v * s + ty)
    };
    segs.iter().map(|&(a, b)| (map(a), map(b))).collect()
}

/// Word-mode figures for one font set.
#[derive(Clone, Copy, Debug, Default)]
pub struct WordScore {
    pub words: usize,
    /// Character error rate of the top-1 reading, with the lexicon.
    pub cer: f32,
    /// Word accuracy, top-1 and top-5.
    pub acc1: f32,
    pub acc5: f32,
    /// Character error rate with no lexicon (open vocabulary).
    pub cer_open: f32,
    /// Time per word, mean and p95 (µs, this machine).
    pub mean_us: f64,
    pub p95_us: f64,
}

/// Writes `per` random lexicon words in each font and reads them back.
#[must_use]
pub fn word_score(
    rec: &sigma_hwr::Recogniser,
    fonts: &[&str],
    words: &[String],
    per: usize,
    seed: u64,
) -> WordScore {
    word_score_with(rec, fonts, words, per, seed, sigma_hwr::word::LETTER_BONUS)
}

/// [`word_score`] with a given letter bonus (tuning).
#[must_use]
pub fn word_score_with(
    rec: &sigma_hwr::Recogniser,
    fonts: &[&str],
    words: &[String],
    per: usize,
    seed: u64,
    bonus: f32,
) -> WordScore {
    use sigma_hwr::word::{edit_distance, recognise_with, Lexicon};
    let lex = Lexicon::new(rec, words);
    let empty = Lexicon::default();
    let mut rng = Rng::new(seed);
    let (mut n, mut chars, mut err, mut err_o, mut ok1, mut ok5) = (0, 0, 0, 0, 0, 0);
    let mut times = Vec::new();
    for name in fonts {
        let f = load_font(name);
        let mut done = 0;
        while done < per {
            let (w, _) = &lex.words[rng.below(lex.words.len())];
            let Some(segs) = synth_word(&f, w, &mut rng) else {
                continue;
            };
            done += 1;
            let t = std::time::Instant::now();
            let top = recognise_with(rec, &segs, &lex, 5, bonus);
            times.push(t.elapsed().as_secs_f64() * 1e6);
            let open = recognise_with(rec, &segs, &empty, 1, bonus);
            let got = top.first().map_or(String::new(), |x| x.0.to_lowercase());
            let got_o = open.first().map_or(String::new(), |x| x.0.to_lowercase());
            n += 1;
            chars += w.chars().count();
            err += edit_distance(&got, w);
            err_o += edit_distance(&got_o, w);
            ok1 += usize::from(got == *w);
            ok5 += usize::from(top.iter().any(|x| x.0.to_lowercase() == *w));
        }
    }
    times.sort_by(f64::total_cmp);
    WordScore {
        words: n,
        cer: err as f32 / chars.max(1) as f32,
        acc1: ok1 as f32 / n.max(1) as f32,
        acc5: ok5 as f32 / n.max(1) as f32,
        cer_open: err_o as f32 / chars.max(1) as f32,
        mean_us: times.iter().sum::<f64>() / times.len().max(1) as f64,
        p95_us: times.get(times.len() * 95 / 100).copied().unwrap_or(0.0),
    }
}

/// Synthetic free writing (v1.5.3): 1–3 words, letters printed apart
/// (`letter_gap` range, line units) or touching, a word gap of 0.5–0.9,
/// then a random size (×0.6–1.6, the panel has no guide) and offset.
/// Returns segments in "pixels" (×100).
#[must_use]
pub fn synth_phrase(
    font: &FontFile,
    words: &[&str],
    letter_gap: (f32, f32),
    rng: &mut Rng,
) -> Option<Vec<Seg>> {
    let mut out = Vec::new();
    let mut pen = 0.0f32;
    for (wi, w) in words.iter().enumerate() {
        if wi > 0 {
            pen += rng.range(0.5, 0.9);
        }
        for ch in w.chars() {
            let sk = skeleton(font, ch)?;
            let mut r2 = Rng::new(rng.next_u64());
            let sk = mild(&jitter_small(&sk, &mut r2), &mut r2);
            let x0 = sk
                .iter()
                .flat_map(|(a, b)| [a.0, b.0])
                .fold(f32::MAX, f32::min);
            let x1 = sk
                .iter()
                .flat_map(|(a, b)| [a.0, b.0])
                .fold(f32::MIN, f32::max);
            let dx = pen - x0;
            out.extend(sk.iter().map(|&(a, b)| ((a.0 + dx, a.1), (b.0 + dx, b.1))));
            pen += (x1 - x0) + rng.range(letter_gap.0, letter_gap.1);
        }
    }
    let k = 100.0 * rng.range(0.6, 1.6);
    let (ox, oy) = (rng.range(0.0, 200.0), rng.range(0.0, 150.0));
    Some(
        out.iter()
            .map(|&(a, b)| ((a.0 * k + ox, a.1 * k + oy), (b.0 * k + ox, b.1 * k + oy)))
            .collect(),
    )
}

/// One writer's letters vary less inside a phrase than across writers:
/// slant ±0.15, aspect ±10 %, a little wobble.
fn jitter_small(segs: &[Seg], rng: &mut Rng) -> Vec<Seg> {
    let sh = rng.range(-0.15, 0.1);
    let sx = rng.range(0.9, 1.1);
    let map = |(x, y): (f32, f32)| {
        let v = y - BASELINE;
        (0.5 + (x - 0.5) * sx - sh * v, y)
    };
    segs.iter().map(|&(a, b)| (map(a), map(b))).collect()
}

/// Free-area figures: CER and exact-phrase accuracy of the reading, and
/// whether the right text is among reading + alternatives.
#[derive(Clone, Copy, Debug, Default)]
pub struct FreeScore {
    pub phrases: usize,
    pub cer: f32,
    pub exact: f32,
    pub in5: f32,
    pub mean_us: f64,
    pub p95_us: f64,
}

#[must_use]
pub fn free_score(
    rec: &sigma_hwr::Recogniser,
    fonts: &[&str],
    per: usize,
    letter_gap: (f32, f32),
    seed: u64,
) -> FreeScore {
    use sigma_hwr::word::{builtin_words, edit_distance};
    let words = builtin_words();
    let lex = sigma_hwr::word::Lexicon::new(rec, &words);
    let mut rng = Rng::new(seed);
    let (mut n, mut chars, mut err, mut ex, mut in5) = (0, 0, 0, 0, 0);
    let mut times = Vec::new();
    for name in fonts {
        let f = load_font(name);
        let mut done = 0;
        while done < per {
            let k = 1 + rng.below(3);
            let ws: Vec<&str> = (0..k)
                .map(|_| words[rng.below(words.len())].as_str())
                .collect();
            let Some(segs) = synth_phrase(&f, &ws, letter_gap, &mut rng) else {
                continue;
            };
            done += 1;
            let want = ws.join(" ");
            let strokes: Vec<Vec<Seg>> = segs.iter().map(|&s| vec![s]).collect();
            let t = std::time::Instant::now();
            let got =
                sigma_hwr::free::read_lex(rec, &strokes, None, Some(&lex), sigma_hwr::free::WIDE);
            times.push(t.elapsed().as_secs_f64() * 1e6);
            n += 1;
            chars += want.chars().count();
            err += edit_distance(&got.text, &want);
            ex += usize::from(got.text == want);
            in5 += usize::from(got.text == want || got.alts.contains(&want));
        }
    }
    times.sort_by(f64::total_cmp);
    FreeScore {
        phrases: n,
        cer: err as f32 / chars.max(1) as f32,
        exact: ex as f32 / n.max(1) as f32,
        in5: in5 as f32 / n.max(1) as f32,
        mean_us: times.iter().sum::<f64>() / times.len().max(1) as f64,
        p95_us: times.get(times.len() * 95 / 100).copied().unwrap_or(0.0),
    }
}
