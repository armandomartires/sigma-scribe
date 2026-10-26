//! Preview mode (brief §4, S3): lays parsed Markdown out into lines with
//! the real faces, cuts the lines into pages for the page-turn buttons, and
//! draws one page. Read-only: nothing here maps a point back to the source.
//!
//! Styles follow the mock (`.preview` CSS): headings Literata 600 at
//! 1.45 / 1.22 / 1.05 em with line height 1.25; blocks 0.8 em apart; lists
//! indented 1.2 em per level; task boxes 14 px outlined (filled when done);
//! quotes with a 2 px bar and muted text; inline and block code in the UI
//! face at 0.86 em on the wash tone; links underlined; rules 1 px ink.

use crate::canvas::{Canvas, Rect};
use crate::fonts::{Face, Fonts};
use crate::theme::{Scale, Theme};
use sigma_core::markdown::{Block, BlockKind, LinkKind, Run, Style};
use std::ops::Range;

/// Text colour of a piece, taken from the theme when drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Normal text.
    Ink,
    /// Muted text (quotes, missing wiki links, images).
    Mute,
}

/// How a link is underlined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Underline {
    /// A link to a web address or an existing note.
    Solid,
    /// A `[[wiki]]` link to a note that does not exist (yet).
    Dashed,
}

/// A run of same-styled text placed on a line.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// Left edge, in px from the left of the text column.
    pub x: i32,
    /// Font face.
    pub face: Face,
    /// Font size in px.
    pub size: f32,
    /// The text to draw.
    pub text: String,
    /// Text colour.
    pub tone: Tone,
    /// Link underline, if any.
    pub underline: Option<Underline>,
    /// Struck through.
    pub strike: bool,
    /// Inline code: wash behind the text.
    pub wash: bool,
    /// `[[wiki]]` target: a tap opens (or creates) that note (v0.6).
    pub target: Option<String>,
}

/// A decoration drawn on a line besides its text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deco {
    /// Code block background from `x`, `w` wide, the full line height.
    Wash {
        /// Left edge, in px from the left of the text column.
        x: i32,
        /// Width in px.
        w: i32,
    },
    /// Quote bar at `x`.
    Bar {
        /// Left edge, in px from the left of the text column.
        x: i32,
    },
    /// Task checkbox: left edge, side, done.
    Box {
        /// Left edge, in px from the left of the text column.
        x: i32,
        /// Side length in px.
        side: i32,
        /// The task is done (box filled).
        on: bool,
    },
    /// Horizontal rule across the text width.
    Rule,
}

/// One laid-out line of a preview, or a spacer between blocks.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// Height in px.
    pub h: i32,
    /// Baseline from the top of the line.
    pub base: i32,
    /// The text on the line, left to right.
    pub pieces: Vec<Piece>,
    /// Decorations drawn on the line.
    pub deco: Vec<Deco>,
    /// Source line of the block this line belongs to.
    pub src: usize,
    /// Vertical space between blocks: dropped at the top of a page.
    pub spacer: bool,
    /// An ink block drawn on this line (v1.5.1), at x = 0.
    pub image: Option<std::sync::Arc<crate::ink::Raster>>,
}

/// A laid-out note: lines and the pages they fall into.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Doc {
    /// Every line, in order.
    pub lines: Vec<Line>,
    /// Each page as a range of `lines`.
    pub pages: Vec<Range<usize>>,
    /// Width of the text column the lines were laid out for, in px.
    pub width: i32,
}

impl Doc {
    /// Source lines of the rendered headings (v1.0.3 Contents in preview).
    #[must_use]
    pub fn heading_lines(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self
            .lines
            .iter()
            .filter(|l| !l.spacer && is_heading(l))
            .map(|l| l.src)
            .collect();
        v.dedup();
        v
    }

    /// Page that shows source line `src` (or the last page before it).
    #[must_use]
    pub fn page_of_line(&self, src: usize) -> usize {
        self.pages
            .iter()
            .rposition(|p| {
                self.lines[p.clone()]
                    .iter()
                    .find(|l| !l.spacer)
                    .is_some_and(|l| l.src <= src)
            })
            .unwrap_or(0)
    }
}

/// Typography inputs (the editor's body size and leading, in panel px).
#[derive(Clone, Copy, Debug)]
pub struct Type {
    /// Body text size in px.
    pub body: f32,
    /// Body line height as a multiple of the size.
    pub leading: f32,
    /// Screen scale, for fixed-size parts such as task boxes.
    pub scale: Scale,
}

#[derive(Clone, Debug, PartialEq)]
struct Look {
    face: Face,
    size: f32,
    tone: Tone,
    underline: Option<Underline>,
    strike: bool,
    wash: bool,
    target: Option<String>,
}

struct Builder<'a> {
    fonts: &'a Fonts,
    width: i32,
    lines: Vec<Line>,
}

impl Builder<'_> {
    fn spacer(&mut self, h: i32, src: usize) {
        if h > 0 {
            self.lines.push(Line {
                h,
                base: 0,
                pieces: Vec::new(),
                deco: Vec::new(),
                src,
                spacer: true,
                image: None,
            });
        }
    }

    /// Greedy word wrap of styled tokens into lines of `width - indent`.
    #[allow(clippy::too_many_arguments)]
    fn flow(
        &mut self,
        tokens: &[(String, Look)],
        indent: i32,
        first_indent: i32,
        line_h: i32,
        base: i32,
        deco_first: &[Deco],
        deco_all: &[Deco],
        src: usize,
    ) {
        let mut pieces: Vec<Piece> = Vec::new();
        let mut first = true;
        let mut x = first_indent;
        // Exact pen position: rounding each word would open gaps between
        // differently styled pieces.
        let mut xf = first_indent as f32;
        let start = |pieces: &mut Vec<Piece>, first: &mut bool, b: &mut Builder<'_>| {
            let mut deco = deco_all.to_vec();
            if *first {
                deco.extend_from_slice(deco_first);
            }
            // A right-to-left line sits flush right (v1.8).
            let text: String = pieces.iter().map(|p| p.text.as_str()).collect();
            if b.fonts.row_rtl(&text) {
                if let Some(last) = pieces.last() {
                    let end =
                        last.x + b.fonts.width(last.face, last.size, &last.text).ceil() as i32;
                    let shift = (b.width - end).max(0);
                    for p in pieces.iter_mut() {
                        p.x += shift;
                    }
                }
            }
            b.lines.push(Line {
                h: line_h,
                base,
                pieces: std::mem::take(pieces),
                deco,
                src,
                spacer: false,
                image: None,
            });
            *first = false;
        };
        let max = self.width;
        for (text, look) in tokens {
            let w = |s: &str| self.fonts.width(look.face, look.size, s).ceil() as i32;
            let word = text.trim_end();
            let ww = w(word);
            let lead = if first { first_indent } else { indent };
            if x + ww > max && x > lead {
                trim_last(&mut pieces);
                start(&mut pieces, &mut first, self);
                x = indent;
                xf = indent as f32;
            }
            let lead = if first { first_indent } else { indent };
            let mut text = if x == lead {
                text.trim_start()
            } else {
                text.as_str()
            }
            .to_owned();
            // A word wider than the line is cut by characters.
            while w(text.trim_end()) > max - x && x == lead && text.chars().count() > 1 {
                let mut cut = String::new();
                for ch in text.chars() {
                    let mut t = cut.clone();
                    t.push(ch);
                    if w(&t) > max - x && !cut.is_empty() {
                        break;
                    }
                    cut = t;
                }
                let rest = text[cut.len()..].to_owned();
                if rest.is_empty() {
                    break;
                }
                put(&mut pieces, x, &cut, look);
                start(&mut pieces, &mut first, self);
                x = indent;
                xf = indent as f32;
                text = rest;
            }
            if text.is_empty() {
                continue;
            }
            put(&mut pieces, x, &text, look);
            xf += self.fonts.width(look.face, look.size, &text);
            x = xf.ceil() as i32;
        }
        trim_last(&mut pieces);
        if !pieces.is_empty() || first {
            start(&mut pieces, &mut first, self);
        }
    }
}

fn trim_last(pieces: &mut [Piece]) {
    if let Some(p) = pieces.last_mut() {
        let t = p.text.trim_end().len();
        p.text.truncate(t);
    }
}

fn put(pieces: &mut Vec<Piece>, x: i32, text: &str, l: &Look) {
    if let Some(p) = pieces.last_mut() {
        if p.face == l.face
            && p.size == l.size
            && p.tone == l.tone
            && p.underline == l.underline
            && p.strike == l.strike
            && p.wash == l.wash
            && p.target == l.target
        {
            p.text.push_str(text);
            return;
        }
    }
    pieces.push(Piece {
        x,
        face: l.face,
        size: l.size,
        text: text.to_owned(),
        tone: l.tone,
        underline: l.underline,
        strike: l.strike,
        wash: l.wash,
        target: l.target.clone(),
    });
}

/// Splits runs into words that keep their trailing spaces.
fn tokens(
    runs: &[Run],
    size: f32,
    heading: bool,
    tone: Tone,
    known: &dyn Fn(&str) -> bool,
) -> Vec<(String, Look)> {
    let mut out = Vec::new();
    for r in runs {
        let look = look(r, size, heading, tone, known);
        let text = if r.style.link == Some(LinkKind::Image) {
            format!("[image: {}]", r.text)
        } else {
            r.text.clone()
        };
        let mut cur = String::new();
        let mut in_space = false;
        for ch in text.chars() {
            let sp = ch.is_whitespace();
            if !sp && in_space {
                out.push((std::mem::take(&mut cur), look.clone()));
            }
            in_space = sp;
            cur.push(if sp { ' ' } else { ch });
        }
        if !cur.is_empty() {
            out.push((cur, look.clone()));
        }
    }
    out
}

fn look(r: &Run, size: f32, heading: bool, tone: Tone, known: &dyn Fn(&str) -> bool) -> Look {
    let Style {
        strong,
        em,
        strike,
        code,
        link,
    } = r.style;
    let face = if code {
        Face::Atkinson
    } else if strong || heading {
        // No bold-italic face is embedded: strong wins (decision S3-D3).
        Face::LiterataSemi
    } else if em {
        Face::LiterataItalic
    } else {
        Face::Literata
    };
    let (underline, tone) = match link {
        Some(LinkKind::Web) => (Some(Underline::Solid), tone),
        Some(LinkKind::Wiki) if r.target.as_deref().is_some_and(known) => {
            (Some(Underline::Solid), tone)
        }
        Some(LinkKind::Wiki) => (Some(Underline::Dashed), Tone::Mute),
        Some(LinkKind::Image) => (None, Tone::Mute),
        None => (None, tone),
    };
    Look {
        face,
        size: if code { size * 0.86 } else { size },
        tone,
        underline,
        strike,
        wash: code,
        target: if link == Some(LinkKind::Wiki) {
            r.target.clone()
        } else {
            None
        },
    }
}

/// Lays the blocks out for a text column `width` px wide.
/// `known` tells whether a `[[wiki]]` target is an existing note.
#[must_use]
pub fn layout(
    blocks: &[Block],
    fonts: &Fonts,
    width: i32,
    ty: Type,
    known: &dyn Fn(&str) -> bool,
) -> Vec<Line> {
    layout_with_images(blocks, fonts, width, ty, known, &|_, _| None)
}

/// An image source to draw: `.ink/…/*.svg` only (our own ink blocks).
#[must_use]
pub fn ink_src(r: &Run) -> Option<&str> {
    let t = r.target.as_deref()?;
    (r.style.link == Some(LinkKind::Image)
        && t.starts_with(".ink/")
        && t.ends_with(".svg")
        && !t.split('/').any(|p| p == ".."))
    .then_some(t)
}

/// As [`layout`]; a paragraph that is only ink images draws them
/// (v1.5.1). `images(src, max_w)` rasterises one, `None` keeps the
/// `[image: …]` text.
pub fn layout_with_images(
    blocks: &[Block],
    fonts: &Fonts,
    width: i32,
    ty: Type,
    known: &dyn Fn(&str) -> bool,
    images: &dyn Fn(&str, i32) -> Option<std::sync::Arc<crate::ink::Raster>>,
) -> Vec<Line> {
    let body = ty.body;
    let em = |f: f32| (body * f).round() as i32;
    let body_h = (body * ty.leading).round() as i32;
    let body_base = ((body_h as f32 - body * 1.2) / 2.0).max(0.0) as i32
        + fonts.ascent(Face::Literata, body) as i32;
    let gap = em(0.8);
    let mut b = Builder {
        fonts,
        width,
        lines: Vec::new(),
    };
    for (n, blk) in blocks.iter().enumerate() {
        let src = blk.line;
        if n > 0 {
            let g = match blk.kind {
                BlockKind::Item { .. } if matches!(blocks[n - 1].kind, BlockKind::Item { .. }) => 0,
                _ => gap,
            };
            b.spacer(g, src);
        }
        match &blk.kind {
            BlockKind::Heading(level) => {
                let size = body
                    * match level {
                        1 => 1.45,
                        2 => 1.22,
                        _ => 1.05,
                    };
                let h = (size * 1.25).round() as i32;
                let base = ((h as f32 - size * 1.2) / 2.0).max(0.0) as i32
                    + fonts.ascent(Face::LiterataSemi, size) as i32;
                let t = tokens(&blk.runs, size, true, Tone::Ink, known);
                b.flow(&t, 0, 0, h, base, &[], &[], src);
                // Heading margin is 0.4 em of its own size, then the gap.
                b.spacer((size * 0.4).round() as i32 - gap / 2, src);
            }
            BlockKind::Paragraph
                if blk.runs.iter().any(|r| ink_src(r).is_some())
                    && blk
                        .runs
                        .iter()
                        .all(|r| ink_src(r).is_some() || r.text.trim().is_empty())
                    && blk
                        .runs
                        .iter()
                        .filter_map(ink_src)
                        .all(|src| images(src, width).is_some()) =>
            {
                let pad = em(0.25);
                for path in blk.runs.iter().filter_map(ink_src) {
                    let img = images(path, width);
                    let h = img.as_ref().map_or(0, |i| i.h as i32) + 2 * pad;
                    b.lines.push(Line {
                        h,
                        base: pad,
                        pieces: Vec::new(),
                        deco: Vec::new(),
                        src,
                        spacer: false,
                        image: img,
                    });
                }
            }
            BlockKind::Paragraph => {
                let t = tokens(&blk.runs, body, false, Tone::Ink, known);
                b.flow(&t, 0, 0, body_h, body_base, &[], &[], src);
            }
            BlockKind::Item {
                depth,
                marker,
                task,
            } => {
                let indent = em(1.2) * (i32::from(*depth) + 1);
                let t = tokens(&blk.runs, body, false, Tone::Ink, known);
                match task {
                    Some(on) => {
                        let side = ty.scale.px(14.0).max(8);
                        let bx = indent - em(1.2);
                        let text_x = bx + side + ty.scale.px(8.0);
                        let deco = [Deco::Box {
                            x: bx,
                            side,
                            on: *on,
                        }];
                        b.flow(&t, text_x, text_x, body_h, body_base, &deco, &[], src);
                    }
                    None => {
                        let label = marker.map_or_else(
                            || {
                                if depth % 2 == 1 {
                                    "\u{2013}"
                                } else {
                                    "\u{2022}"
                                }
                                .to_owned()
                            },
                            |m| format!("{m}."),
                        );
                        let mw = fonts.width(Face::Literata, body, &label).ceil() as i32;
                        let mut first = vec![(
                            label,
                            Look {
                                face: Face::Literata,
                                size: body,
                                tone: Tone::Ink,
                                underline: None,
                                strike: false,
                                wash: false,
                                target: None,
                            },
                        )];
                        let start = b.lines.len();
                        let mx = indent - em(0.4) - mw;
                        b.flow(
                            &first.split_off(0),
                            indent,
                            mx,
                            body_h,
                            body_base,
                            &[],
                            &[],
                            src,
                        );
                        // Put the item text after the marker on the same line.
                        let marker_line = b.lines.pop().map(|l| l.pieces);
                        let tail = b.lines.len();
                        b.flow(&t, indent, indent, body_h, body_base, &[], &[], src);
                        debug_assert_eq!(start, tail);
                        if let (Some(mut m), Some(l)) = (marker_line, b.lines.get_mut(start)) {
                            m.append(&mut l.pieces);
                            l.pieces = m;
                        }
                    }
                }
            }
            BlockKind::Quote => {
                let pad = em(0.8) + 2;
                let t = tokens(&blk.runs, body, false, Tone::Mute, known);
                b.flow(
                    &t,
                    pad,
                    pad,
                    body_h,
                    body_base,
                    &[],
                    &[Deco::Bar { x: 0 }],
                    src,
                );
            }
            BlockKind::Code => {
                let size = body * 0.86;
                let pv = em(0.7);
                let ph = em(0.8);
                let wash = Deco::Wash { x: 0, w: width };
                let look = Look {
                    face: Face::Atkinson,
                    size,
                    tone: Tone::Ink,
                    underline: None,
                    strike: false,
                    wash: false,
                    target: None,
                };
                let code_h = (size * ty.leading).round() as i32;
                let code_base = ((code_h as f32 - size * 1.2) / 2.0).max(0.0) as i32
                    + fonts.ascent(Face::Atkinson, size) as i32;
                let pad_line = |h| Line {
                    h,
                    base: 0,
                    pieces: Vec::new(),
                    deco: vec![wash],
                    src,
                    spacer: false,
                    image: None,
                };
                b.lines.push(pad_line(pv));
                let inner = Builder {
                    fonts,
                    width: width - ph,
                    lines: Vec::new(),
                };
                let mut inner = inner;
                for r in &blk.runs {
                    let t: Vec<(String, Look)> =
                        tokens(std::slice::from_ref(r), size, false, Tone::Ink, known)
                            .into_iter()
                            .map(|(s, _)| (s, look.clone()))
                            .collect();
                    // Leading spaces matter in code: keep them as an indent.
                    let lead = r.text.len() - r.text.trim_start().len();
                    let ind = ph + fonts.width(Face::Atkinson, size, &r.text[..lead]).ceil() as i32;
                    inner.flow(&t, ind, ind, code_h, code_base, &[], &[wash], src);
                }
                b.lines.append(&mut inner.lines);
                b.lines.push(pad_line(pv));
            }
            BlockKind::Rule => {
                b.lines.push(Line {
                    h: em(0.8),
                    base: 0,
                    pieces: Vec::new(),
                    deco: vec![Deco::Rule],
                    src,
                    spacer: false,
                    image: None,
                });
            }
        }
    }
    b.lines
}

/// Cuts lines into pages no taller than `height`. Spacers that would start
/// a page are skipped, and a heading is not left alone at the bottom.
#[must_use]
pub fn paginate(lines: &[Line], height: i32) -> Vec<Range<usize>> {
    let mut pages = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        while start < lines.len() && lines[start].spacer {
            start += 1;
        }
        if start >= lines.len() {
            break;
        }
        let mut y = 0;
        let mut end = start;
        while end < lines.len() && (y + lines[end].h <= height || end == start) {
            y += lines[end].h;
            end += 1;
        }
        // Keep a heading with what follows it.
        if end < lines.len() {
            let mut k = end;
            while k > start + 1 && lines[k - 1].spacer {
                k -= 1;
            }
            if k > start + 1 && is_heading(&lines[k - 1]) {
                let mut h = k - 1;
                while h > start + 1 && is_heading(&lines[h - 1]) {
                    h -= 1;
                }
                if h > start {
                    end = h;
                }
            }
        }
        pages.push(start..end);
        start = end;
    }
    if pages.is_empty() {
        pages.push(0..0);
    }
    pages
}

fn is_heading(l: &Line) -> bool {
    l.pieces.first().is_some_and(|p| {
        p.face == Face::LiterataSemi
            && p.size > 0.0
            && l.base > 0
            && l.deco.is_empty()
            && l.h != 0
            && p.x == 0
    }) && l.pieces.iter().all(|p| p.face == Face::LiterataSemi)
}

/// The `[[wiki]]` target under a point on the shown page (`area` is the
/// text column it was drawn into). A little slop around each link.
#[must_use]
pub fn link_at(
    fonts: &Fonts,
    area: Rect,
    lines: &[Line],
    range: Range<usize>,
    x: i32,
    y: i32,
    slop: i32,
) -> Option<String> {
    let mut top = area.y;
    for l in &lines[range] {
        if y >= top - slop && y < top + l.h + slop {
            for p in &l.pieces {
                let Some(t) = &p.target else { continue };
                let x0 = area.x + p.x;
                let w = fonts.width(p.face, p.size, &p.text).ceil() as i32;
                if x >= x0 - slop && x < x0 + w + slop {
                    return Some(t.clone());
                }
            }
        }
        top += l.h;
    }
    None
}

/// Draws `range` of `lines` into `area`.
pub fn draw_page(
    c: &mut Canvas,
    fonts: &Fonts,
    area: Rect,
    lines: &[Line],
    range: Range<usize>,
    t: Theme,
    s: Scale,
) {
    let mut y = area.y;
    let thin = s.px(1.0).max(1);
    for l in &lines[range] {
        if let Some(img) = &l.image {
            // Pure ink on the page colour (v1.5.1): black on Paper, white
            // on Night, never grey, so later DU updates stay exact.
            let v = if t.paper >= 128 { 0 } else { 255 };
            img.blit(c, area.x, y + l.base, v, area);
        }
        for d in &l.deco {
            match *d {
                Deco::Wash { x, w } => c.fill(Rect::new(area.x + x, y, w, l.h), t.wash),
                Deco::Bar { x } => c.fill(Rect::new(area.x + x, y, s.px(2.0).max(2), l.h), t.ink),
                Deco::Rule => c.fill(Rect::new(area.x, y + l.h / 2, area.w, thin), t.ink),
                Deco::Box { .. } => {}
            }
        }
        let base = y + l.base;
        for d in &l.deco {
            if let Deco::Box { x, side, on } = *d {
                let size = l.pieces.first().map_or(16.0, |p| p.size);
                let top = base - (size * 0.72) as i32 + ((size * 0.72) as i32 - side) / 2;
                let r = Rect::new(area.x + x, top, side, side);
                if on {
                    c.fill(r, t.ink);
                } else {
                    c.stroke(r, thin, t.ink);
                }
            }
        }
        for p in &l.pieces {
            let ink = match p.tone {
                Tone::Ink => t.ink,
                Tone::Mute => t.mute,
            };
            let x = (area.x + p.x) as f32;
            let w = fonts.width(p.face, p.size, &p.text).ceil() as i32;
            if p.wash {
                let pad = (p.size * 0.2) as i32;
                c.fill(
                    Rect::new(
                        area.x + p.x - pad,
                        base - (p.size * 0.95) as i32,
                        w + 2 * pad,
                        (p.size * 1.25) as i32,
                    ),
                    t.wash,
                );
            }
            fonts.draw(c, p.face, p.size, x, base as f32, &p.text, ink);
            if let Some(u) = p.underline {
                let uy = base + s.px(2.0).max(2);
                let dash = s.px(4.0).max(3);
                let mut ux = 0;
                while ux < w {
                    let seg = match u {
                        Underline::Solid => w,
                        Underline::Dashed => dash.min(w - ux),
                    };
                    c.fill(Rect::new(area.x + p.x + ux, uy, seg, thin), ink);
                    ux += seg + if u == Underline::Dashed { dash } else { 0 };
                }
            }
            if p.strike {
                let sy = base - (p.size * 0.3) as i32;
                c.fill(Rect::new(area.x + p.x, sy, w, thin), ink);
            }
        }
        y += l.h;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::markdown::parse;

    fn ty() -> Type {
        let s = Scale::for_width(1440);
        Type {
            body: 20.0 * s.0,
            leading: 1.5,
            scale: s,
        }
    }

    fn lay(src: &str, width: i32) -> Vec<Line> {
        let f = Fonts::new();
        layout(&parse(src), &f, width, ty(), &|n| n == "chapter-01")
    }

    fn text(l: &Line) -> String {
        l.pieces.iter().map(|p| p.text.as_str()).collect()
    }

    #[test]
    fn heading_is_bigger_semibold_and_body_is_literata() {
        let ls = lay("# Title\n\nBody text", 1000);
        let h = &ls[0];
        let body = ls.iter().find(|l| text(l) == "Body text").unwrap();
        assert_eq!(h.pieces[0].face, Face::LiterataSemi);
        assert!(h.pieces[0].size > body.pieces[0].size * 1.4);
        assert_eq!(body.pieces[0].face, Face::Literata);
        assert!(ls.iter().any(|l| l.spacer), "gap between blocks");
    }

    #[test]
    fn emphasis_faces_and_decorations() {
        let ls = lay(
            "a **b** *c* ~~d~~ `e` [f](u) [[chapter-01]] [[missing]]",
            2000,
        );
        let p = &ls[0].pieces;
        let get = |t: &str| p.iter().find(|x| x.text.trim() == t).unwrap();
        assert_eq!(get("b").face, Face::LiterataSemi);
        assert_eq!(get("c").face, Face::LiterataItalic);
        assert!(get("d").strike);
        assert!(get("e").wash && get("e").face == Face::Atkinson);
        assert_eq!(get("f").underline, Some(Underline::Solid));
        assert_eq!(get("chapter-01").underline, Some(Underline::Solid));
        assert_eq!(get("missing").underline, Some(Underline::Dashed));
        assert_eq!(get("missing").tone, Tone::Mute);
    }

    #[test]
    fn wraps_within_width_and_indents_lists() {
        let f = Fonts::new();
        let long = "word ".repeat(80);
        let ls = lay(&format!("- {long}\n  - nested\n- [x] done"), 600);
        let items: Vec<_> = ls.iter().filter(|l| !l.spacer).collect();
        assert!(items.len() > 3);
        for l in &items {
            for p in &l.pieces {
                let end = p.x + f.width(p.face, p.size, &p.text).ceil() as i32;
                assert!(end <= 600, "{end} > 600: {:?}", p.text);
            }
        }
        // Continuation lines hang at the text indent, not under the bullet.
        assert!(items[1].pieces[0].x > items[0].pieces[0].x);
        assert_eq!(items[1].pieces[0].x, items[0].pieces[1].x);
        assert_eq!(items[0].pieces[0].text, "\u{2022}");
        let nested = ls.iter().find(|l| text(l).contains("nested")).unwrap();
        assert!(nested.pieces[0].x > items[0].pieces[0].x);
        let done = ls.iter().find(|l| text(l) == "done").unwrap();
        assert!(matches!(done.deco[0], Deco::Box { on: true, .. }));
    }

    #[test]
    fn quote_code_rule_decorations() {
        let ls = lay("> quoted\n\n```\n  code\n```\n\n---", 800);
        let q = ls.iter().find(|l| text(l) == "quoted").unwrap();
        assert_eq!(q.deco, vec![Deco::Bar { x: 0 }]);
        assert_eq!(q.pieces[0].tone, Tone::Mute);
        let code = ls.iter().find(|l| text(l).contains("code")).unwrap();
        assert!(matches!(code.deco[0], Deco::Wash { .. }));
        assert!(code.pieces[0].x > ty().scale.px(16.0), "keeps indentation");
        assert!(ls.iter().any(|l| l.deco == vec![Deco::Rule]));
    }

    #[test]
    fn overlong_word_is_cut() {
        let ls = lay(&"x".repeat(400), 300);
        assert!(ls.len() > 2);
    }

    #[test]
    fn pages_fit_and_cover_every_line() {
        let src: String = (0..120)
            .map(|i| format!("## Part {i}\n\nA paragraph about part {i} with some words.\n\n"))
            .collect();
        let ls = lay(&src, 1200);
        let pages = paginate(&ls, 1400);
        assert!(pages.len() > 3);
        let mut next = 0;
        for p in &pages {
            assert!(p.start >= next);
            assert!(!ls[p.start].spacer, "no page starts with a gap");
            let h: i32 = ls[p.clone()].iter().map(|l| l.h).sum();
            assert!(h <= 1400);
            assert!(
                !is_heading(&ls[p.end - 1]) || p.end == ls.len(),
                "orphan heading"
            );
            // Skipped lines in between are spacers only.
            assert!(ls[next..p.start].iter().all(|l| l.spacer));
            next = p.end;
        }
        assert_eq!(next, ls.len());
    }

    #[test]
    fn page_of_line_finds_the_source() {
        let src: String = (0..200).map(|i| format!("line {i}\n\n")).collect();
        let ls = lay(&src, 1200);
        let doc = Doc {
            pages: paginate(&ls, 600),
            lines: ls,
            width: 1200,
        };
        assert_eq!(doc.page_of_line(0), 0);
        let p = doc.page_of_line(300);
        let first = doc.lines[doc.pages[p].clone()]
            .iter()
            .find(|l| !l.spacer)
            .unwrap();
        assert!(first.src <= 300);
        assert!(doc.pages[p].clone().any(|i| doc.lines[i].src >= 300 - 2));
    }

    #[test]
    fn draws_a_page() {
        let f = Fonts::new();
        let ls = lay("# Hi\n\n- [ ] box\n\n> q", 600);
        let mut c = Canvas::new(700, 400, 240);
        let pages = paginate(&ls, 380);
        draw_page(
            &mut c,
            &f,
            Rect::new(10, 10, 600, 380),
            &ls,
            pages[0].clone(),
            Theme::DAY,
            ty().scale,
        );
        assert!(c.pixels.iter().filter(|&&p| p < 60).count() > 200);
    }

    #[test]
    fn large_note_lays_out_quickly() {
        let para = "Some **bold** and *em* text with a [[link]] and `code` in it.\n\n";
        let src = para.repeat(1700);
        assert!(src.len() > 100_000);
        let f = Fonts::new();
        let t = std::time::Instant::now();
        let ls = layout(&parse(&src), &f, 1300, ty(), &|_| true);
        let pages = paginate(&ls, 1500);
        assert!(pages.len() > 50);
        // Desktop debug budget; the Forma is measured on device (S3-T05).
        assert!(t.elapsed().as_millis() < 3000, "{:?}", t.elapsed());
    }
}
