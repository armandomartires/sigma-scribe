//! Settings (v1.4.2, was the Options sheet): a Kobo-style list of
//! categories, each a page of rows with the label left and the control
//! right: checkboxes for on/off, inline buttons for two-way choices,
//! steppers for numbers and a choice page for longer lists. A tap returns
//! what it stands for; the app applies it and saves `.sigma-settings`
//! (same keys as before, so 1.4.0 settings carry over unchanged).

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::theme::Theme;
use sigma_core::keyboard::Layout as KbLayout;
use sigma_core::settings::{Change, Orientation, Settings, Words};
use sigma_core::Keyboard;
use std::borrow::Cow;

/// One control in a row: a button (with the change it makes) or a value.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// Text on the button, or the value.
    pub label: String,
    /// The change a tap makes; `None` for a value.
    pub change: Option<Change>,
    /// Whether this is the current choice.
    pub active: bool,
}

/// A setting as a label, a short hint and its controls.
#[derive(Clone, Debug, PartialEq)]
pub struct OptRow {
    /// The setting's name (English; translated when drawn).
    pub label: &'static str,
    /// One line on what the setting does; may be empty.
    pub hint: &'static str,
    /// The controls, left to right.
    pub cells: Vec<Cell>,
}

fn choice(label: &str, change: Change, active: bool) -> Cell {
    Cell {
        label: label.to_owned(),
        change: Some(change),
        active,
    }
}

/// One cell per value of a [`Words`] setting, in the order of its words.
fn word_cells<T: Words>(cur: T, change: fn(T) -> Change) -> Vec<Cell> {
    T::WORDS
        .iter()
        .map(|&(v, _, label)| choice(label, change(v), v == cur))
        .collect()
}

/// An Off / On switch.
fn toggle(on: bool, change: fn(bool) -> Change) -> Vec<Cell> {
    vec![
        choice("Off", change(false), !on),
        choice("On", change(true), on),
    ]
}

fn value(label: String) -> Cell {
    Cell {
        label,
        change: None,
        active: false,
    }
}

fn stepper(v: String, minus: Change, plus: Change) -> Vec<Cell> {
    vec![
        choice("\u{2212}", minus, false),
        value(v),
        choice("+", plus, false),
    ]
}

/// A slider value as text: whole numbers plain, else one decimal.
fn num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.1}")
    }
}

/// The Size slider stop nearest to `v`: its index and value.
fn size_stop(v: f32) -> (usize, f32) {
    nearest_stop(&sigma_core::settings::SIZE_STOPS, v)
}

/// The stop of `stops` nearest to `v`: its index and value.
fn nearest_stop(stops: &[f32], v: f32) -> (usize, f32) {
    stops
        .iter()
        .copied()
        .enumerate()
        .min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs()))
        .unwrap_or((0, v))
}

/// The rows, in the brief's order, plus Keyboard layout and Orientation.
#[must_use]
pub fn option_rows(s: &Settings) -> Vec<OptRow> {
    let mut line = toggle(s.line_on, Change::LineOn);
    line.extend(stepper(
        format!("{} words", s.line_words),
        Change::LineWords(-1),
        Change::LineWords(1),
    ));
    vec![
        OptRow {
            label: "UI scale",
            hint: "Buttons, menus and keys; the note keeps its Size",
            cells: sigma_core::settings::UI_SCALES
                .iter()
                .map(|&v| choice(&format!("{v} %"), Change::UiScaleTo(v), v == s.ui_scale))
                .collect(),
        },
        OptRow {
            label: "Dark mode",
            hint: "Night is the same page, inverted",
            cells: word_cells(s.paper, Change::Paper),
        },
        OptRow {
            label: "Type",
            hint: "Serif for reading, sans for the bus",
            cells: word_cells(s.face, Change::Face),
        },
        OptRow {
            label: "Line",
            hint: "Words per line, 8 to 16",
            cells: line,
        },
        OptRow {
            label: "Size",
            hint: if s.line_on {
                "Tied to Margin while Line is on"
            } else {
                "Type size"
            },
            cells: sigma_core::settings::SIZE_STOPS
                .iter()
                .map(|&v| choice("", Change::SizeTo(v), v == size_stop(s.size).1))
                .collect(),
        },
        OptRow {
            label: "Leading",
            hint: "Space between lines",
            cells: sigma_core::settings::LEADING_STOPS
                .iter()
                .map(|&v| {
                    choice(
                        "",
                        Change::LeadingTo(v),
                        v == nearest_stop(&sigma_core::settings::LEADING_STOPS, s.leading).1,
                    )
                })
                .collect(),
        },
        OptRow {
            label: "Margin",
            hint: "Room for a thumb",
            cells: sigma_core::settings::MARGIN_STOPS
                .iter()
                .map(|&v| {
                    choice(
                        "",
                        Change::MarginTo(v),
                        v == nearest_stop(&sigma_core::settings::MARGIN_STOPS, s.margin).1,
                    )
                })
                .collect(),
        },
        OptRow {
            label: "Tab key",
            hint: "Markdown default is spaces",
            cells: vec![
                choice("Tab", Change::TabSpaces(false), !s.tab_spaces),
                choice("Spaces", Change::TabSpaces(true), s.tab_spaces),
            ],
        },
        OptRow {
            label: "Tab width",
            hint: "Spaces inserted, or columns a tab stands for",
            cells: [2u8, 4, 8]
                .iter()
                .map(|&w| choice(&w.to_string(), Change::TabWidth(w), s.tab_width == w))
                .collect(),
        },
        OptRow {
            label: "Keyboard",
            hint: "On-screen layout",
            cells: vec![
                choice(
                    "Compact",
                    Change::Keyboard(KbLayout::Compact),
                    s.keyboard == KbLayout::Compact,
                ),
                choice(
                    "Full",
                    Change::Keyboard(KbLayout::Full),
                    s.keyboard == KbLayout::Full,
                ),
            ],
        },
        OptRow {
            label: "Layout",
            hint: "On-screen Full board and USB keyboards",
            cells: stepper(
                s.kb_layout.label().to_owned(),
                Change::KbLayout(-1),
                Change::KbLayout(1),
            ),
        },
        OptRow {
            label: "Auto keyboard size",
            hint: "Follow UI scale, up to 150%",
            cells: toggle(s.kb_size == 0, Change::KbAuto),
        },
        OptRow {
            label: "Keyboard size",
            hint: "Auto follows UI scale, up to 150%",
            cells: sigma_core::settings::KB_SIZES
                .iter()
                .map(|&v| {
                    let l = if v == 0 {
                        "Auto".to_owned()
                    } else {
                        format!("{v}%")
                    };
                    choice(&l, Change::KbSize(v), s.kb_size == v)
                })
                .collect(),
        },
        OptRow {
            label: "Ink pen",
            hint: "Stroke width in the Write panel",
            cells: word_cells(s.ink_pen, Change::InkPen),
        },
        OptRow {
            label: "Fast ink (A2)",
            hint: "Experimental 1-bit waveform for live ink",
            cells: toggle(s.ink_a2, Change::InkA2),
        },
        OptRow {
            label: "Experimental features",
            hint: "Asian and right-to-left layouts, Word mode, Fast ink",
            cells: toggle(s.experimental, Change::Experimental),
        },
        OptRow {
            label: "Logging",
            hint: "The app and its plugins write a log file",
            cells: toggle(s.log, Change::Log),
        },
        OptRow {
            label: "Level",
            hint: "The lowest level the log records",
            cells: sigma_core::log::Level::ALL
                .into_iter()
                .map(|l| choice(level_label(l), Change::LogLevel(l), s.log_level == l))
                .collect(),
        },
        OptRow {
            label: "Enhanced keyboard",
            hint: "Off: the Kobo reader's keyboard, with no options on it",
            cells: toggle(s.enhanced_keyboard, Change::EnhancedKeyboard),
        },
        OptRow {
            label: "Letter boxes",
            hint: "One character per box instead of free writing",
            cells: toggle(s.ink_boxes, Change::InkBoxes),
        },
        OptRow {
            label: "Word mode (experimental)",
            hint: "Read every word against a word list; off is more accurate",
            cells: toggle(s.ink_words, Change::InkWords),
        },
        OptRow {
            label: "Learn from corrections",
            hint: "A letter you fix with a candidate teaches it your writing",
            cells: toggle(s.ink_learn, Change::InkLearn),
        },
        OptRow {
            label: "Override system language",
            hint: "Choose the app's language instead of the Kobo's",
            cells: toggle(s.lang_override, Change::LangOverride),
        },
        OptRow {
            label: "Interface language",
            hint: "Used while Override system language is on",
            cells: sigma_core::settings::UiLang::ALL
                .iter()
                .map(|&l| choice(l.name(), Change::UiLang(l), s.ui_lang == l))
                .collect(),
        },
        OptRow {
            label: "Keyboard tone",
            hint: "The keyboard can differ from the page",
            cells: word_cells(s.key_tone, Change::KeyTone),
        },
        OptRow {
            label: "Drawer",
            hint: "Contents, to-dos and scratch open here (v0.5)",
            cells: word_cells(s.drawer, Change::Drawer),
        },
        OptRow {
            label: "USB keyboard & mouse",
            hint: "Needs a powered hub: the reader does not power the port",
            cells: toggle(s.usb, Change::Usb),
        },
        OptRow {
            label: "Scroll",
            hint: "Wheel and two-finger direction; Natural moves the page like a touch screen",
            cells: vec![
                choice(
                    "Traditional",
                    Change::NaturalScroll(false),
                    !s.natural_scroll,
                ),
                choice("Natural", Change::NaturalScroll(true), s.natural_scroll),
            ],
        },
        OptRow {
            label: "Orientation",
            hint: "Auto follows how you hold it",
            cells: word_cells(s.orientation, Change::Orientation),
        },
        OptRow {
            label: "Upside down",
            hint: "Turns Portrait or Landscape 180°",
            cells: toggle(s.upside_down, Change::UpsideDown),
        },
    ]
}

/// The page's side padding: at least wide enough that the scroll bar
/// (a handle width, one and a half handle widths from the edge) never
/// runs over a row.
fn side_pad(s: crate::theme::Scale, page: crate::theme::Scale) -> i32 {
    s.px(20.0)
        .max(crate::scrollbar::Metrics::new(page).content_inset())
}

/// Where Settings is: the category list, one category, or a choice list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    /// The list of categories.
    #[default]
    Root,
    /// A category's page (index into [`CATEGORIES`]).
    Category(usize),
    /// A row's choice list, inside a category.
    Choice(usize, &'static str),
    /// A plugin's own settings: its place in the Plugins list. Reachable
    /// only while the plugin is on.
    Plugin(usize),
}

/// Categories: name, then (section heading, row labels).
/// A section: heading and row labels.
pub type Section = (&'static str, &'static [&'static str]);

/// The Plugins category: its rows come from the installed plugins, not from
/// a fixed list.
pub const PLUGINS_CATEGORY: usize = 6;

/// The Logging category: a switch and, once it is on, the level.
pub const LOGGING_CATEGORY: usize = 7;

/// The Settings categories, in order: name, then its sections.
pub const CATEGORIES: [(&str, &[Section]); 9] = [
    (
        "Writing",
        &[
            ("Text", &["Type", "Size", "Leading", "Margin"]),
            ("Line length", &["Line", "Words per line"]),
            ("Tab key", &["Tab key", "Tab width"]),
        ],
    ),
    (
        "Display",
        &[
            ("Page", &["Dark mode", "UI scale", "Drawer", "Drawer width"]),
            ("Light", &["Backlight"]),
            ("Orientation", &["Orientation", "Upside down"]),
        ],
    ),
    (
        "Keyboard & input",
        &[
            (
                "On-screen keyboard",
                &["Enhanced keyboard", "Keyboard", "Layout", "Keyboard tone"],
            ),
            ("Keyboard size", &["Auto keyboard size", "Keyboard size"]),
            (
                "Handwriting",
                &[
                    "Ink pen",
                    "Fast ink (A2)",
                    "Letter boxes",
                    "Word mode (experimental)",
                    "Learn from corrections",
                    "My handwriting",
                ],
            ),
        ],
    ),
    (
        "Language",
        &[(
            "Interface",
            &["Override system language", "Interface language"],
        )],
    ),
    (
        "USB & devices",
        &[("USB keyboard & mouse", &["USB keyboard & mouse", "Scroll"])],
    ),
    (
        "Experimental features",
        &[("Not fully tested yet", &["Experimental features"])],
    ),
    ("Plugins", &[]),
    ("Logging", &[("Log file", &["Logging", "Level"])]),
    (
        "Help & about",
        &[(
            "Help",
            &[
                "App info",
                "Markdown reference",
                "USB keyboard, mouse & export",
            ],
        )],
    ),
];

/// The geometry of a Settings or Help page's top bar and title.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PageHeader {
    pub back: Option<Rect>,
    pub heading_y: i32,
    /// Where the content starts.
    pub top: i32,
    /// "Page i of n", right of the title.
    pub pager: Rect,
    /// Side padding.
    pub pad: i32,
}

/// How a row is shown and what a tap does.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// Checkbox + "On": tap toggles.
    /// An on/off setting: a toggle switch (`crate::switch`).
    Check {
        /// Whether the setting is on.
        on: bool,
        /// The change a tap makes.
        change: Change,
    },
    /// Two-way choice: inline buttons.
    Inline(Vec<Cell>),
    /// − value +.
    Stepper(Vec<Cell>),
    /// Current value in italics; tap opens the choice list.
    Pick(String),
    /// Opens a Help page (index into `help::MENU`).
    Link(usize),
    /// Opens another Settings page (a plugin's own).
    Open(SettingsPage),
    /// Read-only value.
    Info(String),
    /// A disabled choice (v1.5.5): its value in the slider's grey, no tap.
    Muted(String),
    /// Stepped slider (v1.4.4, `crate::slider`): one cell per stop, the
    /// thumb's index, and whether it can be moved.
    Slider {
        /// One cell per stop.
        stops: Vec<Cell>,
        /// Index of the stop the thumb is on.
        value: usize,
        /// Whether the thumb can be moved.
        enabled: bool,
        /// Also label the two ends (Backlight, Keyboard size, UI scale).
        ends: bool,
    },
}

/// A row as Settings draws it: label, control and an optional note.
#[derive(Clone, Debug, PartialEq)]
pub struct DRow {
    /// Static for every built-in row; a plugin's name is owned.
    pub label: Cow<'static, str>,
    /// How the row is shown and what a tap does.
    pub kind: Kind,
    /// A greyed row says why, in small print under its label (rulebook
    /// §5.5 and §5.7); `None` on every row that is live.
    pub note: Option<Cow<'static, str>>,
}

/// An installed plugin, as the Plugins page lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginRow {
    /// The plugin's id, which a switch acts on (the row's place in the list
    /// is only what the page showed).
    pub id: String,
    /// The name shown.
    pub name: String,
    /// The kind in English ("Screen", "Font", …); translated when drawn.
    pub kind: &'static str,
    /// The plugin's version, as shown.
    pub version: String,
    /// Whether the plugin is on.
    pub enabled: bool,
    /// Data packs are read at start, so a change waits for a restart.
    pub restart: bool,
    /// False when the plugin needs a newer app: shown, but not switchable.
    pub compatible: bool,
    /// The settings the plugin declares, with the reader's choices.
    pub settings: Vec<PluginSetting>,
}

/// What the repository offers for one plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoState {
    /// Not installed: the row offers Install.
    Install,
    /// A newer version than the installed one: the row offers Update.
    Update,
    /// Installed and current.
    Current,
    /// Needs a newer app.
    Incompatible,
}

/// A plugin the repository index lists, as the Plugins page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoRow {
    /// The name shown.
    pub name: String,
    /// Kind, version and size, as the small print.
    pub note: String,
    /// What the row offers.
    pub state: RepoState,
}

/// One of a plugin's settings: a choice between a few options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginSetting {
    /// The setting's name.
    pub label: String,
    /// The options to choose from.
    pub options: Vec<String>,
    /// Index into `options`.
    pub selected: usize,
}

/// Everything Settings needs to lay itself out and draw.
pub struct OptionsModel<'a> {
    /// The current settings.
    pub settings: &'a Settings,
    /// Colours to draw in.
    pub theme: Theme,
    /// The on-screen keyboard (for the editor layout underneath).
    pub keyboard: &'a Keyboard,
    /// What Orientation is doing (v1.2), e.g. "Auto (no sensor) — Portrait".
    pub orientation_status: &'a str,
    /// The page shown.
    pub page: SettingsPage,
    /// Sub-page when a page has more rows than fit.
    pub sub: usize,
    /// Interface language, as shown (e.g. "English").
    pub language: &'a str,
    /// The app version, as shown under App info.
    pub version: &'a str,
    /// The slider row that has keyboard focus (v1.4.4), if any.
    pub focus: Option<&'static str>,
    /// Enrolled handwriting samples (v1.5.2).
    pub hw_samples: usize,
    /// The front light's step when the device has one (v1.12); `None`
    /// hides Display → Backlight.
    pub light: Option<u8>,
    /// UI scale while its slider is being dragged: the thumb moves and the
    /// value shows, but the scale itself only changes on release (v1.13.1).
    pub ui_scale_pending: Option<u16>,
    /// The installed plugins, for Settings → Plugins.
    pub plugins: &'a [PluginRow],
    /// What the plugins repository lists, once checked (v1.16).
    pub repository: &'a [RepoRow],
    /// How the last check went, in small print under "Check for plugins";
    /// empty before the first.
    pub repo_note: &'a str,
}

/// One drawn line of a page: a section band or a row.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A section heading.
    Band(&'static str),
    /// A setting.
    Row(DRow),
    /// A category on the root page, with its summary.
    Category(usize, String),
    /// A choice on a choice page: its label, the change it makes, and
    /// whether it is the current one.
    Choice(String, Change, bool),
}

/// Where everything on a Settings page is, in device pixels.
#[derive(Clone, Debug, Default)]
pub struct OptionsLayout {
    /// The editor layout underneath.
    pub base: Layout,
    /// The close mark.
    pub close: Rect,
    /// Back arrow (not on the root page).
    pub back: Option<Rect>,
    /// "Page i of n" (tap: next page), when there is more than one.
    pub pager: Option<Rect>,
    /// Number of sub-pages.
    pub pages: usize,
    /// The scroll bar over the sub-pages (v1.12): track strip and thumb,
    /// when there is more than one page. Display only.
    pub bar: Option<crate::scrollbar::Paged>,
    /// Big page title baseline.
    pub heading_y: i32,
    /// Items on the current sub-page: the item, its band, its controls.
    pub items: Vec<(Item, Rect, Vec<Rect>)>,
}

/// What a tap in Settings landed on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptionsHit {
    /// The close mark.
    Close,
    /// The back arrow.
    Back,
    /// Next sub-page.
    Page,
    /// Go to another Settings page.
    Open(SettingsPage),
    /// Apply a change.
    Change(Change),
    /// Choose a value and go back to the category.
    Pick(Change),
    /// Open a Help page (index into `help::MENU`).
    Help(usize),
    /// Nothing that responds to a tap.
    Nothing,
}

fn row_kind(r: &OptRow) -> Kind {
    let c = &r.cells;
    if c.len() == 2 && c[0].label == "Off" && c[1].label == "On" {
        let on = c[1].active;
        let change = if on { c[0].change } else { c[1].change };
        return Kind::Check {
            on,
            change: change.expect("toggle"),
        };
    }
    if c.len() == 3 && c[1].change.is_none() {
        return Kind::Stepper(c.clone());
    }
    if c.len() == 2 {
        return Kind::Inline(c.clone());
    }
    Kind::Pick(
        c.iter()
            .find(|x| x.active)
            .map_or_else(String::new, |x| x.label.clone()),
    )
}

/// A level as the Settings choice names it.
fn level_label(l: sigma_core::log::Level) -> &'static str {
    use sigma_core::log::Level;
    match l {
        Level::Debug => "Debug",
        Level::Info => "Info",
        Level::Warning => "Warning",
        Level::Error => "Error",
        Level::Critical => "Critical",
    }
}

/// The rows of every category, by label.
#[must_use]
pub fn display_row(m: &OptionsModel<'_>, label: &'static str) -> Option<DRow> {
    // Experimental features hide their own switches until they are on.
    if matches!(label, "Fast ink (A2)" | "Word mode (experimental)") && !m.settings.experimental {
        return None;
    }
    // The level has nothing to do until the log is on.
    if label == "Level" && !m.settings.log {
        return None;
    }
    // The Compact / Full choice belongs to the enhanced keyboard only (v1.13).
    if label == "Keyboard" && !m.settings.enhanced_keyboard {
        return None;
    }
    let rows = option_rows(m.settings);
    let kind = match label {
        "Line" => {
            let r = rows.iter().find(|r| r.label == "Line")?;
            row_kind(&OptRow {
                label: "Line",
                hint: "",
                cells: r.cells[..2].to_vec(),
            })
        }
        // Greyed (with a note) while Line is off.
        "Words per line" if !m.settings.line_on => {
            Kind::Muted(format!("{} words", m.settings.line_words))
        }
        "Words per line" => {
            let r = rows.iter().find(|r| r.label == "Line")?;
            Kind::Stepper(r.cells[2..].to_vec())
        }
        "Layout" => Kind::Pick(m.settings.kb_layout.label().to_owned()),
        // v1.6.3: a slider with no unit and no labels (it is not points).
        "Size" => {
            let (value, cur) = size_stop(m.settings.size);
            Kind::Slider {
                value,
                stops: sigma_core::settings::SIZE_STOPS
                    .iter()
                    .map(|&v| choice(&num(v), Change::SizeTo(v), v == cur))
                    .collect(),
                enabled: true,
                ends: false,
            }
        }
        // Leading and Margin are sliders like Size: stops, no numbers.
        "Leading" | "Margin" => {
            let (stops, cur): (&[f32], f32) = if label == "Leading" {
                (&sigma_core::settings::LEADING_STOPS, m.settings.leading)
            } else {
                (&sigma_core::settings::MARGIN_STOPS, m.settings.margin)
            };
            let (value, near) = nearest_stop(stops, cur);
            let cells = stops
                .iter()
                .map(|&v| {
                    let change = if label == "Leading" {
                        Change::LeadingTo(v)
                    } else {
                        Change::MarginTo(v)
                    };
                    let text = if label == "Leading" {
                        format!("{v:.1}")
                    } else {
                        num(v)
                    };
                    choice(&text, change, v == near)
                })
                .collect();
            Kind::Slider {
                value,
                stops: cells,
                enabled: true,
                ends: false,
            }
        }
        // v1.12: nine steps from off to full; only on a Kobo with a light.
        "Backlight" => {
            let n = m.light?;
            let stops: Vec<Cell> = (0..=sigma_core::settings::BACKLIGHT_MAX)
                .map(|i| {
                    let text = match i {
                        0 => tr("Off").into_owned(),
                        sigma_core::settings::BACKLIGHT_MAX => tr("Full").into_owned(),
                        i => format!(
                            "{} %",
                            usize::from(i) * 100 / usize::from(sigma_core::settings::BACKLIGHT_MAX)
                        ),
                    };
                    choice(&text, Change::Backlight(i), i == n)
                })
                .collect();
            Kind::Slider {
                value: usize::from(n),
                stops,
                enabled: true,
                ends: true,
            }
        }
        "UI scale" => {
            let shown = m.ui_scale_pending.unwrap_or(m.settings.ui_scale);
            let stops: Vec<Cell> = sigma_core::settings::UI_SCALES
                .iter()
                .map(|&v| choice(&format!("{v} %"), Change::UiScaleTo(v), v == shown))
                .collect();
            Kind::Slider {
                value: stops.iter().position(|c| c.active).unwrap_or(0),
                stops,
                enabled: true,
                ends: true,
            }
        }
        // Five stops from a narrow drawer to half the page.
        "Drawer width" => {
            use sigma_core::settings::{drawer_tenths, DRAWER_WIDTH_STOPS};
            let stops: Vec<Cell> = (0..DRAWER_WIDTH_STOPS)
                .map(|i| {
                    let t = drawer_tenths(i);
                    let text = if t.is_multiple_of(10) {
                        format!("{} %", t / 10)
                    } else {
                        format!("{}.{} %", t / 10, t % 10)
                    };
                    choice(&text, Change::DrawerWidth(i), i == m.settings.drawer_width)
                })
                .collect();
            Kind::Slider {
                value: usize::from(m.settings.drawer_width),
                stops,
                enabled: true,
                ends: false,
            }
        }
        "Keyboard size" => {
            let cur = m.settings.keyboard_scale();
            let stops: Vec<Cell> = sigma_core::settings::KB_SIZES[1..]
                .iter()
                .map(|&v| choice(&format!("{v}%"), Change::KbSize(v), v == cur))
                .collect();
            Kind::Slider {
                value: stops.iter().position(|c| c.active).unwrap_or(0),
                stops,
                enabled: m.settings.kb_size != 0,
                ends: true,
            }
        }
        // v1.6.3: three buttons in the row, like Paper and Drawer (no list).
        "Orientation" => {
            let r = rows.iter().find(|r| r.label == "Orientation")?;
            Kind::Inline(r.cells.clone())
        }
        // v1.5.5 (decision 57): greyed system language unless overridden.
        "Interface language" if m.settings.lang_override => {
            Kind::Pick(m.settings.ui_lang.name().to_owned())
        }
        "Interface language" => Kind::Muted(m.language.to_owned()),
        // Auto follows the sensor, so there is no 180° to choose: greyed.
        "Upside down" if m.settings.orientation == Orientation::Auto => {
            Kind::Muted(tr(if m.settings.upside_down { "On" } else { "Off" }).into_owned())
        }
        // Words per line only matters with Line on; Tab width only with
        // Tab key set to Spaces: greyed, with the reason as a note.
        "Tab width" if !m.settings.tab_spaces => Kind::Muted(m.settings.tab_width.to_string()),
        // v1.5.2: what Train saved, and a way to delete it.
        "My handwriting" => Kind::Inline(vec![
            value(format!("{}: {}", tr("Samples"), m.hw_samples)),
            choice("Train", Change::TrainSamples, false),
            choice("Delete", Change::DeleteSamples, false),
        ]),
        "App info" => Kind::Link(0),
        "Markdown reference" => Kind::Link(1),
        "USB keyboard, mouse & export" => Kind::Link(2),
        _ => row_kind(rows.iter().find(|r| r.label == label)?),
    };
    let note = match (label, &kind) {
        ("Upside down", Kind::Muted(_)) => Some("Needs Portrait or Landscape"),
        ("Interface language", Kind::Muted(_)) => Some("Follows the Kobo's language"),
        ("Words per line", Kind::Muted(_)) => Some("Needs Line on"),
        ("Tab width", Kind::Muted(_)) => Some("Used when Tab key is Spaces"),
        ("Keyboard size", Kind::Slider { enabled: false, .. }) => {
            Some("Turn off Auto keyboard size")
        }
        _ => None,
    };
    Some(DRow {
        label: Cow::Borrowed(label),
        kind,
        note: note.map(Cow::Borrowed),
    })
}

/// The choices of a Pick row: label, change, active.
#[must_use]
pub fn choices(s: &Settings, label: &str) -> Vec<(String, Change, bool)> {
    if label == "Layout" {
        let all = s.layouts();
        let cur = all.iter().position(|&l| l == s.kb_layout).unwrap_or(0) as i32;
        return all
            .iter()
            .enumerate()
            .map(|(i, l)| {
                (
                    l.label().to_owned(),
                    Change::KbLayout((i as i32 - cur) as i8),
                    i as i32 == cur,
                )
            })
            .collect();
    }
    option_rows(s)
        .into_iter()
        .find(|r| r.label == label)
        .map(|r| {
            r.cells
                .into_iter()
                .filter_map(|c| c.change.map(|ch| (c.label, ch, c.active)))
                .collect()
        })
        .unwrap_or_default()
}

fn summary(m: &OptionsModel<'_>, cat: usize) -> String {
    let s = m.settings;
    let on_off = |b: bool| tr(if b { "On" } else { "Off" }).into_owned();
    match cat {
        // The type size has no unit worth showing (v1.6.3: it is a slider).
        0 => tr(s.face.label()).into_owned(),
        1 => format!(
            "{}, {} %",
            tr(if s.paper == sigma_core::settings::Paper::Night {
                "Night"
            } else {
                "Day"
            }),
            s.ui_scale
        ),
        2 => format!(
            "{}, {}",
            if s.kobo_keyboard() {
                "Kobo".into()
            } else {
                tr(if s.keyboard == KbLayout::Full {
                    "Full"
                } else {
                    "Compact"
                })
            },
            tr(s.kb_layout.label())
        ),
        3 => m.language.to_owned(),
        4 => on_off(s.usb),
        5 => on_off(s.experimental),
        PLUGINS_CATEGORY => {
            if m.plugins.is_empty() {
                tr("None").into_owned()
            } else {
                let on = m.plugins.iter().filter(|p| p.enabled).count();
                format!("{on} / {}", m.plugins.len())
            }
        }
        LOGGING_CATEGORY => {
            if s.log {
                tr(level_label(s.log_level)).into_owned()
            } else {
                on_off(false)
            }
        }
        _ => m.version.to_owned(),
    }
}

/// The Plugins page: one switch per installed plugin, then what the
/// repository offers.
fn plugin_items(m: &OptionsModel<'_>) -> Vec<Item> {
    let mut out = vec![Item::Band("Installed")];
    if m.plugins.is_empty() {
        out.push(Item::Row(DRow {
            label: Cow::Borrowed("No plugins installed"),
            kind: Kind::Info(String::new()),
            note: Some(Cow::Borrowed("Put plugins in .adds/sigma-scribe/plugins")),
        }));
    }
    for (i, p) in m.plugins.iter().enumerate() {
        let mut note = format!("{} \u{b7} {}", p.kind, p.version);
        let kind = if p.compatible {
            if p.restart {
                note.push_str(" \u{b7} restart to apply");
            }
            Kind::Check {
                on: p.enabled,
                change: Change::Plugin(i, !p.enabled),
            }
        } else {
            note = "Needs a newer app".to_owned();
            Kind::Muted(tr("Off").into_owned())
        };
        out.push(Item::Row(DRow {
            label: Cow::Owned(p.name.clone()),
            kind,
            note: Some(Cow::Owned(note)),
        }));
        // A plugin with settings of its own: their page opens once the
        // plugin is on, and is locked (greyed, with the reason) while off.
        if p.compatible && !p.settings.is_empty() {
            let label = Cow::Owned(format!("{} \u{b7} Settings", p.name));
            out.push(Item::Row(if p.enabled {
                DRow {
                    label,
                    kind: Kind::Open(SettingsPage::Plugin(i)),
                    note: None,
                }
            } else {
                DRow {
                    label,
                    kind: Kind::Muted(String::new()),
                    note: Some(Cow::Borrowed("Turn the plugin on first")),
                }
            }));
        }
    }
    out.push(Item::Band("Repository"));
    out.push(Item::Row(DRow {
        label: Cow::Borrowed("Check for plugins"),
        kind: Kind::Inline(vec![choice("Check", Change::PluginCheck, false)]),
        note: (!m.repo_note.is_empty()).then(|| Cow::Owned(m.repo_note.to_owned())),
    }));
    for (i, r) in m.repository.iter().enumerate() {
        let kind = match r.state {
            RepoState::Install => {
                Kind::Inline(vec![choice("Install", Change::PluginGet(i as u8), false)])
            }
            RepoState::Update => {
                Kind::Inline(vec![choice("Update", Change::PluginGet(i as u8), false)])
            }
            RepoState::Current => Kind::Muted(tr("Up to date").into_owned()),
            RepoState::Incompatible => Kind::Muted(String::new()),
        };
        let note = if r.state == RepoState::Incompatible {
            format!("{} \u{b7} Needs a newer app", r.note)
        } else {
            r.note.clone()
        };
        out.push(Item::Row(DRow {
            label: Cow::Owned(r.name.clone()),
            kind,
            note: Some(Cow::Owned(note)),
        }));
    }
    out
}

/// A plugin's own settings page: one choice per declared setting. Nothing
/// to show (and so nothing to change) unless the plugin is there and on.
fn plugin_setting_items(m: &OptionsModel<'_>, i: usize) -> Vec<Item> {
    let mut out = vec![Item::Band("Settings")];
    let Some(p) = m.plugins.get(i).filter(|p| p.enabled && p.compatible) else {
        out.push(Item::Row(DRow {
            label: Cow::Borrowed("Turn the plugin on first"),
            kind: Kind::Info(String::new()),
            note: None,
        }));
        return out;
    };
    for (si, st) in p.settings.iter().enumerate() {
        let cells = st
            .options
            .iter()
            .enumerate()
            .map(|(oi, o)| Cell {
                label: o.clone(),
                change: Some(Change::PluginSetting(i as u8, si as u8, oi as u8)),
                active: oi == st.selected,
            })
            .collect();
        out.push(Item::Row(DRow {
            label: Cow::Owned(st.label.clone()),
            kind: Kind::Inline(cells),
            note: None,
        }));
    }
    out
}

/// Every item of a page, before splitting into sub-pages.
#[must_use]
pub fn page_items(m: &OptionsModel<'_>) -> Vec<Item> {
    match m.page {
        SettingsPage::Root => (0..CATEGORIES.len())
            .map(|i| Item::Category(i, summary(m, i)))
            .collect(),
        SettingsPage::Category(PLUGINS_CATEGORY) => plugin_items(m),
        SettingsPage::Plugin(i) => plugin_setting_items(m, i),
        SettingsPage::Category(c) => {
            let mut out = Vec::new();
            for (band, rows) in CATEGORIES[c.min(CATEGORIES.len() - 1)].1 {
                // A band with no row to show (no front light, say) is left out.
                let shown: Vec<DRow> = rows.iter().filter_map(|r| display_row(m, r)).collect();
                if shown.is_empty() {
                    continue;
                }
                out.push(Item::Band(band));
                out.extend(shown.into_iter().map(Item::Row));
            }
            out
        }
        SettingsPage::Choice(_, label) => choices(m.settings, label)
            .into_iter()
            .map(|(l, c, a)| Item::Choice(l, c, a))
            .collect(),
    }
}

/// The `&'static str` of a built-in row (only those open a choice page).
// It matches on the variant to hand back the static string, which `&str` cannot.
#[allow(clippy::ptr_arg)]
fn static_label(l: &Cow<'static, str>) -> &'static str {
    match l {
        Cow::Borrowed(s) => s,
        Cow::Owned(_) => "",
    }
}

/// The page title.
#[must_use]
pub fn page_title(p: SettingsPage) -> &'static str {
    match p {
        SettingsPage::Root => "Settings",
        SettingsPage::Category(c) => CATEGORIES[c.min(CATEGORIES.len() - 1)].0,
        SettingsPage::Choice(_, l) => l,
        SettingsPage::Plugin(_) => CATEGORIES[PLUGINS_CATEGORY].0,
    }
}

/// The page title as the model shows it: a plugin's own page is titled with
/// the plugin's name.
#[must_use]
pub fn page_title_in<'a>(m: &'a OptionsModel<'_>) -> Cow<'a, str> {
    match m.page {
        SettingsPage::Plugin(i) => m
            .plugins
            .get(i)
            .map_or(Cow::Borrowed(page_title(m.page)), |p| {
                Cow::Borrowed(p.name.as_str())
            }),
        p => Cow::Borrowed(page_title(p)),
    }
}

/// The page above.
#[must_use]
pub fn parent(p: SettingsPage) -> Option<SettingsPage> {
    match p {
        SettingsPage::Root => None,
        SettingsPage::Category(_) => Some(SettingsPage::Root),
        SettingsPage::Choice(c, _) => Some(SettingsPage::Category(c)),
        SettingsPage::Plugin(_) => Some(SettingsPage::Category(PLUGINS_CATEGORY)),
    }
}

/// Category holding a row label (or a category name itself).
#[must_use]
pub fn category_of(label: &str) -> Option<usize> {
    CATEGORIES
        .iter()
        .position(|(_, secs)| secs.iter().any(|(_, rows)| rows.contains(&label)))
}

impl EditorScreen {
    fn cell_w(&self, label: &str) -> i32 {
        let s = self.scale();
        (self
            .fonts
            .width(Face::AtkinsonBold, self.ui_px(), &tr(label)) as i32
            + 2 * s.px(14.0))
        .max(s.px(60.0))
    }

    /// The slider's area in a row (v1.4.4): the right part of the row,
    /// at most 300 brief px, never narrower than 44 px per stop.
    fn slider_geom(&self, band: Rect, n: usize) -> crate::slider::Geom {
        let s = self.scale();
        let min = (n as i32) * self.page_scale().px(44.0) + 2 * s.px(24.0);
        let w = s.px(300.0).min(band.w / 2).max(min).min(band.w);
        let h = band.h - s.px(8.0);
        crate::slider::geom(
            Rect::new(band.x + band.w - w, band.y + s.px(4.0), w, h),
            n,
            s,
        )
    }

    /// A slider row on this page: its label, geometry, stops and state.
    fn sliders(
        &self,
        m: &OptionsModel<'_>,
    ) -> Vec<(&'static str, crate::slider::Geom, Vec<Cell>, usize, bool)> {
        let l = self.options_layout(m);
        l.items
            .iter()
            .filter_map(|(it, band, _)| match it {
                Item::Row(DRow {
                    label,
                    kind:
                        Kind::Slider {
                            stops,
                            value,
                            enabled,
                            ..
                        },
                    ..
                }) => Some((
                    static_label(label),
                    self.slider_geom(*band, stops.len()),
                    stops.clone(),
                    *value,
                    *enabled,
                )),
                _ => None,
            })
            .collect()
    }

    /// The slider under a finger (v1.4.4): its label, and the change for
    /// the nearest stop when it is enabled.
    #[must_use]
    pub fn options_slider_at(
        &self,
        m: &OptionsModel<'_>,
        x: i32,
        y: i32,
    ) -> Option<(&'static str, Option<Change>)> {
        self.sliders(m)
            .into_iter()
            .find_map(|(lab, g, stops, _, en)| {
                g.area.contains(x, y).then(|| {
                    (
                        lab,
                        en.then(|| stops[crate::slider::nearest(&g, x)].change)
                            .flatten(),
                    )
                })
            })
    }

    /// Dragging slider `label` to `x` (snaps to the nearest stop).
    #[must_use]
    pub fn options_slider_drag(&self, m: &OptionsModel<'_>, label: &str, x: i32) -> Option<Change> {
        self.sliders(m)
            .into_iter()
            .find(|t| t.0 == label && t.4)
            .and_then(|(_, g, stops, _, _)| stops[crate::slider::nearest(&g, x)].change)
    }

    /// Left (`-1`) / Right (`+1`) on a focused slider.
    #[must_use]
    pub fn options_slider_step(&self, m: &OptionsModel<'_>, label: &str, d: i32) -> Option<Change> {
        self.sliders(m)
            .into_iter()
            .find(|t| t.0 == label && t.4)
            .and_then(|(_, _, stops, v, _)| {
                let j = crate::slider::step(v, stops.len(), d);
                (j != v).then(|| stops[j].change).flatten()
            })
    }

    /// Where the top bar and the page title go, for Settings and Help alike
    /// (the Kobo pattern, rulebook §5.1).
    #[must_use]
    pub(crate) fn page_header(&self, base: &Layout, back_on: bool) -> PageHeader {
        let s = self.scale();
        let w = self.width as i32;
        let pad = side_pad(s, self.page_scale());
        let back = back_on.then(|| {
            Rect::new(
                s.px(4.0),
                base.close_button.y,
                base.close_button.w,
                base.close_button.h,
            )
        });
        let heading_y = base.title.h + s.px(40.0);
        let pw = self
            .fonts
            .width(Face::LiterataItalic, self.ui_px() * 0.9, "Page 00 of 00")
            as i32;
        PageHeader {
            back,
            heading_y,
            top: heading_y + s.px(16.0),
            pager: Rect::new(w - pad - pw, heading_y - s.px(30.0), pw, s.px(44.0)),
            pad,
        }
    }

    /// Draws the top bar (back arrow, small centred title, close), the big
    /// page title and, when there is more than one page, "Page i of n".
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_page_header(
        &self,
        c: &mut Canvas,
        base: &Layout,
        h: &PageHeader,
        small: &str,
        big: &str,
        pages: Option<(usize, usize)>,
        t: Theme,
    ) {
        let ui = self.ui_px();
        let w = self.width as i32;
        // No rule under the bar (Kobo Forma 4.x look).
        let bar_base = base.title.y as f32
            + base.title.h as f32 / 2.0
            + self.fonts.ascent(Face::Literata, ui) / 2.6;
        let top = tr(small);
        let tw = self.fonts.width(Face::Literata, ui * 0.9, &top);
        self.fonts.draw(
            c,
            Face::Literata,
            ui * 0.9,
            (w as f32 - tw) / 2.0,
            bar_base,
            &top,
            t.ink,
        );
        crate::icons::draw_icon(c, crate::icons::Icon::Close, base.close_button, t.ink);
        if let Some(b) = h.back {
            crate::icons::draw_icon(c, crate::icons::Icon::Back, b, t.ink);
        }
        self.fonts.draw(
            c,
            Face::Literata,
            ui * 1.5,
            h.pad as f32,
            h.heading_y as f32,
            &tr(big),
            t.ink,
        );
        if let Some((at, n)) = pages.filter(|&(_, n)| n > 1) {
            let txt = tr(&format!("Page {} of {}", at + 1, n)).into_owned();
            let pw = self.fonts.width(Face::LiterataItalic, ui * 0.9, &txt);
            self.fonts.draw(
                c,
                Face::LiterataItalic,
                ui * 0.9,
                (h.pager.x + h.pager.w) as f32 - pw,
                h.heading_y as f32,
                &txt,
                t.ink,
            );
        }
    }

    /// Lays out the current Settings page and sub-page of `m`.
    #[must_use]
    pub fn options_layout(&self, m: &OptionsModel<'_>) -> OptionsLayout {
        let base = self.layout(false, m.keyboard);
        let s = self.scale();
        let w = self.width as i32;
        let hd = self.page_header(&base, m.page != SettingsPage::Root);
        let (pad, back, heading_y, top) = (hd.pad, hd.back, hd.heading_y, hd.top);
        let row_h = s.px(52.0);
        let band_h = s.px(24.0);
        let bottom = self.height as i32 - s.px(12.0);
        // Split into sub-pages; a band never ends a page.
        let all = page_items(m);
        let slider_h = s.px(84.0);
        let item_h = |it: &Item| match it {
            Item::Band(_) => band_h,
            Item::Row(DRow {
                kind: Kind::Slider { .. },
                ..
            }) => slider_h,
            _ => row_h,
        };
        let mut pages: Vec<Vec<Item>> = vec![Vec::new()];
        let mut y = top;
        for (i, it) in all.iter().enumerate() {
            let h = item_h(it);
            let need = if matches!(it, Item::Band(_)) && i + 1 < all.len() {
                h + row_h
            } else {
                h
            };
            if y + need > bottom && !pages.last().unwrap().is_empty() {
                pages.push(Vec::new());
                y = top;
            }
            pages.last_mut().unwrap().push(it.clone());
            y += h;
        }
        let n = pages.len();
        let sub = m.sub.min(n - 1);
        let mut items = Vec::new();
        let mut y = top;
        let btn = s.px(44.0);
        let gap = s.px(6.0);
        for it in pages.swap_remove(sub) {
            let h = item_h(&it);
            let band = Rect::new(pad, y, w - 2 * pad, h);
            let cells = match &it {
                Item::Row(DRow {
                    kind: Kind::Inline(c) | Kind::Stepper(c),
                    ..
                }) => {
                    let widths: Vec<i32> = c.iter().map(|x| self.cell_w(&x.label)).collect();
                    let total: i32 = widths.iter().sum::<i32>() + gap * (widths.len() as i32 - 1);
                    let mut x = w - pad - total;
                    widths
                        .iter()
                        .map(|&cw| {
                            let r = Rect::new(x, y + (h - btn) / 2, cw, btn);
                            x += cw + gap;
                            r
                        })
                        .collect()
                }
                Item::Row(DRow {
                    kind: Kind::Slider { stops, .. },
                    ..
                }) => {
                    let g = self.slider_geom(band, stops.len());
                    let mut v = vec![g.area];
                    v.extend(g.stops);
                    v
                }
                Item::Row(DRow {
                    kind: Kind::Check { .. },
                    ..
                }) => vec![crate::switch::target(
                    band,
                    self.scale(),
                    &self.fonts,
                    self.ui_px(),
                )],
                Item::Choice(..) => vec![crate::radio::target(band, self.scale())],
                Item::Row(DRow {
                    kind: Kind::Pick(_) | Kind::Link(_) | Kind::Open(_),
                    ..
                })
                | Item::Category(..) => vec![band],
                _ => vec![],
            };
            items.push((it, band, cells));
            y += h;
        }
        let pager = (n > 1).then_some(hd.pager);
        let bar = (n > 1)
            .then(|| {
                let page = Rect::new(0, top, w, bottom - top);
                crate::scrollbar::Paged::new(self.page_scale(), page, n, sub)
            })
            .flatten();
        OptionsLayout {
            close: base.close_button,
            base,
            back,
            pager,
            pages: n,
            bar,
            heading_y,
            items,
        }
    }

    /// What a tap at (`x`, `y`) in Settings means.
    #[must_use]
    pub fn options_hit(&self, m: &OptionsModel<'_>, x: i32, y: i32) -> OptionsHit {
        let l = self.options_layout(m);
        let slop = self.scale().px(4.0);
        if l.close.inset(-slop).contains(x, y) {
            return OptionsHit::Close;
        }
        if l.back.is_some_and(|b| b.inset(-slop).contains(x, y)) {
            return OptionsHit::Back;
        }
        if l.pager.is_some_and(|b| b.contains(x, y)) {
            return OptionsHit::Page;
        }
        for (it, band, cells) in &l.items {
            for (j, r) in cells.iter().enumerate() {
                if !r.inset(-self.scale().px(3.0) / 2).contains(x, y) {
                    continue;
                }
                return match it {
                    Item::Category(i, _) => OptionsHit::Open(SettingsPage::Category(*i)),
                    Item::Choice(_, c, _) => OptionsHit::Pick(*c),
                    Item::Row(d) => match &d.kind {
                        Kind::Check { change, .. } => OptionsHit::Change(*change),
                        Kind::Inline(c) | Kind::Stepper(c) => {
                            c[j].change.map_or(OptionsHit::Nothing, OptionsHit::Change)
                        }
                        Kind::Pick(_) => match m.page {
                            SettingsPage::Category(c) => {
                                OptionsHit::Open(SettingsPage::Choice(c, static_label(&d.label)))
                            }
                            _ => OptionsHit::Nothing,
                        },
                        Kind::Link(i) => OptionsHit::Help(*i),
                        Kind::Open(p) => OptionsHit::Open(*p),
                        Kind::Info(_) | Kind::Muted(_) => OptionsHit::Nothing,
                        Kind::Slider { stops, enabled, .. } => {
                            if !*enabled {
                                return OptionsHit::Nothing;
                            }
                            let g = self.slider_geom(*band, stops.len());
                            stops[crate::slider::nearest(&g, x)]
                                .change
                                .map_or(OptionsHit::Nothing, OptionsHit::Change)
                        }
                    },
                    Item::Band(_) => OptionsHit::Nothing,
                };
            }
        }
        OptionsHit::Nothing
    }

    /// Named controls on the current page (emulator, touch audit):
    /// `Close`, `Back`, `Page`, category names, `Row/Cell` for buttons and
    /// steppers (`Size/+`), `Row/On` / `Row/Off` for
    /// checkboxes, `Row` for a choice row, `Row/Choice` on a choice page.
    #[must_use]
    pub fn options_controls(&self, m: &OptionsModel<'_>) -> Vec<(String, Rect)> {
        let l = self.options_layout(m);
        let mut out = vec![("Close".to_owned(), l.close)];
        if let Some(b) = l.back {
            out.push(("Back".into(), b));
        }
        if let Some(p) = l.pager {
            out.push(("Page".into(), p));
        }
        for (it, band, cells) in &l.items {
            match it {
                Item::Category(i, _) => out.push((CATEGORIES[*i].0.into(), cells[0])),
                Item::Choice(lab, _, _) => {
                    if let SettingsPage::Choice(_, row) = m.page {
                        out.push((format!("{row}/{lab}"), cells[0]));
                    }
                    out.push((lab.clone(), cells[0]));
                }
                Item::Row(d) => match &d.kind {
                    Kind::Check { on, .. } => {
                        // The state already set names the label (a no-op tap).
                        let (set, other) = if *on { ("Off", "On") } else { ("On", "Off") };
                        out.push((format!("{}/{set}", d.label), cells[0]));
                        let lab = Rect::new(band.x, band.y, band.w / 3, band.h);
                        out.push((format!("{}/{other}", d.label), lab));
                    }
                    Kind::Inline(c) | Kind::Stepper(c) => {
                        for (cell, r) in c.iter().zip(cells) {
                            let name = if cell.label == "\u{2212}" {
                                "-"
                            } else {
                                &cell.label
                            };
                            out.push((format!("{}/{name}", d.label), *r));
                        }
                    }
                    Kind::Pick(_) | Kind::Link(_) | Kind::Open(_) => {
                        out.push((d.label.to_string(), cells[0]));
                    }
                    Kind::Info(_) | Kind::Muted(_) => {}
                    Kind::Slider { stops, .. } => {
                        out.push((d.label.to_string(), cells[0]));
                        for (i, (cell, r)) in stops.iter().zip(&cells[1..]).enumerate() {
                            // Unlabelled stops (Size) are `Size/1`, `Size/2` ….
                            let name = if cell.label.is_empty() {
                                (i + 1).to_string()
                            } else {
                                cell.label.clone()
                            };
                            out.push((format!("{}/{name}", d.label), *r));
                        }
                    }
                },
                Item::Band(_) => {}
            }
        }
        // Names from the 1.4 sheet: `Layout/+` / `Layout/-` on the Layout
        // list are the next / previous layout; `Line/-` / `Line/+` the words.
        if let SettingsPage::Choice(_, row) = m.page {
            let ch: Vec<(Rect, bool)> = l
                .items
                .iter()
                .filter_map(|(i, _, c)| match i {
                    Item::Choice(_, _, a) => Some((c[0], *a)),
                    _ => None,
                })
                .collect();
            let n = ch.len() as i32;
            if let Some(cur) = ch.iter().position(|c| c.1) {
                if l.pages == 1 {
                    let at = |d: i32| ch[(cur as i32 + d).rem_euclid(n) as usize].0;
                    out.push((format!("{row}/+"), at(1)));
                    out.push((format!("{row}/-"), at(-1)));
                } else {
                    // A list over several pages (18 layouts since 1.6.0):
                    // only a neighbour on this page, no wrap.
                    for (d, cell) in [(1, "+"), (-1, "-")] {
                        let i = cur as i32 + d;
                        if (0..n).contains(&i) {
                            out.push((format!("{row}/{cell}"), ch[i as usize].0));
                        }
                    }
                }
            }
        }
        let alias: Vec<(String, Rect)> = out
            .iter()
            .filter_map(|(n, r)| {
                n.strip_prefix("Words per line/")
                    .map(|c| (format!("Line/{c}"), *r))
            })
            .collect();
        out.extend(alias);
        out
    }

    /// Centre of a named control on the current page.
    #[must_use]
    pub fn options_control(&self, m: &OptionsModel<'_>, name: &str) -> Option<(i32, i32)> {
        self.options_controls(m)
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| (r.x + r.w / 2, r.y + r.h / 2))
    }

    /// The next control to tap on the way to `name` when it is not on
    /// this page (emulator scripts written for the 1.4 sheet keep working:
    /// `Orientation/Landscape` goes Display → Orientation → Landscape).
    #[must_use]
    pub fn options_step(&self, m: &OptionsModel<'_>, name: &str) -> Option<String> {
        let row = name.split_once('/').map_or(name, |(r, _)| r);
        let cat = category_of(row).or_else(|| CATEGORIES.iter().position(|(n, _)| *n == row))?;
        match m.page {
            SettingsPage::Root => Some(CATEGORIES[cat].0.to_owned()),
            SettingsPage::Choice(_, r) if r == row => {
                // Old stepper names on a choice list: `Layout/+` is the next one.
                let (_, cell) = name.split_once('/')?;
                let ch = choices(m.settings, r);
                let cur = ch.iter().position(|c| c.2)? as i32;
                let d = match cell {
                    "+" => 1,
                    "-" => -1,
                    _ => return None,
                };
                let n = ch.len() as i32;
                Some(ch[(cur + d).rem_euclid(n) as usize].0.clone())
            }
            SettingsPage::Choice(..) | SettingsPage::Plugin(_) => Some("Back".into()),
            SettingsPage::Category(c) if c != cat => Some("Back".into()),
            SettingsPage::Category(_) => {
                let here = self.options_controls(m);
                if here.iter().any(|(n, _)| n == row) && name.contains('/') {
                    return Some(row.to_owned());
                }
                let l = self.options_layout(m);
                (l.pages > 1).then(|| "Page".into())
            }
        }
    }

    /// Draws the current Settings page and returns its layout.
    pub fn draw_options(&self, c: &mut Canvas, m: &OptionsModel<'_>) -> OptionsLayout {
        let t = m.theme;
        let l = self.options_layout(m);
        let s = self.scale();
        let ui = self.ui_px();
        c.fill(c.bounds(), t.paper);
        let hd = self.page_header(&l.base, l.back.is_some());
        self.draw_page_header(
            c,
            &l.base,
            &hd,
            "Settings",
            &page_title_in(m),
            Some((m.sub.min(l.pages - 1), l.pages)),
            t,
        );
        if let Some(b) = l.bar {
            crate::scrollbar::draw_bar(c, self.page_scale(), (b.strip, b.thumb), t, t.paper);
        }
        let n = l.items.len();
        for (k, (it, band, cells)) in l.items.iter().enumerate() {
            let mid = band.y as f32 + band.h as f32 / 2.0;
            let base = mid + self.fonts.ascent(Face::Literata, ui) / 2.6;
            let right = (band.x + band.w) as f32;
            let italic = |c: &mut Canvas, txt: &str, fg: u8| {
                let tw = self.fonts.width(Face::LiterataItalic, ui, txt);
                self.fonts
                    .draw(c, Face::LiterataItalic, ui, right - tw, base, txt, fg);
            };
            let label = |c: &mut Canvas, txt: &str| {
                self.fonts.draw(
                    c,
                    Face::Literata,
                    ui,
                    band.x as f32,
                    base,
                    &format!("{txt}:"),
                    t.ink,
                );
            };
            match it {
                Item::Band(name) => {
                    c.fill(Rect::new(band.x, band.y, band.w, band.h), t.wash);
                    let sm = ui * 0.72;
                    let b = mid + self.fonts.ascent(Face::Atkinson, sm) / 2.6;
                    self.fonts.draw(
                        c,
                        Face::Atkinson,
                        sm,
                        band.x as f32 + s.px(6.0) as f32,
                        b,
                        &tr(name).to_uppercase(),
                        t.ink,
                    );
                    continue;
                }
                Item::Category(i, sum) => {
                    self.fonts.draw(
                        c,
                        Face::Literata,
                        ui,
                        band.x as f32,
                        base,
                        &tr(CATEGORIES[*i].0),
                        t.ink,
                    );
                    italic(c, sum, t.mute);
                }
                Item::Choice(lab, _, on) => {
                    self.fonts
                        .draw(c, Face::Literata, ui, band.x as f32, base, &tr(lab), t.ink);
                    crate::radio::draw(c, s, cells[0], *on, t);
                }
                Item::Row(d) => {
                    if let Some(note) = &d.note {
                        // Label up, the reason in small print under it.
                        let nb = mid + ui * 1.0;
                        self.fonts.draw(
                            c,
                            Face::Literata,
                            ui,
                            band.x as f32,
                            base - ui * 0.45,
                            &format!("{}:", tr(&d.label)),
                            t.ink,
                        );
                        // Kept to the column left of the controls: smaller,
                        // then cut, so it never runs under them.
                        let room = cells.iter().map(|r| r.x).min().unwrap_or(band.x + band.w)
                            - band.x
                            - s.px(12.0);
                        let text = tr(note).into_owned();
                        let mut size = ui * 0.8;
                        let w = self.fonts.width(Face::Atkinson, size, &text);
                        if w > room as f32 {
                            size = (size * room as f32 / w).max(ui * 0.62);
                        }
                        let mut shown = text;
                        while self.fonts.width(Face::Atkinson, size, &shown) > room as f32
                            && shown.chars().count() > 3
                        {
                            shown.pop();
                            shown = shown.trim_end().to_owned();
                            if self.fonts.width(Face::Atkinson, size, &format!("{shown}…"))
                                <= room as f32
                            {
                                shown.push('…');
                                break;
                            }
                        }
                        self.fonts
                            .draw(c, Face::Atkinson, size, band.x as f32, nb, &shown, t.mute);
                    } else {
                        label(c, &tr(&d.label));
                    }
                    match &d.kind {
                        Kind::Check { on, .. } => {
                            crate::switch::draw(c, s, &self.fonts, ui, cells[0], *on, t);
                        }
                        Kind::Pick(v) => italic(c, &tr(v), t.ink),
                        Kind::Link(_) | Kind::Open(_) => italic(c, &tr("Open"), t.ink),
                        Kind::Info(v) => italic(c, v, t.mute),
                        Kind::Muted(v) => italic(c, v, crate::slider::MUTED),
                        Kind::Slider {
                            stops,
                            value,
                            enabled,
                            ends,
                        } => {
                            let g = self.slider_geom(*band, stops.len());
                            let sl = crate::slider::Slider {
                                stops: stops.iter().map(|x| tr(&x.label).into_owned()).collect(),
                                value: *value,
                                enabled: *enabled,
                                ends: *ends,
                                focused: m.focus == Some(static_label(&d.label)),
                            };
                            let ink = if t.paper < 128 { 0xFF } else { 0x00 };
                            crate::slider::draw(c, &g, &sl, &self.fonts, ui, s, (ink, t.paper));
                        }
                        Kind::Inline(cs) | Kind::Stepper(cs) => {
                            for (r, cell) in cells.iter().zip(cs) {
                                let fg = if cell.change.is_none() {
                                    t.ink
                                } else if cell.active {
                                    c.fill(*r, t.ink);
                                    t.paper
                                } else {
                                    c.stroke(*r, 1, t.rule);
                                    t.ink
                                };
                                let face = if cell.change.is_some() {
                                    Face::AtkinsonBold
                                } else {
                                    Face::Atkinson
                                };
                                let lab = tr(&cell.label);
                                let lw = self.fonts.width(face, ui, &lab);
                                let lb = r.y as f32
                                    + r.h as f32 / 2.0
                                    + self.fonts.ascent(face, ui) / 2.6;
                                self.fonts.draw(
                                    c,
                                    face,
                                    ui,
                                    r.x as f32 + (r.w as f32 - lw) / 2.0,
                                    lb,
                                    &lab,
                                    fg,
                                );
                            }
                        }
                    }
                }
            }
            // Thin separator under a row, not before a band.
            let next_band = l
                .items
                .get(k + 1)
                .is_some_and(|(i, _, _)| matches!(i, Item::Band(_)));
            if k + 1 < n && !next_band {
                c.fill(Rect::new(band.x, band.y + band.h - 1, band.w, 1), t.rule);
            }
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model<'a>(
        st: &'a Settings,
        k: &'a Keyboard,
        page: SettingsPage,
        sub: usize,
    ) -> OptionsModel<'a> {
        OptionsModel {
            settings: st,
            theme: Theme::DAY,
            keyboard: k,
            orientation_status: "Auto (no sensor) — Portrait",
            page,
            sub,
            language: "English",
            version: "1.4.2",
            focus: None,
            hw_samples: 0,
            light: None,
            ui_scale_pending: None,
            plugins: &[],
            repository: &[],
            repo_note: "",
        }
    }

    fn graph_row(enabled: bool) -> PluginRow {
        PluginRow {
            id: "graph-map".into(),
            name: "Graph".into(),
            kind: "Screen",
            version: "1.1.0".into(),
            enabled,
            restart: false,
            compatible: true,
            settings: vec![PluginSetting {
                label: "Start on".into(),
                options: vec!["Map".into(), "List".into()],
                selected: 1,
            }],
        }
    }

    #[test]
    fn a_plugins_settings_row_is_locked_until_it_is_on_and_its_page_encodes_changes() {
        let (st, k) = (Settings::default(), Keyboard::default());
        let kinds = |rows: &[PluginRow], page| -> Vec<(String, Kind)> {
            let m = OptionsModel {
                plugins: rows,
                repository: &[],
                repo_note: "",
                ..model(&st, &k, page, 0)
            };
            page_items(&m)
                .into_iter()
                .filter_map(|i| match i {
                    Item::Row(d) => Some((d.label.to_string(), d.kind)),
                    _ => None,
                })
                .collect()
        };
        let list = SettingsPage::Category(PLUGINS_CATEGORY);
        // On: a switch (to turn it off) and an open row for its page.
        let on = kinds(&[graph_row(true)], list);
        assert_eq!(on.len(), 3, "switch, its settings, Check for plugins");
        assert!(matches!(
            on[0].1,
            Kind::Check {
                on: true,
                change: Change::Plugin(0, false)
            }
        ));
        assert_eq!(on[1].0, "Graph \u{b7} Settings");
        assert_eq!(on[1].1, Kind::Open(SettingsPage::Plugin(0)));
        // Off: the same row, greyed, and not openable.
        let off = kinds(&[graph_row(false)], list);
        assert!(matches!(
            off[0].1,
            Kind::Check {
                on: false,
                change: Change::Plugin(0, true)
            }
        ));
        assert!(matches!(off[1].1, Kind::Muted(_)));
        // Its page: one choice per setting, each cell naming its change.
        let page = kinds(&[graph_row(true)], SettingsPage::Plugin(0));
        let Kind::Inline(cells) = &page[0].1 else {
            panic!("a choice")
        };
        assert_eq!(page[0].0, "Start on");
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].change, Some(Change::PluginSetting(0, 0, 0)));
        assert_eq!(cells[1].change, Some(Change::PluginSetting(0, 0, 1)));
        assert!(
            !cells[0].active && cells[1].active,
            "the chosen option is active"
        );
        // While the plugin is off its page has nothing to set.
        assert!(matches!(
            kinds(&[graph_row(false)], SettingsPage::Plugin(0))[0].1,
            Kind::Info(_)
        ));
        assert_eq!(parent(SettingsPage::Plugin(0)), Some(list));
    }

    /// One bar everywhere: Settings, the editor and Preview draw the same
    /// handle width whatever the UI scale.
    #[test]
    fn the_scroll_bar_is_the_same_size_at_every_ui_scale() {
        let mut want = None;
        for ui in [100, 125, 150, 175, 200] {
            let mut e = EditorScreen::new(1440, 1920);
            let mut st = Settings {
                ui_scale: ui,
                ..Settings::default()
            };
            e.configure(&mut st, None);
            let k = Keyboard::default();
            // Writing has few rows; at big UI scales Settings pages split.
            let l = e.options_layout(&model(&st, &k, SettingsPage::Category(0), 0));
            let page = Rect::new(0, 100, 1440, 1000);
            let (strip, _) = e.scroll_bar(page, 400, 40).unwrap();
            if let Some(crate::scrollbar::Paged { strip: s, .. }) = l.bar {
                assert_eq!(s.w, strip.w, "Settings vs editor at {ui}%");
            }
            assert_eq!(*want.get_or_insert(strip.w), strip.w, "editor at {ui}%");
        }
    }

    fn pages() -> Vec<SettingsPage> {
        let mut v = vec![SettingsPage::Root];
        for c in 0..CATEGORIES.len() {
            v.push(SettingsPage::Category(c));
        }
        for l in ["Keyboard tone", "Tab width", "Layout"] {
            v.push(SettingsPage::Choice(category_of(l).unwrap(), l));
        }
        v
    }

    fn check(w: u32, h: u32) {
        let e = EditorScreen::new(w, h);
        let st = Settings::default();
        let k = Keyboard::default();
        for p in pages() {
            let l = e.options_layout(&model(&st, &k, p, 0));
            for sub in 0..l.pages {
                let m = model(&st, &k, p, sub);
                for (n, r) in e.options_controls(&m) {
                    assert!(r.h >= 44 && r.w >= 44, "{p:?} {n} {r:?}");
                    assert!(
                        r.x >= 0 && r.x + r.w <= w as i32 && r.y + r.h <= h as i32,
                        "{p:?} {n}"
                    );
                }
                let mut c = Canvas::new(w, h, 0);
                let _ = e.draw_options(&mut c, &m);
            }
        }
    }

    #[test]
    fn every_page_fits_in_both_orientations() {
        check(1440, 1920);
        check(1920, 1440);
    }

    #[test]
    fn every_old_row_has_a_home_and_kinds_follow_the_approval() {
        for r in option_rows(&Settings::default()) {
            assert!(
                category_of(r.label).is_some(),
                "{} has no category",
                r.label
            );
        }
        let st = Settings::default();
        let k = Keyboard::default();
        let m = model(&st, &k, SettingsPage::Root, 0);
        let kind = |l| display_row(&m, l).unwrap().kind;
        // Auto has no 180° to pick: greyed until Portrait or Landscape.
        assert!(matches!(kind("Upside down"), Kind::Muted(_)));
        let fixed = Settings {
            orientation: Orientation::Portrait,
            ..Settings::default()
        };
        let mf = model(&fixed, &k, SettingsPage::Root, 0);
        assert!(matches!(
            display_row(&mf, "Upside down").unwrap().kind,
            Kind::Check { .. }
        ));
        assert!(matches!(kind("USB keyboard & mouse"), Kind::Check { .. }));
        assert!(matches!(kind("Line"), Kind::Check { .. }));
        assert!(matches!(kind("Dark mode"), Kind::Check { .. }));
        assert!(matches!(kind("Scroll"), Kind::Inline(_)));
        assert!(matches!(kind("Size"), Kind::Slider { .. }));
        assert!(matches!(kind("Orientation"), Kind::Inline(_)));
        assert!(matches!(kind("Layout"), Kind::Pick(_)));
        assert!(choices(&st, "Layout").len() < sigma_core::layouts::ALL.len());
        let on = Settings {
            experimental: true,
            ..st
        };
        assert_eq!(choices(&on, "Layout").len(), sigma_core::layouts::ALL.len());
    }

    #[test]
    fn taps_open_toggle_pick_and_go_back() {
        let e = EditorScreen::new(1440, 1920);
        let st = Settings::default();
        let k = Keyboard::default();
        let m = model(&st, &k, SettingsPage::Root, 0);
        let (x, y) = e.options_control(&m, "Display").unwrap();
        assert_eq!(
            e.options_hit(&m, x, y),
            OptionsHit::Open(SettingsPage::Category(1))
        );
        let st = Settings {
            orientation: Orientation::Portrait,
            ..st
        };
        let m = model(&st, &k, SettingsPage::Category(1), 0);
        let (x, y) = e.options_control(&m, "Upside down/On").unwrap();
        assert_eq!(
            e.options_hit(&m, x, y),
            OptionsHit::Change(Change::UpsideDown(true))
        );
        let (x, y) = e.options_control(&m, "Upside down/Off").unwrap();
        assert_eq!(e.options_hit(&m, x, y), OptionsHit::Nothing, "already off");
        // v1.6.3: Orientation is three buttons in the row, no sub-page.
        let (x, y) = e.options_control(&m, "Orientation/Landscape").unwrap();
        assert_eq!(
            e.options_hit(&m, x, y),
            OptionsHit::Change(Change::Orientation(Orientation::Landscape))
        );
        let p = SettingsPage::Choice(1, "Keyboard tone");
        let m = model(&st, &k, p, 0);
        let (x, y) = e.options_control(&m, "Back").unwrap();
        assert_eq!(e.options_hit(&m, x, y), OptionsHit::Back);
        assert_eq!(e.options_step(&m, "Dark mode/On").as_deref(), Some("Back"));
        let m = model(&st, &k, SettingsPage::Root, 0);
        assert_eq!(
            e.options_step(&m, "Layout/+").as_deref(),
            Some("Keyboard & input")
        );
    }
}
