//! The toggle switch (v1.13.1), after the Kobo's own: an upper-case state
//! word, then a capsule. On = a filled capsule with a paper knob ringed in
//! ink at the right; off = an outlined capsule with a ringed knob at the
//! left. Every on/off setting uses this one drawing and this one target, so
//! a change here changes them all. Pure ink on pure paper, no grey.

use crate::canvas::{Canvas, Rect};
use crate::fonts::{Face, Fonts};
use crate::i18n::tr;
use crate::ink::round_outline_in;
use crate::theme::{Scale, Theme};

/// Every size of the switch, in brief px (scaled by `Scale`).
pub struct Metrics {
    /// Width of the capsule.
    pub track_w: i32,
    /// Height of the capsule.
    pub track_h: i32,
    /// Between the track's edge and the knob.
    pub inset: i32,
    /// Stroke of the track outline and the knob ring.
    pub line: i32,
    /// Between the state word and the capsule.
    pub gap: i32,
    /// The state word's size, as a share of the row's text size.
    pub word: f32,
}

impl Metrics {
    /// The sizes at scale `s`.
    #[must_use]
    pub fn new(s: Scale) -> Self {
        Self {
            track_w: s.px(52.0),
            track_h: s.px(28.0),
            inset: s.px(4.0),
            line: s.px(2.0).max(2),
            gap: s.px(14.0),
            word: 0.85,
        }
    }
}

/// The state word ("ON" / "OFF", translated, upper-case).
fn word(on: bool) -> String {
    tr(if on { "On" } else { "Off" }).to_uppercase()
}

/// The touch target inside `band`: flush right, 44 px high, wide enough for
/// the longer state word, the gap and the capsule.
#[must_use]
pub fn target(band: Rect, s: Scale, fonts: &Fonts, ui: f32) -> Rect {
    let m = Metrics::new(s);
    let words = [true, false]
        .map(|o| fonts.width(Face::Atkinson, ui * m.word, &word(o)) as i32)
        .into_iter()
        .max()
        .unwrap_or(0);
    let (w, h) = (words + m.gap + m.track_w, s.px(44.0));
    Rect::new(band.x + band.w - w, band.y + (band.h - h) / 2, w, h)
}

/// Draws the switch in `r` (a [`target`]).
pub fn draw(c: &mut Canvas, s: Scale, fonts: &Fonts, ui: f32, r: Rect, on: bool, t: Theme) {
    let m = Metrics::new(s);
    let track = Rect::new(
        r.x + r.w - m.track_w,
        r.y + (r.h - m.track_h) / 2,
        m.track_w,
        m.track_h,
    );
    let rad = (m.track_h - 1) / 2;
    if on {
        round_outline_in(c, track, rad, 0, t.ink);
    } else {
        round_outline_in(c, track, rad, m.line, t.ink);
    }
    // The knob: a paper disc ringed in ink, at the end the state points to.
    // Off, the ring is the disc's edge; on, it sits just inside the disc so
    // it still shows against the dark track.
    let d = m.track_h - 2 * m.inset;
    let kx = if on {
        track.x + track.w - m.inset - d
    } else {
        track.x + m.inset
    };
    let knob = Rect::new(kx, track.y + (track.h - d) / 2, d, d);
    round_outline_in(c, knob, (d - 1) / 2, 0, t.paper);
    let ring = if on { knob.inset(m.line) } else { knob };
    round_outline_in(
        c,
        ring,
        (ring.w - 1) / 2,
        m.line.min((ring.w - 1) / 2),
        t.ink,
    );
    // The state word, to the left, upper-case in the sans.
    let size = ui * m.word;
    let text = word(on);
    let tw = fonts.width(Face::Atkinson, size, &text);
    let x = (track.x - m.gap) as f32 - tw;
    let base = r.y as f32 + r.h as f32 / 2.0 + fonts.ascent(Face::Atkinson, size) / 2.6;
    fonts.draw(c, Face::Atkinson, size, x, base, &text, t.ink);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_big_enough_flush_right_and_inside_the_band() {
        let fonts = Fonts::new();
        for scale in [0.6, 1.0, 1.139, 2.28] {
            let s = Scale(scale);
            let band = Rect::new(20, 100, 1400, s.px(52.0));
            let r = target(band, s, &fonts, 22.0 * scale);
            assert!(r.h >= 44.min(s.px(44.0)) && r.w >= 44, "{r:?}");
            assert_eq!(r.x + r.w, band.x + band.w, "flush right");
        }
    }

    #[test]
    fn on_is_dark_off_is_mostly_paper_and_only_two_colours_beyond_the_word() {
        let fonts = Fonts::new();
        let s = Scale(1.139);
        let t = Theme::DAY;
        let band = Rect::new(0, 0, 400, 80);
        let r = target(band, s, &fonts, 22.0);
        let dark = |on: bool| {
            let mut c = Canvas::new(400, 80, t.paper);
            draw(&mut c, s, &fonts, 22.0, r, on, t);
            let m = Metrics::new(s);
            let track = Rect::new(r.x + r.w - m.track_w, 0, m.track_w, 80);
            let n = (track.x..track.x + track.w)
                .flat_map(|x| (0..80).map(move |y| (x, y)))
                .filter(|&(x, y)| c.pixels[(y * 400 + x) as usize] == t.ink)
                .count();
            // Inside the capsule only ink and paper.
            assert!((track.x..track.x + track.w).all(|x| (0..80).all(|y| {
                let v = c.pixels[(y * 400 + x) as usize];
                v == t.ink || v == t.paper
            })));
            n
        };
        assert!(
            dark(true) > 3 * dark(false),
            "on is filled, off is an outline"
        );
    }
}
