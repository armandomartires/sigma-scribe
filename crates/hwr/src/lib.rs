//! Per-character box recognition (v1.5.2, plan §12.5, decision 53).
//!
//! Offline, no dependencies, integer-light: a character written in a
//! box becomes a fixed 406-value feature vector (direction histograms of
//! the ink on two grids: one in box coordinates, which keeps size and
//! position so `.`/`,`/`'` and `o`/`O` differ, and one in the ink's own
//! bounding square, which keeps shape), and a small two-layer network
//! with int8 weights scores the 126 characters of [`CHARSET`].
//!
//! The default network is trained only on synthetic ink (glyph skeletons
//! from OFL fonts with random jitter, `tools/hwr-train`). The user's own
//! enrolled samples ([`Sample`]) are matched by nearest neighbour and
//! blended in; they never leave the device.

use std::fmt::Write as _;

/// EN/FR/PT letters (with the accents those languages use), digits and
/// common punctuation, in model order.
pub const CHARSET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\
àâçéèêëîïôùûüœæáãõóíúÀÂÇÉÈÊËÎÏÔÙÛÜŒÆÁÃÕÓÍÚ.,;:!?'\"-()/@&%+=*#€«»";

/// The guide line in a box, as a fraction of its height from the top.
pub const BASELINE: f32 = 0.7;

const G_BOX: usize = 6;
const G_SHAPE: usize = 8;
const DIRS: usize = 4;
const N_BOX: usize = G_BOX * G_BOX * DIRS;
const N_SHAPE: usize = G_SHAPE * G_SHAPE * DIRS;
const N_GLOBAL: usize = 6;
/// Feature vector length.
pub const DIM: usize = N_BOX + N_SHAPE + N_GLOBAL;

/// A line of ink in box units: the box is the unit square (x right, y
/// down), the baseline at `y = BASELINE`.
pub type Seg = ((f32, f32), (f32, f32));

/// Segments of polylines (strokes) in box units; a one-point stroke is a
/// dot (a tiny segment).
#[must_use]
pub fn segments(strokes: &[Vec<(f32, f32)>]) -> Vec<Seg> {
    let mut out = Vec::new();
    for s in strokes {
        match s.len() {
            0 => {}
            1 => out.push((s[0], (s[0].0 + 0.01, s[0].1))),
            _ => out.extend(s.windows(2).map(|w| (w[0], w[1]))),
        }
    }
    out
}

fn deposit(v: &mut [f32], g: usize, x: f32, y: f32, ang: f32, w: f32) {
    // Orientation (undirected): soft between the two nearest of 4 bins.
    let b = ang / (std::f32::consts::PI / DIRS as f32);
    let b0 = b.floor();
    let fb = b - b0;
    let b0 = (b0 as i32).rem_euclid(DIRS as i32) as usize;
    let b1 = (b0 + 1) % DIRS;
    let u = (x.clamp(0.0, 1.0) * g as f32 - 0.5).clamp(0.0, (g - 1) as f32);
    let t = (y.clamp(0.0, 1.0) * g as f32 - 0.5).clamp(0.0, (g - 1) as f32);
    let (u0, t0) = (u.floor() as usize, t.floor() as usize);
    let (fu, ft) = (u - u0 as f32, t - t0 as f32);
    let (u1, t1) = ((u0 + 1).min(g - 1), (t0 + 1).min(g - 1));
    for (cy, wy) in [(t0, 1.0 - ft), (t1, ft)] {
        for (cx, wx) in [(u0, 1.0 - fu), (u1, fu)] {
            let cell = (cy * g + cx) * DIRS;
            v[cell + b0] += w * wy * wx * (1.0 - fb);
            v[cell + b1] += w * wy * wx * fb;
        }
    }
}

fn finish(block: &mut [f32]) {
    let mut n = 0.0;
    for x in block.iter_mut() {
        *x = x.sqrt();
        n += *x * *x;
    }
    if n > 0.0 {
        let k = 1.0 / n.sqrt();
        for x in block.iter_mut() {
            *x *= k;
        }
    }
}

/// The feature vector of some ink (box units). Empty ink gives zeros.
#[must_use]
pub fn features(segs: &[Seg]) -> Vec<f32> {
    let mut v = vec![0.0f32; DIM];
    if segs.is_empty() {
        return v;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(a, b) in segs {
        for p in [a, b] {
            x0 = x0.min(p.0);
            y0 = y0.min(p.1);
            x1 = x1.max(p.0);
            y1 = y1.max(p.1);
        }
    }
    let side = (x1 - x0).max(y1 - y0).max(0.15);
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (bx, rest) = v.split_at_mut(N_BOX);
    let (sh, gl) = rest.split_at_mut(N_SHAPE);
    for &(a, b) in segs {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(0.005);
        let ang = dy.atan2(dx).rem_euclid(std::f32::consts::PI);
        let n = (len / 0.02).ceil().max(1.0) as usize;
        let w = len / n as f32;
        for i in 0..n {
            let t = (i as f32 + 0.5) / n as f32;
            let (x, y) = (a.0 + dx * t, a.1 + dy * t);
            deposit(bx, G_BOX, x, y, ang, w);
            let (sx, sy) = ((x - cx) / side + 0.5, (y - cy) / side + 0.5);
            deposit(sh, G_SHAPE, sx, sy, ang, w);
        }
    }
    finish(bx);
    finish(sh);
    gl.copy_from_slice(&[x0, y0, x1, y1, x1 - x0, y1 - y0]);
    for g in gl.iter_mut() {
        *g = g.clamp(-0.2, 1.2);
    }
    v
}

/// One int8 layer: `out[r] = scale[r] * Σ w[r][i] * x[i] + bias[r]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub rows: usize,
    pub cols: usize,
    pub scale: Vec<f32>,
    pub bias: Vec<f32>,
    pub w: Vec<i8>,
}

impl Layer {
    /// Quantises float weights (row-major) to int8 with a scale per row.
    #[must_use]
    pub fn quantise(rows: usize, cols: usize, w: &[f32], bias: &[f32]) -> Self {
        let mut q = Vec::with_capacity(rows * cols);
        let mut scale = Vec::with_capacity(rows);
        for r in 0..rows {
            let row = &w[r * cols..(r + 1) * cols];
            let m = row.iter().fold(0.0f32, |m, x| m.max(x.abs())).max(1e-8);
            let s = m / 127.0;
            scale.push(s);
            q.extend(
                row.iter()
                    .map(|x| (x / s).round().clamp(-127.0, 127.0) as i8),
            );
        }
        Self {
            rows,
            cols,
            scale,
            bias: bias.to_vec(),
            w: q,
        }
    }

    fn run(&self, x: &[f32], out: &mut Vec<f32>) {
        out.clear();
        for r in 0..self.rows {
            let row = &self.w[r * self.cols..(r + 1) * self.cols];
            let mut acc = 0.0f32;
            for (w, x) in row.iter().zip(x) {
                acc += f32::from(*w) * x;
            }
            out.push(acc * self.scale[r] + self.bias[r]);
        }
    }
}

/// The default (or any) network.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub chars: Vec<char>,
    pub hidden: Layer,
    pub out: Layer,
}

const MAGIC: &[u8; 4] = b"SGHW";
const VERSION: u16 = 1;

fn take<'a>(b: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
    if b.len() < n {
        return None;
    }
    let (h, t) = b.split_at(n);
    *b = t;
    Some(h)
}

fn u16_at(b: &mut &[u8]) -> Option<u16> {
    take(b, 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn f32s(b: &mut &[u8], n: usize) -> Option<Vec<f32>> {
    let s = take(b, 4 * n)?;
    Some(
        s.chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    )
}

impl Layer {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend((self.rows as u16).to_le_bytes());
        out.extend((self.cols as u16).to_le_bytes());
        for s in self.scale.iter().chain(&self.bias) {
            out.extend(s.to_le_bytes());
        }
        out.extend(self.w.iter().map(|w| w.to_le_bytes()[0]));
    }

    fn read(b: &mut &[u8]) -> Option<Self> {
        let rows = usize::from(u16_at(b)?);
        let cols = usize::from(u16_at(b)?);
        let scale = f32s(b, rows)?;
        let bias = f32s(b, rows)?;
        let w = take(b, rows * cols)?
            .iter()
            .map(|&x| i8::from_le_bytes([x]))
            .collect();
        Some(Self {
            rows,
            cols,
            scale,
            bias,
            w,
        })
    }
}

impl Model {
    /// The file format: `SGHW`, version, charset (UTF-8, length-prefixed),
    /// then the two layers (rows, cols, scales, biases, int8 weights).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend(VERSION.to_le_bytes());
        let cs: String = self.chars.iter().collect();
        out.extend((cs.len() as u16).to_le_bytes());
        out.extend(cs.as_bytes());
        self.hidden.write(&mut out);
        self.out.write(&mut out);
        out
    }

    #[must_use]
    pub fn from_bytes(mut b: &[u8]) -> Option<Self> {
        if take(&mut b, 4)? != MAGIC || u16_at(&mut b)? != VERSION {
            return None;
        }
        let n = usize::from(u16_at(&mut b)?);
        let chars = std::str::from_utf8(take(&mut b, n)?)
            .ok()?
            .chars()
            .collect();
        let hidden = Layer::read(&mut b)?;
        let out = Layer::read(&mut b)?;
        let m = Self { chars, hidden, out };
        (m.hidden.cols == DIM && m.out.cols == m.hidden.rows && m.out.rows == m.chars.len())
            .then_some(m)
    }

    /// The model shipped in the binary (`assets/hwr/default.hwr`).
    #[must_use]
    pub fn default_model() -> Self {
        Self::from_bytes(include_bytes!("../../../assets/hwr/default.hwr"))
            .expect("bundled handwriting model")
    }

    /// Softmax probabilities over `chars`.
    #[must_use]
    pub fn probs(&self, f: &[f32]) -> Vec<f32> {
        let mut h = Vec::with_capacity(self.hidden.rows);
        self.hidden.run(f, &mut h);
        for x in &mut h {
            *x = x.max(0.0);
        }
        let mut o = Vec::with_capacity(self.out.rows);
        self.out.run(&h, &mut o);
        softmax(&mut o);
        o
    }
}

/// In-place softmax.
pub fn softmax(o: &mut [f32]) {
    let m = o.iter().copied().fold(f32::MIN, f32::max);
    let mut s = 0.0;
    for x in o.iter_mut() {
        *x = (*x - m).exp();
        s += *x;
    }
    for x in o.iter_mut() {
        *x /= s;
    }
}

/// One enrolled sample of the user's handwriting: the character and its
/// strokes in box units.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub ch: char,
    pub strokes: Vec<Vec<(f32, f32)>>,
}

/// Weight of the user's own samples against the network (tuned on
/// held-out fonts standing in for a user, `tools/hwr-train`).
pub const USER_WEIGHT: f32 = 1.5;
/// Feature distance scale for user samples.
pub const USER_SIGMA: f32 = 0.45;
/// Passes of output-layer fine-tuning over the user's samples.
pub const ADAPT_EPOCHS: usize = 20;

/// The recogniser: the network plus the user's samples.
#[derive(Clone, Debug)]
pub struct Recogniser {
    pub model: Model,
    user: Vec<(usize, Vec<f32>)>,
    /// [`USER_WEIGHT`] and [`USER_SIGMA`] (fields so the trainer can tune).
    pub user_weight: f32,
    pub user_sigma: f32,
    /// Output layer fine-tuned on the user's samples (float), if any.
    adapted: Option<(Vec<f32>, Vec<f32>)>,
    /// Fine-tuning passes over the samples (0 turns it off).
    pub adapt_epochs: usize,
}

impl Recogniser {
    #[must_use]
    pub fn new(model: Model) -> Self {
        Self {
            model,
            user: Vec::new(),
            user_weight: USER_WEIGHT,
            user_sigma: USER_SIGMA,
            adapted: None,
            adapt_epochs: ADAPT_EPOCHS,
        }
    }

    /// Replaces the user samples (characters outside the charset are
    /// ignored); returns how many were taken.
    pub fn set_samples(&mut self, samples: &[Sample]) -> usize {
        self.user = samples
            .iter()
            .filter_map(|s| {
                let i = self.model.chars.iter().position(|&c| c == s.ch)?;
                Some((i, features(&segments(&s.strokes))))
            })
            .collect();
        self.adapt();
        self.user.len()
    }

    /// Fine-tunes a float copy of the output layer on the user's samples
    /// (hidden layer fixed), pulled back towards the default weights so a
    /// few samples cannot wreck the other characters.
    fn adapt(&mut self) {
        self.adapted = None;
        if self.user.is_empty() || self.adapt_epochs == 0 {
            return;
        }
        let o = &self.model.out;
        let mut w: Vec<f32> = (0..o.rows * o.cols)
            .map(|i| f32::from(o.w[i]) * o.scale[i / o.cols])
            .collect();
        let w0 = w.clone();
        let mut b = o.bias.clone();
        let hs: Vec<(usize, Vec<f32>)> = self
            .user
            .iter()
            .map(|(c, f)| {
                let mut h = Vec::new();
                self.model.hidden.run(f, &mut h);
                h.iter_mut().for_each(|x| *x = x.max(0.0));
                (*c, h)
            })
            .collect();
        let (rows, cols) = (o.rows, o.cols);
        let mut out = vec![0.0f32; rows];
        for ep in 0..self.adapt_epochs {
            let lr = 0.01 / (1.0 + ep as f32 * 0.1);
            for (c, h) in &hs {
                for r in 0..rows {
                    let row = &w[r * cols..(r + 1) * cols];
                    out[r] = row.iter().zip(h).map(|(a, x)| a * x).sum::<f32>() + b[r];
                }
                softmax(&mut out);
                out[*c] -= 1.0;
                for r in 0..rows {
                    let g = out[r];
                    let row = &mut w[r * cols..(r + 1) * cols];
                    let row0 = &w0[r * cols..(r + 1) * cols];
                    for j in 0..cols {
                        row[j] -= lr * (g * h[j] + 0.01 * (row[j] - row0[j]));
                    }
                    b[r] -= lr * g;
                }
            }
        }
        self.adapted = Some((w, b));
    }

    #[must_use]
    pub fn user_samples(&self) -> usize {
        self.user.len()
    }

    /// The `n` best characters for some ink, best first, with scores.
    #[must_use]
    pub fn recognise(&self, strokes: &[Vec<(f32, f32)>], n: usize) -> Vec<(char, f32)> {
        let segs = segments(strokes);
        if segs.is_empty() {
            return Vec::new();
        }
        self.rank(&features(&segs), n)
    }

    /// Ranking from a feature vector.
    #[must_use]
    pub fn rank(&self, f: &[f32], n: usize) -> Vec<(char, f32)> {
        let mut p = match &self.adapted {
            None => self.model.probs(f),
            Some((w, b)) => {
                let mut h = Vec::new();
                self.model.hidden.run(f, &mut h);
                let cols = h.len();
                let mut o: Vec<f32> = b
                    .iter()
                    .enumerate()
                    .map(|(r, bias)| {
                        w[r * cols..(r + 1) * cols]
                            .iter()
                            .zip(&h)
                            .map(|(a, x)| a * x.max(0.0))
                            .sum::<f32>()
                            + bias
                    })
                    .collect();
                softmax(&mut o);
                o
            }
        };
        if !self.user.is_empty() && self.user_weight > 0.0 {
            let mut best = vec![f32::MAX; p.len()];
            for (c, u) in &self.user {
                let d: f32 = u.iter().zip(f).map(|(a, b)| (a - b) * (a - b)).sum();
                best[*c] = best[*c].min(d);
            }
            let s2 = self.user_sigma * self.user_sigma;
            for (pi, d) in p.iter_mut().zip(&best) {
                if *d < f32::MAX {
                    *pi += self.user_weight * (-d / s2).exp();
                }
            }
            let s: f32 = p.iter().sum();
            for x in &mut p {
                *x /= s;
            }
        }
        let mut idx: Vec<usize> = (0..p.len()).collect();
        idx.sort_by(|&a, &b| p[b].total_cmp(&p[a]));
        idx.into_iter()
            .take(n)
            .map(|i| (self.model.chars[i], p[i]))
            .collect()
    }
}

/// Samples file (`notes/.handwriting/samples.txt`): one sample per line,
/// `<char>\t<stroke>;<stroke>…`, a stroke `x,y x,y …` in box units with
/// 3 decimals. Plain text so the user can inspect or delete it.
#[must_use]
pub fn samples_to_text(samples: &[Sample]) -> String {
    let mut s = String::from("# sigma-scribe handwriting samples v1 (box units)\n");
    for smp in samples {
        let _ = write!(s, "{}\t", if smp.ch == '\t' { ' ' } else { smp.ch });
        for (i, st) in smp.strokes.iter().enumerate() {
            if i > 0 {
                s.push(';');
            }
            for (j, (x, y)) in st.iter().enumerate() {
                if j > 0 {
                    s.push(' ');
                }
                let _ = write!(s, "{x:.3},{y:.3}");
            }
        }
        s.push('\n');
    }
    s
}

/// Parses [`samples_to_text`]; bad lines are skipped.
#[must_use]
pub fn samples_from_text(text: &str) -> Vec<Sample> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let (c, rest) = l.split_once('\t')?;
            let mut cs = c.chars();
            let ch = cs.next()?;
            if cs.next().is_some() {
                return None;
            }
            let strokes: Option<Vec<Vec<(f32, f32)>>> = rest
                .split(';')
                .map(|st| {
                    st.split_whitespace()
                        .map(|p| {
                            let (x, y) = p.split_once(',')?;
                            Some((x.parse().ok()?, y.parse().ok()?))
                        })
                        .collect()
                })
                .collect();
            let strokes = strokes?;
            (!strokes.iter().all(Vec::is_empty)).then_some(Sample { ch, strokes })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charset_has_126_distinct_characters() {
        let v: Vec<char> = CHARSET.chars().collect();
        let mut d = v.clone();
        d.sort_unstable();
        d.dedup();
        assert_eq!(v.len(), 126);
        assert_eq!(d.len(), 126);
    }

    #[test]
    fn features_keep_position_and_shape() {
        let dot = features(&segments(&[vec![(0.5, 0.68)]]));
        let tick = features(&segments(&[vec![(0.5, 0.1)]]));
        assert_eq!(dot.len(), DIM);
        let d: f32 = dot.iter().zip(&tick).map(|(a, b)| (a - b).powi(2)).sum();
        assert!(d > 0.5, "{d}");
        assert!(features(&[]).iter().all(|&x| x == 0.0));
    }

    #[test]
    fn bundled_model_loads_and_round_trips() {
        let m = Model::default_model();
        assert_eq!(m.chars, CHARSET.chars().collect::<Vec<_>>());
        assert_eq!(Model::from_bytes(&m.to_bytes()).as_ref(), Some(&m));
        assert!(Model::from_bytes(b"SGHW").is_none());
    }

    #[test]
    fn samples_text_round_trips_and_skips_junk() {
        let s = vec![
            Sample {
                ch: 'é',
                strokes: vec![vec![(0.1, 0.2), (0.3, 0.4)], vec![(0.5, 0.5)]],
            },
            Sample {
                ch: '"',
                strokes: vec![vec![(0.0, 0.0), (1.0, 1.0)]],
            },
        ];
        let t = samples_to_text(&s);
        assert_eq!(samples_from_text(&t), s);
        assert!(samples_from_text("ab\t1,2\nx\tnope\n").is_empty());
    }

    #[test]
    fn user_samples_pull_their_character_up() {
        let mut r = Recogniser::new(Model::default_model());
        // A deliberately odd "a": a single diagonal.
        let ink = vec![vec![(0.2, 0.2), (0.8, 0.7)]];
        let before = r.recognise(&ink, 1)[0].0;
        let target = if before == 'q' { 'k' } else { 'q' };
        assert_eq!(
            r.set_samples(&[Sample {
                ch: target,
                strokes: ink.clone()
            }]),
            1
        );
        assert_eq!(r.recognise(&ink, 1)[0].0, target);
        assert!(r.recognise(&[], 3).is_empty());
    }
}
