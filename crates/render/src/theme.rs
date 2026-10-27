//! Design tokens from .ai/design-brief.md §2, as 8-bit grey (Rec. 601 luma of
//! the brief's hex values; the Forma is greyscale) and as a scale from the
//! brief's 1264 px page to the actual panel width.

/// Page colours as 8-bit grey levels (0 is black, 255 white).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Page background.
    pub paper: u8,
    /// Text and strokes.
    pub ink: u8,
    /// Thin rules and dividers.
    pub rule: u8,
    /// Secondary text.
    pub mute: u8,
    /// Light fill behind highlighted spans and blocks.
    pub wash: u8,
    /// The desk: the margins beside the text column.
    pub gutter: u8,
}

impl Theme {
    /// Day: paper #f3f0e8, ink #1c1c1c, rule #c9c2b4, mute #5e584e, wash #e7e1d4, gutter #e0dcd4.
    pub const DAY: Theme = Theme {
        paper: 240,
        ink: 28,
        rule: 195,
        mute: 89,
        wash: 226,
        gutter: 220,
    };
    /// Night: the same drawing inverted (brief: not a grey card).
    pub const NIGHT: Theme = Theme {
        paper: 17,
        ink: 233,
        rule: 60,
        mute: 161,
        wash: 28,
        gutter: 36,
    };

    /// The page theme for Options → Paper.
    #[must_use]
    pub fn for_paper(p: sigma_core::settings::Paper) -> Self {
        match p {
            sigma_core::settings::Paper::Day => Self::DAY,
            sigma_core::settings::Paper::Night => Self::NIGHT,
        }
    }
}

/// Brief sizes are given for a 1264 px wide portrait page.
pub const BRIEF_WIDTH: f32 = 1264.0;

/// Converts brief pixels to panel pixels.
#[derive(Clone, Copy, Debug)]
pub struct Scale(pub f32);

impl Scale {
    /// The scale for a panel `width` pixels wide.
    #[must_use]
    pub fn for_width(width: u32) -> Self {
        Self(width as f32 / BRIEF_WIDTH)
    }

    /// Converts a size in brief pixels to whole panel pixels, rounded.
    #[must_use]
    pub fn px(self, brief: f32) -> i32 {
        (brief * self.0).round() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forma_scale() {
        let s = Scale::for_width(1440);
        assert_eq!(s.px(44.0), 50); // minimum hit target
        assert_eq!(s.px(52.0), 59); // title bar
        assert_eq!(s.px(20.0), 23); // body text
    }
}
