//! One font file: its character map, metrics and outlines (`skrifa`), and
//! glyphs drawn into coverage masks (`zeno`). [`crate::fonts::Fonts`] holds
//! one per face; `hwr-train` uses it to draw its training letters.

use skrifa::instance::{LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::{Command, Mask, Vector};

/// A parsed font file.
#[derive(Clone)]
pub struct FontFile {
    font: FontRef<'static>,
    charmap: skrifa::charmap::Charmap<'static>,
    /// Advances in font units.
    metrics: GlyphMetrics<'static>,
    upem: f32,
    /// Ascent in font units, when the font has line metrics.
    ascent: Option<f32>,
}

/// A glyph drawn into an 8-bit coverage mask, positioned against the pen
/// and the baseline.
#[derive(Clone, Debug, Default)]
pub struct GlyphMask {
    /// Pixels from the pen to the mask's left column.
    pub left: i32,
    /// Pixels from the baseline down to the mask's top row (negative above
    /// the baseline).
    pub top: i32,
    /// Width of the mask in pixels.
    pub width: usize,
    /// Height of the mask in pixels.
    pub height: usize,
    /// Coverage row by row from the top left, 0 (none) to 255 (full).
    pub coverage: Vec<u8>,
}

impl FontFile {
    /// Parses font `index` of `bytes` (a single font takes 0; a collection
    /// picks one). `None` when it is not a font this can read.
    #[must_use]
    pub fn from_static(bytes: &'static [u8], index: u32) -> Option<Self> {
        let font = FontRef::from_index(bytes, index).ok()?;
        let line = font.metrics(Size::unscaled(), LocationRef::default());
        if line.units_per_em == 0 {
            return None;
        }
        Some(Self {
            charmap: font.charmap(),
            metrics: font.glyph_metrics(Size::unscaled(), LocationRef::default()),
            upem: f32::from(line.units_per_em),
            ascent: (line.ascent > 0.0).then_some(line.ascent),
            font,
        })
    }

    /// [`FontFile::from_static`] for bytes read at run time. The bytes of a
    /// font that parses are kept for the rest of the process, so load each
    /// file once.
    #[must_use]
    pub fn from_owned(bytes: Vec<u8>, index: u32) -> Option<Self> {
        FontRef::from_index(&bytes, index).ok()?;
        Self::from_static(Box::leak(bytes.into_boxed_slice()), index)
    }

    /// The file's bytes.
    #[must_use]
    pub fn bytes(&self) -> &'static [u8] {
        self.font.data().as_bytes()
    }

    /// The glyph for `c`, or 0 (the font's "missing" box) when it has none.
    #[must_use]
    pub fn glyph(&self, c: char) -> u32 {
        self.charmap.map(c).map_or(0, GlyphId::to_u32)
    }

    /// Advance of glyph `id` in pixels at `px` pixels per em.
    #[must_use]
    pub fn advance(&self, id: u32, px: f32) -> f32 {
        self.metrics.advance_width(GlyphId::new(id)).unwrap_or(0.0) * px / self.upem
    }

    /// Advance of `c` in pixels at `px` pixels per em.
    #[must_use]
    pub fn char_advance(&self, c: char, px: f32) -> f32 {
        self.advance(self.glyph(c), px)
    }

    /// Ascent in pixels at `px` pixels per em, when the font has line
    /// metrics.
    #[must_use]
    pub fn ascent(&self, px: f32) -> Option<f32> {
        self.ascent.map(|a| a * px / self.upem)
    }

    /// Font units per em.
    #[must_use]
    pub fn units_per_em(&self) -> f32 {
        self.upem
    }

    /// Glyph `id` drawn at `px` pixels per em, unhinted, with its origin on
    /// a pixel corner. An empty mask for a glyph with no outline (a space).
    #[must_use]
    pub fn rasterize(&self, id: u32, px: f32) -> GlyphMask {
        let mut pen = Pen(Vec::new());
        let drawn = self.font.outline_glyphs().get(GlyphId::new(id)).map(|g| {
            g.draw(
                DrawSettings::unhinted(Size::new(px), LocationRef::default()),
                &mut pen,
            )
        });
        if !matches!(drawn, Some(Ok(_))) || pen.0.is_empty() {
            return GlyphMask::default();
        }
        let (coverage, at) = Mask::new(&pen.0).render();
        GlyphMask {
            left: at.left,
            top: at.top,
            width: at.width as usize,
            height: at.height as usize,
            coverage,
        }
    }

    /// [`FontFile::rasterize`] for the glyph of `c`.
    #[must_use]
    pub fn rasterize_char(&self, c: char, px: f32) -> GlyphMask {
        self.rasterize(self.glyph(c), px)
    }
}

/// Collects an outline as zeno commands, flipped to y down.
struct Pen(Vec<Command>);

fn pt(x: f32, y: f32) -> Vector {
    Vector::new(x, -y)
}

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::MoveTo(pt(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::LineTo(pt(x, y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.push(Command::QuadTo(pt(cx0, cy0), pt(x, y)));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .push(Command::CurveTo(pt(cx0, cy0), pt(cx1, cy1), pt(x, y)));
    }

    fn close(&mut self) {
        self.0.push(Command::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static LITERATA: &[u8] = include_bytes!("../../../assets/fonts/Literata.ttf");

    #[test]
    fn letters_have_glyphs_advances_and_ink() {
        let f = FontFile::from_static(LITERATA, 0).unwrap();
        assert_ne!(f.glyph('m'), 0);
        assert_eq!(f.glyph('\u{E000}'), 0);
        assert!(f.char_advance('m', 30.0) > f.char_advance('i', 30.0));
        let a = f.ascent(30.0).unwrap();
        assert!(a > 20.0 && a < 40.0, "{a}");
        let m = f.rasterize_char('H', 30.0);
        assert!(m.width > 10 && m.height > 15, "{m:?}");
        assert_eq!(m.coverage.len(), m.width * m.height);
        assert!(m.coverage.contains(&255));
        // H sits on the baseline: its mask ends there.
        assert_eq!(m.top + m.height as i32, 0);
        assert!(f.rasterize_char(' ', 30.0).coverage.is_empty());
    }

    #[test]
    fn a_descender_reaches_below_the_baseline() {
        let f = FontFile::from_static(LITERATA, 0).unwrap();
        let p = f.rasterize_char('p', 30.0);
        assert!(p.top + p.height as i32 > 3, "{p:?}");
    }
}
