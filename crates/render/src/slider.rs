//! A reusable stepped slider (v1.4.4, S14.4): a horizontal track with a dot
//! per stop and a thumb. Taps and drags snap to the nearest stop; Left /
//! Right move it when it has focus. Drawn in pure black and white (0x00 /
//! 0xFF only, text thresholded), so it is safe for DU updates. While
//! disabled it is drawn muted: dotted track, hollow dots and a hollow thumb.
//!
//! First user: Settings → Keyboard & input → Keyboard size. Other numeric
//! settings (Size, Leading, Margin, UI scale …) could adopt it later.

use crate::canvas::{Canvas, Rect};
use crate::fonts::{Face, Fonts};
use crate::theme::Scale;

/// Minimum tap target, device pixels at the device scale (as elsewhere).
pub const MIN_TARGET: i32 = 44;

#[derive(Clone, Debug, PartialEq)]
pub struct Slider {
    /// Stop labels, left to right (already translated).
    pub stops: Vec<String>,
    /// Index of the thumb.
    pub value: usize,
    pub enabled: bool,
    pub focused: bool,
}

/// Where a slider sits on screen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Geom {
    /// The whole control (hit area for drags; the focus frame).
    pub area: Rect,
    /// One tap target per stop, side by side, each at least 44 px.
    pub stops: Vec<Rect>,
    /// Stop centres on the track.
    pub xs: Vec<i32>,
    /// Track centre line.
    pub track_y: i32,
}

/// Lays out `n` stops across `area`; the track sits in the upper half and
/// the labels under it.
#[must_use]
pub fn geom(area: Rect, n: usize, s: Scale) -> Geom {
    let n = n.max(1) as i32;
    let inset = s.px(24.0);
    let x0 = area.x + inset;
    let x1 = area.x + area.w - inset;
    let xs: Vec<i32> = (0..n)
        .map(|i| {
            if n == 1 {
                (x0 + x1) / 2
            } else {
                x0 + (x1 - x0) * i / (n - 1)
            }
        })
        .collect();
    let sw = area.w / n;
    let stops = (0..n)
        .map(|i| {
            Rect::new(
                area.x + i * sw,
                area.y,
                if i == n - 1 { area.w - i * sw } else { sw },
                area.h,
            )
        })
        .collect();
    Geom {
        area,
        stops,
        xs,
        track_y: area.y + area.h * 2 / 5,
    }
}

/// The stop nearest to `x` (drags outside the track clamp to the ends).
#[must_use]
pub fn nearest(g: &Geom, x: i32) -> usize {
    g.xs.iter()
        .enumerate()
        .min_by_key(|(_, &c)| (c - x).abs())
        .map_or(0, |(i, _)| i)
}

/// One step left (`-1`) or right (`+1`), clamped.
#[must_use]
pub fn step(value: usize, n: usize, d: i32) -> usize {
    (value as i32 + d).clamp(0, n.max(1) as i32 - 1) as usize
}

fn disc(c: &mut Canvas, cx: i32, cy: i32, r: i32, v: u8) {
    for dy in -r..=r {
        let half = f64::from(r * r - dy * dy).sqrt() as i32;
        c.fill(Rect::new(cx - half, cy + dy, 2 * half + 1, 1), v);
    }
}

fn ring(c: &mut Canvas, cx: i32, cy: i32, r: i32, t: i32, ink: u8, paper: u8) {
    disc(c, cx, cy, r, ink);
    disc(c, cx, cy, (r - t).max(0), paper);
}

/// Draws the slider. `dark` = Night page (white on black).
pub fn draw(c: &mut Canvas, g: &Geom, sl: &Slider, fonts: &Fonts, ui: f32, s: Scale, dark: bool) {
    let (ink, paper) = if dark { (0xFF, 0x00) } else { (0x00, 0xFF) };
    c.fill(g.area, paper);
    let (Some(&first), Some(&last)) = (g.xs.first(), g.xs.last()) else {
        return;
    };
    let th = s.px(3.0).max(2);
    let ty = g.track_y - th / 2;
    if sl.enabled {
        c.fill(Rect::new(first, ty, last - first, th), ink);
    } else {
        let dash = s.px(4.0).max(2);
        let mut x = first;
        while x < last {
            c.fill(Rect::new(x, ty, dash.min(last - x), th), ink);
            x += 2 * dash;
        }
    }
    let dot = s.px(5.0).max(3);
    let line = s.px(1.5).max(1);
    for &x in &g.xs {
        if sl.enabled {
            disc(c, x, g.track_y, dot, ink);
        } else {
            ring(c, x, g.track_y, dot, line, ink, paper);
        }
    }
    if let Some(&tx) = g.xs.get(sl.value) {
        let r = s.px(11.0).max(MIN_TARGET / 4);
        if sl.enabled {
            ring(c, tx, g.track_y, r + line, line, ink, paper);
            disc(c, tx, g.track_y, r - line, ink);
        } else {
            ring(c, tx, g.track_y, r, line * 2, ink, paper);
        }
    }
    let size = ui * 0.85;
    let face = if sl.enabled {
        Face::AtkinsonBold
    } else {
        Face::Atkinson
    };
    let base = g.track_y as f32 + s.px(30.0) as f32;
    for (i, (&x, lab)) in g.xs.iter().zip(&sl.stops).enumerate() {
        let f = if i == sl.value {
            Face::AtkinsonBold
        } else {
            face
        };
        let w = fonts.width(f, size, lab);
        fonts.draw(c, f, size, x as f32 - w / 2.0, base, lab, ink);
    }
    if sl.focused {
        c.stroke(g.area, line.max(2), ink);
    }
    // Pure black and white only: threshold the anti-aliased text.
    threshold(c, g.area);
}

fn threshold(c: &mut Canvas, r: Rect) {
    let r = r.intersect(&c.bounds());
    let w = c.width as usize;
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            let p = &mut c.pixels[y as usize * w + x as usize];
            *p = if *p >= 128 { 0xFF } else { 0x00 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_and_steps() {
        let g = geom(Rect::new(0, 0, 300, 100), 3, Scale(1.0));
        assert_eq!(nearest(&g, -50), 0);
        assert_eq!(nearest(&g, 140), 1);
        assert_eq!(nearest(&g, 260), 2);
        assert_eq!(nearest(&g, 999), 2);
        assert_eq!(step(0, 3, -1), 0);
        assert_eq!(step(1, 3, 1), 2);
        assert_eq!(step(2, 3, 1), 2);
        assert!(g
            .stops
            .iter()
            .all(|r| r.w >= MIN_TARGET && r.h >= MIN_TARGET));
    }

    #[test]
    fn pure_black_and_white_in_both_states() {
        let fonts = Fonts::new();
        for enabled in [true, false] {
            for dark in [false, true] {
                let mut c = Canvas::new(400, 120, 0xF0);
                let g = geom(Rect::new(10, 10, 380, 100), 3, Scale(1.0));
                let sl = Slider {
                    stops: vec!["100%".into(), "125%".into(), "150%".into()],
                    value: 2,
                    enabled,
                    focused: true,
                };
                draw(&mut c, &g, &sl, &fonts, 17.0, Scale(1.0), dark);
                let w = c.width as usize;
                for y in 10..110 {
                    for x in 10..390 {
                        let p = c.pixels[y * w + x];
                        assert!(p == 0 || p == 0xFF, "grey {p} at {x},{y}");
                    }
                }
            }
        }
    }

    #[test]
    fn muted_differs_from_enabled() {
        let fonts = Fonts::new();
        let g = geom(Rect::new(0, 0, 300, 100), 3, Scale(1.0));
        let mut sl = Slider {
            stops: vec!["a".into(), "b".into(), "c".into()],
            value: 1,
            enabled: true,
            focused: false,
        };
        let mut a = Canvas::new(300, 100, 0xFF);
        draw(&mut a, &g, &sl, &fonts, 17.0, Scale(1.0), false);
        sl.enabled = false;
        let mut b = Canvas::new(300, 100, 0xFF);
        draw(&mut b, &g, &sl, &fonts, 17.0, Scale(1.0), false);
        assert_ne!(a.pixels, b.pixels);
    }
}
