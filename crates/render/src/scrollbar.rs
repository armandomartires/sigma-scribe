//! The scroll bar (v1.12), after the Kobo's own: a hairline track down the
//! right margin and a hollow, rounded thumb on it. The editor and every
//! Settings page use this one drawing. Pure ink on pure paper, no grey, so
//! the fast DU waveform renders it cleanly; it is display only.

use crate::canvas::{Canvas, Rect};

/// The strip a bar occupies in a page's right margin: `w` wide, `margin`
/// from the right edge of `page`, `inset` from its top and bottom.
#[must_use]
pub fn strip(page: Rect, w: i32, margin: i32, inset: i32) -> Rect {
    Rect::new(
        page.x + page.w - w - margin,
        page.y + inset,
        w,
        (page.h - 2 * inset).max(1),
    )
}

/// Every size of the bar, in one place: change a number here and the
/// editor, Preview, Help and Settings all change with it. Brief units,
/// scaled by the *page* scale (never the UI scale), so the bar is the same
/// size on every screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metrics {
    /// Handle width.
    pub w: i32,
    /// Gap between the handle and the screen edge (1.5 handle widths).
    pub margin: i32,
    /// Gap above and below the track.
    pub inset: i32,
    /// The shortest the thumb gets.
    pub min_thumb: i32,
    /// Outline of the thumb.
    pub line: i32,
    /// Air between a page's content (a switch, a radio, a value) and the
    /// bar's inner side.
    pub content_gap: i32,
}

impl Metrics {
    const WIDTH: f32 = 17.0;
    const INSET: f32 = 8.0;
    const MIN_THUMB: f32 = 40.0;
    const LINE: f32 = 2.5;
    const CONTENT_GAP: f32 = 24.0;

    #[must_use]
    pub fn new(page_scale: crate::theme::Scale) -> Self {
        let w = page_scale.px(Self::WIDTH).max(8);
        Self {
            w,
            margin: w * 3 / 2,
            inset: page_scale.px(Self::INSET),
            min_thumb: page_scale.px(Self::MIN_THUMB),
            line: page_scale.px(Self::LINE).max(2),
            content_gap: page_scale.px(Self::CONTENT_GAP),
        }
    }

    /// How far in from the screen edge the bar's inner side is: pages keep
    /// their content clear of this.
    #[must_use]
    pub fn clearance(self) -> i32 {
        self.w + self.margin
    }

    /// How far in from the screen edge a page's content stops: the bar and
    /// the air before it.
    #[must_use]
    pub fn content_inset(self) -> i32 {
        self.clearance() + self.content_gap
    }
}

/// The one bar every screen uses (editor, preview, Help, Settings): the
/// strip and the thumb for a view of `visible` of `total` units that starts
/// at `at`, in `page`'s right margin. `None` when everything fits.
#[must_use]
pub fn bar(
    s: crate::theme::Scale,
    page: Rect,
    total: usize,
    visible: usize,
    at: usize,
) -> Option<(Rect, Rect)> {
    if total <= visible || page.h <= 0 {
        return None;
    }
    let m = Metrics::new(s);
    let strip = strip(page, m.w, m.margin, m.inset);
    Some((strip, thumb(strip, total, visible, at, m.min_thumb)))
}

/// Where a finger grabs a bar: the strip widened to a touch target, out to
/// the page's right edge, the whole height of `page`. Shared by every screen
/// that lets the bar be dragged or tapped.
#[must_use]
pub fn zone(s: crate::theme::Scale, page: Rect, strip: Rect) -> Rect {
    let left = (strip.x - s.px(4.0)).max(page.x);
    Rect::new(left, page.y, page.x + page.w - left, page.h)
}

/// A bar that stands for `n` pages (Settings, Help, Preview): where it is,
/// where a finger can grab it, and what a drag or a tap on it means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paged {
    pub zone: Rect,
    pub strip: Rect,
    pub thumb: Rect,
    pub n: usize,
    pub at: usize,
}

impl Paged {
    /// `None` when there is one page or none.
    #[must_use]
    pub fn new(s: crate::theme::Scale, page: Rect, n: usize, at: usize) -> Option<Self> {
        let (strip, thumb) = bar(s, page, n, 1, at)?;
        Some(Self {
            zone: zone(s, page, strip),
            strip,
            thumb,
            n,
            at,
        })
    }

    /// Drag: the page the thumb lands on when its centre follows `y`.
    #[must_use]
    pub fn drag_to(&self, y: i32) -> usize {
        let span = (self.strip.h - self.thumb.h).max(1) as f32;
        let f = ((y - self.thumb.h / 2 - self.strip.y) as f32 / span).clamp(0.0, 1.0);
        (f * (self.n - 1) as f32).round() as usize
    }

    /// Tap: the page before when `y` is above the thumb's centre, the page
    /// after when below.
    #[must_use]
    pub fn tap(&self, y: i32) -> usize {
        if y < self.thumb.y + self.thumb.h / 2 {
            self.at.saturating_sub(1)
        } else {
            (self.at + 1).min(self.n - 1)
        }
    }
}

/// Draws [`bar`] on `bg` (the paper, or a margin's tone) in the theme's ink:
/// the bar itself is only black and white, night inverted.
pub fn draw_bar(
    c: &mut Canvas,
    s: crate::theme::Scale,
    (strip, thumb): (Rect, Rect),
    t: crate::theme::Theme,
    bg: u8,
) {
    let ink = if t.paper < 128 { 0xFF } else { 0x00 };
    c.fill(strip, bg);
    draw(c, strip, thumb, ink, bg, Metrics::new(s).line);
}

/// The thumb on `strip` for a view of `visible` of `total` units that starts
/// at unit `at`. At least `min_h` tall, never taller than the strip.
#[must_use]
pub fn thumb(strip: Rect, total: usize, visible: usize, at: usize, min_h: i32) -> Rect {
    let total = total.max(1);
    let th = ((strip.h as f32 * visible.min(total) as f32 / total as f32) as i32)
        .max(min_h)
        .min(strip.h);
    let span = total.saturating_sub(visible).max(1) as f32;
    let at = (at.min(total.saturating_sub(visible)) as f32 / span * (strip.h - th) as f32) as i32;
    Rect::new(strip.x, strip.y + at, strip.w, th)
}

/// Draws the track and the thumb: `ink` on `paper` (the page colours), a
/// `line`-px outline for the thumb, a capsule when the strip is narrow.
pub fn draw(c: &mut Canvas, strip: Rect, thumb: Rect, ink: u8, paper: u8, line: i32) {
    let b = c.bounds();
    let mid = strip.x + strip.w / 2;
    for y in strip.y..strip.y + strip.h {
        if b.contains(mid, y) {
            c.pixels[(y * c.width as i32 + mid) as usize] = ink;
        }
    }
    let r = (thumb.w / 2).min(thumb.h / 2);
    let line = line.max(1);
    for y in thumb.y..thumb.y + thumb.h {
        for x in thumb.x..thumb.x + thumb.w {
            if !b.contains(x, y) {
                continue;
            }
            let outer = inside(x, y, thumb, r);
            if !outer {
                continue;
            }
            let inner = Rect::new(
                thumb.x + line,
                thumb.y + line,
                thumb.w - 2 * line,
                thumb.h - 2 * line,
            );
            let hollow = inner.w > 0 && inner.h > 0 && inside(x, y, inner, (r - line).max(0));
            c.pixels[(y * c.width as i32 + x) as usize] = if hollow { paper } else { ink };
        }
    }
}

/// Whether pixel (x, y) is inside the rounded rectangle `r` with corner
/// radius `rad`.
fn inside(x: i32, y: i32, r: Rect, rad: i32) -> bool {
    if x < r.x || y < r.y || x >= r.x + r.w || y >= r.y + r.h {
        return false;
    }
    let cx = if x < r.x + rad {
        r.x + rad
    } else if x >= r.x + r.w - rad {
        r.x + r.w - rad - 1
    } else {
        return true;
    };
    let cy = if y < r.y + rad {
        r.y + rad
    } else if y >= r.y + r.h - rad {
        r.y + r.h - rad - 1
    } else {
        return true;
    };
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= rad * rad
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumb_spans_the_strip_from_top_to_bottom() {
        let s = Rect::new(1400, 100, 12, 1000);
        let top = thumb(s, 400, 40, 0, 40);
        assert_eq!(top.y, s.y);
        assert_eq!(top.h, 100, "a tenth of the strip");
        let end = thumb(s, 400, 40, 360, 40);
        assert_eq!(end.y + end.h, s.y + s.h);
        let mid = thumb(s, 400, 40, 180, 40);
        assert!(mid.y > top.y && mid.y < end.y);
        assert_eq!(
            thumb(s, 400, 1, 0, 40).h,
            40,
            "never thinner than the minimum"
        );
    }

    #[test]
    fn it_draws_a_hairline_and_a_hollow_capsule_in_two_colours_only() {
        for (ink, paper) in [(0x00, 0xFF), (0xFF, 0x00)] {
            let mut c = Canvas::new(200, 300, 0x80);
            let s = Rect::new(150, 20, 14, 260);
            let t = thumb(s, 10, 5, 0, 40);
            draw(&mut c, s, t, ink, paper, 2);
            let px = |x: i32, y: i32| c.pixels[(y * 200 + x) as usize];
            // The track runs the whole strip on its centre column.
            assert_eq!(px(s.x + s.w / 2, s.y + s.h - 1), ink);
            // Inside the thumb is paper (the track does not show through).
            assert_eq!(px(s.x + s.w / 2, t.y + t.h / 2), paper);
            // The thumb's outline is ink.
            assert_eq!(px(s.x + s.w / 2, t.y), ink);
            // Nothing else is touched, and no grey is made.
            for y in 0..300 {
                for x in 0..200 {
                    let v = px(x, y);
                    let in_strip = x >= s.x && x < s.x + s.w && y >= s.y && y < s.y + s.h;
                    assert!(in_strip || v == 0x80, "{x},{y} touched outside the strip");
                    assert!(v == 0x80 || v == ink || v == paper);
                }
            }
        }
    }
}
