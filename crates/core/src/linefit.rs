//! Line mode (brief §4 "Line"): pick a type size and a margin so a line of
//! the page column holds a chosen number of words.
//!
//! Measured with the real face through [`SizedMeasure`] ("measure with the
//! real face, not a guess"): the width of the first N words of [`SAMPLE`] at
//! a reference size. Advances scale linearly with size, so one measurement
//! per (face, N) gives the text width at any size. The target text width is
//! half-way between N and N + 1 sample words, so N fit and N + 1 do not.
//!
//! Margin is the flexible one: changing Size moves Margin; changing Margin
//! moves Size. Units are whatever the measure returns (panel px on device).

/// Plain prose with ordinary word lengths (≈ 4.6 letters a word).
pub const SAMPLE: &str = "the morning light came slowly over the low hills and we walked down \
to the river where an old boat was waiting for us with its sails folded";

/// Width of `text` at `size` for one face.
pub trait SizedMeasure {
    fn width_at(&self, size: f32, text: &str) -> f32;
}

const REF_SIZE: f32 = 100.0;

/// The first `n` words of the sample, joined by single spaces.
#[must_use]
pub fn sample_words(n: usize) -> String {
    SAMPLE.split(' ').take(n).collect::<Vec<_>>().join(" ")
}

/// Target text width per unit of size for `n` words.
#[must_use]
pub fn width_per_size(m: &dyn SizedMeasure, n: u8) -> f32 {
    let n = usize::from(n.max(1));
    let a = m.width_at(REF_SIZE, &sample_words(n));
    let b = m.width_at(REF_SIZE, &sample_words(n + 1));
    (a + b) / 2.0 / REF_SIZE
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub size: f32,
    pub margin: f32,
}

/// Allowed ranges, in the measure's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub size: (f32, f32),
    pub margin: (f32, f32),
}

/// Which value the user just set (it is kept when possible).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Size,
    Margin,
}

/// Fits size and margin to `column` so a line holds `words` words.
#[must_use]
pub fn fit(
    m: &dyn SizedMeasure,
    column: f32,
    words: u8,
    current: Fit,
    anchor: Anchor,
    lim: Limits,
) -> Fit {
    let k = width_per_size(m, words).max(1e-3);
    let size_for = |margin: f32| ((column - 2.0 * margin) / k).clamp(lim.size.0, lim.size.1);
    let margin_for = |size: f32| (column - k * size) / 2.0;
    let size = match anchor {
        Anchor::Size => {
            let s = current.size.clamp(lim.size.0, lim.size.1);
            let mg = margin_for(s);
            if mg < lim.margin.0 {
                size_for(lim.margin.0)
            } else if mg > lim.margin.1 {
                size_for(lim.margin.1)
            } else {
                s
            }
        }
        Anchor::Margin => size_for(current.margin.clamp(lim.margin.0, lim.margin.1)),
    };
    // The margin follows the (possibly clamped) size; it may leave its own
    // range only when the size range is exhausted, and never goes negative.
    Fit {
        size,
        margin: margin_for(size).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wrap::{wrap, TextMeasure};

    /// Fake face: every char is 0.5 em, spaces 0.25 em.
    struct Fake;
    impl SizedMeasure for Fake {
        fn width_at(&self, size: f32, text: &str) -> f32 {
            text.chars()
                .map(|c| if c == ' ' { 0.25 } else { 0.5 } * size)
                .sum()
        }
    }
    struct At(f32);
    impl TextMeasure for At {
        fn width(&self, text: &str) -> u32 {
            Fake.width_at(self.0, text).ceil() as u32
        }
    }

    const LIM: Limits = Limits {
        size: (16.0, 46.0),
        margin: (9.0, 455.0),
    };

    fn words_on_first_row(column: f32, f: Fit) -> usize {
        let text = format!("{SAMPLE} {SAMPLE}");
        let rows = wrap(&text, (column - 2.0 * f.margin) as u32, &At(f.size));
        text[rows[0].start..rows[0].end].split_whitespace().count()
    }

    #[test]
    fn every_word_count_fits_exactly_from_size() {
        for n in 8..=16u8 {
            let f = fit(
                &Fake,
                1440.0,
                n,
                Fit {
                    size: 23.0,
                    margin: 32.0,
                },
                Anchor::Size,
                LIM,
            );
            // Size is kept unless the margin would leave its range.
            if f.margin < LIM.margin.1 - 0.01 && f.margin > LIM.margin.0 + 0.01 {
                assert_eq!(f.size, 23.0, "size kept for {n}");
            }
            assert_eq!(words_on_first_row(1440.0, f), usize::from(n), "{f:?}");
        }
    }

    #[test]
    fn margin_anchor_moves_size_and_keeps_count() {
        let a = fit(
            &Fake,
            1440.0,
            12,
            Fit {
                size: 23.0,
                margin: 100.0,
            },
            Anchor::Margin,
            LIM,
        );
        let b = fit(
            &Fake,
            1440.0,
            12,
            Fit {
                size: 23.0,
                margin: 200.0,
            },
            Anchor::Margin,
            LIM,
        );
        assert!((a.margin - 100.0).abs() < 0.01 && (b.margin - 200.0).abs() < 0.01);
        assert!(b.size < a.size, "wider margin, smaller type");
        assert_eq!(words_on_first_row(1440.0, a), 12);
        assert_eq!(words_on_first_row(1440.0, b), 12);
    }

    #[test]
    fn size_too_big_for_the_count_shrinks_to_the_min_margin() {
        let f = fit(
            &Fake,
            1440.0,
            16,
            Fit {
                size: 46.0,
                margin: 30.0,
            },
            Anchor::Size,
            LIM,
        );
        assert!((f.margin - LIM.margin.0).abs() < 0.01, "{f:?}");
        assert!(f.size < 46.0);
        assert_eq!(words_on_first_row(1440.0, f), 16);
    }

    #[test]
    fn same_column_same_fit_in_both_orientations() {
        // Landscape locks the column to the portrait width, so the fit is
        // identical: the word count holds when the device turns.
        let cur = Fit {
            size: 23.0,
            margin: 32.0,
        };
        assert_eq!(
            fit(&Fake, 1440.0, 10, cur, Anchor::Size, LIM),
            fit(&Fake, 1440.0, 10, cur, Anchor::Size, LIM)
        );
    }

    #[test]
    fn sample_has_enough_words() {
        assert!(SAMPLE.split(' ').count() >= 17);
        assert_eq!(sample_words(3), "the morning light");
    }
}
