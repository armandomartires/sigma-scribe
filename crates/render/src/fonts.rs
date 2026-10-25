//! Embedded faces (all SIL OFL 1.1, licences in assets/fonts/) and glyph
//! drawing. Measurement implements `sigma_core::TextMeasure`, so wrapping
//! uses real advances ("measure with the real face, not a guess").

use crate::canvas::Canvas;
use fontdue::{Font, FontSettings};
use std::cell::RefCell;
use std::collections::HashMap;

static LITERATA: &[u8] = include_bytes!("../../../assets/fonts/Literata.ttf");
static LITERATA_SEMI: &[u8] = include_bytes!("../../../assets/fonts/Literata-SemiBold.ttf");
static LITERATA_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/Literata-Italic.ttf");
static ATKINSON: &[u8] = include_bytes!("../../../assets/fonts/AtkinsonHyperlegible-Regular.ttf");
static ATKINSON_BOLD: &[u8] = include_bytes!("../../../assets/fonts/AtkinsonHyperlegible-Bold.ttf");
// v1.8: Arabic script (Arabic, Persian, Urdu) and Hebrew, small enough to
// ship in the binary (Noto Naskh Arabic 176 KB, Noto Sans Hebrew 27 KB).
static NASKH: &[u8] = include_bytes!("../../../assets/fonts/NotoNaskhArabic-Regular.ttf");
static HEBREW: &[u8] = include_bytes!("../../../assets/fonts/NotoSansHebrew-Regular.ttf");
// v1.9: Devanagari (Hindi, Marathi, Nepali), Bengali, Tamil, Thai: 229 + 202
// + 73 + 38 KB.
static DEVANAGARI: &[u8] = include_bytes!("../../../assets/fonts/NotoSansDevanagari-Regular.ttf");
static BENGALI: &[u8] = include_bytes!("../../../assets/fonts/NotoSansBengali-Regular.ttf");
static TAMIL: &[u8] = include_bytes!("../../../assets/fonts/NotoSansTamil-Regular.ttf");
static THAI: &[u8] = include_bytes!("../../../assets/fonts/NotoSansThai-Regular.ttf");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Literata,
    Atkinson,
    AtkinsonBold,
    /// Literata at weight 600 (preview headings and `**strong**`, as the mock).
    LiterataSemi,
    /// Literata Italic at weight 400 (preview `*em*`).
    LiterataItalic,
    /// Noto Naskh Arabic (Arabic, Persian, Urdu; v1.8).
    Naskh,
    /// Noto Sans Hebrew (v1.8).
    Hebrew,
    /// Noto Sans Devanagari, Bengali, Tamil, Thai (v1.9).
    Devanagari,
    Bengali,
    Tamil,
    Thai,
    /// The optional script font pack (Korean, v1.7.1): only ever returned
    /// by [`Fonts::face_for`], for a character no bundled face has.
    Pack,
}

type GlyphKey = (Face, char, u32);
/// A rasterised glyph: metrics and coverage.
type Glyph = (fontdue::Metrics, Vec<u8>);

pub struct Fonts {
    faces: [Font; 11],
    buzz: RefCell<HashMap<Face, Option<std::rc::Rc<rustybuzz::Face<'static>>>>>,
    shapes: RefCell<HashMap<(Face, u32, String), std::rc::Rc<crate::shape::Shaped>>>,
    gcache: RefCell<HashMap<(Face, u16, u32), Glyph>>,
    /// The font pack, when one is installed (see [`Fonts::load_pack`]).
    pack: Option<&'static Font>,
    cache: RefCell<HashMap<GlyphKey, (fontdue::Metrics, Vec<u8>)>>,
}

impl Default for Fonts {
    fn default() -> Self {
        Self::new()
    }
}

impl Fonts {
    /// # Panics
    /// Only if an embedded font file is corrupt (caught by tests).
    #[must_use]
    pub fn new() -> Self {
        let load =
            |b: &'static [u8]| Font::from_bytes(b, FontSettings::default()).expect("embedded font");
        Self {
            faces: [
                load(LITERATA),
                load(ATKINSON),
                load(ATKINSON_BOLD),
                load(LITERATA_SEMI),
                load(LITERATA_ITALIC),
                load(NASKH),
                load(HEBREW),
                load(DEVANAGARI),
                load(BENGALI),
                load(TAMIL),
                load(THAI),
            ],
            buzz: RefCell::default(),
            shapes: RefCell::default(),
            gcache: RefCell::default(),
            pack: Self::pack(),
            cache: RefCell::default(),
        }
    }

    /// The script font pack (v1.7.1, plan §12.6): `SIGMA_FONT_PACK` (a
    /// path, `#N` picks a face of a collection), else `fonts/korean.ttf`
    /// beside the app on the Kobo, else the system Noto Sans CJK on a
    /// development machine. None when there is none: the script's
    /// characters then draw as empty boxes and its layout is not offered.
    fn pack() -> Option<&'static Font> {
        // Parsed once per process: a development machine's system font is
        // 19 MB, and every `Fonts` (the tests make many) would reparse it.
        static PACK: std::sync::OnceLock<Option<Font>> = std::sync::OnceLock::new();
        PACK.get_or_init(Self::load_pack).as_ref()
    }

    fn load_pack() -> Option<Font> {
        let env = std::env::var("SIGMA_FONT_PACK").ok();
        // Enabled plugins come before the loose files, so a plugin wins.
        let plugins: Vec<String> = ["fonts/cjk.ttf", "fonts/korean.ttf"]
            .into_iter()
            .flat_map(sigma_core::plugins::enabled_files)
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let spec = env
            .iter()
            .chain(plugins.iter())
            .map(String::as_str)
            .chain([
                "/mnt/onboard/.adds/sigma-scribe/fonts/cjk.ttf",
                "/mnt/onboard/.adds/sigma-scribe/fonts/korean.ttf",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc#0",
            ]);
        for s in spec {
            let (path, index) = s
                .split_once('#')
                .map_or((s, 0), |(p, i)| (p, i.parse().unwrap_or(0)));
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            let settings = FontSettings {
                collection_index: index,
                ..FontSettings::default()
            };
            if let Ok(f) = Font::from_bytes(bytes, settings) {
                return Some(f);
            }
        }
        None
    }

    /// Whether a script font pack is installed.
    #[must_use]
    pub fn has_pack(&self) -> bool {
        self.pack.is_some()
    }

    /// The shaping face of a bundled font (None for a font pack).
    pub(crate) fn buzz(&self, face: Face) -> Option<std::rc::Rc<rustybuzz::Face<'static>>> {
        let bytes: &'static [u8] = match face {
            Face::Literata => LITERATA,
            Face::Atkinson => ATKINSON,
            Face::AtkinsonBold => ATKINSON_BOLD,
            Face::LiterataSemi => LITERATA_SEMI,
            Face::LiterataItalic => LITERATA_ITALIC,
            Face::Naskh => NASKH,
            Face::Hebrew => HEBREW,
            Face::Devanagari => DEVANAGARI,
            Face::Bengali => BENGALI,
            Face::Tamil => TAMIL,
            Face::Thai => THAI,
            Face::Pack => return None,
        };
        self.buzz
            .borrow_mut()
            .entry(face)
            .or_insert_with(|| rustybuzz::Face::from_slice(bytes, 0).map(std::rc::Rc::new))
            .clone()
    }

    pub(crate) fn font(&self, face: Face) -> &Font {
        match (face, &self.pack) {
            (Face::Pack, Some(p)) => p,
            (Face::Pack, None) => &self.faces[0],
            _ => &self.faces[face as usize],
        }
    }

    /// Advance width of `text` in pixels at `size` px. Tabs are 4 spaces.
    #[must_use]
    pub fn width(&self, face: Face, size: f32, text: &str) -> f32 {
        if crate::shape::is_complex_text(text) {
            return self.shaped(face, size, text).width;
        }
        let f = self.font(face);
        text.chars()
            .map(|c| {
                if c == '\t' {
                    4.0 * f.metrics(' ', size).advance_width
                } else {
                    self.font(self.face_for(face, c))
                        .metrics(c, size)
                        .advance_width
                }
            })
            .sum()
    }

    /// Bidi order and shaping of a row (cached; v1.8).
    pub(crate) fn shaped(
        &self,
        face: Face,
        size: f32,
        text: &str,
    ) -> std::rc::Rc<crate::shape::Shaped> {
        let key = (face, (size * 4.0) as u32, text.to_string());
        if let Some(s) = self.shapes.borrow().get(&key) {
            return s.clone();
        }
        let s = std::rc::Rc::new(self.shape(face, size, text));
        let mut cache = self.shapes.borrow_mut();
        if cache.len() > 4096 {
            cache.clear();
        }
        cache.insert(key, s.clone());
        s
    }

    /// Whether `face` has a real glyph for `c` (v1.4.6): fontdue maps a
    /// missing character to glyph 0, which draws as an empty box.
    #[must_use]
    pub fn has_glyph(&self, face: Face, c: char) -> bool {
        c.is_whitespace()
            || (face != Face::Pack || self.pack.is_some())
                && self.font(face).lookup_glyph_index(c) != 0
    }

    /// The face that draws `c` (v1.4.6): `face` when it has the glyph,
    /// else the first bundled face that does (Atkinson has no arrows, for
    /// one), so no character ever draws as an empty box.
    #[must_use]
    pub fn face_for(&self, face: Face, c: char) -> Face {
        if self.has_glyph(face, c) {
            return face;
        }
        [
            Face::Literata,
            Face::Atkinson,
            Face::LiterataSemi,
            Face::LiterataItalic,
            Face::AtkinsonBold,
            Face::Naskh,
            Face::Hebrew,
            Face::Devanagari,
            Face::Bengali,
            Face::Tamil,
            Face::Thai,
        ]
        .into_iter()
        .chain([Face::Pack])
        .find(|&f| self.has_glyph(f, c))
        .unwrap_or(face)
    }

    /// Characters of `text` that no bundled face can draw (v1.4.6 audit).
    #[must_use]
    pub fn missing(&self, face: Face, text: &str) -> Vec<char> {
        text.chars()
            .filter(|&c| c != '\t' && !self.has_glyph(self.face_for(face, c), c))
            .collect()
    }

    /// Ascent in px for baseline placement.
    #[must_use]
    pub fn ascent(&self, face: Face, size: f32) -> f32 {
        self.font(face)
            .horizontal_line_metrics(size)
            .map_or(size * 0.8, |m| m.ascent)
    }

    /// Draws `text` with its baseline at `y`, starting at `x`. Returns the end x.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        c: &mut Canvas,
        face: Face,
        size: f32,
        x: f32,
        baseline: f32,
        text: &str,
        ink: u8,
    ) -> f32 {
        if crate::shape::is_complex_text(text) {
            return self.draw_shaped(c, face, size, x, baseline, text, ink);
        }
        let mut pen = x;
        let key_size = (size * 4.0) as u32;
        for ch in text.chars() {
            if ch == '\t' {
                pen += self.width(face, size, "\t");
                continue;
            }
            let fc = self.face_for(face, ch);
            let mut cache = self.cache.borrow_mut();
            let (m, bitmap) = cache
                .entry((fc, ch, key_size))
                .or_insert_with(|| self.font(fc).rasterize(ch, size));
            let gx = (pen + m.xmin as f32).round() as i32;
            let gy = (baseline - m.height as f32 - m.ymin as f32).round() as i32;
            for row in 0..m.height {
                for col in 0..m.width {
                    let a = bitmap[row * m.width + col];
                    c.blend(gx + col as i32, gy + row as i32, ink, a);
                }
            }
            pen += m.advance_width;
        }
        pen
    }
}

impl Fonts {
    #[allow(clippy::too_many_arguments)]
    fn draw_shaped(
        &self,
        c: &mut Canvas,
        face: Face,
        size: f32,
        x: f32,
        baseline: f32,
        text: &str,
        ink: u8,
    ) -> f32 {
        let shaped = self.shaped(face, size, text);
        let key_size = (size * 4.0) as u32;
        let mut pen = x;
        for seg in &shaped.segs {
            for g in &seg.glyphs {
                let mut cache = self.gcache.borrow_mut();
                let (m, bitmap) = cache.entry((seg.face, g.id, key_size)).or_insert_with(|| {
                    self.font(seg.face)
                        .rasterize_indexed(g.id, size * crate::shape::face_scale(seg.face))
                });
                let gx = (pen + g.dx + m.xmin as f32).round() as i32;
                let gy = (baseline - g.dy - m.height as f32 - m.ymin as f32).round() as i32;
                for row in 0..m.height {
                    for col in 0..m.width {
                        let a = bitmap[row * m.width + col];
                        c.blend(gx + col as i32, gy + row as i32, ink, a);
                    }
                }
                pen += g.adv;
            }
        }
        pen
    }

    /// Whether a row reads right to left (its first strong character).
    #[must_use]
    pub fn row_rtl(&self, text: &str) -> bool {
        crate::shape::is_rtl_text(text)
            && unicode_bidi::BidiInfo::new(text, None)
                .paragraphs
                .first()
                .is_some_and(|p| p.level.is_rtl())
    }
}

/// A `TextMeasure` for one face and size, in whole pixels.
pub struct Measure<'a> {
    pub fonts: &'a Fonts,
    pub face: Face,
    pub size: f32,
}

impl sigma_core::TextMeasure for Measure<'_> {
    fn width(&self, text: &str) -> u32 {
        self.fonts.width(self.face, self.size, text).ceil() as u32
    }

    fn caret_x(&self, row: &str, at: usize) -> u32 {
        if crate::shape::is_complex_text(row) {
            self.fonts
                .shaped(self.face, self.size, row)
                .caret_x(at)
                .round()
                .max(0.0) as u32
        } else {
            self.width(&row[..at])
        }
    }

    fn index_at(&self, row: &str, x: u32) -> Option<usize> {
        crate::shape::is_complex_text(row).then(|| {
            self.fonts
                .shaped(self.face, self.size, row)
                .index_at(x as f32, row.len())
        })
    }

    fn rtl(&self, row: &str) -> bool {
        self.fonts.row_rtl(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_load_and_measure() {
        let f = Fonts::new();
        let w1 = f.width(Face::Literata, 23.0, "m");
        let w2 = f.width(Face::Literata, 23.0, "i");
        assert!(w1 > w2, "proportional face: m wider than i");
        assert!(f.width(Face::Atkinson, 20.0, "Writing") > 40.0);
        // The semibold and italic instances are real, different faces.
        let r = f.width(Face::Literata, 23.0, "Writing");
        assert!(f.width(Face::LiterataSemi, 23.0, "Writing") > r);
        assert!((f.width(Face::LiterataItalic, 23.0, "Writing") - r).abs() > 0.5);
    }

    #[test]
    fn draws_ink() {
        let f = Fonts::new();
        let mut c = Canvas::new(100, 40, 255);
        f.draw(&mut c, Face::Atkinson, 24.0, 2.0, 30.0, "Hi", 0);
        assert!(c.pixels.iter().any(|&p| p < 50));
    }
}

#[cfg(test)]
mod layout_glyphs {
    use sigma_core::layouts::{compose, spacing, ALL, CODES};

    /// v1.3: every character a layout can type (and every composed accent)
    /// has a glyph in both text faces and the key-label face.
    #[test]
    fn every_layout_character_has_a_glyph() {
        let faces = [
            ("Literata", super::LITERATA),
            ("Atkinson", super::ATKINSON),
            ("Atkinson Bold", super::ATKINSON_BOLD),
        ];
        let mut want = Vec::new();
        for l in ALL {
            for i in 0..CODES.len() {
                let (b, s, a) = l.chars(i);
                // Hangul jamo and the 半 key come from the font pack, not the bundled faces.
                want.extend(
                    [b, s, a]
                        .into_iter()
                        .flatten()
                        .filter(|&c| !sigma_core::hangul::is_jamo(c) && c != '半' && c != '中')
                        .map(spacing),
                );
            }
        }
        for d in [
            '\u{300}', '\u{301}', '\u{302}', '\u{303}', '\u{308}', '\u{30A}', '\u{30C}',
        ] {
            want.extend(
                "aeiouycnszdtrlAEIOUYCNSZDTRLαεηιουωΑΕΗΙΟΥΩ"
                    .chars()
                    .filter_map(|c| compose(d, c)),
            );
        }
        // Drawing falls back per character (decision 47), so a glyph
        // missing from Atkinson is fine when Literata has it; Literata
        // itself must cover everything.
        let lit =
            fontdue::Font::from_bytes(super::LITERATA, fontdue::FontSettings::default()).unwrap();
        // Arabic script and Hebrew come from their own faces (v1.8).
        let others: Vec<fontdue::Font> = [
            super::NASKH,
            super::HEBREW,
            super::DEVANAGARI,
            super::BENGALI,
            super::TAMIL,
            super::THAI,
        ]
        .into_iter()
        .map(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).unwrap())
        .collect();
        for (name, bytes) in faces {
            let f = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).unwrap();
            let missing: String = want
                .iter()
                .filter(|&&c| {
                    f.lookup_glyph_index(c) == 0
                        && lit.lookup_glyph_index(c) == 0
                        && others.iter().all(|o| o.lookup_glyph_index(c) == 0)
                })
                .collect();
            assert!(
                missing.is_empty(),
                "{name} (with fallback) lacks {missing:?}"
            );
        }
    }
}
