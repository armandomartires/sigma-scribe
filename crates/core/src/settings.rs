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
    /// Follow the accelerometer (v1.2 default); without a sensor the last
    /// applied rotation stays.
    #[default]
    Auto,
    Portrait,
    Landscape,
}

/// The rotation the screen is drawn in (v1.2), clockwise from the panel's
/// portrait: 0 portrait, 90 landscape, 180 portrait inverted, 270
/// landscape inverted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Rotation {
    #[default]
    R0,
    R90,
    R180,
    R270,
}

impl Rotation {
    #[must_use]
    pub fn is_landscape(self) -> bool {
        matches!(self, Self::R90 | Self::R270)
    }

    #[must_use]
    pub fn is_inverted(self) -> bool {
        matches!(self, Self::R180 | Self::R270)
    }

    /// The manual rotation for an orientation choice (`None` for Auto).
    #[must_use]
    pub fn manual(o: Orientation, upside_down: bool) -> Option<Self> {
        Some(match (o, upside_down) {
            (Orientation::Auto, _) => return None,
            (Orientation::Portrait, false) => Self::R0,
            (Orientation::Landscape, false) => Self::R90,
            (Orientation::Portrait, true) => Self::R180,
            (Orientation::Landscape, true) => Self::R270,
        })
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::R0 => "Portrait",
            Self::R90 => "Landscape",
            Self::R180 => "Portrait inverted",
            Self::R270 => "Landscape inverted",
        }
    }

    fn degrees(self) -> u16 {
        match self {
            Self::R0 => 0,
            Self::R90 => 90,
            Self::R180 => 180,
            Self::R270 => 270,
        }
    }

    fn from_degrees(d: &str) -> Option<Self> {
        Some(match d {
            "0" => Self::R0,
            "90" => Self::R90,
            "180" => Self::R180,
            "270" => Self::R270,
            _ => return None,
        })
    }
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
    /// Options → Orientation: Auto (default, v1.2), Portrait or Landscape.
    pub orientation: Orientation,
    /// Options → Upside down (v1.2): turns a manual orientation 180°.
    pub upside_down: bool,
    /// The rotation in force: the manual choice, or the last one Auto
    /// applied (kept when there is no sensor).
    pub rotation: Rotation,
    pub land_keys: LandKeys,
    pub pad: Pad,
    pub pad_side: Side,
    /// UI scale in percent (v1.1): 100, 125, 150, 175 or 200 (default).
    /// Scales the interface; the note text Size stays separate.
    pub ui_scale: u16,
    /// Options → Layout (v1.3): on-screen Full board and USB keyboards.
    pub kb_layout: crate::layouts::KeyLayout,
    /// Keyboard & input → Keyboard size (v1.4.3): 0 = Auto (the UI scale,
    /// at most 150 %), or 100 / 125 / 150. Saved as `keyboard_size=`.
    pub kb_size: u16,
    /// Keyboard & input → Ink pen (v1.5.0): stroke width. `ink_pen=`.
    pub ink_pen: crate::ink::Pen,
    /// Keyboard & input → Fast ink (A2) (v1.5.0): live ink with the 1-bit
    /// A2 waveform instead of DU. Off by default; decided in the device
    /// pass (plan §12.9 decision 6). `ink_a2=`.
    pub ink_a2: bool,
    /// Word mode in the Write panel (v1.5.3, experimental, failed its
    /// gate: off by default). `ink_words=`.
    pub ink_words: bool,
    /// Letter boxes instead of the free area (v1.5.3 fallback). `ink_boxes=`.
    pub ink_boxes: bool,
    /// Language → Override system language (v1.5.5, decision 57). Off: the
    /// interface follows the Kobo's `CurrentLocale` (read-only, never
    /// written). `ui_lang_override=`.
    pub lang_override: bool,
    /// The interface language chosen while the override is on. `ui_lang=`.
    pub ui_lang: UiLang,
}

/// An interface language the user can pick (v1.5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UiLang {
    #[default]
    En,
    Fr,
    Pt,
    De,
    Es,
    It,
    Nl,
    Ca,
    Da,
    Sv,
    Fi,
    Nb,
    Tr,
    Pl,
    Cs,
    Ro,
}

impl UiLang {
    pub const ALL: [UiLang; 16] = [
        UiLang::En,
        UiLang::Fr,
        UiLang::Pt,
        UiLang::De,
        UiLang::Es,
        UiLang::It,
        UiLang::Nl,
        UiLang::Ca,
        UiLang::Da,
        UiLang::Sv,
        UiLang::Fi,
        UiLang::Nb,
        UiLang::Tr,
        UiLang::Pl,
        UiLang::Cs,
        UiLang::Ro,
    ];

    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
            Self::Pt => "pt",
            Self::De => "de",
            Self::Es => "es",
            Self::It => "it",
            Self::Nl => "nl",
            Self::Ca => "ca",
            Self::Da => "da",
            Self::Sv => "sv",
            Self::Fi => "fi",
            Self::Nb => "nb",
            Self::Tr => "tr",
            Self::Pl => "pl",
            Self::Cs => "cs",
            Self::Ro => "ro",
        }
    }

    #[must_use]
    pub fn from_code(v: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.code() == v)
    }

    /// The language's own name (not translated).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Fr => "Français",
            Self::Pt => "Português",
            Self::De => "Deutsch",
            Self::Es => "Español",
            Self::It => "Italiano",
            Self::Nl => "Nederlands",
            Self::Ca => "Català",
            Self::Da => "Dansk",
            Self::Sv => "Svenska",
            Self::Fi => "Suomi",
            Self::Nb => "Norsk",
            Self::Tr => "Türkçe",
            Self::Pl => "Polski",
            Self::Cs => "Čeština",
            Self::Ro => "Română",
        }
    }
}

/// Keyboard size choices (v1.4.3); 0 is Auto.
pub const KB_SIZES: [u16; 4] = [0, 100, 125, 150];
/// Auto keyboard size never goes above this (v1.4.3, user report).
pub const KB_AUTO_MAX: u16 = 150;

/// UI scale steps (v1.1, S11).
pub const UI_SCALES: [u16; 5] = [100, 125, 150, 175, 200];

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
            orientation: Orientation::Auto,
            upside_down: false,
            rotation: Rotation::R0,
            land_keys: LandKeys::Centre,
            pad: Pad::Numbers,
            pad_side: Side::Right,
            ui_scale: 200,
            kb_layout: crate::layouts::KeyLayout::Us,
            kb_size: 0,
            ink_pen: crate::ink::Pen::Finger,
            ink_a2: false,
            ink_words: false,
            ink_boxes: false,
            lang_override: false,
            ui_lang: UiLang::En,
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
    UpsideDown(bool),
    /// Auto applied a sensor reading (v1.2).
    Rotate(Rotation),
    LandKeys(LandKeys),
    Pad(Pad),
    PadSide(Side),
    /// One UI-scale step up (+1) or down (-1).
    UiScale(i8),
    /// One step through the keyboard layouts (v1.3).
    KbLayout(i8),
    /// Keyboard size (v1.4.3): 0 = Auto, or a percentage from `KB_SIZES`.
    KbSize(u16),
    /// Keyboard size Auto on / off (v1.4.4). Off keeps the size Auto was
    /// giving; on goes back to following the UI scale.
    KbAuto(bool),
    /// Ink pen width (v1.5.0).
    InkPen(crate::ink::Pen),
    /// Fast ink A2 waveform on / off (v1.5.0).
    InkA2(bool),
    /// Word mode on / off (v1.5.3).
    InkWords(bool),
    /// Letter boxes on / off (v1.5.3).
    InkBoxes(bool),
    /// Override system language on / off (v1.5.5).
    LangOverride(bool),
    /// Interface language while the override is on (v1.5.5).
    UiLang(UiLang),
    /// Open Train from Settings (v1.5.3; not a setting).
    TrainSamples,
    /// Delete the enrolled handwriting samples (v1.5.2). Not a setting:
    /// the app acts on it; `apply` ignores it.
    DeleteSamples,
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
            Self::LineOn(_)
            | Self::LineWords(_)
            | Self::Face(_)
            | Self::Orientation(_)
            | Self::UpsideDown(_)
            | Self::Rotate(_) => Some(None),
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
                | Self::UpsideDown(_)
                | Self::Rotate(_)
                | Self::Keyboard(_)
                | Self::LangOverride(_)
                | Self::UiLang(_)
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
            Change::InkPen(p) => self.ink_pen = p,
            Change::InkA2(on) => self.ink_a2 = on,
            Change::InkWords(on) => self.ink_words = on,
            Change::InkBoxes(on) => self.ink_boxes = on,
            Change::LangOverride(on) => self.lang_override = on,
            Change::UiLang(l) => self.ui_lang = l,
            Change::TrainSamples => {}
            Change::DeleteSamples => {}
            Change::Orientation(o) => {
                self.orientation = o;
                self.sync_rotation();
            }
            Change::UpsideDown(b) => {
                self.upside_down = b;
                self.sync_rotation();
            }
            Change::Rotate(r) => {
                if self.orientation == Orientation::Auto {
                    self.rotation = r;
                }
            }
            Change::LandKeys(k) => self.land_keys = k,
            Change::Pad(p) => self.pad = p,
            Change::PadSide(s) => self.pad_side = s,
            Change::KbLayout(d) => {
                use crate::layouts::ALL;
                let i = ALL.iter().position(|&l| l == self.kb_layout).unwrap_or(0) as i32;
                let n = ALL.len() as i32;
                self.kb_layout = ALL[(i + i32::from(d)).rem_euclid(n) as usize];
            }
            Change::KbAuto(on) => {
                self.kb_size = if on { 0 } else { self.keyboard_scale() };
            }
            Change::KbSize(v) => {
                if KB_SIZES.contains(&v) {
                    self.kb_size = v;
                }
            }
            Change::UiScale(d) => {
                let i = UI_SCALES
                    .iter()
                    .position(|&v| v == self.ui_scale)
                    .unwrap_or(4) as i32;
                let j = (i + i32::from(d)).clamp(0, UI_SCALES.len() as i32 - 1);
                self.ui_scale = UI_SCALES[j as usize];
            }
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

    /// A manual orientation sets the rotation; Auto keeps the last one.
    fn sync_rotation(&mut self) {
        if let Some(r) = Rotation::manual(self.orientation, self.upside_down) {
            self.rotation = r;
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
                "layout" => {
                    s.kb_layout = crate::layouts::KeyLayout::parse(v).unwrap_or(s.kb_layout);
                }
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
                "ui_scale" => {
                    if let Ok(n) = v.parse::<u16>() {
                        if UI_SCALES.contains(&n) {
                            s.ui_scale = n;
                        }
                    }
                }
                "keyboard_size" => {
                    if v == "auto" {
                        s.kb_size = 0;
                    } else if let Ok(n) = v.parse::<u16>() {
                        if KB_SIZES.contains(&n) {
                            s.kb_size = n;
                        }
                    }
                }
                "scroll" => {
                    s.natural_scroll = match v {
                        "natural" => true,
                        "traditional" => false,
                        _ => s.natural_scroll,
                    }
                }
                "orientation" => {
                    s.orientation = match v {
                        "auto" => Orientation::Auto,
                        "portrait" => Orientation::Portrait,
                        "landscape" => Orientation::Landscape,
                        _ => s.orientation,
                    }
                }
                "upside_down" => s.upside_down = b().unwrap_or(s.upside_down),
                "rotation" => s.rotation = Rotation::from_degrees(v).unwrap_or(s.rotation),
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
                "ink_pen" => {
                    s.ink_pen = match v {
                        "finger" => crate::ink::Pen::Finger,
                        "stylus" => crate::ink::Pen::Stylus,
                        _ => s.ink_pen,
                    }
                }
                "ink_a2" => s.ink_a2 = b().unwrap_or(s.ink_a2),
                "ink_words" => s.ink_words = b().unwrap_or(s.ink_words),
                "ink_boxes" => s.ink_boxes = b().unwrap_or(s.ink_boxes),
                "ui_lang_override" => s.lang_override = b().unwrap_or(s.lang_override),
                "ui_lang" => s.ui_lang = UiLang::from_code(v).unwrap_or(s.ui_lang),
                _ => {}
            }
        }
        // Files before v1.2 have no rotation= line and always wrote
        // orientation=portrait as the default: those move to Auto (the v1.2
        // default); a saved landscape stays manual (decision S12-D2).
        if !text.lines().any(|l| l.starts_with("rotation="))
            && s.orientation == Orientation::Portrait
        {
            s.orientation = Orientation::Auto;
        }
        s.sync_rotation();
        s
    }

    /// The keyboard's scale in percent (v1.4.3): Auto follows the UI scale
    /// up to 150 %; a fixed size ignores the UI scale.
    #[must_use]
    pub fn keyboard_scale(&self) -> u16 {
        if self.kb_size == 0 {
            self.ui_scale.min(KB_AUTO_MAX)
        } else {
            self.kb_size
        }
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
             upside_down={}\nrotation={}\nlandscape_keys={}\npad={}\npad_side={}\nui_scale={}\nlayout={}\nkeyboard_size={}\nink_pen={}\nink_a2={}\nink_words={}\nink_boxes={}\nui_lang_override={}\nui_lang={}\n",
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
                Orientation::Auto => "auto",
                Orientation::Portrait => "portrait",
                Orientation::Landscape => "landscape",
            },
            on(self.upside_down),
            self.rotation.degrees(),
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
            self.ui_scale,
            self.kb_layout.name(),
            if self.kb_size == 0 {
                "auto".to_owned()
            } else {
                self.kb_size.to_string()
            },
            match self.ink_pen {
                crate::ink::Pen::Finger => "finger",
                crate::ink::Pen::Stylus => "stylus",
            },
            on(self.ink_a2),
            on(self.ink_words),
            on(self.ink_boxes),
            on(self.lang_override),
            self.ui_lang.code(),
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
