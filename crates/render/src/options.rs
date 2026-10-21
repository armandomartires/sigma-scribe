//! The Options sheet (brief §6): a full-page overlay of rows, each with a
//! short label, a smaller hint and its controls (segmented choices or a
//! stepper). A tap returns the [`Change`] it stands for; the app applies it,
//! refits Line mode and saves `.sigma-settings`.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
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
            label: "USB keyboard",
            hint: "Needs a powered hub: the reader does not power the port",
            cells: vec![
                choice("Off", Change::Usb(false), !s.usb),
                choice("On", Change::Usb(true), s.usb),
            ],
        },
        OptRow {
            label: "Orientation",
            hint: "Turn the page sideways",
            cells: vec![
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
    ]
}

pub struct OptionsModel<'a> {
    pub settings: &'a Settings,
    pub theme: Theme,
    pub keyboard: &'a Keyboard,
}

#[derive(Clone, Debug, Default)]
pub struct OptionsLayout {
    pub base: Layout,
    pub close: Rect,
    /// Per row: the row band and its cells.
    pub rows: Vec<(Rect, Vec<Rect>)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptionsHit {
    Close,
    Change(Change),
    Nothing,
}

impl EditorScreen {
    fn cell_w(&self, label: &str) -> i32 {
        let s = self.scale();
        (self.fonts.width(Face::AtkinsonBold, self.ui_px(), label) as i32 + 2 * s.px(14.0))
            .max(s.px(60.0))
    }

    #[must_use]
    pub fn options_layout(&self, m: &OptionsModel<'_>) -> OptionsLayout {
        let base = self.layout(false, m.keyboard);
        let s = self.scale();
        let w = self.width as i32;
        let rows = option_rows(m.settings);
        let top = base.title.h + s.px(8.0);
        let avail = self.height as i32 - top - s.px(8.0);
        let row_h = (avail / rows.len() as i32).min(s.px(96.0));
        let btn = s.px(44.0).min(row_h - s.px(8.0));
        let pad = s.px(20.0);
        let gap = s.px(6.0);
        let mut out = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            let band = Rect::new(0, top + i as i32 * row_h, w, row_h);
            let widths: Vec<i32> = r.cells.iter().map(|c| self.cell_w(&c.label)).collect();
            let total: i32 = widths.iter().sum::<i32>() + gap * (widths.len() as i32 - 1);
            let mut x = w - pad - total;
            let y = band.y + (row_h - btn) / 2;
            let cells = widths
                .iter()
                .map(|&cw| {
                    let r = Rect::new(x, y, cw, btn);
                    x += cw + gap;
                    r
                })
                .collect();
            out.push((band, cells));
        }
        OptionsLayout {
            close: base.close_button,
            base,
            rows: out,
        }
    }

    #[must_use]
    pub fn options_hit(&self, m: &OptionsModel<'_>, x: i32, y: i32) -> OptionsHit {
        let l = self.options_layout(m);
        if l.close.inset(-self.scale().px(4.0)).contains(x, y) {
            return OptionsHit::Close;
        }
        let rows = option_rows(m.settings);
        for ((_, cells), row) in l.rows.iter().zip(&rows) {
            for (r, cell) in cells.iter().zip(&row.cells) {
                if r.inset(-self.scale().px(3.0) / 2).contains(x, y) {
                    return cell.change.map_or(OptionsHit::Nothing, OptionsHit::Change);
                }
            }
        }
        OptionsHit::Nothing
    }

    /// Centre of a control, as `Row/Label` (emulator scripts), e.g.
    /// `Paper/Night`, `Size/+`, `Line/On`.
    #[must_use]
    pub fn options_control(&self, m: &OptionsModel<'_>, name: &str) -> Option<(i32, i32)> {
        let l = self.options_layout(m);
        if name == "Close" {
            return Some((l.close.x + l.close.w / 2, l.close.y + l.close.h / 2));
        }
        let (row, cell) = name.split_once('/')?;
        let cell = if cell == "-" { "\u{2212}" } else { cell };
        let rows = option_rows(m.settings);
        let i = rows.iter().position(|r| r.label == row)?;
        let j = rows[i].cells.iter().position(|c| c.label == cell)?;
        let r = l.rows[i].1[j];
        Some((r.x + r.w / 2, r.y + r.h / 2))
    }

    pub fn draw_options(&self, c: &mut Canvas, m: &OptionsModel<'_>) -> OptionsLayout {
        let t = m.theme;
        let l = self.options_layout(m);
        let s = self.scale();
        let ui = self.ui_px();
        c.fill(c.bounds(), t.paper);
        self.draw_title(c, &l.base, "Options", None, t, false);
        let r = l.close;
        let (cx, cy, d) = (r.x + r.w / 2, r.y + r.h / 2, s.px(16.0));
        for k in -d..=d {
            for th in 0..s.px(3.5).max(3) {
                c.blend(cx + k, cy + k + th, t.ink, 255);
                c.blend(cx + k, cy - k + th, t.ink, 255);
            }
        }
        let rows = option_rows(m.settings);
        let small = ui * 0.82;
        for ((band, cells), row) in l.rows.iter().zip(&rows) {
            let lx = s.px(20.0) as f32;
            let label_base = band.y as f32 + band.h as f32 * 0.44;
            self.fonts
                .draw(c, Face::AtkinsonBold, ui, lx, label_base, row.label, t.ink);
            self.fonts.draw(
                c,
                Face::Atkinson,
                small,
                lx,
                label_base + small * 1.35,
                row.hint,
                t.mute,
            );
            for (r, cell) in cells.iter().zip(&row.cells) {
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
                let w = self.fonts.width(face, ui, &cell.label);
                let b = r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, ui) / 2.6;
                self.fonts.draw(
                    c,
                    face,
                    ui,
                    r.x as f32 + (r.w as f32 - w) / 2.0,
                    b,
                    &cell.label,
                    fg,
                );
            }
            c.fill(Rect::new(0, band.y + band.h - 1, band.w, 1), t.rule);
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(w: u32, h: u32) {
        let e = EditorScreen::new(w, h);
        let st = Settings::default();
        let k = Keyboard::default();
        let m = OptionsModel {
            settings: &st,
            theme: Theme::DAY,
            keyboard: &k,
        };
        let l = e.options_layout(&m);
        let rows = option_rows(&st);
        assert_eq!(l.rows.len(), rows.len());
        for ((band, cells), row) in l.rows.iter().zip(&rows) {
            assert!(band.y + band.h <= h as i32, "{} off screen", row.label);
            let label_w = e.fonts.width(Face::Atkinson, e.ui_px() * 0.82, row.hint) as i32;
            for (r, cell) in cells.iter().zip(&row.cells) {
                assert!(r.h >= 44 && r.w >= 44, "{} target {r:?}", row.label);
                assert!(r.x > label_w / 2 && r.x + r.w <= w as i32, "{r:?}");
                let hit = e.options_hit(&m, r.x + r.w / 2, r.y + r.h / 2);
                match cell.change {
                    Some(c) => assert_eq!(hit, OptionsHit::Change(c)),
                    None => assert_eq!(hit, OptionsHit::Nothing),
                }
            }
        }
        let mut c = Canvas::new(w, h, 0);
        let _ = e.draw_options(&mut c, &m);
        assert_eq!(
            e.options_hit(&m, l.close.x + 5, l.close.y + 5),
            OptionsHit::Close
        );
        assert!(e.options_control(&m, "Paper/Night").is_some());
        assert!(e.options_control(&m, "Size/-").is_some());
    }

    #[test]
    fn every_control_fits_and_hits_in_both_orientations() {
        check(1440, 1920);
        check(1920, 1440);
    }

    #[test]
    fn rows_follow_the_brief() {
        let labels: Vec<_> = option_rows(&Settings::default())
            .iter()
            .map(|r| r.label)
            .collect();
        assert_eq!(
            labels,
            [
                "Paper",
                "Type",
                "Line",
                "Size",
                "Leading",
                "Margin",
                "Tab key",
                "Tab width",
                "Keyboard",
                "Keyboard tone",
                "Drawer",
                "USB keyboard",
                "Orientation"
            ]
        );
    }
}
