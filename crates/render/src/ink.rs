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
/// Panel ink: pure black.
pub const BLACK: u8 = 0;

/// A button of the writing panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InkButton {
    /// Takes back the last step.
    Undo,
    /// Sketch: shows or hides the dotted guide line (v1.13.1).
    Guide,
    /// Takes back an Undo of typed text (v1.13.1).
    Redo,
    /// Sketch: removes every stroke.
    Clear,
    /// Sketch: switches between finger and stylus.
    Pen,
    /// Sketch: closes the panel ("Cancel"); elsewhere labelled "Done".
    Cancel,
    /// Sketch: puts the drawing into the note.
    Insert,
    /// Draw ↔ Letters (v1.5.2).
    Mode,
    /// Types a space.
    Space,
    /// Erases one character ("Erase").
    Back,
    /// Types a line break ("Return").
    Enter,
    /// Opens enrolment ("Train").
    Train,
    /// Enrolment: moves on to the next character without saving.
    Skip,
    /// Enrolment: saves the drawn sample.
    Save,
    /// The n-th candidate of the last letter.
    Cand(usize),
    /// Free panel (v1.5.3): ‹ › candidate pages, a key that types its
    /// character, 123 (digits) and the language key.
    Prev,
    /// Next page of candidates (›).
    Next,
    /// A key that types its character.
    Type(char),
    /// 123: limits candidates to digits and signs.
    Digits,
    /// The language key: cycles the language filter.
    Lang,
    /// Caret and page keys of the panels (v1.11.2).
    Home,
    /// Caret to the end.
    End,
    /// Page up.
    PgUp,
    /// Page down.
    PgDn,
    /// Insert / overwrite.
    Ins,
    /// Moves the key block to the other side of the panel (v1.16).
    Side,
}

/// What the panel shows: free ink, character boxes, or enrolment of the
/// n-th character of `sigma_hwr::CHARSET`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InkMode {
    /// Sketch: free ink, inserted as a drawing.
    #[default]
    Draw,
    /// Handwriting in square character boxes.
    Letters,
    /// Enrolment of the n-th character of `sigma_hwr::CHARSET`.
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
    /// Free: language code shown on the language key (`None`: no filter).
    pub lang: Option<&'static str>,
}

/// Where the parts of the writing panel go, in screen pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InkLayout {
    /// The whole panel (white).
    pub panel: Rect,
    /// Where ink goes; touches elsewhere never ink.
    pub area: Rect,
    /// y of the writing baseline (in Sketch, where the dotted guide goes).
    pub baseline_y: i32,
    /// Every button and where it is.
    pub buttons: Vec<(Rect, InkButton)>,
    /// Letters / Train (v1.5.2): the square character boxes (ink only
    /// goes in a box); empty in Draw.
    pub boxes: Vec<Rect>,
    /// Where the candidate row or the enrolment prompt goes.
    pub strip: Rect,
    /// Sketch (Draw): the line of small print under the drawing area;
    /// empty elsewhere.
    pub footer: Rect,
    /// Hairlines between the groups of the key block (v1.16).
    pub rules: Vec<Rect>,
}

/// Share of the panel width the key block takes, in % (v1.16): about a
/// third, so the other two thirds are for writing.
pub const KEY_BLOCK: i32 = 32;

/// A row of the key block: keys and how many of its 3 columns each takes
/// (`None` leaves a gap).
type BlockRow = &'static [(Option<InkButton>, i32)];

/// A group of rows that belong together, set apart from the next group.
type BlockGroup = &'static [BlockRow];

/// The key block of a mode, in groups by what the keys do (v1.16): the
/// panel (mode switch, side switch, Done, Line, Train), editing (Undo,
/// Redo, Erase, Return, Space), typing (Write: punctuation, 123, the
/// language key) and moving (Home, End, Ins, PgUp, PgDn); Train and Sketch
/// have their own actions after the panel group. Every mode starts with
/// the same panel group, so the panels look alike.
fn block_groups(mode: InkMode) -> &'static [BlockGroup] {
    use InkButton as B;
    const PANEL: BlockGroup = &[
        &[(Some(B::Mode), 1), (Some(B::Side), 1), (Some(B::Cancel), 1)],
        &[(Some(B::Guide), 1), (Some(B::Train), 1)],
    ];
    const EDIT: BlockGroup = &[
        &[(Some(B::Undo), 1), (Some(B::Redo), 1), (Some(B::Back), 1)],
        &[(Some(B::Enter), 1), (Some(B::Space), 2)],
    ];
    const TYPE: BlockGroup = &[
        &[
            (Some(B::Type(',')), 1),
            (Some(B::Type('.')), 1),
            (Some(B::Type('?')), 1),
        ],
        &[(Some(B::Type('!')), 1), (Some(B::Type('@')), 1)],
        &[(Some(B::Digits), 1), (Some(B::Lang), 1)],
    ];
    const MOVE: BlockGroup = &[
        &[(Some(B::Home), 1), (Some(B::End), 1), (Some(B::Ins), 1)],
        &[(Some(B::PgUp), 1), (Some(B::PgDn), 1)],
    ];
    match mode {
        InkMode::Free => &[PANEL, EDIT, TYPE, MOVE],
        InkMode::Letters => &[PANEL, EDIT, MOVE],
        InkMode::Train(_) => &[
            &[
                &[(None, 1), (Some(B::Side), 1), (Some(B::Cancel), 1)],
                &[(Some(B::Guide), 1)],
            ],
            &[&[(Some(B::Undo), 1), (Some(B::Skip), 1), (Some(B::Save), 1)]],
        ],
        InkMode::Draw => &[
            &[
                &[(Some(B::Mode), 1), (Some(B::Side), 1), (Some(B::Cancel), 1)],
                &[(Some(B::Guide), 1)],
            ],
            &[
                &[(Some(B::Undo), 1), (Some(B::Clear), 1), (Some(B::Pen), 1)],
                &[(Some(B::Insert), 2)],
            ],
        ],
    }
}

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
        | InkButton::Lang
        | InkButton::Side => "",
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
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
    /// One flag per pixel, row by row; `true` is ink.
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
        let on = pm
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[3] >= 128)
            .collect();
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

    /// The panel for a mode (v1.5.2), with no candidates.
    #[must_use]
    pub fn ink_layout_in(&self, mode: InkMode) -> InkLayout {
        self.panel_layout(mode, &LettersView::default())
    }

    /// The Write panel (v1.5.3) for what it shows.
    #[must_use]
    pub fn free_layout(&self, v: &LettersView) -> InkLayout {
        self.panel_layout(InkMode::Free, v)
    }

    /// The panel layout for a mode and what it shows.
    #[must_use]
    pub fn ink_layout_for(&self, mode: InkMode, v: &LettersView) -> InkLayout {
        self.panel_layout(mode, v)
    }

    /// Every mode of the panel has the same shape (v1.16): a block of keys
    /// [`KEY_BLOCK`] % of the width on one side ([`EditorScreen::ink_keys_right`]
    /// picks which), a grid of 3 columns that always starts with the mode
    /// switch, the side switch and Done; the rest of the width is for
    /// writing, with the candidate bar (or the Train prompt) along its top.
    fn panel_layout(&self, mode: InkMode, v: &LettersView) -> InkLayout {
        let (w, h) = (self.width as i32, self.height as i32);
        let ph = h * if w > h { 55 } else { 45 } / 100;
        let panel = Rect::new(0, h - ph, w, ph);
        let pad = self.px(8.0);
        let top = panel.y + self.px(2.0).max(2) + pad;
        let bottom = panel.y + ph - pad;
        // The key block, at least three 44 px keys wide.
        let bw = (w * KEY_BLOCK / 100).max(3 * 44 + 4 * pad);
        let (bx, wx) = if self.ink_keys_right {
            (w - bw, pad)
        } else {
            (0, bw + pad)
        };
        let ww = w - bw - 2 * pad;
        // Groups are set apart by a wider gap with a hairline in it.
        let gap = pad * 3;
        // Keys are as tall in every mode, sized for the mode with the most
        // rows (Write), so switching modes keeps them in place.
        let most = block_groups(InkMode::Free);
        let (rows_n, groups_n) = (
            most.iter().map(|g| g.len() as i32).sum::<i32>(),
            most.len() as i32,
        );
        let kh = ((bottom - top - (rows_n - groups_n) * pad - (groups_n - 1) * gap) / rows_n)
            .clamp(44, self.px(64.0).max(44));
        let kw = (bw - 4 * pad) / 3;
        let mut buttons = Vec::new();
        let mut rules = Vec::new();
        let mut y = top;
        for (g, group) in block_groups(mode).iter().enumerate() {
            if g > 0 {
                y += gap - pad;
                rules.push(Rect::new(bx + pad, y - gap / 2, bw - 2 * pad, 1));
            }
            for row in *group {
                let mut col = 0;
                for &(b, span) in *row {
                    if let Some(b) = b {
                        let r = Rect::new(
                            bx + pad + col * (kw + pad),
                            y,
                            kw * span + pad * (span - 1),
                            kh,
                        );
                        buttons.push((r, b));
                    }
                    col += span;
                }
                y += kh + pad;
            }
        }
        // The writing side: a strip on top (candidates or the prompt).
        let bh = self.px(48.0).max(44);
        let strip = if mode == InkMode::Draw {
            Rect::new(wx, top, ww, 0)
        } else {
            Rect::new(wx, top, ww, bh)
        };
        match mode {
            InkMode::Free => {
                buttons.push((Rect::new(strip.x, strip.y, bh, bh), InkButton::Prev));
                let nx = strip.x + strip.w - bh;
                buttons.push((Rect::new(nx, strip.y, bh, bh), InkButton::Next));
                // Candidates: plain words, a wide space apart, as many as fit.
                let ui = self.ui_px();
                let gap = self.fonts.width(Face::Atkinson, ui, "  ") as i32 + pad;
                let (mut x, right) = (strip.x + bh + pad * 2, nx - pad * 2);
                for (k, t) in v.cands.iter().enumerate().skip(v.first) {
                    let tw = (self.fonts.width(Face::Atkinson, ui * 1.15, t) as i32 + pad).max(bh);
                    if x + tw > right {
                        break;
                    }
                    buttons.push((Rect::new(x, strip.y, tw, bh), InkButton::Cand(k)));
                    x += tw + gap;
                }
            }
            InkMode::Letters => {
                let cw = ((strip.w - 4 * pad) / 5).min(bh * 2);
                for k in 0..5 {
                    let r = Rect::new(strip.x + k * (cw + pad), strip.y, cw, bh);
                    buttons.push((r, InkButton::Cand(k as usize)));
                }
            }
            _ => {}
        }
        let below = strip.y + strip.h + if strip.h > 0 { pad } else { 0 };
        // Sketch keeps a line of small print under the drawing.
        let fh = if mode == InkMode::Draw {
            self.px(28.0)
        } else {
            0
        };
        let footer = Rect::new(wx, bottom - fh, ww, fh);
        let area = Rect::new(
            wx,
            below,
            ww,
            bottom - below - if fh > 0 { fh + pad } else { 0 },
        );
        let mut boxes = Vec::new();
        match mode {
            InkMode::Free => boxes.push(area),
            InkMode::Letters | InkMode::Train(_) => {
                let n = if mode == InkMode::Letters {
                    box_count(w, h)
                } else {
                    1
                };
                let side = ((area.w - pad * (n as i32 - 1)) / n as i32)
                    .min(area.h)
                    .max(1);
                let total = side * n as i32 + pad * (n as i32 - 1);
                let x0 = area.x + (area.w - total) / 2;
                let y0 = area.y + (area.h - side) / 2;
                for i in 0..n {
                    boxes.push(Rect::new(x0 + i as i32 * (side + pad), y0, side, side));
                }
            }
            InkMode::Draw => {}
        }
        InkLayout {
            baseline_y: match boxes.first() {
                Some(b) if mode != InkMode::Free => b.y + (b.h as f32 * sigma_hwr::BASELINE) as i32,
                _ => area.y + area.h * 6 / 10,
            },
            panel,
            area,
            buttons,
            boxes,
            strip,
            footer,
            rules,
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

    /// The Write panel (free writing).
    pub fn draw_free(&self, c: &mut Canvas, l: &InkLayout, ink: &Ink, pen: Pen, v: &LettersView) {
        self.draw_panel(c, l, ink, pen, InkMode::Free, v);
    }

    /// The letter boxes / enrolment panel (v1.5.2).
    pub fn draw_letters(
        &self,
        c: &mut Canvas,
        l: &InkLayout,
        ink: &Ink,
        pen: Pen,
        mode: InkMode,
        v: &LettersView,
    ) {
        self.draw_panel(c, l, ink, pen, mode, v);
    }

    /// The Sketch panel.
    pub fn draw_ink(&self, c: &mut Canvas, l: &InkLayout, ink: &Ink, pen: Pen) {
        self.draw_panel(c, l, ink, pen, InkMode::Draw, &LettersView::default());
    }

    /// Any mode of the panel: white, a black rule on top and one between
    /// the keys and the writing side, outlined rounded keys with icons or
    /// 1-bit text (Insert, Save and Line while the guide shows are dark),
    /// the candidates or prompt, the dotted guide (when on) and the
    /// strokes. Pure black and white throughout (DU-safe).
    fn draw_panel(
        &self,
        c: &mut Canvas,
        l: &InkLayout,
        ink: &Ink,
        pen: Pen,
        mode: InkMode,
        v: &LettersView,
    ) {
        use crate::icons::{draw_icon_1bit, Icon};
        c.fill(l.panel, WHITE);
        let rule = self.px(2.0).max(2);
        let thin = self.px(1.0).max(1);
        c.fill(Rect::new(l.panel.x, l.panel.y, l.panel.w, rule), BLACK);
        // The rule between the key block and the writing side.
        let pad = self.px(8.0);
        let sep_x = if self.ink_keys_right {
            l.area.x + l.area.w + pad / 2
        } else {
            l.area.x - pad / 2
        };
        c.fill(
            Rect::new(sep_x, l.panel.y + rule, thin, l.panel.h - rule),
            BLACK,
        );
        for r in &l.rules {
            c.fill(Rect::new(r.x, r.y, r.w, thin), BLACK);
        }
        if l.strip.h > 0 {
            let y = l.strip.y + l.strip.h + pad / 2;
            c.fill(Rect::new(l.strip.x, y, l.strip.w, thin), BLACK);
        }
        let ui = self.ui_px();
        let rad = self.px(8.0).max(4);
        let line = self.px(1.5).max(1);
        for (r, b) in &l.buttons {
            let icon = match b {
                InkButton::Prev => Some(Icon::PrevPage),
                InkButton::Next => Some(Icon::NextPage),
                InkButton::Side => Some(Icon::Swap),
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
            let cand = matches!(b, InkButton::Cand(_));
            let text = match b {
                InkButton::Cand(k) => match v.cands.get(*k) {
                    Some(t) => Some(t.clone()),
                    None => continue,
                },
                InkButton::Type(ch) => Some(ch.to_string()),
                InkButton::Digits => Some(if v.digits { "abc".into() } else { "123".into() }),
                _ if icon.is_some() => None,
                _ => Some(label_of(*b, pen, mode, self.ink_words)),
            };
            let dark = matches!(b, InkButton::Insert | InkButton::Save)
                || (*b == InkButton::Guide && self.sketch_guide);
            if dark {
                round_outline_in(c, *r, rad, 0, BLACK);
            } else if !cand && !matches!(b, InkButton::Prev | InkButton::Next) {
                round_outline(c, *r, rad, line);
            }
            if let Some(i) = icon {
                draw_icon_1bit(c, i, *r, BLACK);
            }
            if let Some(t) = text {
                let size = if cand { ui * 1.15 } else { ui };
                // A label wider than its key is drawn smaller, never clipped.
                let room = (r.w - 2 * rad) as f32;
                let tw = self.fonts.width(Face::Atkinson, size, &t);
                let size = if tw > room && !cand {
                    size * room / tw
                } else {
                    size
                };
                let tw = self.fonts.width(Face::Atkinson, size, &t);
                let (tx, ty) = (
                    r.x as f32 + (r.w as f32 - tw) / 2.0,
                    (r.y + r.h / 2) as f32 + size * 0.35,
                );
                if dark {
                    self.fonts.draw(c, Face::Atkinson, size, tx, ty, &t, WHITE);
                } else {
                    self.text_1bit(c, Face::Atkinson, size, tx, ty, &t);
                }
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
        let sep_h = l.strip.h * 5 / 10;
        let sy = l.strip.y + (l.strip.h - sep_h) / 2;
        for (r, b) in &l.buttons {
            match b {
                InkButton::Prev => {
                    c.fill(Rect::new(r.x + r.w + self.px(6.0), sy, thin, sep_h), BLACK);
                }
                InkButton::Next => c.fill(Rect::new(r.x - self.px(6.0), sy, thin, sep_h), BLACK),
                _ => {}
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
        if mode == InkMode::Draw {
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
        }
        // Letter boxes are outlined; the dotted guide (1-bit dots) goes
        // through each box at the baseline, or across the writing area.
        let dot = self.px(2.0).max(1);
        let dotted = |c: &mut Canvas, x0: i32, x1: i32, y: i32| {
            let mut x = x0;
            while x < x1 {
                c.fill(Rect::new(x, y, dot, dot), BLACK);
                x += dot * 4;
            }
        };
        let boxed = matches!(mode, InkMode::Letters | InkMode::Train(_));
        if boxed {
            for b in &l.boxes {
                c.stroke(*b, line, BLACK);
                if self.sketch_guide {
                    let by = b.y + (b.h as f32 * sigma_hwr::BASELINE) as i32;
                    dotted(c, b.x + dot * 2, b.x + b.w - dot * 2, by);
                }
            }
        } else if self.sketch_guide {
            dotted(c, l.area.x, l.area.x + l.area.w, l.baseline_y);
        }
        let clip = if boxed {
            l.boxes
                .iter()
                .copied()
                .reduce(|a, b| a.union(&b))
                .unwrap_or(l.area)
        } else {
            l.area
        };
        draw_strokes(c, ink, pen_px(pen), clip);
    }

    /// The button at (x, y), with a few pixels of slop around each.
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
