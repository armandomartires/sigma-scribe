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
pub fn load_font(name: &str) -> fontdue::Font {
    let b = std::fs::read(font_path(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    fontdue::Font::from_bytes(b, fontdue::FontSettings::default())
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

const RES: usize = 96;

/// The skeleton of a glyph in box units, or `None` if the font lacks it.
#[must_use]
pub fn skeleton(font: &fontdue::Font, ch: char) -> Option<Vec<Seg>> {
    if font.lookup_glyph_index(ch) == 0 {
        return None;
    }
    let px = RES as f32 * 0.8;
    let (m, bmp) = font.rasterize(ch, px);
    if m.width == 0 || m.height == 0 {
        return None;
    }
    let base = BASELINE * RES as f32;
    let top = (base - (m.ymin as f32 + m.height as f32)).round() as i32;
    let left = ((RES as i32 - m.width as i32) / 2).max(0);
    let pad = 2usize;
    let (w, h) = (m.width + 2 * pad, m.height + 2 * pad);
    let mut on = vec![false; w * h];
    for y in 0..m.height {
        for x in 0..m.width {
            on[(y + pad) * w + x + pad] = bmp[y * m.width + x] >= 110;
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
        Model {
            chars: CHARSET.chars().collect(),
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
