//! Graph map layout (v1.0, S9.1): positions for the notes of the link
//! graph, computed deterministically. Linked notes get a seeded
//! Fruchterman–Reingold layout with a fixed number of iterations, scaled
//! into the unit disc (radius ≤ 0.8); notes without links ("orphans") sit
//! evenly on a ring of radius 1.0 around it. Same names + edges → same
//! picture. Pure logic, no device dependencies.

/// Iterations of the force simulation (fixed, so time is predictable).
pub const ITERATIONS: usize = 160;
/// Radius of the orphan ring; linked notes stay inside `INNER`.
pub const RING: f64 = 1.0;
pub const INNER: f64 = 0.8;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapLayout {
    /// Unit-space position of every note (x right, y down).
    pub pos: Vec<(f64, f64)>,
    pub orphan: Vec<bool>,
}

/// Cache key over the note names and edges (FNV-1a, 64 bit).
#[must_use]
pub fn layout_key(names: &[String], edges: &[(usize, usize)]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: &[u8]| {
        for &x in b {
            h ^= u64::from(x);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for n in names {
        eat(n.as_bytes());
        eat(&[0]);
    }
    eat(&[0xff]);
    for &(a, b) in edges {
        eat(&(a as u64).to_le_bytes());
        eat(&(b as u64).to_le_bytes());
    }
    h
}

/// xorshift64*: tiny deterministic generator for the starting positions.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Lays out `n` notes joined by directed `edges` (direction is ignored for
/// the forces). `seed` fixes the starting positions.
#[must_use]
pub fn layout(n: usize, edges: &[(usize, usize)], seed: u64) -> MapLayout {
    let mut deg = vec![0usize; n];
    let mut pairs: Vec<(usize, usize)> = edges
        .iter()
        .filter(|&&(a, b)| a != b && a < n && b < n)
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    for &(a, b) in &pairs {
        deg[a] += 1;
        deg[b] += 1;
    }
    let linked: Vec<usize> = (0..n).filter(|&i| deg[i] > 0).collect();
    let mut pos = vec![(0.0, 0.0); n];
    let mut orphan = vec![false; n];

    // Force layout over the linked notes, in a square of side 1.
    let m = linked.len();
    if m > 0 {
        let mut slot = vec![usize::MAX; n];
        for (k, &i) in linked.iter().enumerate() {
            slot[i] = k;
        }
        let mut rng = Rng(seed | 1);
        let mut p: Vec<(f64, f64)> = (0..m)
            .map(|_| (rng.next() - 0.5, rng.next() - 0.5))
            .collect();
        let k = (1.0 / m as f64).sqrt();
        let mut temp = 0.1;
        let cool = temp / (ITERATIONS as f64 + 1.0);
        let mut disp = vec![(0.0f64, 0.0f64); m];
        for _ in 0..ITERATIONS {
            disp.iter_mut().for_each(|d| *d = (0.0, 0.0));
            for a in 0..m {
                for b in (a + 1)..m {
                    let (dx, dy) = (p[a].0 - p[b].0, p[a].1 - p[b].1);
                    let d2 = (dx * dx + dy * dy).max(1e-6);
                    let f = k * k / d2; // repulsion k²/d, along the unit vector
                    disp[a].0 += dx * f;
                    disp[a].1 += dy * f;
                    disp[b].0 -= dx * f;
                    disp[b].1 -= dy * f;
                }
            }
            for &(a, b) in &pairs {
                let (a, b) = (slot[a], slot[b]);
                let (dx, dy) = (p[a].0 - p[b].0, p[a].1 - p[b].1);
                let d = (dx * dx + dy * dy).sqrt().max(1e-3);
                let f = d / k; // attraction d²/k
                disp[a].0 -= dx * f;
                disp[a].1 -= dy * f;
                disp[b].0 += dx * f;
                disp[b].1 += dy * f;
            }
            // Mild gravity keeps separate clusters on screen.
            for (q, d) in p.iter().zip(disp.iter_mut()) {
                d.0 -= q.0 * 0.6 * k * m as f64;
                d.1 -= q.1 * 0.6 * k * m as f64;
            }
            for (q, d) in p.iter_mut().zip(&disp) {
                let len = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-9);
                let step = len.min(temp);
                q.0 += d.0 / len * step;
                q.1 += d.1 / len * step;
            }
            temp -= cool;
        }
        // Centre and scale into the inner disc.
        let (cx, cy) = p.iter().fold((0.0, 0.0), |a, q| {
            (a.0 + q.0 / m as f64, a.1 + q.1 / m as f64)
        });
        let r = p
            .iter()
            .map(|q| ((q.0 - cx).powi(2) + (q.1 - cy).powi(2)).sqrt())
            .fold(0.0f64, f64::max)
            .max(1e-6);
        let s = if m == 1 { 0.0 } else { INNER / r };
        for (kk, &i) in linked.iter().enumerate() {
            pos[i] = ((p[kk].0 - cx) * s, (p[kk].1 - cy) * s);
        }
    }

    // Orphans: evenly on the ring, in library order, starting at the top.
    let lone: Vec<usize> = (0..n).filter(|&i| deg[i] == 0).collect();
    let ring = if m == 0 { INNER } else { RING };
    for (k, &i) in lone.iter().enumerate() {
        let a = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::TAU * k as f64 / lone.len().max(1) as f64;
        pos[i] = (ring * a.cos(), ring * a.sin());
        orphan[i] = true;
    }
    MapLayout { pos, orphan }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring_graph(n: usize) -> Vec<(usize, usize)> {
        (0..n).map(|i| (i, (i + 1) % n)).collect()
    }

    #[test]
    fn deterministic_and_inside_the_disc() {
        let e = ring_graph(12);
        let a = layout(15, &e, 7);
        let b = layout(15, &e, 7);
        assert_eq!(a, b, "same input, same picture");
        for i in 0..12 {
            let (x, y) = a.pos[i];
            assert!((x * x + y * y).sqrt() <= INNER + 1e-9);
            assert!(!a.orphan[i]);
        }
        for i in 12..15 {
            let (x, y) = a.pos[i];
            assert!(((x * x + y * y).sqrt() - RING).abs() < 1e-9);
            assert!(a.orphan[i]);
        }
        // Nodes are spread out, not collapsed.
        let mut min = f64::MAX;
        for i in 0..12 {
            for j in (i + 1)..12 {
                let d =
                    ((a.pos[i].0 - a.pos[j].0).powi(2) + (a.pos[i].1 - a.pos[j].1).powi(2)).sqrt();
                min = min.min(d);
            }
        }
        assert!(min > 0.05, "min distance {min}");
    }

    #[test]
    fn neighbours_are_closer_than_strangers() {
        // Two triangles joined by nothing: within-triangle distances are
        // shorter than across.
        let e = [(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)];
        let l = layout(6, &e, 1);
        let d = |a: usize, b: usize| {
            ((l.pos[a].0 - l.pos[b].0).powi(2) + (l.pos[a].1 - l.pos[b].1).powi(2)).sqrt()
        };
        assert!(d(0, 1) < d(0, 4) && d(3, 4) < d(2, 5));
    }

    #[test]
    fn edge_cases_and_key() {
        assert_eq!(layout(0, &[], 1).pos.len(), 0);
        let one = layout(1, &[], 1);
        assert!(one.orphan[0]);
        let pair = layout(2, &[(0, 1), (1, 0), (1, 1)], 3);
        assert!(!pair.orphan[0] && !pair.orphan[1]);
        let names = vec!["a".to_string(), "b".to_string()];
        let k1 = layout_key(&names, &[(0, 1)]);
        assert_eq!(k1, layout_key(&names, &[(0, 1)]));
        assert_ne!(k1, layout_key(&names, &[(1, 0)]));
        assert_ne!(k1, layout_key(&["a".into(), "c".into()], &[(0, 1)]));
    }
}
