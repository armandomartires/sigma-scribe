//! User settings (brief §6 Options, S4) and their `.sigma-settings` file.
//!
//! The file is plain `key=value` lines. Unknown keys and bad values are
//! ignored (the default is used), so an older or newer file never stops the
//! app. v0.2.1 files (`keyboard=full` only) load unchanged.

use crate::buffer::TabKind;
use crate::keyboard::Layout;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Paper {
    #[default]
    Day,
    Night,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Typeface {
    #[default]
    Literata,
    Atkinson,
}

/// Keyboard tone: Theme follows the page, Light is always paper, Dark is
/// always night.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyTone {
    #[default]
    Theme,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Side {
    Left,
    #[default]
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// Landscape keyboard arrangement (brief §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LandKeys {
    #[default]
    Centre,
    Split,
    Full,
}

/// The pad beside the board in landscape Full.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pad {
    #[default]
    Numbers,
    Arrows,
}

pub const SIZE_RANGE: (f32, f32) = (14.0, 40.0);
pub const LEADING_RANGE: (f32, f32) = (1.2, 2.0);
pub const MARGIN_RANGE: (f32, f32) = (8.0, 400.0);
pub const WORDS_RANGE: (u8, u8) = (8, 16);
pub const SIZE_STEP: f32 = 1.0;
pub const LEADING_STEP: f32 = 0.1;
pub const MARGIN_STEP: f32 = 4.0;

/// Every setting. Sizes are in brief px (a 1264 px wide page); the renderer
/// scales them to the panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub paper: Paper,
    pub face: Typeface,
    pub line_on: bool,
    pub line_words: u8,
    pub size: f32,
    pub leading: f32,
    pub margin: f32,
    /// Tab key inserts spaces (Markdown default) or a tab character.
    pub tab_spaces: bool,
    pub tab_width: u8,
    pub keyboard: Layout,
    pub key_tone: KeyTone,
    pub drawer: Side,
    pub usb: bool,
    /// Reverse scroll (v1.0.1): wheel / two-finger notches move content the
    /// other way ("natural", like a touch screen). Default off = 1.0.
    pub natural_scroll: bool,
    pub orientation: Orientation,
    pub land_keys: LandKeys,
    pub pad: Pad,
    pub pad_side: Side,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            paper: Paper::Day,
            face: Typeface::Literata,
            line_on: false,
            line_words: 12,
            size: 20.0,
            leading: 1.5,
            margin: 28.0,
            tab_spaces: true,
            tab_width: 4,
            keyboard: Layout::Full,
            key_tone: KeyTone::Theme,
            drawer: Side::Right,
            usb: false,
            natural_scroll: false,
            orientation: Orientation::Portrait,
            land_keys: LandKeys::Centre,
            pad: Pad::Numbers,
            pad_side: Side::Right,
        }
    }
}

/// One change from the Options sheet or the landscape keyboard bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    Paper(Paper),
    Face(Typeface),
    LineOn(bool),
    /// +1 / -1 word.
    LineWords(i8),
    /// +1 / -1 step.
    Size(i8),
    Leading(i8),
    Margin(i8),
    TabSpaces(bool),
    TabWidth(u8),
    Keyboard(Layout),
    KeyTone(KeyTone),
    Drawer(Side),
    Usb(bool),
    NaturalScroll(bool),
    Orientation(Orientation),
    LandKeys(LandKeys),
    Pad(Pad),
    PadSide(Side),
}

impl Change {
    /// Changes after which Line mode must refit size and margin, and which
    /// of the two the user moved (`Some(true)` = size, `Some(false)` =
    /// margin, `None` = keep size, refit margin).
    #[must_use]
    pub fn refit(self) -> Option<Option<bool>> {
        match self {
            Self::Size(_) => Some(Some(true)),
            Self::Margin(_) => Some(Some(false)),
            Self::LineOn(_) | Self::LineWords(_) | Self::Face(_) | Self::Orientation(_) => {
                Some(None)
            }
            _ => None,
        }
    }

    /// Paper, orientation and keyboard re-layouts repaint everything with a
    /// flashing refresh (a large partial change would leave ghosting).
    #[must_use]
    pub fn needs_full_refresh(self) -> bool {
        matches!(
            self,
            Self::Paper(_)
                | Self::Orientation(_)
                | Self::Keyboard(_)
                | Self::LandKeys(_)
                | Self::Pad(_)
                | Self::PadSide(_)
        )
    }
}

fn clampf(v: f32, (lo, hi): (f32, f32)) -> f32 {
    v.clamp(lo, hi)
}

/// Rounds to one decimal (steppers never drift).
fn tenth(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

impl Settings {
    /// Applies a change, clamping every value to its range.
    pub fn apply(&mut self, c: Change) {
        match c {
            Change::Paper(p) => self.paper = p,
            Change::Face(f) => self.face = f,
            Change::LineOn(on) => self.line_on = on,
            Change::LineWords(d) => {
                self.line_words = (i16::from(self.line_words) + i16::from(d))
                    .clamp(i16::from(WORDS_RANGE.0), i16::from(WORDS_RANGE.1))
                    as u8;
            }
            Change::Size(d) => {
                self.size = clampf(tenth(self.size + f32::from(d) * SIZE_STEP), SIZE_RANGE);
            }
            Change::Leading(d) => {
                self.leading = clampf(
                    tenth(self.leading + f32::from(d) * LEADING_STEP),
                    LEADING_RANGE,
                );
            }
            Change::Margin(d) => {
                self.margin = clampf(
                    tenth(self.margin + f32::from(d) * MARGIN_STEP),
                    MARGIN_RANGE,
                );
            }
            Change::TabSpaces(on) => self.tab_spaces = on,
            Change::TabWidth(w) => {
                if matches!(w, 2 | 4 | 8) {
                    self.tab_width = w;
                }
            }
            Change::Keyboard(l) => self.keyboard = l,
            Change::KeyTone(t) => self.key_tone = t,
            Change::Drawer(s) => self.drawer = s,
            Change::Usb(on) => self.usb = on,
            Change::NaturalScroll(on) => self.natural_scroll = on,
            Change::Orientation(o) => self.orientation = o,
            Change::LandKeys(k) => self.land_keys = k,
            Change::Pad(p) => self.pad = p,
            Change::PadSide(s) => self.pad_side = s,
        }
    }

    /// What the Tab key inserts.
    #[must_use]
    pub fn tab_kind(&self) -> TabKind {
        if self.tab_spaces {
            TabKind::Spaces(self.tab_width)
        } else {
            TabKind::Tab
        }
    }

    /// Whether the keyboard draws night colours, given the page's paper.
    #[must_use]
    pub fn keys_dark(&self) -> bool {
        match self.key_tone {
            KeyTone::Theme => self.paper == Paper::Night,
            KeyTone::Light => false,
            KeyTone::Dark => true,
        }
    }

    /// Reads a settings file; anything missing or unreadable is a default.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            let f = || v.parse::<f32>().ok().filter(|x| x.is_finite());
            let b = || match v {
                "on" | "true" | "1" => Some(true),
                "off" | "false" | "0" => Some(false),
                _ => None,
            };
            match k.trim() {
                "keyboard" => s.keyboard = Layout::parse(v).unwrap_or(s.keyboard),
                "paper" => {
                    s.paper = match v {
                        "night" => Paper::Night,
                        "day" => Paper::Day,
                        _ => s.paper,
                    }
                }
                "type" => {
                    s.face = match v {
                        "atkinson" => Typeface::Atkinson,
                        "literata" => Typeface::Literata,
                        _ => s.face,
                    }
                }
                "line" => s.line_on = b().unwrap_or(s.line_on),
                "line_words" => {
                    if let Ok(n) = v.parse::<u8>() {
                        s.line_words = n.clamp(WORDS_RANGE.0, WORDS_RANGE.1);
                    }
                }
                "size" => s.size = f().map_or(s.size, |x| clampf(x, SIZE_RANGE)),
                "leading" => s.leading = f().map_or(s.leading, |x| clampf(x, LEADING_RANGE)),
                "margin" => s.margin = f().map_or(s.margin, |x| clampf(x, MARGIN_RANGE)),
                "tab" => {
                    s.tab_spaces = match v {
                        "tab" => false,
                        "spaces" => true,
                        _ => s.tab_spaces,
                    }
                }
                "tab_width" => {
                    if let Ok(n @ (2 | 4 | 8)) = v.parse::<u8>() {
                        s.tab_width = n;
                    }
                }
                "key_tone" => {
                    s.key_tone = match v {
                        "theme" => KeyTone::Theme,
                        "light" => KeyTone::Light,
                        "dark" => KeyTone::Dark,
                        _ => s.key_tone,
                    }
                }
                "drawer" => s.drawer = parse_side(v).unwrap_or(s.drawer),
                "usb" => s.usb = b().unwrap_or(s.usb),
                "scroll" => {
                    s.natural_scroll = match v {
                        "natural" => true,
                        "traditional" => false,
                        _ => s.natural_scroll,
                    }
                }
                "orientation" => {
                    s.orientation = match v {
                        "portrait" => Orientation::Portrait,
                        "landscape" => Orientation::Landscape,
                        _ => s.orientation,
                    }
                }
                "landscape_keys" => {
                    s.land_keys = match v {
                        "centre" => LandKeys::Centre,
                        "split" => LandKeys::Split,
                        "full" => LandKeys::Full,
                        _ => s.land_keys,
                    }
                }
                "pad" => {
                    s.pad = match v {
                        "numbers" => Pad::Numbers,
                        "arrows" => Pad::Arrows,
                        _ => s.pad,
                    }
                }
                "pad_side" => s.pad_side = parse_side(v).unwrap_or(s.pad_side),
                _ => {}
            }
        }
        s
    }

    /// The file body; [`Settings::parse`] reads it back unchanged.
    #[must_use]
    pub fn serialize(&self) -> String {
        let side = |s: Side| match s {
            Side::Left => "left",
            Side::Right => "right",
        };
        let on = |b: bool| if b { "on" } else { "off" };
        format!(
            "keyboard={}\npaper={}\ntype={}\nline={}\nline_words={}\nsize={}\nleading={}\nmargin={}\n\
             tab={}\ntab_width={}\nkey_tone={}\ndrawer={}\nusb={}\nscroll={}\norientation={}\n\
             landscape_keys={}\npad={}\npad_side={}\n",
            self.keyboard.name(),
            match self.paper {
                Paper::Day => "day",
                Paper::Night => "night",
            },
            match self.face {
                Typeface::Literata => "literata",
                Typeface::Atkinson => "atkinson",
            },
            on(self.line_on),
            self.line_words,
            tenth(self.size),
            tenth(self.leading),
            tenth(self.margin),
            if self.tab_spaces { "spaces" } else { "tab" },
            self.tab_width,
            match self.key_tone {
                KeyTone::Theme => "theme",
                KeyTone::Light => "light",
                KeyTone::Dark => "dark",
            },
            side(self.drawer),
            on(self.usb),
            if self.natural_scroll { "natural" } else { "traditional" },
            match self.orientation {
                Orientation::Portrait => "portrait",
                Orientation::Landscape => "landscape",
            },
            match self.land_keys {
                LandKeys::Centre => "centre",
                LandKeys::Split => "split",
                LandKeys::Full => "full",
            },
            match self.pad {
                Pad::Numbers => "numbers",
                Pad::Arrows => "arrows",
            },
            side(self.pad_side),
        )
    }
}

fn parse_side(v: &str) -> Option<Side> {
    match v {
        "left" => Some(Side::Left),
        "right" => Some(Side::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_brief() {
        let s = Settings::default();
        assert_eq!((s.size, s.leading, s.margin), (20.0, 1.5, 28.0));
        assert_eq!(s.line_words, 12);
        assert!(!s.line_on && s.tab_spaces && s.tab_width == 4);
        assert_eq!(s.tab_kind(), TabKind::Spaces(4));
        assert_eq!(s.keyboard, Layout::Full);
    }

    #[test]
    fn round_trip_every_field() {
        let mut s = Settings::default();
        for c in [
            Change::Paper(Paper::Night),
            Change::Face(Typeface::Atkinson),
            Change::LineOn(true),
            Change::LineWords(3),
            Change::Size(2),
            Change::Leading(-1),
            Change::Margin(5),
            Change::TabSpaces(false),
            Change::TabWidth(8),
            Change::Keyboard(Layout::Compact),
            Change::KeyTone(KeyTone::Light),
            Change::Drawer(Side::Left),
            Change::Usb(true),
            Change::Orientation(Orientation::Landscape),
            Change::LandKeys(LandKeys::Split),
            Change::Pad(Pad::Arrows),
            Change::PadSide(Side::Left),
        ] {
            s.apply(c);
        }
        assert_eq!(Settings::parse(&s.serialize()), s);
        assert_eq!(s.tab_kind(), TabKind::Tab);
        assert_eq!(
            (s.size, s.leading, s.margin, s.line_words),
            (22.0, 1.4, 48.0, 15)
        );
    }

    #[test]
    fn old_files_and_garbage_load() {
        let s = Settings::parse("keyboard=compact\n");
        assert_eq!(s.keyboard, Layout::Compact);
        assert_eq!(s.paper, Paper::Day);
        let g = Settings::parse(
            "size=NaN\nmargin=-5\nline_words=99\ntab_width=3\nwhat=ever\nnoequals\n",
        );
        assert_eq!(g.size, 20.0);
        assert_eq!(g.margin, MARGIN_RANGE.0);
        assert_eq!(g.line_words, 16);
        assert_eq!(g.tab_width, 4);
    }

    #[test]
    fn steppers_clamp() {
        let mut s = Settings::default();
        for _ in 0..100 {
            s.apply(Change::Size(1));
            s.apply(Change::Leading(-1));
            s.apply(Change::LineWords(-1));
        }
        assert_eq!((s.size, s.leading, s.line_words), (40.0, 1.2, 8));
        s.apply(Change::TabWidth(5));
        assert_eq!(s.tab_width, 4);
    }

    #[test]
    fn keyboard_tone() {
        let mut s = Settings::default();
        assert!(!s.keys_dark());
        s.apply(Change::Paper(Paper::Night));
        assert!(s.keys_dark());
        s.apply(Change::KeyTone(KeyTone::Light));
        assert!(!s.keys_dark());
        s.apply(Change::Paper(Paper::Day));
        s.apply(Change::KeyTone(KeyTone::Dark));
        assert!(s.keys_dark());
    }
}
