//! The radio button (v1.13.1): one choice out of a list. A ring, with an ink
//! dot inside when it is the chosen one. Every single-choice list (keyboard
//! layout, interface language, keyboard tone, tab width) uses this one
//! drawing and this one target, so a change here changes them all. Pure ink
//! on paper, no grey.

use crate::canvas::{Canvas, Rect};
use crate::ink::round_outline_in;
use crate::theme::{Scale, Theme};

/// Every size of the radio button, in brief px (scaled by `Scale`).
pub struct Metrics {
    pub diameter: i32,
    pub line: i32,
    /// Between the ring and the dot.
    pub gap: i32,
}

impl Metrics {
    #[must_use]
    pub fn new(s: Scale) -> Self {
        Self {
            diameter: s.px(26.0),
            line: s.px(2.0).max(2),
            gap: s.px(5.0),
        }
    }
}

/// The touch target inside `band`: flush right, 44 × 44.
#[must_use]
pub fn target(band: Rect, s: Scale) -> Rect {
    let side = s.px(44.0);
    Rect::new(
        band.x + band.w - side,
        band.y + (band.h - side) / 2,
        side,
        side,
    )
}

/// Draws the radio button in `r` (a [`target`]), the ring flush right.
pub fn draw(c: &mut Canvas, s: Scale, r: Rect, on: bool, t: Theme) {
    let m = Metrics::new(s);
    let d = m.diameter;
    let ring = Rect::new(r.x + r.w - d, r.y + (r.h - d) / 2, d, d);
    round_outline_in(c, ring, (d - 1) / 2, m.line, t.ink);
    if on {
        let dot = ring.inset(m.line + m.gap);
        if dot.w > 1 {
            round_outline_in(c, dot, (dot.w - 1) / 2, 0, t.ink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_44_flush_right_and_the_dot_marks_the_chosen_one() {
        for scale in [0.6, 1.0, 1.139, 2.28] {
            let s = Scale(scale);
            let band = Rect::new(20, 100, 1400, s.px(52.0));
            let r = target(band, s);
            assert!(r.w >= s.px(44.0) && r.h >= s.px(44.0));
            assert_eq!(r.x + r.w, band.x + band.w);
            let t = Theme::DAY;
            let ink = |on: bool| {
                let mut c = Canvas::new(1500, 400, t.paper);
                draw(&mut c, s, r, on, t);
                assert!(c.pixels.iter().all(|&p| p == t.ink || p == t.paper));
                c.pixels.iter().filter(|&&p| p == t.ink).count()
            };
            assert!(ink(true) > ink(false), "a dot inside the ring when chosen");
        }
    }
}
