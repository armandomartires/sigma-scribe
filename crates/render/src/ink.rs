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
    /// Draw ↔ Letters (v1.5.2).
    Mode,
    Space,
    Back,
    Enter,
    /// Opens enrolment ("Train").
    Train,
    Skip,
    Save,
    /// The n-th candidate of the last letter.
    Cand(usize),
}

/// What the panel shows: free ink, character boxes, or enrolment of the
/// n-th character of `sigma_hwr::CHARSET`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InkMode {
    #[default]
    Draw,
    Letters,
    Train(usize),
}

/// What the letters view needs beyond the strokes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LettersView {
    /// Candidates for the last letter, best first.
    pub cands: Vec<char>,
    /// Enrolment: samples saved this session.
    pub saved: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InkLayout {
    /// The whole panel (white).
    pub panel: Rect,
    /// Where ink goes; touches elsewhere never ink.
    pub area: Rect,
    pub baseline_y: i32,
    pub buttons: Vec<(Rect, InkButton)>,
    /// Letters / Train (v1.5.2): the square character boxes (ink only
    /// goes in a box); empty in Draw.
    pub boxes: Vec<Rect>,
    /// Where the candidate row or the enrolment prompt goes.
    pub strip: Rect,
}

/// Buttons and the labels they may show (widest wins).
type Row = &'static [(InkButton, &'static [&'static str])];

/// Boxes per row: portrait 5, landscape 6.
#[must_use]
pub fn box_count(w: i32, h: i32) -> usize {
    if w > h {
        6
    } else {
        5
    }
}

/// A point in a box, in box units (the box is the unit square).
#[must_use]
pub fn box_units(b: Rect, x: i32, y: i32) -> (f32, f32) {
    (
        (x - b.x) as f32 / b.w.max(1) as f32,
        (y - b.y) as f32 / b.h.max(1) as f32,
    )
}

fn label_of(b: InkButton, pen: Pen, mode: InkMode) -> String {
    let l = match b {
        InkButton::Undo => "Undo",
        InkButton::Clear => "Clear",
        InkButton::Pen => match pen {
            Pen::Finger => "Finger",
            Pen::Stylus => "Stylus",
        },
        InkButton::Cancel => match mode {
            InkMode::Draw => "Cancel",
            _ => "Done",
        },
        InkButton::Insert => "Insert",
        InkButton::Mode => match mode {
            InkMode::Draw => "Letters",
            _ => "Draw",
        },
        InkButton::Space => "Space",
        InkButton::Back => "Erase",
        InkButton::Enter => "Return",
        InkButton::Train => "Train",
        InkButton::Skip => "Skip",
        InkButton::Save => "Save",
        InkButton::Cand(_) => "",
    };
    tr(l).into_owned()
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
        self.ink_layout_in(InkMode::Draw)
    }

    /// The panel for a mode (v1.5.2).
    #[must_use]
    pub fn ink_layout_in(&self, mode: InkMode) -> InkLayout {
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
        let (left, right): (Row, Row) = match mode {
            InkMode::Draw => (
                &[
                    (InkButton::Mode, &["Letters", "Draw"]),
                    (InkButton::Undo, &["Undo"]),
                    (InkButton::Clear, &["Clear"]),
                    (InkButton::Pen, &["Stylus", "Finger"]),
                ],
                &[
                    (InkButton::Insert, &["Insert"]),
                    (InkButton::Cancel, &["Cancel"]),
                ],
            ),
            InkMode::Letters => (
                &[
                    (InkButton::Mode, &["Letters", "Draw"]),
                    (InkButton::Undo, &["Undo"]),
                    (InkButton::Space, &["Space"]),
                    (InkButton::Back, &["Erase"]),
                    (InkButton::Enter, &["Return"]),
                ],
                &[
                    (InkButton::Cancel, &["Done"]),
                    (InkButton::Train, &["Train"]),
                ],
            ),
            InkMode::Train(_) => (
                &[(InkButton::Undo, &["Undo"]), (InkButton::Skip, &["Skip"])],
                &[(InkButton::Cancel, &["Done"]), (InkButton::Save, &["Save"])],
            ),
        };
        for (b, ls) in left {
            let ww = ls.iter().map(|l| bw(l)).max().unwrap_or(0);
            buttons.push((Rect::new(x, by, ww, bh), *b));
            x += ww + pad;
        }
        let mut rx = w - pad;
        for (b, ls) in right {
            let ww = ls.iter().map(|l| bw(l)).max().unwrap_or(0);
            rx -= ww;
            buttons.push((Rect::new(rx, by, ww, bh), *b));
            rx -= pad;
        }
        let top = by + bh + pad;
        let area = Rect::new(pad, top, w - 2 * pad, panel.y + ph - top - pad);
        let mut boxes = Vec::new();
        let mut strip = Rect::new(area.x, area.y, area.w, 0);
        if mode != InkMode::Draw {
            let sh = bh + pad;
            strip = Rect::new(area.x, area.y, area.w, bh);
            let avail = Rect::new(area.x, area.y + sh, area.w, area.h - sh);
            let n = if mode == InkMode::Letters {
                box_count(w, h)
            } else {
                1
            };
            let gap = pad;
            let side = ((avail.w - gap * (n as i32 - 1)) / n as i32)
                .min(avail.h)
                .max(1);
            let total = side * n as i32 + gap * (n as i32 - 1);
            let x0 = avail.x + (avail.w - total) / 2;
            let y0 = avail.y + (avail.h - side) / 2;
            for i in 0..n {
                boxes.push(Rect::new(x0 + i as i32 * (side + gap), y0, side, side));
            }
            if mode == InkMode::Letters {
                // Candidate chips, left to right.
                let cw = bh * 3 / 2;
                for k in 0..5 {
                    let r = Rect::new(strip.x + k * (cw + pad), strip.y, cw, bh);
                    buttons.push((r, InkButton::Cand(k as usize)));
                }
            }
        }
        InkLayout {
            baseline_y: if boxes.is_empty() {
                area.y + area.h * 7 / 10
            } else {
                boxes[0].y + (boxes[0].h as f32 * sigma_hwr::BASELINE) as i32
            },
            panel,
            area,
            buttons,
            boxes,
            strip,
        }
    }

    /// The letters / enrolment panel (v1.5.2): buttons, candidate chips
    /// (or the prompt), boxes with their baseline, and the strokes.
    pub fn draw_letters(
        &self,
        c: &mut Canvas,
        l: &InkLayout,
        ink: &Ink,
        pen: Pen,
        mode: InkMode,
        v: &LettersView,
    ) {
        c.fill(l.panel, WHITE);
        c.fill(
            Rect::new(l.panel.x, l.panel.y, l.panel.w, self.px(2.0).max(2)),
            BLACK,
        );
        let ui = self.ui_px();
        let line = self.px(1.5).max(1);
        for (r, b) in &l.buttons {
            let t = match b {
                InkButton::Cand(k) => match v.cands.get(*k) {
                    Some(ch) => ch.to_string(),
                    None => continue,
                },
                _ => label_of(*b, pen, mode),
            };
            let filled = matches!(b, InkButton::Save | InkButton::Cand(0));
            if filled {
                c.fill(*r, BLACK);
            } else {
                c.stroke(*r, line, BLACK);
            }
            let size = if matches!(b, InkButton::Cand(_)) {
                ui * 1.3
            } else {
                ui
            };
            let face = if matches!(b, InkButton::Cand(_)) {
                Face::Literata
            } else {
                Face::Atkinson
            };
            let tw = self.fonts.width(face, size, &t);
            self.fonts.draw(
                c,
                face,
                size,
                r.x as f32 + (r.w as f32 - tw) / 2.0,
                (r.y + r.h / 2) as f32 + size * 0.35,
                &t,
                if filled { WHITE } else { BLACK },
            );
        }
        if let InkMode::Train(i) = mode {
            let ch = sigma_hwr::CHARSET.chars().nth(i).unwrap_or(' ');
            let total = sigma_hwr::CHARSET.chars().count();
            let t = format!(
                "{} {ch}   ({}/{total}, {} {})",
                tr("Write:"),
                i + 1,
                v.saved,
                tr("saved")
            );
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui * 1.2,
                l.strip.x as f32,
                (l.strip.y + l.strip.h / 2) as f32 + ui * 0.42,
                &t,
                BLACK,
            );
        }
        let dot = self.px(2.0).max(1);
        for b in &l.boxes {
            c.stroke(*b, line, BLACK);
            let by = b.y + (b.h as f32 * sigma_hwr::BASELINE) as i32;
            let mut x = b.x + dot * 2;
            while x < b.x + b.w - dot * 2 {
                c.fill(Rect::new(x, by, dot, dot), BLACK);
                x += dot * 4;
            }
        }
        let clip = l
            .boxes
            .iter()
            .copied()
            .reduce(|a, b| a.union(&b))
            .unwrap_or(l.area);
        draw_strokes(c, ink, pen_px(pen), clip);
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
            if *b == InkButton::Insert {
                c.fill(*r, BLACK);
            } else {
                c.stroke(*r, self.px(1.5).max(1), BLACK);
            }
            let t = label_of(*b, pen, InkMode::Draw);
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
