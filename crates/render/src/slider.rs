//! A reusable stepped slider (v1.4.4, S14.4): a horizontal track with a dot
//! per stop and a thumb. Taps and drags snap to the nearest stop; Left /
//! Right move it when it has focus. Drawn straight on the page (v1.4.5):
//! enabled in solid ink, so moves are DU-safe (only the thumb changes);
//! disabled in a static grey (dotted track, hollow dots and thumb), shown
//! with a greyscale refresh.
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

/// Lays out `n` stops across `area`; the value label sits above the thumb,
/// the track in the lower half, the end labels on the value's line.
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
        track_y: area.y + area.h * 3 / 5,
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

/// The grey of a disabled slider (v1.4.5): static, drawn with a
/// greyscale refresh; never shown while the slider moves.
pub const MUTED: u8 = 0x88;

/// Draws the slider straight on the page, after the Kobo's own (v1.13.1):
/// a thick track up to a hollow ring thumb, a hairline after it, the
/// current value above the thumb and the end labels above the track ends.
/// There is no dot per stop; taps and drags still snap to the stops.
/// Enabled: solid `ink` only (DU-safe, only the thumb and the track change).
/// Disabled: everything in [`MUTED`] grey. Focus: a bigger thumb with a
/// thicker ring. `paper` is the page colour behind it.
pub fn draw(
    c: &mut Canvas,
    g: &Geom,
    sl: &Slider,
    fonts: &Fonts,
    ui: f32,
    s: Scale,
    (ink, paper): (u8, u8),
) {
    let fg = if sl.enabled { ink } else { MUTED };
    let (Some(&first), Some(&last)) = (g.xs.first(), g.xs.last()) else {
        return;
    };
    let tx = g.xs.get(sl.value).copied().unwrap_or(first);
    let thick = s.px(4.0).max(3);
    let hair = s.px(1.5).max(1);
    // Done part thick, the rest a hairline.
    c.fill(
        Rect::new(first, g.track_y - thick / 2, tx - first, thick),
        fg,
    );
    c.fill(Rect::new(tx, g.track_y - hair / 2, last - tx, hair), fg);
    // The thumb: a hollow ring (paper inside).
    let mut r = s.px(15.0).max(MIN_TARGET / 3);
    let mut t = s.px(3.0).max(2);
    if sl.focused && sl.enabled {
        r += s.px(3.0).max(2);
        t += s.px(1.5).max(1);
    }
    ring(c, tx, g.track_y, r, t, fg, paper);
    // The value over the thumb, the end labels under the ends.
    let size = ui * 0.85;
    let lift = r + s.px(12.0);
    if let Some(lab) = sl.stops.get(sl.value).filter(|l| !l.is_empty()) {
        let w = fonts.width(Face::AtkinsonBold, size, lab);
        let (lo, hi) = (g.area.x as f32, (g.area.x + g.area.w) as f32 - w);
        let x = (tx as f32 - w / 2.0).clamp(lo, hi.max(lo));
        fonts.draw(
            c,
            Face::AtkinsonBold,
            size,
            x,
            (g.track_y - lift) as f32,
            lab,
            fg,
        );
    }
    // End labels on the same line as the value, above the track's ends.
    let line_y = (g.track_y - lift) as f32;
    for i in [0, sl.stops.len().saturating_sub(1)] {
        if i == sl.value {
            continue;
        }
        let (Some(lab), Some(&x)) = (sl.stops.get(i), g.xs.get(i)) else {
            continue;
        };
        if lab.is_empty() {
            continue;
        }
        let w = fonts.width(Face::Atkinson, size, lab);
        let (lo, hi) = (g.area.x as f32, (g.area.x + g.area.w) as f32 - w);
        let lx = (x as f32 - w / 2.0).clamp(lo, hi.max(lo));
        if sl.enabled {
            text_1bit(c, fonts, (Face::Atkinson, size), lx, line_y, lab, fg);
        } else {
            fonts.draw(c, Face::Atkinson, size, lx, line_y, lab, fg);
        }
    }
}

/// Text in 1-bit ink (coverage >= 50 % is `ink`, the rest is left alone): a
/// label that changes while the thumb moves must stay pure black and white.
fn text_1bit(
    c: &mut Canvas,
    fonts: &Fonts,
    (face, size): (Face, f32),
    x: f32,
    base: f32,
    t: &str,
    ink: u8,
) {
    let tw = fonts.width(face, size, t).ceil() as i32 + 4;
    let th = (size * 1.6).ceil() as i32;
    let (ox, oy) = (x.floor() as i32 - 2, (base - size * 1.15).floor() as i32);
    let mut tmp = Canvas::new(tw.max(1) as u32, th.max(1) as u32, 255);
    fonts.draw(&mut tmp, face, size, x - ox as f32, base - oy as f32, t, 0);
    let b = c.bounds();
    for yy in 0..th {
        for xx in 0..tw {
            if tmp.pixels[(yy * tw + xx) as usize] < 128 && b.contains(ox + xx, oy + yy) {
                c.pixels[((oy + yy) * c.width as i32 + ox + xx) as usize] = ink;
            }
        }
    }
}

/// The track-and-thumb band (labels excluded), for tests and audits.
#[must_use]
pub fn track_band(g: &Geom, s: Scale) -> Rect {
    let r = s.px(18.0);
    Rect::new(g.area.x, g.track_y - r, g.area.w, 2 * r)
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
    fn enabled_is_solid_ink_and_muted_is_grey() {
        let fonts = Fonts::new();
        for (ink, paper) in [(0x00, 0xF0), (0xFF, 0x12)] {
            for enabled in [true, false] {
                for focused in [false, true] {
                    let mut c = Canvas::new(400, 120, paper);
                    let g = geom(Rect::new(10, 10, 380, 100), 3, Scale(1.0));
                    let sl = Slider {
                        stops: vec!["100%".into(), "125%".into(), "150%".into()],
                        value: 2,
                        enabled,
                        focused,
                    };
                    draw(&mut c, &g, &sl, &fonts, 17.0, Scale(1.0), (ink, paper));
                    let b = track_band(&g, Scale(1.0));
                    let w = c.width as usize;
                    let mut seen = false;
                    for y in b.y..b.y + b.h {
                        for x in b.x..b.x + b.w {
                            let p = c.pixels[y as usize * w + x as usize];
                            if enabled {
                                assert!(p == ink || p == paper, "enabled {p:#x}");
                            } else {
                                assert!(p == MUTED || p == paper, "muted {p:#x}");
                            }
                            seen |= p != paper;
                        }
                    }
                    assert!(seen);
                    // No plate: the page colour shows at the corners.
                    assert_eq!(c.pixels[11 * w + 11], paper);
                }
            }
        }
    }

    #[test]
    fn focus_is_a_bigger_thumb_not_a_frame() {
        let fonts = Fonts::new();
        let g = geom(Rect::new(0, 0, 300, 100), 3, Scale(1.0));
        let mut sl = Slider {
            stops: vec!["a".into(), "b".into(), "c".into()],
            value: 1,
            enabled: true,
            focused: false,
        };
        let mut a = Canvas::new(300, 100, 0xFF);
        draw(&mut a, &g, &sl, &fonts, 17.0, Scale(1.0), (0, 0xFF));
        sl.focused = true;
        let mut b = Canvas::new(300, 100, 0xFF);
        draw(&mut b, &g, &sl, &fonts, 17.0, Scale(1.0), (0, 0xFF));
        let ink = |c: &Canvas| c.pixels.iter().filter(|&&p| p == 0).count();
        assert!(ink(&b) > ink(&a));
        // Edges untouched: no frame.
        assert!(b.pixels[..300].iter().all(|&p| p == 0xFF));
        assert!((0..100).all(|y| b.pixels[y * 300] == 0xFF));
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
        draw(&mut a, &g, &sl, &fonts, 17.0, Scale(1.0), (0, 0xFF));
        sl.enabled = false;
        let mut b = Canvas::new(300, 100, 0xFF);
        draw(&mut b, &g, &sl, &fonts, 17.0, Scale(1.0), (0, 0xFF));
        assert_ne!(a.pixels, b.pixels);
    }
}
