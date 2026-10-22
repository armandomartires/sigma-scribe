//! The writing panel (v1.5.0, plan §12.3–12.4): a pure white panel over
//! the bottom of the screen, pure black ink. Live segments go out as DU
//! (or A2) regions; the chrome is drawn with the normal partial refresh.

use crate::canvas::{Canvas, Rect};
use crate::editor::EditorScreen;
use crate::fonts::Face;
use crate::i18n::tr;
use sigma_core::ink::{Ink, Pen, Point, PX_PER_MM};

/// Panel paper and ink: pure white and pure black, so DU is exact.
pub const WHITE: u8 = 255;
pub const BLACK: u8 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InkButton {
    Undo,
    Clear,
    Pen,
    Cancel,
    Insert,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InkLayout {
    /// The whole panel (white).
    pub panel: Rect,
    /// Where ink goes; touches elsewhere never ink.
    pub area: Rect,
    pub baseline_y: i32,
    pub buttons: Vec<(Rect, InkButton)>,
}

/// Stroke width in panel px for a pen (at least 2 px).
#[must_use]
pub fn pen_px(pen: Pen) -> i32 {
    ((pen.width_mm() * PX_PER_MM).round() as i32).max(2)
}

/// Draws one segment as stamped round discs, pure black; returns the
/// dirty rectangle (clipped to `clip`).
pub fn draw_segment(c: &mut Canvas, a: Point, b: Point, width: i32, clip: Rect) -> Rect {
    let r = width as f32 / 2.0;
    let (dx, dy) = ((b.x - a.x) as f32, (b.y - a.y) as f32);
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    let ri = r.ceil() as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let (cx, cy) = (a.x as f32 + dx * t, a.y as f32 + dy * t);
        for yy in (cy as i32 - ri)..=(cy as i32 + ri) {
            for xx in (cx as i32 - ri)..=(cx as i32 + ri) {
                let (fx, fy) = (xx as f32 + 0.5 - cx, yy as f32 + 0.5 - cy);
                if fx * fx + fy * fy <= r * r && clip.contains(xx, yy) {
                    let i = yy as usize * c.width as usize + xx as usize;
                    c.pixels[i] = BLACK;
                }
            }
        }
    }
    Rect::new(
        a.x.min(b.x) - ri - 1,
        a.y.min(b.y) - ri - 1,
        (a.x - b.x).abs() + 2 * ri + 3,
        (a.y - b.y).abs() + 2 * ri + 3,
    )
    .intersect(&clip)
}

/// Draws every stroke of `ink` (and the live one).
pub fn draw_strokes(c: &mut Canvas, ink: &Ink, width: i32, clip: Rect) {
    for s in ink.strokes.iter().chain(ink.live.iter()) {
        let pts = &s.points;
        if pts.len() == 1 {
            draw_segment(c, pts[0], pts[0], width, clip);
        }
        for w in pts.windows(2) {
            draw_segment(c, w[0], w[1], width, clip);
        }
    }
}

/// A 1-bit ink image (v1.5.1): `on[y * w + x]` is an ink pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    pub w: u32,
    pub h: u32,
    pub on: Vec<bool>,
}

impl Raster {
    /// Rasterises an SVG with resvg at its natural size (mm → 300 ppi),
    /// shrunk to `max_w` if wider; coverage ≥ 50 % becomes ink, so the
    /// result is pure 1-bit (DU-safe). `None` if it does not parse.
    #[must_use]
    pub fn from_svg(svg: &str, max_w: i32) -> Option<Self> {
        use resvg::{tiny_skia, usvg};
        let opt = usvg::Options {
            dpi: 300.0,
            ..usvg::Options::default()
        };
        let tree = usvg::Tree::from_str(svg, &opt).ok()?;
        let sz = tree.size();
        let k = (max_w.max(1) as f32 / sz.width()).min(1.0);
        let (w, h) = (
            (sz.width() * k).ceil().max(1.0) as u32,
            (sz.height() * k).ceil().max(1.0) as u32,
        );
        if w > 4096 || h > 4096 {
            return None;
        }
        let mut pm = tiny_skia::Pixmap::new(w, h)?;
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(k, k),
            &mut pm.as_mut(),
        );
        let on = pm.data().chunks_exact(4).map(|p| p[3] >= 128).collect();
        Some(Self { w, h, on })
    }

    /// Draws the ink pixels in `v` at (x, y), clipped to `clip`.
    pub fn blit(&self, c: &mut Canvas, x: i32, y: i32, v: u8, clip: Rect) {
        let clip = clip.intersect(&c.bounds());
        for yy in 0..self.h as i32 {
            for xx in 0..self.w as i32 {
                if self.on[(yy * self.w as i32 + xx) as usize] && clip.contains(x + xx, y + yy) {
                    c.pixels[((y + yy) * c.width as i32 + x + xx) as usize] = v;
                }
            }
        }
    }
}

impl EditorScreen {
    /// The panel: the bottom 45 % (portrait) or 55 % (landscape).
    #[must_use]
    pub fn ink_layout(&self) -> InkLayout {
        let (w, h) = (self.width as i32, self.height as i32);
        let ph = h * if w > h { 55 } else { 45 } / 100;
        let panel = Rect::new(0, h - ph, w, ph);
        let pad = self.px(8.0);
        let bh = self.px(44.0);
        let by = panel.y + pad;
        let ui = self.ui_px();
        let mut buttons = Vec::new();
        let mut x = pad;
        let bw = |label: &str| {
            (self.fonts.width(Face::Atkinson, ui, &tr(label)) as i32 + self.px(24.0))
                .max(self.px(64.0))
        };
        for (b, l) in [
            (InkButton::Undo, "Undo"),
            (InkButton::Clear, "Clear"),
            (InkButton::Pen, "Stylus"),
        ] {
            let ww = bw(l).max(bw("Finger"));
            buttons.push((Rect::new(x, by, ww, bh), b));
            x += ww + pad;
        }
        let mut rx = w - pad;
        for (b, l) in [(InkButton::Insert, "Insert"), (InkButton::Cancel, "Cancel")] {
            let ww = bw(l);
            rx -= ww;
            buttons.push((Rect::new(rx, by, ww, bh), b));
            rx -= pad;
        }
        let top = by + bh + pad;
        let area = Rect::new(pad, top, w - 2 * pad, panel.y + ph - top - pad);
        InkLayout {
            baseline_y: area.y + area.h * 7 / 10,
            panel,
            area,
            buttons,
        }
    }

    /// The whole panel: white, a black rule on top, the buttons, the
    /// dotted baseline and the strokes.
    pub fn draw_ink(&self, c: &mut Canvas, l: &InkLayout, ink: &Ink, pen: Pen) {
        c.fill(l.panel, WHITE);
        c.fill(
            Rect::new(l.panel.x, l.panel.y, l.panel.w, self.px(2.0).max(2)),
            BLACK,
        );
        let ui = self.ui_px();
        for (r, b) in &l.buttons {
            let label = match b {
                InkButton::Undo => "Undo",
                InkButton::Clear => "Clear",
                InkButton::Pen => match pen {
                    Pen::Finger => "Finger",
                    Pen::Stylus => "Stylus",
                },
                InkButton::Cancel => "Cancel",
                InkButton::Insert => "Insert",
            };
            if *b == InkButton::Insert {
                c.fill(*r, BLACK);
            } else {
                c.stroke(*r, self.px(1.5).max(1), BLACK);
            }
            let t = tr(label);
            let tw = self.fonts.width(Face::Atkinson, ui, &t);
            let ink_c = if *b == InkButton::Insert {
                WHITE
            } else {
                BLACK
            };
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui,
                r.x as f32 + (r.w as f32 - tw) / 2.0,
                (r.y + r.h / 2) as f32 + ui * 0.35,
                &t,
                ink_c,
            );
        }
        // Dotted baseline: 1-px black dots every 8 px (no grey).
        let dot = self.px(2.0).max(1);
        let mut x = l.area.x;
        while x < l.area.x + l.area.w {
            c.fill(Rect::new(x, l.baseline_y, dot, dot), BLACK);
            x += dot * 4;
        }
        draw_strokes(c, ink, pen_px(pen), l.area);
    }

    #[must_use]
    pub fn ink_hit(&self, l: &InkLayout, x: i32, y: i32) -> Option<InkButton> {
        l.buttons
            .iter()
            .find(|(r, _)| r.inset(-self.px(3.0)).contains(x, y))
            .map(|(_, b)| *b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_are_pure_black_and_clipped() {
        let mut c = Canvas::new(100, 60, WHITE);
        let clip = Rect::new(10, 10, 80, 40);
        let a = Point {
            x: 5,
            y: 20,
            t: 0,
            major: None,
        };
        let b = Point {
            x: 60,
            y: 30,
            t: 10,
            major: None,
        };
        let r = draw_segment(&mut c, a, b, 6, clip);
        assert!(c.pixels.iter().all(|&p| p == WHITE || p == BLACK));
        assert!(c.pixels.contains(&BLACK));
        for y in 0..60 {
            for x in 0..100 {
                if c.pixels[y * 100 + x] == BLACK {
                    assert!(clip.contains(x as i32, y as i32) && r.contains(x as i32, y as i32));
                }
            }
        }
    }

    #[test]
    fn svg_rasterises_to_pure_one_bit_at_300_ppi() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10mm\" height=\"5mm\" viewBox=\"0 0 10 5\"><path d=\"M1 2.5 L9 2.5\" stroke=\"#000\" stroke-width=\"1\" fill=\"none\"/></svg>";
        let r = Raster::from_svg(svg, 1000).unwrap();
        assert_eq!((r.w, r.h), (119, 60));
        let n = r.on.iter().filter(|&&b| b).count();
        assert!(n > 800 && n < 1400, "{n}");
        let small = Raster::from_svg(svg, 50).unwrap();
        assert_eq!(small.w, 50);
        assert!(Raster::from_svg("not svg", 100).is_none());
        let mut c = Canvas::new(200, 100, 240);
        let b = c.bounds();
        r.blit(&mut c, 10, 10, 0, b);
        assert!(c.pixels.iter().all(|&p| p == 240 || p == 0));
    }

    #[test]
    fn pen_widths() {
        assert_eq!(pen_px(Pen::Finger), 14);
        assert_eq!(pen_px(Pen::Stylus), 7);
    }
}
