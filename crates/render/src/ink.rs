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
    /// Sketch: shows or hides the dotted guide line (v1.13.1).
    Guide,
    /// Takes back an Undo of typed text (v1.13.1).
    Redo,
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
    /// Free panel (v1.5.3): ‹ › candidate pages, a key that types its
    /// character, 123 (digits) and the language key.
    Prev,
    Next,
    Type(char),
    Digits,
    Lang,
    /// Caret and page keys of the panels (v1.11.2).
    Home,
    End,
    PgUp,
    PgDn,
    /// Insert / overwrite.
    Ins,
}

/// What the panel shows: free ink, character boxes, or enrolment of the
/// n-th character of `sigma_hwr::CHARSET`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InkMode {
    #[default]
    Draw,
    Letters,
    Train(usize),
    /// Free writing, Boox-style (v1.5.3, decision 55): candidate bar,
    /// key grid, open writing area.
    Free,
}

/// What the letters view needs beyond the strokes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LettersView {
    /// Candidates for the last letter, best first.
    pub cands: Vec<String>,
    /// Enrolment: samples saved this session.
    pub saved: usize,
    /// Free: first candidate shown (‹ › page through them).
    pub first: usize,
    /// Free: 123 (digits and signs only) and the language key's filter.
    pub digits: bool,
    pub lang: Option<&'static str>,
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
    /// Sketch (Draw): the line of small print under the drawing area;
    /// empty elsewhere.
    pub footer: Rect,
}

/// Buttons and the labels they may show (widest wins).
type Row = &'static [(InkButton, &'static [&'static str])];

/// A rounded outline (1-bit): pixels between the outer rounded rectangle
/// and the one `t` inside it.
pub fn round_outline(c: &mut Canvas, r: Rect, rad: i32, t: i32) {
    round_outline_in(c, r, rad, t, BLACK);
}

/// [`round_outline`] in any colour; `t == 0` fills the whole rounded
/// rectangle (an inverted key).
pub fn round_outline_in(c: &mut Canvas, r: Rect, rad: i32, t: i32, color: u8) {
    let inside = |x: i32, y: i32, r: Rect, rad: i32| -> bool {
        if !r.contains(x, y) {
            return false;
        }
        let cx = x.clamp(r.x + rad, r.x + r.w - 1 - rad);
        let cy = y.clamp(r.y + rad, r.y + r.h - 1 - rad);
        let (dx, dy) = (x - cx, y - cy);
        dx * dx + dy * dy <= rad * rad
    };
    let inner = Rect::new(r.x + t, r.y + t, r.w - 2 * t, r.h - 2 * t);
    let solid = t <= 0;
    let b = c.bounds();
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            if inside(x, y, r, rad)
                && (solid || !inside(x, y, inner, (rad - t).max(0)))
                && b.contains(x, y)
            {
                c.pixels[(y * c.width as i32 + x) as usize] = color;
            }
        }
    }
}

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

/// The caret and page keys, in panel order.
const NAV: [InkButton; 5] = [
    InkButton::Home,
    InkButton::End,
    InkButton::Ins,
    InkButton::PgUp,
    InkButton::PgDn,
];

fn label_of_nav(b: InkButton) -> &'static str {
    match b {
        InkButton::Home => "Home",
        InkButton::End => "End",
        InkButton::PgUp => "PgUp",
        InkButton::PgDn => "PgDn",
        _ => "Ins",
    }
}

fn label_of(b: InkButton, pen: Pen, mode: InkMode, _words: bool) -> String {
    let l = match b {
        InkButton::Undo => "Undo",
        InkButton::Redo => "Redo",
        InkButton::Clear => "Clear",
        InkButton::Guide => "Line",
        InkButton::Pen => match pen {
            Pen::Finger => "Finger",
            Pen::Stylus => "Stylus",
        },
        InkButton::Cancel => match mode {
            InkMode::Draw => "Cancel",
            _ => "Done",
        },
        InkButton::Insert => "Insert",
        // The button names where it goes: from writing to a sketch, and back.
        InkButton::Mode => match mode {
            InkMode::Draw => "Write",
            _ => "Sketch",
        },
        InkButton::Space => "Space",
        InkButton::Back => "Erase",
        InkButton::Enter => "Return",
        InkButton::Train => "Train",
        InkButton::Skip => "Skip",
        InkButton::Save => "Save",
        InkButton::Home => "Home",
        InkButton::End => "End",
        InkButton::PgUp => "PgUp",
        InkButton::PgDn => "PgDn",
        InkButton::Ins => "Ins",
        InkButton::Cand(_)
        | InkButton::Prev
        | InkButton::Next
        | InkButton::Type(_)
        | InkButton::Digits
        | InkButton::Lang => "",
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
        // Letters, like the Write panel, has its candidate row on top.
        let cands_on_top = mode == InkMode::Letters;
        let by = panel.y + pad + if cands_on_top { bh + pad } else { 0 };
        let ui = self.ui_px();
        let mut buttons = Vec::new();
        let mut x = pad;
        let bw = |label: &str| {
            (self.fonts.width(Face::Atkinson, ui, &tr(label)) as i32 + self.px(24.0))
                .max(self.px(64.0))
        };
        let (left, right): (Row, Row) = match mode {
            InkMode::Draw => (
                &[(InkButton::Mode, &["Write", "Sketch"])],
                &[
                    (InkButton::Insert, &["Insert"]),
                    (InkButton::Cancel, &["Cancel"]),
                ],
            ),
            // Letters: Words stays left, Train and Done right, the keys
            // between them are centred (placed below); Free uses its own
            // layout (`free_layout`).
            InkMode::Letters | InkMode::Free => (
                &[(InkButton::Mode, &["Write", "Sketch"])],
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
        // The keys between the two ends of the row, centred.
        let mid: &[(InkButton, &str)] = match mode {
            InkMode::Letters => &[
                (InkButton::Undo, "Undo"),
                (InkButton::Redo, "Redo"),
                (InkButton::Back, "Erase"),
                (InkButton::Enter, "Return"),
                (InkButton::Space, "Space"),
            ],
            InkMode::Draw => &[
                (InkButton::Undo, "Undo"),
                (InkButton::Clear, "Clear"),
                (InkButton::Pen, "Stylus"),
                (InkButton::Guide, "Line"),
            ],
            _ => &[],
        };
        if !mid.is_empty() {
            let widths: Vec<i32> = mid.iter().map(|(_, l)| bw(l)).collect();
            let total = widths.iter().sum::<i32>() + pad * (mid.len() as i32 - 1);
            let free_l = x;
            let free_r = rx;
            let mut mx = (free_l + (free_r - free_l - total) / 2).max(free_l);
            for ((b, _), ww) in mid.iter().zip(&widths) {
                buttons.push((Rect::new(mx, by, *ww, bh), *b));
                mx += ww + pad;
            }
        }
        let mut top = by + bh + pad;
        if mode == InkMode::Letters {
            // A second row: the caret and page keys, centred.
            let widths: Vec<i32> = NAV.iter().map(|b| bw(label_of_nav(*b))).collect();
            let total = widths.iter().sum::<i32>() + pad * (NAV.len() as i32 - 1);
            let mut nx = ((w - total) / 2).max(pad);
            for (b, ww) in NAV.iter().zip(&widths) {
                buttons.push((Rect::new(nx, top, *ww, bh), *b));
                nx += ww + pad;
            }
            top += bh + pad;
        }
        // Sketch keeps a line of small print under the drawing.
        let fh = if mode == InkMode::Draw {
            self.px(28.0)
        } else {
            0
        };
        let footer = Rect::new(pad, panel.y + ph - pad - fh, w - 2 * pad, fh);
        let area = Rect::new(
            pad,
            top,
            w - 2 * pad,
            panel.y + ph - top - pad - if fh > 0 { fh + pad } else { 0 },
        );
        let mut boxes = Vec::new();
        let mut strip = Rect::new(area.x, area.y, area.w, 0);
        if mode != InkMode::Draw {
            let sh = bh + pad;
            let avail = if cands_on_top {
                strip = Rect::new(area.x, panel.y + pad, area.w, bh);
                area
            } else {
                strip = Rect::new(area.x, area.y, area.w, bh);
                Rect::new(area.x, area.y + sh, area.w, area.h - sh)
            };
            let n = match mode {
                InkMode::Letters => box_count(w, h),
                _ => 1,
            };
            let gap = pad;
            let side = ((avail.w - gap * (n as i32 - 1)) / n as i32)
                .min(avail.h)
                .max(1);
            let total = side * n as i32 + gap * (n as i32 - 1);
            let x0 = avail.x + (avail.w - total) / 2;
            let y0 = avail.y + (avail.h - side) / 2;
            if mode == InkMode::Free {
                // One writing line, the whole width, a Letters box high.
                let side = ((avail.w - gap * 4) / 5).min(avail.h).max(1);
                let y0 = avail.y + (avail.h - side) / 2;
                boxes.push(Rect::new(avail.x, y0, avail.w, side));
            } else {
                for i in 0..n {
                    boxes.push(Rect::new(x0 + i as i32 * (side + gap), y0, side, side));
                }
            }
            if matches!(mode, InkMode::Letters | InkMode::Free) {
                // Candidate chips, left to right (words need wider ones).
                let cw = if mode == InkMode::Free {
                    (strip.w - 4 * pad) / 5
                } else {
                    bh * 3 / 2
                };
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
            footer,
        }
    }

    /// The Free panel (v1.5.3, decision 55), after the Boox one: a
    /// full-width candidate bar (‹, words, ›, ×), a 3 × 4 grid of outlined
    /// keys on the left and an open writing area (no boxes, no baseline,
    /// no frame) on the right. Every target is at least 44 px.
    #[must_use]
    pub fn free_layout(&self, v: &LettersView) -> InkLayout {
        let (w, h) = (self.width as i32, self.height as i32);
        let ph = h * if w > h { 55 } else { 45 } / 100;
        let panel = Rect::new(0, h - ph, w, ph);
        let pad = self.px(8.0);
        let bh = self.px(48.0).max(44);
        let ui = self.ui_px();
        let mut buttons = Vec::new();
        let bar = Rect::new(0, panel.y + self.px(2.0).max(2), w, bh + pad);
        let by = bar.y + pad / 2;
        buttons.push((Rect::new(pad, by, bh, bh), InkButton::Prev));
        let xr = w - pad - bh;
        buttons.push((Rect::new(xr, by, bh, bh), InkButton::Cancel));
        let nx = xr - pad - bh;
        buttons.push((Rect::new(nx, by, bh, bh), InkButton::Next));
        // Candidates: plain words, a wide space apart, as many as fit.
        let left = pad + bh + pad * 2;
        let right = nx - pad * 2;
        let gap = self.fonts.width(Face::Atkinson, ui, "  ") as i32 + pad;
        let mut x = left;
        for (k, t) in v.cands.iter().enumerate().skip(v.first) {
            let tw = (self.fonts.width(Face::Atkinson, ui * 1.15, t) as i32 + pad).max(bh);
            if x + tw > right {
                break;
            }
            buttons.push((Rect::new(x, by, tw, bh), InkButton::Cand(k)));
            x += tw + gap;
        }
        // The key grid: 3 × 4, the space key two columns wide.
        let top = bar.y + bar.h + pad;
        let avail = panel.y + ph - pad - top;
        let kh = ((avail - 3 * pad) / 4).clamp(44, self.px(56.0).max(44));
        let kw = self.px(64.0).max(kh);
        let grid: [[Option<InkButton>; 3]; 4] = [
            [
                Some(InkButton::Mode),
                Some(InkButton::Type(',')),
                Some(InkButton::Type('.')),
            ],
            [
                Some(InkButton::Type('?')),
                Some(InkButton::Type('!')),
                Some(InkButton::Type('@')),
            ],
            [Some(InkButton::Digits), Some(InkButton::Lang), None],
            [Some(InkButton::Space), None, None],
        ];
        for (r, row) in grid.iter().enumerate() {
            for (c, b) in row.iter().enumerate() {
                let Some(b) = b else { continue };
                let span = if *b == InkButton::Space { 3 } else { 1 };
                let rect = Rect::new(
                    pad + c as i32 * (kw + pad),
                    top + r as i32 * (kh + pad),
                    kw * span + pad * (span - 1),
                    kh,
                );
                buttons.push((rect, *b));
            }
        }
        // Two rows above the writing area, as wide as the area: Undo,
        // Redo, Erase, Return; then Home, End, Insert, PgUp, PgDn.
        let ax = pad + 3 * kw + 3 * pad + pad;
        let rows: [&[InkButton]; 2] = [
            &[
                InkButton::Undo,
                InkButton::Redo,
                InkButton::Back,
                InkButton::Enter,
            ],
            &NAV,
        ];
        for (n, row) in rows.iter().enumerate() {
            let rw = ((w - pad - ax - (row.len() as i32 - 1) * pad) / row.len() as i32).max(44);
            let y = top + n as i32 * (kh + pad);
            for (k, b) in row.iter().enumerate() {
                buttons.push((Rect::new(ax + k as i32 * (rw + pad), y, rw, kh), *b));
            }
        }
        let ay = top + 2 * (kh + pad);
        let area = Rect::new(ax, ay, w - ax - pad, panel.y + ph - ay - pad);
        InkLayout {
            baseline_y: area.y + area.h * 7 / 10,
            panel,
            area,
            buttons,
            boxes: vec![area],
            strip: bar,
            footer: Rect::new(0, 0, 0, 0),
        }
    }

    /// The panel layout for a mode and what it shows.
    #[must_use]
    pub fn ink_layout_for(&self, mode: InkMode, v: &LettersView) -> InkLayout {
        if mode == InkMode::Free {
            self.free_layout(v)
        } else {
            self.ink_layout_in(mode)
        }
    }

    /// Text in 1-bit ink (coverage ≥ 50 % → `ink`): the Free panel stays
    /// pure black and white, DU-safe.
    fn text_1bit(&self, c: &mut Canvas, face: Face, size: f32, x: f32, base: f32, t: &str) {
        let tw = self.fonts.width(face, size, t).ceil() as i32 + 4;
        let th = (size * 1.6).ceil() as i32;
        let (ox, oy) = (x.floor() as i32 - 2, (base - size * 1.15).floor() as i32);
        let mut tmp = Canvas::new(tw.max(1) as u32, th.max(1) as u32, WHITE);
        self.fonts.draw(
            &mut tmp,
            face,
            size,
            x - ox as f32,
            base - oy as f32,
            t,
            BLACK,
        );
        let b = c.bounds();
        for yy in 0..th {
            for xx in 0..tw {
                if tmp.pixels[(yy * tw + xx) as usize] < 128 && b.contains(ox + xx, oy + yy) {
                    c.pixels[((oy + yy) * c.width as i32 + ox + xx) as usize] = BLACK;
                }
            }
        }
    }

    /// Draws the Free panel: all pure black on pure white.
    pub fn draw_free(&self, c: &mut Canvas, l: &InkLayout, ink: &Ink, pen: Pen, v: &LettersView) {
        use crate::icons::{draw_icon_1bit, Icon};
        c.fill(l.panel, WHITE);
        let rule = self.px(2.0).max(2);
        let thin = self.px(1.0).max(1);
        c.fill(Rect::new(l.panel.x, l.panel.y, l.panel.w, rule), BLACK);
        let bar = l.strip;
        c.fill(Rect::new(0, bar.y + bar.h, l.panel.w, thin), BLACK);
        let ui = self.ui_px();
        let rad = self.px(8.0).max(4);
        let line = self.px(1.5).max(1);
        let mut cand_x = (i32::MAX, i32::MIN);
        for (r, b) in &l.buttons {
            let icon = match b {
                InkButton::Prev => Some(Icon::PrevPage),
                InkButton::Next => Some(Icon::NextPage),
                InkButton::Cancel => Some(Icon::Close),
                InkButton::Mode => Some(Icon::Swap),
                InkButton::Back => Some(Icon::Backspace),
                InkButton::Enter => Some(Icon::Return),
                InkButton::Lang => Some(Icon::Globe),
                InkButton::Space => Some(Icon::Space),
                InkButton::Undo => Some(Icon::Undo),
                InkButton::Redo => Some(Icon::Redo),
                InkButton::Home => Some(Icon::LineStart),
                InkButton::End => Some(Icon::LineEnd),
                InkButton::PgUp => Some(Icon::PageUp),
                InkButton::PgDn => Some(Icon::PageDown),
                _ => None,
            };
            let key = !matches!(
                b,
                InkButton::Prev | InkButton::Next | InkButton::Cancel | InkButton::Cand(_)
            );
            if key {
                round_outline(c, *r, rad, line);
            }
            if let Some(i) = icon {
                draw_icon_1bit(c, i, *r, BLACK);
            }
            let text = match b {
                InkButton::Cand(k) => {
                    cand_x = (cand_x.0.min(r.x), cand_x.1.max(r.x + r.w));
                    v.cands.get(*k).cloned()
                }
                InkButton::Type(ch) => Some(ch.to_string()),
                InkButton::Digits => Some(if v.digits { "abc".into() } else { "123".into() }),
                InkButton::Ins => Some(label_of(*b, Pen::Finger, InkMode::Free, false)),
                _ => None,
            };
            if let Some(t) = text {
                let size = if matches!(b, InkButton::Cand(_)) {
                    ui * 1.15
                } else {
                    ui
                };
                let tw = self.fonts.width(Face::Atkinson, size, &t);
                self.text_1bit(
                    c,
                    Face::Atkinson,
                    size,
                    r.x as f32 + (r.w as f32 - tw) / 2.0,
                    (r.y + r.h / 2) as f32 + size * 0.35,
                    &t,
                );
            }
            if *b == InkButton::Lang {
                if let Some(code) = v.lang {
                    let s = ui * 0.7;
                    self.text_1bit(
                        c,
                        Face::Atkinson,
                        s,
                        (r.x + r.w - rad) as f32 - self.fonts.width(Face::Atkinson, s, code),
                        (r.y + r.h - rad / 2) as f32,
                        &code.to_uppercase(),
                    );
                }
            }
        }
        // Thin separators after ‹ and before ›.
        let sep_h = bar.h * 5 / 10;
        let sy = bar.y + (bar.h - sep_h) / 2;
        for (r, b) in &l.buttons {
            match b {
                InkButton::Prev => {
                    c.fill(Rect::new(r.x + r.w + self.px(6.0), sy, thin, sep_h), BLACK)
                }
                InkButton::Next => c.fill(Rect::new(r.x - self.px(6.0), sy, thin, sep_h), BLACK),
                _ => {}
            }
        }
        let _ = cand_x;
        draw_strokes(c, ink, pen_px(pen), l.area);
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
        // Same look as the Write panel (`draw_free`): outlined keys, plain
        // candidate words, pure black on white.
        let thin = self.px(1.0).max(1);
        c.fill(l.panel, WHITE);
        c.fill(
            Rect::new(l.panel.x, l.panel.y, l.panel.w, self.px(2.0).max(2)),
            BLACK,
        );
        let ui = self.ui_px();
        let line = self.px(1.5).max(1);
        let rad = self.px(8.0).max(4);
        let rule_y = l.strip.y + l.strip.h + self.px(4.0);
        if !matches!(mode, InkMode::Train(_)) {
            c.fill(Rect::new(0, rule_y, l.panel.w, thin), BLACK);
        }
        for (r, b) in &l.buttons {
            let cand = matches!(b, InkButton::Cand(_));
            let t = match b {
                InkButton::Cand(k) => match v.cands.get(*k) {
                    Some(ch) => ch.clone(),
                    None => continue,
                },
                _ => label_of(*b, pen, mode, self.ink_words),
            };
            let filled = matches!(b, InkButton::Save);
            if filled {
                c.fill(*r, BLACK);
            } else if !cand {
                round_outline(c, *r, rad, line);
            }
            let icon = match b {
                InkButton::Undo => Some(crate::icons::Icon::Undo),
                InkButton::Redo => Some(crate::icons::Icon::Redo),
                InkButton::Back => Some(crate::icons::Icon::Backspace),
                InkButton::Enter => Some(crate::icons::Icon::Return),
                InkButton::Space => Some(crate::icons::Icon::Space),
                InkButton::Home => Some(crate::icons::Icon::LineStart),
                InkButton::End => Some(crate::icons::Icon::LineEnd),
                InkButton::PgUp => Some(crate::icons::Icon::PageUp),
                InkButton::PgDn => Some(crate::icons::Icon::PageDown),
                _ => None,
            };
            if let Some(i) = icon {
                crate::icons::draw_icon_1bit(c, i, *r, BLACK);
                continue;
            }
            let size = if cand { ui * 1.15 } else { ui };
            let tw = self.fonts.width(Face::Atkinson, size, &t);
            let (tx, ty) = (
                r.x as f32 + (r.w as f32 - tw) / 2.0,
                (r.y + r.h / 2) as f32 + size * 0.35,
            );
            if filled {
                self.fonts.draw(c, Face::Atkinson, size, tx, ty, &t, WHITE);
            } else {
                self.text_1bit(c, Face::Atkinson, size, tx, ty, &t);
            }
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
            self.text_1bit(
                c,
                Face::Atkinson,
                ui * 1.2,
                l.strip.x as f32,
                (l.strip.y + l.strip.h / 2) as f32 + ui * 0.42,
                &t,
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
        // Same look as the Write panel: outlined rounded keys, 1-bit text.
        c.fill(l.panel, WHITE);
        c.fill(
            Rect::new(l.panel.x, l.panel.y, l.panel.w, self.px(2.0).max(2)),
            BLACK,
        );
        let ui = self.ui_px();
        let line = self.px(1.5).max(1);
        let rad = self.px(8.0).max(4);
        for (r, b) in &l.buttons {
            // Insert is the one dark key; Line is dark while the guide shows.
            let insert = *b == InkButton::Insert || (*b == InkButton::Guide && self.sketch_guide);
            if insert {
                round_outline_in(c, *r, rad, 0, BLACK);
            } else {
                round_outline(c, *r, rad, line);
            }
            if *b == InkButton::Undo {
                crate::icons::draw_icon_1bit(c, crate::icons::Icon::Undo, *r, BLACK);
                continue;
            }
            let t = label_of(*b, pen, InkMode::Draw, self.ink_words);
            let tw = self.fonts.width(Face::Atkinson, ui, &t);
            let (tx, ty) = (
                r.x as f32 + (r.w as f32 - tw) / 2.0,
                (r.y + r.h / 2) as f32 + ui * 0.35,
            );
            if insert {
                self.fonts.draw(c, Face::Atkinson, ui, tx, ty, &t, WHITE);
            } else {
                self.text_1bit(c, Face::Atkinson, ui, tx, ty, &t);
            }
        }
        // The footer: what Insert does, in small print.
        let note = tr("In Write mode a sketch is added to the note as an image link. Preview shows the whole sketch.");
        let size = ui * 0.8;
        let nw = self.fonts.width(Face::Atkinson, size, &note);
        let size = if nw > l.footer.w as f32 {
            size * l.footer.w as f32 / nw
        } else {
            size
        };
        self.text_1bit(
            c,
            Face::Atkinson,
            size,
            l.footer.x as f32,
            (l.footer.y + l.footer.h / 2) as f32 + size * 0.35,
            &note,
        );
        // Dotted baseline: 1-px black dots every 8 px (no grey).
        let dot = self.px(2.0).max(1);
        let mut x = l.area.x;
        while self.sketch_guide && x < l.area.x + l.area.w {
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
