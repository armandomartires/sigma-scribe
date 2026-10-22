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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Literata,
    Atkinson,
    AtkinsonBold,
    /// Literata at weight 600 (preview headings and `**strong**`, as the mock).
    LiterataSemi,
    /// Literata Italic at weight 400 (preview `*em*`).
    LiterataItalic,
}

type GlyphKey = (Face, char, u32);

pub struct Fonts {
    faces: [Font; 5],
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
            ],
            cache: RefCell::default(),
        }
    }

    fn font(&self, face: Face) -> &Font {
        &self.faces[face as usize]
    }

    /// Advance width of `text` in pixels at `size` px. Tabs are 4 spaces.
    #[must_use]
    pub fn width(&self, face: Face, size: f32, text: &str) -> f32 {
        let f = self.font(face);
        text.chars()
            .map(|c| {
                if c == '\t' {
                    4.0 * f.metrics(' ', size).advance_width
                } else {
                    f.metrics(c, size).advance_width
                }
            })
            .sum()
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
        let mut pen = x;
        let key_size = (size * 4.0) as u32;
        for ch in text.chars() {
            if ch == '\t' {
                pen += self.width(face, size, "\t");
                continue;
            }
            let mut cache = self.cache.borrow_mut();
            let (m, bitmap) = cache
                .entry((face, ch, key_size))
                .or_insert_with(|| self.font(face).rasterize(ch, size));
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
                want.extend([b, s, a].into_iter().flatten().map(spacing));
            }
        }
        for d in ['\u{300}', '\u{301}', '\u{302}', '\u{303}', '\u{308}'] {
            want.extend("aeiouycnAEIOUYCN".chars().filter_map(|c| compose(d, c)));
        }
        for (name, bytes) in faces {
            let f = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).unwrap();
            let missing: String = want
                .iter()
                .filter(|&&c| f.lookup_glyph_index(c) == 0)
                .collect();
            assert!(missing.is_empty(), "{name} lacks {missing:?}");
        }
    }
}
