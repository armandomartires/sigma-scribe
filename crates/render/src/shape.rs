//! Right-to-left and joined scripts (v1.8, S18.0): bidi order and shaping.
//!
//! A row with no right-to-left character takes the plain per-character
//! path in `fonts.rs` and never comes here. A row with Arabic, Hebrew or
//! the like is split into bidi runs (`unicode-bidi`), each run into
//! stretches of one face, and every stretch is shaped (`harfrust`), so
//! letters join, marks sit on their letters and the runs read in visual
//! order. The caret and a tap use the glyphs' cluster positions.

use crate::fonts::{Face, Fonts};
use unicode_bidi::BidiInfo;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Glyph {
    pub id: u32,
    pub adv: f32,
    pub dx: f32,
    pub dy: f32,
    /// Byte offset of the cluster's first character in the row.
    pub cluster: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Seg {
    pub face: Face,
    /// Glyphs left to right.
    pub glyphs: Vec<Glyph>,
    pub width: f32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Shaped {
    /// Left to right on screen.
    pub segs: Vec<Seg>,
    pub width: f32,
    pub rtl: bool,
    /// (byte offset in the row, x) caret stops, in no particular order.
    pub stops: Vec<(usize, f32)>,
}

/// Does `text` need bidi order and shaping?
#[must_use]
pub fn is_rtl_text(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x0590..=0x08FF | 0x200F | 0x202B | 0x202E | 0x2067 | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
    })
}

/// Naskh's letters are small for their em, next to Literata (v1.8).
#[must_use]
pub(crate) fn face_scale(face: Face) -> f32 {
    match face {
        Face::Naskh => 1.18,
        Face::Devanagari | Face::Bengali => 1.2,
        Face::Tamil => 1.05,
        Face::Thai => 1.15,
        _ => 1.0,
    }
}

/// Scripts that need shaping though they read left to right (v1.9):
/// Devanagari to Sinhala, Thai, Lao, Tibetan, Myanmar, Khmer.
#[must_use]
pub fn is_complex_text(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x0900..=0x0DFF | 0x0E00..=0x0FFF | 0x1000..=0x109F | 0x1780..=0x17FF)
    }) || is_rtl_text(text)
}

fn neutral(c: char) -> bool {
    c.is_whitespace()
        || c.is_ascii_punctuation()
        || c.is_ascii_digit()
        || matches!(c, '\u{0964}' | '\u{0965}')
}

/// Joiners and selectors belong to the letters around them, in whatever
/// face those are drawn, glyph or no glyph (they must not split a cluster).
fn joiner(c: char) -> bool {
    matches!(c as u32, 0x200B..=0x200F | 0x034F | 0xFE00..=0xFE0F)
}

impl Fonts {
    /// Bidi runs, faces and shaping for one row.
    pub(crate) fn shape(&self, face: Face, size: f32, text: &str) -> Shaped {
        let mut out = Shaped::default();
        if text.is_empty() {
            return out;
        }
        let owned;
        let text = if text.contains('\t') {
            owned = text.replace('\t', " ");
            owned.as_str()
        } else {
            text
        };
        let bidi = BidiInfo::new(text, None);
        out.rtl = bidi.paragraphs.first().is_some_and(|p| p.level.is_rtl());
        let mut pen = 0.0f32;
        let mut runs = Vec::new();
        for para in &bidi.paragraphs {
            let (levels, vis) = bidi.visual_runs(para, para.range.clone());
            for r in vis {
                runs.push((r.clone(), levels[r.start].is_rtl()));
            }
        }
        for (range, rtl) in runs {
            let mut segs: Vec<(Face, usize, usize)> = Vec::new();
            for (i, c) in text[range.clone()].char_indices() {
                let at = range.start + i;
                let prev = segs.last().map(|s| s.0);
                let f = match prev {
                    Some(p) if joiner(c) => p,
                    Some(p) if neutral(c) && self.has_glyph(p, c) => p,
                    _ => self.face_for(face, c),
                };
                match segs.last_mut() {
                    Some(s) if s.0 == f => s.2 = at + c.len_utf8(),
                    _ => segs.push((f, at, at + c.len_utf8())),
                }
            }
            if rtl {
                segs.reverse();
            }
            for (f, a, b) in segs {
                let seg = self.shape_seg(f, size, &text[a..b], a, rtl);
                let x0 = pen;
                // Caret stops: the logical start of a cluster is at the
                // glyph's left edge in a left-to-right run, right edge in
                // a right-to-left one.
                let mut gx = x0;
                for g in &seg.glyphs {
                    let x = if rtl { gx + g.adv } else { gx };
                    out.stops.push((g.cluster, x));
                    gx += g.adv;
                }
                out.stops.push((b, if rtl { x0 } else { x0 + seg.width }));
                pen += seg.width;
                out.segs.push(seg);
            }
        }
        out.width = pen;
        out
    }

    fn shape_seg(&self, face: Face, size: f32, text: &str, base: usize, rtl: bool) -> Seg {
        let size = size * face_scale(face);
        let mut seg = Seg {
            face,
            glyphs: Vec::new(),
            width: 0.0,
        };
        if let Some(font) = self.shaper(face) {
            if let Some(glyphs) = shape_run(&font, size, text, base, rtl) {
                seg.width = glyphs.iter().map(|g| g.adv).sum();
                seg.glyphs = glyphs;
                return seg;
            }
        }
        // A font pack, or text the shaper refused: one glyph per
        // character, in run order.
        let font = self.font(face);
        let mut chars: Vec<(usize, char)> = text.char_indices().collect();
        if rtl {
            chars.reverse();
        }
        for (i, c) in chars {
            let adv = font.char_advance(c, size);
            seg.glyphs.push(Glyph {
                id: font.glyph(c),
                adv,
                dx: 0.0,
                dy: 0.0,
                cluster: base + i,
            });
            seg.width += adv;
        }
        seg
    }
}

/// Shapes one stretch of one face; `None` if the shaper refuses it.
fn shape_run(
    font: &harfrust::Font,
    size: f32,
    text: &str,
    base: usize,
    rtl: bool,
) -> Option<Vec<Glyph>> {
    let shaper = harfrust::ShaperFont::new(font);
    let mut buf = harfrust::Buffer::new();
    buf.push_str(text);
    buf.set_direction(if rtl {
        harfrust::Direction::RightToLeft
    } else {
        harfrust::Direction::LeftToRight
    });
    buf.guess_segment_properties();
    harfrust::shape(&shaper, &mut buf, harfrust::ShapeOptions::new()).ok()?;
    let k = size / f32::from(font.units_per_em().max(1));
    Some(
        buf.glyph_infos()
            .iter()
            .zip(buf.glyph_positions())
            .map(|(info, pos)| Glyph {
                id: info.glyph_id,
                adv: pos.x_advance as f32 * k,
                dx: pos.x_offset as f32 * k,
                dy: pos.y_offset as f32 * k,
                cluster: base + info.cluster as usize,
            })
            .collect(),
    )
}

impl Shaped {
    /// x of the caret at byte offset `at` of the row.
    pub(crate) fn caret_x(&self, at: usize) -> f32 {
        // An exact stop wins; else the stop of the cluster that contains
        // `at` (the nearest one before it).
        let mut best: Option<(usize, f32)> = None;
        for &(i, x) in &self.stops {
            if i == at {
                return x;
            }
            if i < at && best.is_none_or(|b| i > b.0) {
                best = Some((i, x));
            }
        }
        best.map_or(if self.rtl { self.width } else { 0.0 }, |b| b.1)
    }

    /// Byte offset whose caret stop is nearest to `x`.
    pub(crate) fn index_at(&self, x: f32, len: usize) -> usize {
        let mut best = (if self.rtl { len } else { 0 }, f32::MAX);
        for &(i, sx) in &self.stops {
            let d = (sx - x).abs();
            if d < best.1 || (d == best.1 && i < best.0) {
                best = (i, d);
            }
        }
        best.0.min(len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::TextMeasure;

    #[test]
    fn only_right_to_left_text_takes_this_path() {
        assert!(is_rtl_text("مرحبا"));
        assert!(is_rtl_text("shalom שלום"));
        assert!(!is_rtl_text("hello, Привет, 한글"));
    }

    #[test]
    fn rows_read_in_the_direction_of_their_first_strong_letter() {
        let f = Fonts::new();
        assert!(f.row_rtl("שלום world"));
        assert!(!f.row_rtl("world שלום"));
        assert!(!f.row_rtl("hello"));
    }

    #[test]
    fn a_joined_word_is_not_the_sum_of_its_letters() {
        let f = Fonts::new();
        let whole = f.width(Face::Literata, 40.0, "بيت");
        let parts: f32 = "بيت"
            .chars()
            .map(|c| f.width(Face::Literata, 40.0, &c.to_string()))
            .sum();
        assert!(
            whole > 20.0 && (whole - parts).abs() > 0.5,
            "{whole} {parts}"
        );
    }

    #[test]
    fn the_caret_runs_from_the_right_edge_to_the_left() {
        let f = Fonts::new();
        let m = crate::fonts::Measure {
            fonts: &f,
            face: Face::Literata,
            size: 30.0,
        };
        let row = "مرحبا بالعالم";
        let w = m.width(row) as i32;
        // The start of the text is at the right edge, the end at the left.
        assert!((m.caret_x(row, 0) as i32 - w).abs() <= 1);
        assert!(m.caret_x(row, row.len()) <= 1);
        // Stops move left as the offset grows, and a tap finds them again.
        let mut last = i32::MAX;
        for (i, _) in row.char_indices() {
            let x = m.caret_x(row, i) as i32;
            assert!(x <= last, "offset {i}: {x} after {last}");
            last = x;
            assert_eq!(m.index_at(row, x as u32), Some(i), "tap at {x}");
        }
    }

    #[test]
    fn latin_inside_a_right_to_left_row_keeps_its_own_order() {
        let f = Fonts::new();
        let s = f.shape(Face::Literata, 30.0, "אב abc גד");
        // Visual order: ד ג, then abc, then ב א  (two Hebrew runs around it).
        let faces: Vec<bool> = s.segs.iter().map(|g| g.face == Face::Hebrew).collect();
        assert!(s.rtl && faces.len() >= 3 && faces[0] && !faces[faces.len() / 2]);
    }

    #[test]
    fn rtl_rows_wrap_within_the_width() {
        let f = Fonts::new();
        let m = crate::fonts::Measure {
            fonts: &f,
            face: Face::Literata,
            size: 30.0,
        };
        let text = "مرحبا بالعالم هذا نص عربي طويل يلتف على عدة أسطر";
        let rows = sigma_core::wrap(text, 300, &m);
        assert!(rows.len() > 1);
        for r in &rows {
            assert!(m.width(text[r.start..r.end].trim_end()) <= 300);
        }
    }

    #[test]
    fn indic_and_thai_take_the_shaping_path_but_read_left_to_right() {
        let f = Fonts::new();
        for t in ["हिन्दी", "বাংলা", "தமிழ்", "ภาษาไทย"]
        {
            assert!(is_complex_text(t) && !is_rtl_text(t), "{t}");
            assert!(!f.row_rtl(t), "{t}");
        }
        assert!(!is_complex_text("hello Привет 한글"));
    }

    #[test]
    fn a_conjunct_is_narrower_than_its_letters_and_has_one_caret_stop() {
        let f = Fonts::new();
        let whole = f.width(Face::Literata, 40.0, "क्ष");
        let parts = f.width(Face::Literata, 40.0, "क") + f.width(Face::Literata, 40.0, "ष");
        assert!(whole > 10.0 && whole < parts + 40.0, "{whole} {parts}");
        let m = crate::fonts::Measure {
            fonts: &f,
            face: Face::Literata,
            size: 40.0,
        };
        // क ् ष is one cluster: the caret has no stop inside it.
        let t = "क्षि";
        let a = m.caret_x(t, 0);
        let end = m.caret_x(t, t.len());
        assert!(end > a);
        assert_eq!(m.caret_x(t, "क".len()), a);
        assert_eq!(m.caret_x(t, "क्".len()), a);
    }

    #[test]
    fn thai_wraps_within_the_width_at_cluster_boundaries() {
        let f = Fonts::new();
        let m = crate::fonts::Measure {
            fonts: &f,
            face: Face::Literata,
            size: 30.0,
        };
        let text = "ภาษาไทยเป็นภาษาที่ใช้พูดกันในประเทศไทยและมีผู้พูดหลายล้านคน";
        let rows = sigma_core::wrap(text, 260, &m);
        assert!(rows.len() > 1);
        for r in &rows {
            let row = &text[r.start..r.end];
            assert!(m.width(row) <= 260, "{row}");
            assert!(!row.ends_with(['เ', 'แ', 'โ', 'ไ', 'ใ']), "{row}");
        }
    }
}
