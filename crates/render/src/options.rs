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
use sigma_core::settings::{Change, KeyTone, Orientation, Paper, Settings, Side, Typeface};
use sigma_core::Keyboard;

/// One control in a row: a button (with the change it makes) or a value.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub label: String,
    pub change: Option<Change>,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptRow {
    pub label: &'static str,
    pub hint: &'static str,
    pub cells: Vec<Cell>,
}

fn choice(label: &str, change: Change, active: bool) -> Cell {
    Cell {
        label: label.to_owned(),
        change: Some(change),
        active,
    }
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

fn num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.1}")
    }
}

/// The rows, in the brief's order, plus Keyboard layout and Orientation.
#[must_use]
pub fn option_rows(s: &Settings) -> Vec<OptRow> {
    let mut line = vec![
        choice("Off", Change::LineOn(false), !s.line_on),
        choice("On", Change::LineOn(true), s.line_on),
    ];
    line.extend(stepper(
        format!("{} words", s.line_words),
        Change::LineWords(-1),
        Change::LineWords(1),
    ));
    vec![
        OptRow {
            label: "UI scale",
            hint: "Buttons, menus and keys; the note keeps its Size",
            cells: stepper(
                format!("{} %", s.ui_scale),
                Change::UiScale(-1),
                Change::UiScale(1),
            ),
        },
        OptRow {
            label: "Paper",
            hint: "Night is the same page, inverted",
            cells: vec![
                choice("Day", Change::Paper(Paper::Day), s.paper == Paper::Day),
                choice(
                    "Night",
                    Change::Paper(Paper::Night),
                    s.paper == Paper::Night,
                ),
            ],
        },
        OptRow {
            label: "Type",
            hint: "Serif for reading, sans for the bus",
            cells: vec![
                choice(
                    "Literata",
                    Change::Face(Typeface::Literata),
                    s.face == Typeface::Literata,
                ),
                choice(
                    "Atkinson",
                    Change::Face(Typeface::Atkinson),
                    s.face == Typeface::Atkinson,
                ),
            ],
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
            cells: stepper(num(s.size), Change::Size(-1), Change::Size(1)),
        },
        OptRow {
            label: "Leading",
            hint: "Space between lines",
            cells: stepper(
                format!("{:.1}", s.leading),
                Change::Leading(-1),
                Change::Leading(1),
            ),
        },
        OptRow {
            label: "Margin",
            hint: "Room for a thumb",
            cells: stepper(num(s.margin), Change::Margin(-1), Change::Margin(1)),
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
            label: "Keyboard tone",
            hint: "The keyboard can differ from the page",
            cells: vec![
                choice(
                    "Theme",
                    Change::KeyTone(KeyTone::Theme),
                    s.key_tone == KeyTone::Theme,
                ),
                choice(
                    "Light",
                    Change::KeyTone(KeyTone::Light),
                    s.key_tone == KeyTone::Light,
                ),
                choice(
                    "Dark",
                    Change::KeyTone(KeyTone::Dark),
                    s.key_tone == KeyTone::Dark,
                ),
            ],
        },
        OptRow {
            label: "Drawer",
            hint: "Contents, to-dos and scratch open here (v0.5)",
            cells: vec![
                choice("Left", Change::Drawer(Side::Left), s.drawer == Side::Left),
                choice(
                    "Right",
                    Change::Drawer(Side::Right),
                    s.drawer == Side::Right,
                ),
            ],
        },
        OptRow {
            label: "USB keyboard & mouse",
            hint: "Needs a powered hub: the reader does not power the port",
            cells: vec![
                choice("Off", Change::Usb(false), !s.usb),
                choice("On", Change::Usb(true), s.usb),
            ],
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
            cells: vec![
                choice(
                    "Auto",
                    Change::Orientation(Orientation::Auto),
                    s.orientation == Orientation::Auto,
                ),
                choice(
                    "Portrait",
                    Change::Orientation(Orientation::Portrait),
                    s.orientation == Orientation::Portrait,
                ),
                choice(
                    "Landscape",
                    Change::Orientation(Orientation::Landscape),
                    s.orientation == Orientation::Landscape,
                ),
            ],
        },
        OptRow {
            label: "Upside down",
            hint: "Turns Portrait or Landscape 180°",
            cells: vec![
                choice("Off", Change::UpsideDown(false), !s.upside_down),
                choice("On", Change::UpsideDown(true), s.upside_down),
            ],
        },
    ]
}

/// Where Settings is: the category list, one category, or a choice list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Root,
    Category(usize),
    /// A row's choice list, inside a category.
    Choice(usize, &'static str),
}

/// Categories: name, then (section heading, row labels).
/// A section: heading and row labels.
pub type Section = (&'static str, &'static [&'static str]);

pub const CATEGORIES: [(&str, &[Section]); 6] = [
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
            ("Page", &["Paper", "UI scale", "Drawer"]),
            ("Orientation", &["Orientation", "Upside down"]),
        ],
    ),
    (
        "Keyboard & input",
        &[(
            "On-screen keyboard",
            &["Keyboard", "Layout", "Keyboard tone"],
        )],
    ),
    ("Language", &[("Interface", &["Interface language"])]),
    (
        "USB & devices",
        &[("USB keyboard & mouse", &["USB keyboard & mouse", "Scroll"])],
    ),
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

/// How a row is shown and what a tap does.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// Checkbox + "On": tap toggles.
    Check { on: bool, change: Change },
    /// Two-way choice: inline buttons.
    Inline(Vec<Cell>),
    /// − value +.
    Stepper(Vec<Cell>),
    /// Current value in italics; tap opens the choice list.
    Pick(String),
    /// Opens a Help page (index into `help::MENU`).
    Link(usize),
    /// Read-only value.
    Info(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DRow {
    pub label: &'static str,
    pub kind: Kind,
}

pub struct OptionsModel<'a> {
    pub settings: &'a Settings,
    pub theme: Theme,
    pub keyboard: &'a Keyboard,
    /// What Orientation is doing (v1.2), e.g. "Auto (no sensor) — Portrait".
    pub orientation_status: &'a str,
    pub page: SettingsPage,
    /// Sub-page when a page has more rows than fit.
    pub sub: usize,
    /// Interface language, as shown (e.g. "English").
    pub language: &'a str,
    pub version: &'a str,
}

/// One drawn line of a page: a section band or a row.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Band(&'static str),
    Row(DRow),
    /// A category on the root page, with its summary.
    Category(usize, String),
    /// A choice on a choice page.
    Choice(String, Change, bool),
}

#[derive(Clone, Debug, Default)]
pub struct OptionsLayout {
    pub base: Layout,
    pub close: Rect,
    /// Back arrow (not on the root page).
    pub back: Option<Rect>,
    /// "Page i of n" (tap: next page), when there is more than one.
    pub pager: Option<Rect>,
    pub pages: usize,
    /// Big page title baseline.
    pub heading_y: i32,
    /// Items on the current sub-page: the item, its band, its controls.
    pub items: Vec<(Item, Rect, Vec<Rect>)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptionsHit {
    Close,
    Back,
    /// Next sub-page.
    Page,
    Open(SettingsPage),
    Change(Change),
    /// Choose a value and go back to the category.
    Pick(Change),
    Help(usize),
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

/// The rows of every category, by label.
#[must_use]
pub fn display_row(m: &OptionsModel<'_>, label: &'static str) -> Option<DRow> {
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
        "Words per line" => {
            let r = rows.iter().find(|r| r.label == "Line")?;
            Kind::Stepper(r.cells[2..].to_vec())
        }
        "Layout" => Kind::Pick(m.settings.kb_layout.label().to_owned()),
        // Auto shows what it is doing, e.g. "Auto (no sensor) — Portrait".
        "Orientation" if m.settings.orientation == Orientation::Auto => {
            Kind::Pick(m.orientation_status.to_owned())
        }
        "Interface language" => Kind::Info(m.language.to_owned()),
        "App info" => Kind::Link(0),
        "Markdown reference" => Kind::Link(1),
        "USB keyboard, mouse & export" => Kind::Link(2),
        _ => row_kind(rows.iter().find(|r| r.label == label)?),
    };
    Some(DRow { label, kind })
}

/// The choices of a Pick row: label, change, active.
#[must_use]
pub fn choices(s: &Settings, label: &str) -> Vec<(String, Change, bool)> {
    if label == "Layout" {
        use sigma_core::layouts::ALL;
        let cur = ALL.iter().position(|&l| l == s.kb_layout).unwrap_or(0) as i32;
        return ALL
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
        0 => format!(
            "{}, {}",
            tr(match s.face {
                Typeface::Literata => "Literata",
                Typeface::Atkinson => "Atkinson",
            }),
            num(s.size)
        ),
        1 => format!(
            "{}, {} %",
            tr(if s.paper == Paper::Night {
                "Night"
            } else {
                "Day"
            }),
            s.ui_scale
        ),
        2 => format!(
            "{}, {}",
            tr(if s.keyboard == KbLayout::Full {
                "Full"
            } else {
                "Compact"
            }),
            tr(s.kb_layout.label())
        ),
        3 => m.language.to_owned(),
        4 => on_off(s.usb),
        _ => m.version.to_owned(),
    }
}

/// Every item of a page, before splitting into sub-pages.
#[must_use]
pub fn page_items(m: &OptionsModel<'_>) -> Vec<Item> {
    match m.page {
        SettingsPage::Root => (0..CATEGORIES.len())
            .map(|i| Item::Category(i, summary(m, i)))
            .collect(),
        SettingsPage::Category(c) => {
            let mut out = Vec::new();
            for (band, rows) in CATEGORIES[c.min(CATEGORIES.len() - 1)].1 {
                out.push(Item::Band(band));
                for r in *rows {
                    if let Some(d) = display_row(m, r) {
                        out.push(Item::Row(d));
                    }
                }
            }
            out
        }
        SettingsPage::Choice(_, label) => choices(m.settings, label)
            .into_iter()
            .map(|(l, c, a)| Item::Choice(l, c, a))
            .collect(),
    }
}

/// The page title.
#[must_use]
pub fn page_title(p: SettingsPage) -> &'static str {
    match p {
        SettingsPage::Root => "Settings",
        SettingsPage::Category(c) => CATEGORIES[c.min(CATEGORIES.len() - 1)].0,
        SettingsPage::Choice(_, l) => l,
    }
}

/// The page above.
#[must_use]
pub fn parent(p: SettingsPage) -> Option<SettingsPage> {
    match p {
        SettingsPage::Root => None,
        SettingsPage::Category(_) => Some(SettingsPage::Root),
        SettingsPage::Choice(c, _) => Some(SettingsPage::Category(c)),
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

    fn check_box(&self, band: Rect) -> Rect {
        // Box + "On" read as one target, Kobo style.
        let s = self.scale();
        let w = s.px(44.0)
            + self
                .fonts
                .width(Face::LiterataItalic, self.ui_px(), &tr("On")) as i32
            + s.px(8.0);
        Rect::new(
            band.x + band.w - s.px(20.0) - w,
            band.y + (band.h - s.px(44.0)) / 2,
            w,
            s.px(44.0),
        )
    }

    #[must_use]
    pub fn options_layout(&self, m: &OptionsModel<'_>) -> OptionsLayout {
        let base = self.layout(false, m.keyboard);
        let s = self.scale();
        let w = self.width as i32;
        let pad = s.px(20.0);
        let back = (m.page != SettingsPage::Root).then(|| {
            Rect::new(
                s.px(4.0),
                base.close_button.y,
                base.close_button.w,
                base.close_button.h,
            )
        });
        let heading_y = base.title.h + s.px(40.0);
        let top = heading_y + s.px(16.0);
        let row_h = s.px(52.0);
        let band_h = s.px(24.0);
        let bottom = self.height as i32 - s.px(12.0);
        // Split into sub-pages; a band never ends a page.
        let all = page_items(m);
        let mut pages: Vec<Vec<Item>> = vec![Vec::new()];
        let mut y = top;
        for (i, it) in all.iter().enumerate() {
            let h = if matches!(it, Item::Band(_)) {
                band_h
            } else {
                row_h
            };
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
            let h = if matches!(it, Item::Band(_)) {
                band_h
            } else {
                row_h
            };
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
                    kind: Kind::Check { .. },
                    ..
                })
                | Item::Choice(..) => vec![self.check_box(band)],
                Item::Row(DRow {
                    kind: Kind::Pick(_) | Kind::Link(_),
                    ..
                })
                | Item::Category(..) => vec![band],
                _ => vec![],
            };
            items.push((it, band, cells));
            y += h;
        }
        let pager = (n > 1).then(|| {
            let pw = self
                .fonts
                .width(Face::LiterataItalic, self.ui_px() * 0.9, "Page 00 of 00")
                as i32;
            Rect::new(w - pad - pw, heading_y - s.px(30.0), pw, s.px(44.0))
        });
        OptionsLayout {
            close: base.close_button,
            base,
            back,
            pager,
            pages: n,
            heading_y,
            items,
        }
    }

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
        for (it, _, cells) in &l.items {
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
                                OptionsHit::Open(SettingsPage::Choice(c, d.label))
                            }
                            _ => OptionsHit::Nothing,
                        },
                        Kind::Link(i) => OptionsHit::Help(*i),
                        Kind::Info(_) => OptionsHit::Nothing,
                    },
                    Item::Band(_) => OptionsHit::Nothing,
                };
            }
        }
        OptionsHit::Nothing
    }

    /// Named controls on the current page (emulator, touch audit):
    /// `Close`, `Back`, `Page`, category names, `Row/Cell` for buttons and
    /// steppers (`Paper/Night`, `Size/+`), `Row/On` / `Row/Off` for
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
                    Kind::Pick(_) | Kind::Link(_) => out.push((d.label.into(), cells[0])),
                    Kind::Info(_) => {}
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
            SettingsPage::Choice(..) => Some("Back".into()),
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

    pub fn draw_options(&self, c: &mut Canvas, m: &OptionsModel<'_>) -> OptionsLayout {
        let t = m.theme;
        let l = self.options_layout(m);
        let s = self.scale();
        let ui = self.ui_px();
        let w = self.width as i32;
        c.fill(c.bounds(), t.paper);
        // Top bar: back arrow, small centred "Settings", close; no rule
        // (Kobo Forma 4.x look).
        let bar_base = l.base.title.y as f32
            + l.base.title.h as f32 / 2.0
            + self.fonts.ascent(Face::Literata, ui) / 2.6;
        let top = tr("Settings");
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
        crate::icons::draw_icon(c, crate::icons::Icon::Close, l.close, t.ink);
        if let Some(b) = l.back {
            crate::icons::draw_icon(c, crate::icons::Icon::Back, b, t.ink);
        }
        let pad = s.px(20.0) as f32;
        self.fonts.draw(
            c,
            Face::Literata,
            ui * 1.5,
            pad,
            l.heading_y as f32,
            &tr(page_title(m.page)),
            t.ink,
        );
        if let Some(p) = l.pager {
            let txt = tr(&format!(
                "Page {} of {}",
                m.sub.min(l.pages - 1) + 1,
                l.pages
            ))
            .into_owned();
            let pw = self.fonts.width(Face::LiterataItalic, ui * 0.9, &txt);
            self.fonts.draw(
                c,
                Face::LiterataItalic,
                ui * 0.9,
                (p.x + p.w) as f32 - pw,
                l.heading_y as f32,
                &txt,
                t.ink,
            );
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
                    self.draw_check(c, cells[0], *on, false, t);
                }
                Item::Row(d) => {
                    label(c, &tr(d.label));
                    match &d.kind {
                        Kind::Check { on, .. } => self.draw_check(c, cells[0], *on, true, t),
                        Kind::Pick(v) => italic(c, &tr(v), t.ink),
                        Kind::Link(_) => italic(c, &tr("Open"), t.ink),
                        Kind::Info(v) => italic(c, v, t.mute),
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

    /// Kobo-style checkbox: an outlined square, ticked when on, and an
    /// italic "On" beside it (`with_on`).
    fn draw_check(&self, c: &mut Canvas, r: Rect, on: bool, with_on: bool, t: Theme) {
        let s = self.scale();
        let ui = self.ui_px();
        let b = s.px(20.0);
        let on_w = if with_on {
            self.fonts.width(Face::LiterataItalic, ui, &tr("On")) as i32 + s.px(8.0)
        } else {
            0
        };
        let bx = r.x + r.w - on_w - b - s.px(4.0);
        let by = r.y + (r.h - b) / 2;
        let bw = s.px(1.5).max(1);
        c.stroke(Rect::new(bx, by, b, b), bw, t.ink);
        if on {
            let i = s.px(4.0);
            c.fill(Rect::new(bx + i, by + i, b - 2 * i, b - 2 * i), t.ink);
        }
        if with_on {
            let base =
                r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(Face::LiterataItalic, ui) / 2.6;
            self.fonts.draw(
                c,
                Face::LiterataItalic,
                ui,
                (r.x + r.w - on_w + s.px(8.0)) as f32,
                base,
                &tr("On"),
                t.ink,
            );
        }
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
        }
    }

    fn pages() -> Vec<SettingsPage> {
        let mut v = vec![SettingsPage::Root];
        for c in 0..CATEGORIES.len() {
            v.push(SettingsPage::Category(c));
        }
        for l in ["Orientation", "Keyboard tone", "Tab width", "Layout"] {
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
        assert!(matches!(kind("Upside down"), Kind::Check { .. }));
        assert!(matches!(kind("USB keyboard & mouse"), Kind::Check { .. }));
        assert!(matches!(kind("Line"), Kind::Check { .. }));
        assert!(matches!(kind("Paper"), Kind::Inline(_)));
        assert!(matches!(kind("Scroll"), Kind::Inline(_)));
        assert!(matches!(kind("Size"), Kind::Stepper(_)));
        assert!(matches!(kind("Orientation"), Kind::Pick(_)));
        assert!(matches!(kind("Layout"), Kind::Pick(_)));
        assert_eq!(choices(&st, "Layout").len(), sigma_core::layouts::ALL.len());
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
        let m = model(&st, &k, SettingsPage::Category(1), 0);
        let (x, y) = e.options_control(&m, "Upside down/On").unwrap();
        assert_eq!(
            e.options_hit(&m, x, y),
            OptionsHit::Change(Change::UpsideDown(true))
        );
        let (x, y) = e.options_control(&m, "Upside down/Off").unwrap();
        assert_eq!(e.options_hit(&m, x, y), OptionsHit::Nothing, "already off");
        let (x, y) = e.options_control(&m, "Orientation").unwrap();
        let p = SettingsPage::Choice(1, "Orientation");
        assert_eq!(e.options_hit(&m, x, y), OptionsHit::Open(p));
        let m = model(&st, &k, p, 0);
        let (x, y) = e.options_control(&m, "Orientation/Landscape").unwrap();
        assert_eq!(
            e.options_hit(&m, x, y),
            OptionsHit::Pick(Change::Orientation(Orientation::Landscape))
        );
        let (x, y) = e.options_control(&m, "Back").unwrap();
        assert_eq!(e.options_hit(&m, x, y), OptionsHit::Back);
        assert_eq!(e.options_step(&m, "Paper/Night").as_deref(), Some("Back"));
        let m = model(&st, &k, SettingsPage::Root, 0);
        assert_eq!(
            e.options_step(&m, "Layout/+").as_deref(),
            Some("Keyboard & input")
        );
    }
}
