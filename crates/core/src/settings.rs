//! User settings (brief §6 Options, S4) and their `.sigma-settings` file.
//!
//! The file is plain `key=value` lines. Unknown keys and bad values are
//! ignored (the default is used), so an older or newer file never stops the
//! app. v0.2.1 files (`keyboard=full` only) load unchanged.

use crate::buffer::TabKind;
use crate::keyboard::Layout;

/// The page theme: Day, or Night when Dark mode is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Paper {
    /// Dark mode off (the default).
    #[default]
    Day,
    /// Dark mode on.
    Night,
}

/// The typeface of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Typeface {
    /// Literata (the default).
    #[default]
    Literata,
    /// Atkinson.
    Atkinson,
}

/// Keyboard tone: Theme follows the page, Light is always paper, Dark is
/// always night.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyTone {
    /// Follow the page: night colours on [`Paper::Night`] (the default).
    #[default]
    Theme,
    /// Always paper colours.
    Light,
    /// Always night colours.
    Dark,
}

/// The Drawer width slider has five stops, evenly spaced from 20 % of the
/// page to 50 % (the middle of the screen at most): 20, 27.5, 35, 42.5, 50.
/// The widest is the default.
pub const DRAWER_WIDTH_STOPS: u8 = 5;
/// The default Drawer width stop: the widest (50 %).
pub const DRAWER_WIDTH_DEFAULT: u8 = DRAWER_WIDTH_STOPS - 1;

/// The drawer's width at `stop`, in tenths of a percent of the page.
/// A `stop` past the last one counts as the last.
#[must_use]
pub fn drawer_tenths(stop: u8) -> u16 {
    200 + 75 * u16::from(stop.min(DRAWER_WIDTH_DEFAULT))
}

/// The drawer's share of the page at `stop`.
#[must_use]
pub fn drawer_share(stop: u8) -> f32 {
    f32::from(drawer_tenths(stop)) / 1000.0
}

/// A side of the screen: where the drawer opens, or where the landscape
/// pad sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Side {
    /// The left side.
    Left,
    /// The right side (the default).
    #[default]
    Right,
}

/// The Options → Orientation choice: Auto, or a fixed orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Follow the accelerometer (v1.2 default); without a sensor the last
    /// applied rotation stays.
    #[default]
    Auto,
    /// Always portrait ([`Rotation::R0`], or [`Rotation::R180`] upside down).
    Portrait,
    /// Always landscape ([`Rotation::R90`], or [`Rotation::R270`] upside
    /// down).
    Landscape,
}

/// The rotation the screen is drawn in (v1.2), clockwise from the panel's
/// portrait: 0 portrait, 90 landscape, 180 portrait inverted, 270
/// landscape inverted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Rotation {
    /// Portrait (the default).
    #[default]
    R0,
    /// Landscape.
    R90,
    /// Portrait inverted.
    R180,
    /// Landscape inverted.
    R270,
}

impl Rotation {
    /// Whether the screen is landscape ([`Rotation::R90`] or [`Rotation::R270`]).
    #[must_use]
    pub fn is_landscape(self) -> bool {
        matches!(self, Self::R90 | Self::R270)
    }

    /// Whether the screen is upside down ([`Rotation::R180`] or
    /// [`Rotation::R270`]).
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

    /// The rotation's label in Settings, as [`Words::label`] gives it.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.label()
    }
}

/// Landscape keyboard arrangement (brief §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs)] // Named after the arrangements they are; Centre is the default.
pub enum LandKeys {
    #[default]
    Centre,
    Split,
    Full,
}

/// The pad beside the board in landscape Full.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Pad {
    /// Number keys (the default).
    #[default]
    Numbers,
    /// Arrow keys.
    Arrows,
}

/// The smallest and largest text size, in brief px.
pub const SIZE_RANGE: (f32, f32) = (14.0, 40.0);
/// The smallest and largest leading (line spacing factor).
pub const LEADING_RANGE: (f32, f32) = (1.2, 2.0);
/// The smallest and largest margin, in brief px.
pub const MARGIN_RANGE: (f32, f32) = (8.0, 400.0);
/// The fewest and most words per line in Line mode.
pub const WORDS_RANGE: (u8, u8) = (8, 16);
/// One step of the Size stepper, in brief px.
pub const SIZE_STEP: f32 = 1.0;
/// Stops of the Size slider (no unit is shown); the default 20 is one.
pub const SIZE_STOPS: [f32; 9] = [14.0, 17.0, 20.0, 23.0, 26.0, 29.0, 32.0, 35.0, 38.0];
/// Stops of the Leading slider, 1.2 to 2.0; the default 1.5 is one.
pub const LEADING_STOPS: [f32; 9] = [1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0];
/// Stops of the Margin slider (brief px), 8 to 400; the default 28 is one.
pub const MARGIN_STOPS: [f32; 10] = [
    8.0, 16.0, 28.0, 40.0, 56.0, 80.0, 120.0, 180.0, 260.0, 400.0,
];
/// One step of the Leading stepper.
pub const LEADING_STEP: f32 = 0.1;
/// One step of the Margin stepper, in brief px.
pub const MARGIN_STEP: f32 = 4.0;

/// Every setting. Sizes are in brief px (a 1264 px wide page); the renderer
/// scales them to the panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Dark mode: the page theme. `paper=`.
    pub paper: Paper,
    /// The typeface. `type=`.
    pub face: Typeface,
    /// Line mode on / off (size and margin refit to
    /// [`Settings::line_words`]). `line=`.
    pub line_on: bool,
    /// Words per line in Line mode, within [`WORDS_RANGE`]. `line_words=`.
    pub line_words: u8,
    /// Text size in brief px, within [`SIZE_RANGE`]. `size=`.
    pub size: f32,
    /// Leading (line spacing factor), within [`LEADING_RANGE`]. `leading=`.
    pub leading: f32,
    /// Margin in brief px, within [`MARGIN_RANGE`]. `margin=`.
    pub margin: f32,
    /// Tab key inserts spaces (Markdown default) or a tab character.
    pub tab_spaces: bool,
    /// Spaces a Tab inserts when [`Settings::tab_spaces`] is on: 2, 4 or 8.
    /// `tab_width=`.
    pub tab_width: u8,
    /// The on-screen keyboard board. `keyboard=`.
    pub keyboard: Layout,
    /// The keyboard's colours. `key_tone=`.
    pub key_tone: KeyTone,
    /// The side the drawer opens on. `drawer=`.
    pub drawer: Side,
    /// The drawer's width as a slider stop, `0..DRAWER_WIDTH_STOPS`.
    pub drawer_width: u8,
    /// USB keyboard & mouse on / off. `usb=`.
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
    /// The landscape keyboard arrangement. `landscape_keys=`.
    pub land_keys: LandKeys,
    /// The pad beside the board in landscape Full. `pad=`.
    pub pad: Pad,
    /// The side the landscape pad sits on. `pad_side=`.
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
    /// Keyboard & input → Handwriting → Learn from corrections: a letter
    /// corrected by picking a candidate is kept as a sample of the user's
    /// handwriting (`.handwriting/learned.txt`). On by default.
    /// `ink_learn=`.
    pub ink_learn: bool,
    /// Language → Override system language (v1.5.5, decision 57). Off: the
    /// interface follows the Kobo's `CurrentLocale` (read-only, never
    /// written). `ui_lang_override=`.
    pub lang_override: bool,
    /// The interface language chosen while the override is on. `ui_lang=`.
    pub ui_lang: UiLang,
    /// Settings → Experimental features (v1.12): features that are not fully
    /// tested yet (Asian and right-to-left layouts, Word mode, Fast ink).
    /// Off by default. `experimental=`.
    pub experimental: bool,
    /// Keyboard & input → Enhanced keyboard (v1.13). On (default): the
    /// Compact / Full boards. Off: the Kobo style board, with no options on
    /// the keyboard itself (layouts that do not fit it keep the Full board).
    /// `enhanced_keyboard=`.
    pub enhanced_keyboard: bool,
    /// Settings → Logging: write the log file (the app and its plugins).
    /// Global, off by default. `log=`.
    pub log: bool,
    /// The lowest level the log file records, from DEBUG to CRITICAL (the
    /// levels of Python's logging). `log_level=`.
    pub log_level: crate::log::Level,
    /// The warm-to-cool step the reader chose, 0 (cool) to
    /// [`WARMTH_MAX`] (warm), or `None` while they have not chosen one: the
    /// app then leaves the mixer as it finds it. Applied at start
    /// (v1.17). `warmth=` is `keep` or the step.
    pub warmth: Option<u8>,
}

/// Backlight steps above off: nine stops, 0 to 8 (Display → Backlight).
pub const BACKLIGHT_MAX: u8 = 8;

/// Warmth steps above the coolest light: eleven stops, 0 to 10 (Display →
/// Warmth). The same scale as the Forma's mixer file, turned the right way
/// round (`sigma_device::light::WARMTH_MAX`).
pub const WARMTH_MAX: u8 = 10;

/// An interface language the user can pick (v1.5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs)] // Named after their language codes; English is the default.
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
}

impl UiLang {
    /// Every language, in the order Settings lists them.
    pub const ALL: [UiLang; 10] = [
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
    ];

    /// The language's two-letter code, as `ui_lang=` saves it.
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
        }
    }

    /// The language with this code, if any.
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
            drawer_width: DRAWER_WIDTH_DEFAULT,
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
            ink_learn: true,
            lang_override: false,
            ui_lang: UiLang::En,
            experimental: false,
            enhanced_keyboard: true,
            log: false,
            log_level: crate::log::Level::Info,
            warmth: None,
        }
    }
}

/// One change from the Options sheet or the landscape keyboard bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    /// Dark mode: the page theme.
    Paper(Paper),
    /// The typeface.
    Face(Typeface),
    /// Line mode on / off.
    LineOn(bool),
    /// +1 / -1 word.
    LineWords(i8),
    /// +1 / -1 step.
    Size(i8),
    /// Size to a value (the slider's stops, [`SIZE_STOPS`]).
    SizeTo(f32),
    /// +1 / -1 step.
    Leading(i8),
    /// Leading to a value (the slider's stops, [`LEADING_STOPS`]).
    LeadingTo(f32),
    /// +1 / -1 step.
    Margin(i8),
    /// Margin to a value (the slider's stops, [`MARGIN_STOPS`]).
    MarginTo(f32),
    /// Tab inserts spaces (on) or a tab character (off).
    TabSpaces(bool),
    /// Tab width: 2, 4 or 8 (other values are ignored).
    TabWidth(u8),
    /// The on-screen keyboard board.
    Keyboard(Layout),
    /// The keyboard's colours.
    KeyTone(KeyTone),
    /// The side the drawer opens on.
    Drawer(Side),
    /// The drawer's width, as a slider stop.
    DrawerWidth(u8),
    /// USB keyboard & mouse on / off.
    Usb(bool),
    /// Reverse scroll on / off.
    NaturalScroll(bool),
    /// The orientation choice.
    Orientation(Orientation),
    /// Upside down on / off.
    UpsideDown(bool),
    /// Auto applied a sensor reading (v1.2).
    Rotate(Rotation),
    /// The landscape keyboard arrangement.
    LandKeys(LandKeys),
    /// The pad beside the board in landscape Full.
    Pad(Pad),
    /// The side the landscape pad sits on.
    PadSide(Side),
    /// One UI-scale step up (+1) or down (-1).
    UiScale(i8),
    /// UI scale to a value (the slider's stops, `UI_SCALES`).
    UiScaleTo(u16),
    /// One step through the keyboard layouts (v1.3).
    KbLayout(i8),
    /// Experimental features on / off (v1.12). Turning them off puts an
    /// experimental layout back to US.
    Experimental(bool),
    /// Enhanced keyboard on / off (v1.13): off is the Kobo style board.
    EnhancedKeyboard(bool),
    /// The backlight step, 0 to [`BACKLIGHT_MAX`] (v1.12). Not a saved
    /// setting: the app sends it to the front light, which keeps it.
    Backlight(u8),
    /// The warmth step, 0 (cool) to [`WARMTH_MAX`] (warm) (v1.17). Saved,
    /// because the mixer cannot be read back reliably; the app sends it to
    /// the front light at start and when it changes.
    Warmth(u8),
    /// Turn plugin number `.0` (its place in the Settings → Plugins list) on
    /// or off. Kept in `plugins.state`, not in the settings file, so
    /// `Settings` ignores it and the app applies it.
    Plugin(usize, bool),
    /// Settings → Logging: the log file on or off (applied at once).
    Log(bool),
    /// Settings → Logging → Level.
    LogLevel(crate::log::Level),
    /// Set a setting of a plugin: plugin number, the setting's place in its
    /// declared list, and the chosen option's place. Kept in the plugin's own
    /// `settings` file, so `Settings` ignores it and the app applies it.
    PluginSetting(u8, u8, u8),
    /// Settings → Plugins → Repository: fetch the repository index. The app
    /// applies it.
    PluginCheck,
    /// Install or update the repository's plugin number `i` (its place in
    /// the list the page showed). The app applies it.
    PluginGet(u8),
    /// Remove the installed plugin number `i`, which must be off. The app
    /// applies it.
    PluginRemove(u8),
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
    /// Learn from corrections on / off.
    InkLearn(bool),
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
            Self::Size(_) | Self::SizeTo(_) => Some(Some(true)),
            Self::Margin(_) | Self::MarginTo(_) => Some(Some(false)),
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
    /// Applies a change, clamping every value to its range. Changes that
    /// are not saved settings (see [`Change`]) leave it as it is.
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
            Change::SizeTo(v) => self.size = clampf(tenth(v), SIZE_RANGE),
            Change::Leading(d) => {
                self.leading = clampf(
                    tenth(self.leading + f32::from(d) * LEADING_STEP),
                    LEADING_RANGE,
                );
            }
            Change::LeadingTo(v) => self.leading = clampf(tenth(v), LEADING_RANGE),
            Change::MarginTo(v) => self.margin = clampf(tenth(v), MARGIN_RANGE),
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
            Change::DrawerWidth(w) => self.drawer_width = w.min(DRAWER_WIDTH_DEFAULT),
            Change::Usb(on) => self.usb = on,
            Change::NaturalScroll(on) => self.natural_scroll = on,
            Change::InkPen(p) => self.ink_pen = p,
            Change::InkA2(on) => self.ink_a2 = on,
            Change::InkWords(on) => self.ink_words = on,
            Change::InkBoxes(on) => self.ink_boxes = on,
            Change::InkLearn(on) => self.ink_learn = on,
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
                let all = self.layouts();
                let i = all.iter().position(|&l| l == self.kb_layout).unwrap_or(0) as i32;
                let n = all.len() as i32;
                self.kb_layout = all[(i + i32::from(d)).rem_euclid(n) as usize];
            }
            Change::Experimental(on) => {
                self.experimental = on;
                if !on && self.kb_layout.experimental() {
                    self.kb_layout = crate::layouts::KeyLayout::Us;
                }
            }
            Change::EnhancedKeyboard(on) => self.enhanced_keyboard = on,
            Change::Log(on) => self.log = on,
            Change::LogLevel(l) => self.log_level = l,
            Change::Warmth(n) => self.warmth = Some(n.min(WARMTH_MAX)),
            Change::Backlight(_)
            | Change::Plugin(..)
            | Change::PluginSetting(..)
            | Change::PluginCheck
            | Change::PluginGet(_)
            | Change::PluginRemove(_) => {}
            Change::KbAuto(on) => {
                self.kb_size = if on { 0 } else { self.keyboard_scale() };
            }
            Change::KbSize(v) => {
                if KB_SIZES.contains(&v) {
                    self.kb_size = v;
                }
            }
            Change::UiScaleTo(v) => {
                let near = UI_SCALES
                    .iter()
                    .copied()
                    .min_by_key(|&s| s.abs_diff(v))
                    .unwrap_or(200);
                self.ui_scale = near;
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
    /// Each line goes to its entry in [`FIELDS`]; unknown keys are skipped.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            if let Some(f) = FIELDS.iter().find(|f| f.key == k.trim()) {
                (f.read)(&mut s, v.trim());
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
        // Files before v1.12 have no experimental= line: whoever already
        // uses an experimental feature keeps it (the switch starts on).
        if !text.lines().any(|l| l.starts_with("experimental=")) {
            s.experimental = s.ink_a2 || s.ink_words || s.kb_layout.experimental();
        }
        if !s.experimental && s.kb_layout.experimental() {
            s.kb_layout = crate::layouts::KeyLayout::Us;
        }
        s.sync_rotation();
        s
    }

    /// The Kobo style board is in use: Enhanced keyboard is off and the
    /// layout fits it (the Indic, Thai, Japanese and Chinese layouts keep the
    /// Full board).
    #[must_use]
    pub fn kobo_keyboard(&self) -> bool {
        !self.enhanced_keyboard && self.kb_layout.has_compact()
    }

    /// The layouts the list offers: the experimental ones only while
    /// Settings → Experimental features is on.
    #[must_use]
    pub fn layouts(&self) -> Vec<crate::layouts::KeyLayout> {
        crate::layouts::ALL
            .iter()
            .copied()
            .filter(|l| self.experimental || !l.experimental())
            .collect()
    }

    /// Word mode as it is in force: an experimental feature.
    #[must_use]
    pub fn word_mode(&self) -> bool {
        self.experimental && self.ink_words
    }

    /// Fast ink (A2) as it is in force: an experimental feature.
    #[must_use]
    pub fn fast_ink(&self) -> bool {
        self.experimental && self.ink_a2
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

    /// The file body, one line per entry of [`FIELDS`] in its order;
    /// [`Settings::parse`] reads it back unchanged.
    #[must_use]
    pub fn serialize(&self) -> String {
        FIELDS
            .iter()
            .map(|f| format!("{}={}\n", f.key, (f.write)(self)))
            .collect()
    }
}

/// A setting with a fixed set of values: for each value, its word in the
/// settings file and its label in Settings (English; translated when drawn).
/// The one place a value's spelling lives.
pub trait Words: Copy + PartialEq + Sized + 'static {
    /// Each value with its word in the settings file and its label in
    /// Settings.
    const WORDS: &'static [(Self, &'static str, &'static str)];

    /// The value as the settings file writes it.
    #[must_use]
    fn word(self) -> &'static str {
        Self::WORDS.iter().find(|w| w.0 == self).map_or("", |w| w.1)
    }

    /// The value a word in the settings file names, if any.
    #[must_use]
    fn from_word(v: &str) -> Option<Self> {
        Self::WORDS.iter().find(|w| w.1 == v).map(|w| w.0)
    }

    /// The value's label in Settings.
    #[must_use]
    fn label(self) -> &'static str {
        Self::WORDS.iter().find(|w| w.0 == self).map_or("", |w| w.2)
    }
}

impl Words for Paper {
    // Dark mode is an Off / On switch.
    const WORDS: &'static [(Self, &'static str, &'static str)] =
        &[(Self::Day, "day", "Off"), (Self::Night, "night", "On")];
}

impl Words for Typeface {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Literata, "literata", "Literata"),
        (Self::Atkinson, "atkinson", "Atkinson"),
    ];
}

impl Words for KeyTone {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Theme, "theme", "Theme"),
        (Self::Light, "light", "Light"),
        (Self::Dark, "dark", "Dark"),
    ];
}

impl Words for Side {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Left, "left", "Left"),
        (Self::Right, "right", "Right"),
    ];
}

impl Words for Orientation {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Auto, "auto", "Auto"),
        (Self::Portrait, "portrait", "Portrait"),
        (Self::Landscape, "landscape", "Landscape"),
    ];
}

impl Words for Rotation {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::R0, "0", "Portrait"),
        (Self::R90, "90", "Landscape"),
        (Self::R180, "180", "Portrait inverted"),
        (Self::R270, "270", "Landscape inverted"),
    ];
}

impl Words for LandKeys {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Centre, "centre", "Centre"),
        (Self::Split, "split", "Split"),
        (Self::Full, "full", "Full"),
    ];
}

impl Words for Pad {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Numbers, "numbers", "Numbers"),
        (Self::Arrows, "arrows", "Arrows"),
    ];
}

impl Words for crate::ink::Pen {
    const WORDS: &'static [(Self, &'static str, &'static str)] = &[
        (Self::Finger, "finger", "Finger"),
        (Self::Stylus, "stylus", "Stylus"),
    ];
}

/// One line of the settings file: its key, how a value is read (a bad value
/// leaves the setting as it was), and how it is written.
pub struct Field {
    /// The key before the `=`.
    pub key: &'static str,
    /// Sets the setting from a value; a bad value changes nothing.
    pub read: fn(&mut Settings, &str),
    /// The setting's value as the file writes it.
    pub write: fn(&Settings) -> String,
}

fn read_bool(v: &str) -> Option<bool> {
    match v {
        "on" | "true" | "1" => Some(true),
        "off" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn read_f32(v: &str, range: (f32, f32)) -> Option<f32> {
    v.parse::<f32>()
        .ok()
        .filter(|x| x.is_finite())
        .map(|x| clampf(x, range))
}

fn on(b: bool) -> String {
    (if b { "on" } else { "off" }).to_owned()
}

/// An on / off setting.
macro_rules! toggle {
    ($key:literal, $f:ident) => {
        Field {
            key: $key,
            read: |s, v| {
                if let Some(b) = read_bool(v) {
                    s.$f = b;
                }
            },
            write: |s| on(s.$f),
        }
    };
}

/// A setting whose values are [`Words`].
macro_rules! words {
    ($key:literal, $f:ident) => {
        Field {
            key: $key,
            read: |s, v| {
                if let Some(x) = Words::from_word(v) {
                    s.$f = x;
                }
            },
            write: |s| s.$f.word().to_owned(),
        }
    };
}

/// A size in brief px or a factor: clamped when read, a tenth when written.
macro_rules! tenths {
    ($key:literal, $f:ident, $range:expr) => {
        Field {
            key: $key,
            read: |s, v| {
                if let Some(x) = read_f32(v, $range) {
                    s.$f = x;
                }
            },
            write: |s| tenth(s.$f).to_string(),
        }
    };
}

/// Every line of the settings file, in the order it is written. A new
/// setting is one entry here (plus its field and default in [`Settings`]).
pub const FIELDS: &[Field] = &[
    Field {
        key: "keyboard",
        read: |s, v| s.keyboard = Layout::parse(v).unwrap_or(s.keyboard),
        write: |s| s.keyboard.name().to_owned(),
    },
    words!("paper", paper),
    words!("type", face),
    toggle!("line", line_on),
    Field {
        key: "line_words",
        read: |s, v| {
            if let Ok(n) = v.parse::<u8>() {
                s.line_words = n.clamp(WORDS_RANGE.0, WORDS_RANGE.1);
            }
        },
        write: |s| s.line_words.to_string(),
    },
    tenths!("size", size, SIZE_RANGE),
    tenths!("leading", leading, LEADING_RANGE),
    tenths!("margin", margin, MARGIN_RANGE),
    Field {
        key: "tab",
        read: |s, v| match v {
            "tab" => s.tab_spaces = false,
            "spaces" => s.tab_spaces = true,
            _ => {}
        },
        write: |s| (if s.tab_spaces { "spaces" } else { "tab" }).to_owned(),
    },
    Field {
        key: "tab_width",
        read: |s, v| {
            if let Ok(n @ (2 | 4 | 8)) = v.parse::<u8>() {
                s.tab_width = n;
            }
        },
        write: |s| s.tab_width.to_string(),
    },
    words!("key_tone", key_tone),
    words!("drawer", drawer),
    Field {
        key: "drawer_width",
        // Words from the first builds of this setting still read.
        read: |s, v| {
            s.drawer_width = match v {
                "narrow" => 1,
                "medium" => 2,
                "wide" => DRAWER_WIDTH_DEFAULT,
                n => n
                    .parse::<u8>()
                    .map_or(s.drawer_width, |n| n.min(DRAWER_WIDTH_DEFAULT)),
            };
        },
        write: |s| s.drawer_width.to_string(),
    },
    toggle!("usb", usb),
    Field {
        key: "scroll",
        read: |s, v| match v {
            "natural" => s.natural_scroll = true,
            "traditional" => s.natural_scroll = false,
            _ => {}
        },
        write: |s| {
            (if s.natural_scroll {
                "natural"
            } else {
                "traditional"
            })
            .to_owned()
        },
    },
    words!("orientation", orientation),
    toggle!("upside_down", upside_down),
    words!("rotation", rotation),
    words!("landscape_keys", land_keys),
    words!("pad", pad),
    words!("pad_side", pad_side),
    Field {
        key: "ui_scale",
        read: |s, v| {
            if let Ok(n) = v.parse::<u16>() {
                if UI_SCALES.contains(&n) {
                    s.ui_scale = n;
                }
            }
        },
        write: |s| s.ui_scale.to_string(),
    },
    Field {
        key: "layout",
        read: |s, v| s.kb_layout = crate::layouts::KeyLayout::parse(v).unwrap_or(s.kb_layout),
        write: |s| s.kb_layout.name().to_owned(),
    },
    Field {
        key: "keyboard_size",
        read: |s, v| {
            if v == "auto" {
                s.kb_size = 0;
            } else if let Ok(n) = v.parse::<u16>() {
                if KB_SIZES.contains(&n) {
                    s.kb_size = n;
                }
            }
        },
        write: |s| {
            if s.kb_size == 0 {
                "auto".to_owned()
            } else {
                s.kb_size.to_string()
            }
        },
    },
    words!("ink_pen", ink_pen),
    toggle!("ink_a2", ink_a2),
    toggle!("ink_words", ink_words),
    toggle!("ink_boxes", ink_boxes),
    toggle!("ink_learn", ink_learn),
    toggle!("ui_lang_override", lang_override),
    Field {
        key: "ui_lang",
        read: |s, v| s.ui_lang = UiLang::from_code(v).unwrap_or(s.ui_lang),
        write: |s| s.ui_lang.code().to_owned(),
    },
    toggle!("experimental", experimental),
    toggle!("enhanced_keyboard", enhanced_keyboard),
    toggle!("log", log),
    Field {
        key: "log_level",
        read: |s, v| s.log_level = crate::log::Level::parse(v).unwrap_or(s.log_level),
        write: |s| s.log_level.key().to_owned(),
    },
    Field {
        key: "warmth",
        read: |s, v| {
            s.warmth = match v.parse::<u8>() {
                Ok(n) if n <= WARMTH_MAX => Some(n),
                _ => None,
            };
        },
        write: |s| s.warmth.map_or("keep".to_owned(), |n| n.to_string()),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every field away from its default (each one different from it).
    fn every_field_changed() -> Settings {
        Settings {
            paper: Paper::Night,
            face: Typeface::Atkinson,
            line_on: true,
            line_words: 15,
            size: 23.5,
            leading: 1.8,
            margin: 120.0,
            tab_spaces: false,
            tab_width: 8,
            keyboard: Layout::Compact,
            key_tone: KeyTone::Dark,
            drawer: Side::Left,
            drawer_width: 1,
            usb: true,
            natural_scroll: true,
            orientation: Orientation::Landscape,
            upside_down: true,
            rotation: Rotation::R270,
            land_keys: LandKeys::Split,
            pad: Pad::Arrows,
            pad_side: Side::Left,
            ui_scale: 150,
            kb_layout: crate::layouts::ALL[3],
            kb_size: 125,
            ink_pen: crate::ink::Pen::Stylus,
            ink_a2: true,
            ink_words: true,
            ink_boxes: true,
            ink_learn: false,
            lang_override: true,
            ui_lang: UiLang::Pt,
            experimental: true,
            enhanced_keyboard: false,
            log: true,
            log_level: crate::log::Level::Error,
            warmth: Some(7),
        }
    }

    /// The file is byte for byte what the hand-written serializer wrote
    /// before the field table (v1.14), plus the lines added since (`warmth`
    /// in v1.17), so no saved file changes.
    #[test]
    fn the_file_is_unchanged_by_the_field_table() {
        assert_eq!(
            Settings::default().serialize(),
            "keyboard=full\npaper=day\ntype=literata\nline=off\nline_words=12\nsize=20\n\
             leading=1.5\nmargin=28\ntab=spaces\ntab_width=4\nkey_tone=theme\ndrawer=right\n\
             drawer_width=4\nusb=off\nscroll=traditional\norientation=auto\nupside_down=off\n\
             rotation=0\nlandscape_keys=centre\npad=numbers\npad_side=right\nui_scale=200\n\
             layout=us\nkeyboard_size=auto\nink_pen=finger\nink_a2=off\nink_words=off\n\
             ink_boxes=off\nink_learn=on\nui_lang_override=off\nui_lang=en\nexperimental=off\n\
             enhanced_keyboard=on\nlog=off\nlog_level=info\nwarmth=keep\n"
        );
        assert_eq!(
            every_field_changed().serialize(),
            "keyboard=compact\npaper=night\ntype=atkinson\nline=on\nline_words=15\nsize=23.5\n\
             leading=1.8\nmargin=120\ntab=tab\ntab_width=8\nkey_tone=dark\ndrawer=left\n\
             drawer_width=1\nusb=on\nscroll=natural\norientation=landscape\nupside_down=on\n\
             rotation=270\nlandscape_keys=split\npad=arrows\npad_side=left\nui_scale=150\n\
             layout=de\nkeyboard_size=125\nink_pen=stylus\nink_a2=on\nink_words=on\n\
             ink_boxes=on\nink_learn=off\nui_lang_override=on\nui_lang=pt\nexperimental=on\n\
             enhanced_keyboard=off\nlog=on\nlog_level=error\nwarmth=7\n"
        );
    }

    /// A field missing from the table would come back as its default.
    #[test]
    fn every_field_has_a_line_and_round_trips() {
        let d = Settings::default();
        let c = every_field_changed();
        assert_eq!(Settings::parse(&c.serialize()), c);
        let mut keys: Vec<&str> = FIELDS.iter().map(|f| f.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), FIELDS.len(), "a key is listed twice");
        for f in FIELDS {
            assert_ne!((f.write)(&d), (f.write)(&c), "{} is not exercised", f.key);
            let mut s = d;
            (f.read)(&mut s, &(f.write)(&c));
            assert_eq!((f.write)(&s), (f.write)(&c), "{}", f.key);
        }
    }

    #[test]
    fn warmth_is_kept_until_chosen_and_stays_in_range() {
        assert_eq!(Settings::default().warmth, None, "the mixer is left alone");
        let mut s = Settings::default();
        s.apply(Change::Warmth(4));
        assert_eq!(s.warmth, Some(4));
        s.apply(Change::Warmth(200));
        assert_eq!(s.warmth, Some(WARMTH_MAX));
        assert_eq!(Settings::parse(&s.serialize()).warmth, Some(WARMTH_MAX));
        for bad in [
            "warmth=11\n",
            "warmth=warm\n",
            "warmth=-1\n",
            "warmth=\n",
            "",
        ] {
            assert_eq!(Settings::parse(bad).warmth, None, "{bad:?}");
        }
        assert_eq!(Settings::parse("warmth=0\n").warmth, Some(0));
    }

    fn words_round_trip<T: Words + std::fmt::Debug>() {
        for &(v, w, l) in T::WORDS {
            assert_eq!(T::from_word(w), Some(v));
            assert_eq!((v.word(), v.label()), (w, l));
        }
        assert_eq!(T::from_word("sideways"), None);
    }

    #[test]
    fn every_value_has_one_word() {
        words_round_trip::<Paper>();
        words_round_trip::<Typeface>();
        words_round_trip::<KeyTone>();
        words_round_trip::<Side>();
        words_round_trip::<Orientation>();
        words_round_trip::<Rotation>();
        words_round_trip::<LandKeys>();
        words_round_trip::<Pad>();
        words_round_trip::<crate::ink::Pen>();
        assert_eq!(Rotation::R180.name(), "Portrait inverted");
    }

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
            Change::DrawerWidth(1),
            Change::Usb(true),
            Change::Orientation(Orientation::Landscape),
            Change::LandKeys(LandKeys::Split),
            Change::Pad(Pad::Arrows),
            Change::PadSide(Side::Left),
            Change::Log(true),
            Change::LogLevel(crate::log::Level::Error),
        ] {
            s.apply(c);
        }
        assert_eq!(Settings::parse(&s.serialize()), s);
        assert!(s.log && s.log_level == crate::log::Level::Error);
        assert_eq!(s.tab_kind(), TabKind::Tab);
        assert_eq!(
            (s.size, s.leading, s.margin, s.line_words),
            (22.0, 1.4, 48.0, 15)
        );
    }

    #[test]
    fn logging_is_off_at_info_by_default_and_every_level_round_trips() {
        use crate::log::Level;
        let d = Settings::default();
        assert!(!d.log && d.log_level == Level::Info);
        for l in Level::ALL {
            let mut s = Settings::default();
            s.apply(Change::Log(true));
            s.apply(Change::LogLevel(l));
            let text = s.serialize();
            assert!(
                text.contains(&format!("log=on\nlog_level={}\n", l.key())),
                "{text}"
            );
            assert_eq!(Settings::parse(&text), s, "{l:?}");
        }
        // A file from before logging has neither key: off, INFO.
        let old = Settings::parse("keyboard=compact\nusb=on\n");
        assert!(!old.log && old.log_level == Level::Info);
        // Garbage keeps the default (or what was there), in any case.
        let g = Settings::parse("log=maybe\nlog_level=LOUD\n");
        assert!(!g.log && g.log_level == Level::Info);
        let g = Settings::parse("log=on\nlog_level=WARNING\n");
        assert!(
            g.log && g.log_level == Level::Warning,
            "names are case-insensitive"
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
    fn size_slider_stops_are_valid_and_include_the_default() {
        assert!(SIZE_STOPS.contains(&Settings::default().size));
        for v in SIZE_STOPS {
            let mut s = Settings::default();
            s.apply(Change::SizeTo(v));
            assert_eq!(s.size, v);
            assert!((SIZE_RANGE.0..=SIZE_RANGE.1).contains(&v));
        }
        let mut s = Settings::default();
        s.apply(Change::SizeTo(500.0));
        assert_eq!(s.size, SIZE_RANGE.1);
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

    #[test]
    fn drawer_width_has_five_even_stops_reads_old_words_and_clamps() {
        assert_eq!(Settings::default().drawer_width, DRAWER_WIDTH_DEFAULT);
        let tenths: Vec<u16> = (0..DRAWER_WIDTH_STOPS).map(drawer_tenths).collect();
        assert_eq!(tenths, [200, 275, 350, 425, 500]);
        assert_eq!(drawer_share(DRAWER_WIDTH_DEFAULT), 0.5);
        let mut s = Settings::default();
        s.apply(Change::DrawerWidth(1));
        assert_eq!(Settings::parse(&s.serialize()).drawer_width, 1);
        s.apply(Change::DrawerWidth(200));
        assert_eq!(s.drawer_width, DRAWER_WIDTH_DEFAULT);
        for (v, want) in [
            ("narrow", 1),
            ("medium", 2),
            ("wide", 4),
            ("3", 3),
            ("99", 4),
        ] {
            let g = Settings::parse(&format!("drawer_width={v}\n"));
            assert_eq!(g.drawer_width, want, "{v}");
        }
        let g = Settings::parse("drawer_width=sideways\n");
        assert_eq!(g.drawer_width, DRAWER_WIDTH_DEFAULT);
    }
}
