//! Editing gestures in the free writing area (v1.5.4, sprint S15.4, plan
//! §12.3): scratch-out, flicks. Pure geometry on one stroke, in pixels;
//! `size` is the writing area's height, so the thresholds follow the panel.

/// What a stroke means, when it is not ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// A zig-zag back and forth over ink or a word: delete it.
    ScratchOut,
    /// A long, flat stroke right to left: Backspace (joins lines at a
    /// line start).
    FlickLeft,
    /// A long, flat stroke left to right: Space.
    FlickRight,
}

/// Fewest changes of horizontal direction in a scratch-out (a "z" has 2).
pub const SCRATCH_TURNS: usize = 3;
/// A flick is at least this long, in area heights (a dash is far shorter).
pub const FLICK_LEN: f32 = 0.35;
/// A flick's height is at most this share of its width.
pub const FLICK_FLAT: f32 = 0.2;

/// Classifies one finished stroke; `None` is ordinary ink.
#[must_use]
pub fn classify(points: &[(f32, f32)], size: f32) -> Option<Gesture> {
    if points.len() < 2 || size <= 0.0 {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in points {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (w, h) = (x1 - x0, y1 - y0);
    // Direction changes along x, ignoring wobble below 8 % of the width.
    let tol = (w * 0.08).max(size * 0.01);
    let mut turns = 0;
    let mut dir = 0i8;
    let mut anchor = points[0].0;
    let mut path = 0.0;
    for k in 1..points.len() {
        let (a, b) = (points[k - 1], points[k]);
        path += ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let dx = b.0 - anchor;
        if dx.abs() >= tol {
            let d = if dx > 0.0 { 1 } else { -1 };
            if dir != 0 && d != dir {
                turns += 1;
            }
            dir = d;
            anchor = b.0;
        } else if (dir > 0 && b.0 > anchor) || (dir < 0 && b.0 < anchor) {
            anchor = b.0;
        }
    }
    // Wider than tall, so a looped "8" (3 turns, narrow) stays a letter.
    if turns >= SCRATCH_TURNS && w >= h && w >= size * 0.08 && path >= 2.5 * w {
        return Some(Gesture::ScratchOut);
    }
    if turns == 0 && w >= size * FLICK_LEN && h <= w * FLICK_FLAT {
        let (s, e) = (points[0].0, points[points.len() - 1].0);
        return Some(if e < s {
            Gesture::FlickLeft
        } else {
            Gesture::FlickRight
        });
    }
    None
}

/// Whether two boxes `(x0, y0, x1, y1)` overlap.
#[must_use]
pub fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zig(n: usize, w: f32, h: f32) -> Vec<(f32, f32)> {
        (0..=n * 8)
            .map(|k| {
                let t = k as f32 / 8.0;
                let f = t.fract();
                let x = if (t as usize) % 2 == 0 { f } else { 1.0 - f };
                (100.0 + x * w, 100.0 + h * t / n as f32)
            })
            .collect()
    }

    #[test]
    fn scratch_flicks_and_letters() {
        assert_eq!(
            classify(&zig(4, 120.0, 40.0), 600.0),
            Some(Gesture::ScratchOut)
        );
        assert_eq!(
            classify(&zig(6, 200.0, 20.0), 600.0),
            Some(Gesture::ScratchOut)
        );
        // A "z" (2 turns) and a "w" (no turns) are letters.
        assert_eq!(classify(&zig(2, 80.0, 80.0), 600.0), None);
        let w: Vec<(f32, f32)> = (0..=20)
            .map(|k| (k as f32 * 4.0, if k % 5 < 3 { k as f32 } else { 0.0 }))
            .collect();
        assert_eq!(classify(&w, 600.0), None);
        // An "8" from the top: left, right, left, right, but narrow.
        let eight: Vec<(f32, f32)> = (0..=64)
            .map(|k| {
                let a = k as f32 / 64.0 * std::f32::consts::TAU;
                (
                    100.0 - 20.0 * (a).sin() * (a).cos().signum(),
                    100.0 + 40.0 * (1.0 - a.cos()),
                )
            })
            .collect();
        assert_eq!(classify(&eight, 600.0), None);
        let line = |a: f32, b: f32| -> Vec<(f32, f32)> {
            (0..=10)
                .map(|k| (a + (b - a) * k as f32 / 10.0, 300.0 + k as f32))
                .collect()
        };
        assert_eq!(
            classify(&line(500.0, 200.0), 600.0),
            Some(Gesture::FlickLeft)
        );
        assert_eq!(
            classify(&line(200.0, 500.0), 600.0),
            Some(Gesture::FlickRight)
        );
        // A dash is ink.
        assert_eq!(classify(&line(200.0, 260.0), 600.0), None);
    }
}
