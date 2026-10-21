//! The v0.1 screen (brief §3, §4, §7): title bar, page, status bar,
//! on-screen keyboard. Layout, drawing and hit-testing live together so a tap
//! always lands on what was drawn.

use crate::canvas::{Canvas, Rect};
use crate::fonts::{Face, Fonts, Measure};
use crate::preview::{self, Doc, Type};
use crate::theme::{Scale, Theme};
use sigma_core::keyboard::{Dir, Key};
use sigma_core::linefit::{self, Anchor, Fit, Limits, SizedMeasure};
use sigma_core::settings::{KeyTone, LandKeys, Pad, Side, Typeface, MARGIN_RANGE, SIZE_RANGE};
use sigma_core::wrap::{caret_cell, offset_at, wrap, VisualLine};
use sigma_core::Settings;
use sigma_core::{Buffer, Keyboard, SaveState};

/// What the screen shows; owned by the app.
pub struct Model<'a> {
    pub file_name: &'a str,
    pub buffer: &'a Buffer,
    pub keyboard: &'a Keyboard,
    pub keys_on: bool,
    pub save: &'a SaveState,
    pub theme: Theme,
    /// Name field being edited in the title bar (rename in place).
    pub rename: Option<&'a Buffer>,
    /// Touch selection chrome (handles and action bar), v0.2.2.
    pub sel_ui: Option<SelUi>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SelUi {
    /// Show the action bar (hidden while a handle is dragged).
    pub bar: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarAction {
    Cut,
    Copy,
    Paste,
    SelectAll,
}

impl BarAction {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::SelectAll => "Select all",
        }
    }
}

/// Where the selection chrome sits on screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelGeom {
    /// Touch targets of the start / end handles (None when off-screen or
    /// when there is no selection).
    pub handles: [Option<Rect>; 2],
    pub bar: Option<Rect>,
    pub buttons: Vec<(Rect, BarAction)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Key(Key),
    ToggleKeys,
    /// Byte offset in the buffer where the caret should go.
    Caret(usize),
    /// Selection handle 0 = start, 1 = end.
    Handle(usize),
    Bar(BarAction),
    /// Folder icon: open the Files sheet.
    Files,
    /// Eye (write mode) or pencil (preview): toggle preview, v0.3.
    TogglePreview,
    /// Tap on the file name: rename in place.
    StartRename,
    /// "Aa" in the title bar: the Options sheet (v0.4).
    Options,
    /// Magnifying glass: the Find bar (v0.6).
    Find,
    /// Connected nodes: the Graph sheet (v0.6).
    Graph,
    /// Contents / To-dos / Scratch icon: toggle that drawer pane (v0.5).
    Drawer(crate::drawer::Pane),
    /// Landscape keyboard bar: Split / Full / Centre, pad toggles (v0.4).
    KbButton(KbButton),
    RenameOk,
    RenameCancel,
    Nothing,
}

/// Keys and landscape bar buttons with their rectangles.
type Board = (Vec<(Rect, Key)>, Vec<(Rect, KbButton)>);

/// Buttons in the landscape keyboard bar (brief §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KbButton {
    Mode(LandKeys),
    /// Arrows ⇄ 123 pad (Full only).
    PadToggle,
    /// Pad left ⇄ Pad right (Full only).
    PadSide,
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub title: Rect,
    pub page: Rect,
    pub status: Rect,
    pub keys_button: Rect,
    pub keyboard: Rect,
    pub keys: Vec<(Rect, Key)>,
    pub files_button: Rect,
    /// Eye / pencil, left of the folder (brief §3 order: Files, Preview).
    pub preview_button: Rect,
    pub name_rect: Rect,
    pub ok_rect: Rect,
    pub cancel_rect: Rect,
    /// "Aa" Options button, left of the drawer icons.
    pub options_button: Rect,
    /// Drawer pane icons, right to left after the eye: Scratch, To-dos,
    /// Contents (brief §3 order, Find comes in S6).
    pub scratch_button: Rect,
    pub todos_button: Rect,
    pub contents_button: Rect,
    /// Find (between the eye and Scratch) and Graph (between Contents and
    /// Options), brief §3 order (v0.6).
    pub find_button: Rect,
    pub graph_button: Rect,
    /// The Find bar under the title bar when open (zero height otherwise).
    pub find_bar: Rect,
    /// The text column inside the page: the page itself, or the locked
    /// portrait-width column (Line on, landscape) with gutters beside it.
    pub column: Rect,
    pub kb_buttons: Vec<(Rect, KbButton)>,
}

pub struct EditorScreen {
    pub width: u32,
    pub height: u32,
    pub fonts: Fonts,
    pub scroll: usize,
    s: Scale,
    // Typography defaults from the brief: 20 px, leading 1.5, margin 28 px.
    pub body_px: f32,
    pub leading: f32,
    pub margin_px: f32,
    /// Page face (Options → Type).
    pub face: Face,
    /// Keyboard colours when they differ from the page (Options → Keyboard tone).
    pub key_theme: Option<Theme>,
    /// The settings in force (landscape keyboard, Line, USB flag).
    pub cfg: Settings,
    /// Open drawer panes (Contents, To-dos, Scratch): their icons draw active.
    pub drawer_open: [bool; 3],
    /// The Find bar is open: it sits under the title bar and the page
    /// starts below it; the magnifier draws active.
    pub find_open: bool,
}

/// Real-face measurement for Line mode.
struct FaceMeasure<'a> {
    fonts: &'a Fonts,
    face: Face,
}

impl SizedMeasure for FaceMeasure<'_> {
    fn width_at(&self, size: f32, text: &str) -> f32 {
        self.fonts.width(self.face, size, text)
    }
}

impl EditorScreen {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            fonts: Fonts::new(),
            scroll: 0,
            s: Scale::for_width(width.min(height)),
            body_px: 20.0,
            leading: 1.5,
            margin_px: 28.0,
            face: Face::Literata,
            key_theme: None,
            cfg: Settings::default(),
            drawer_open: [false; 3],
            find_open: false,
        }
    }

    /// Scrolls so logical line `line` is the top row and puts the caret at
    /// its start (Contents jump, to-do open).
    pub fn jump_to_line(&mut self, buffer: &mut Buffer, page: Rect, line: usize) {
        let at = sigma_core::outline::line_range(buffer.text(), line).map_or(0, |r| r.0);
        buffer.set_caret(at);
        let rows = self.rows(buffer, page);
        self.scroll = rows.iter().position(|r| r.start == at).unwrap_or(0);
    }

    /// New panel size (orientation change). The scale follows the short
    /// side, so type and keys keep their size when the device turns.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.s = Scale::for_width(width.min(height));
    }

    #[must_use]
    pub fn landscape(&self) -> bool {
        self.width > self.height
    }

    /// Applies settings. With Line on, fits size and margin to the
    /// portrait-width column with the real face and writes the result back
    /// into `st` (brief §4: Size moves Margin, Margin moves Size).
    pub fn configure(&mut self, st: &mut Settings, anchor: Option<Anchor>) {
        self.face = match st.face {
            Typeface::Literata => Face::Literata,
            Typeface::Atkinson => Face::Atkinson,
        };
        self.key_theme = match st.key_tone {
            KeyTone::Theme => None,
            KeyTone::Light => Some(Theme::DAY),
            KeyTone::Dark => Some(Theme::NIGHT),
        };
        self.leading = st.leading;
        if st.line_on {
            let k = self.s.0;
            let column = self.width.min(self.height) as f32;
            let f = linefit::fit(
                &FaceMeasure {
                    fonts: &self.fonts,
                    face: self.face,
                },
                column,
                st.line_words,
                Fit {
                    size: st.size * k,
                    margin: st.margin * k,
                },
                anchor.unwrap_or(Anchor::Size),
                Limits {
                    size: (SIZE_RANGE.0 * k, SIZE_RANGE.1 * k),
                    margin: (MARGIN_RANGE.0 * k, MARGIN_RANGE.1 * k),
                },
            );
            self.body_px = f.size / k;
            self.margin_px = f.margin / k;
            st.size = (self.body_px * 10.0).round() / 10.0;
            st.margin = (self.margin_px * 10.0).round() / 10.0;
        } else {
            self.body_px = st.size;
            self.margin_px = st.margin;
        }
        self.cfg = *st;
    }

    /// Words on the first row of the Line sample at the current settings
    /// (Options shows it; tests check it).
    #[must_use]
    pub fn sample_words_per_row(&self, page: Rect) -> usize {
        let text = format!("{} {}", linefit::SAMPLE, linefit::SAMPLE);
        let rows = wrap(&text, self.text_area(page).w as u32, &self.measure());
        text[rows[0].start..rows[0].end].split_whitespace().count()
    }

    fn px(&self, v: f32) -> i32 {
        self.s.px(v)
    }

    fn body(&self) -> f32 {
        self.body_px * self.s.0
    }

    fn line_h(&self) -> i32 {
        (self.body() * self.leading).round() as i32
    }

    fn measure(&self) -> Measure<'_> {
        Measure {
            fonts: &self.fonts,
            face: self.face,
            size: self.body(),
        }
    }

    #[must_use]
    pub fn layout(&self, keys_on: bool, keyboard: &Keyboard) -> Layout {
        let (w, h) = (self.width as i32, self.height as i32);
        let title = Rect::new(0, 0, w, self.px(52.0).max(self.px(44.0) + 2 * self.px(4.0)));
        let key_h = self.px(58.0);
        let gap = self.px(6.0);
        let pad = self.px(10.0);
        let rows = keyboard.rows();
        // Landscape adds the Split · Full · Centre bar above the keys.
        let bar_h = if self.landscape() {
            self.px(44.0) + gap
        } else {
            0
        };
        let kb_h = if keys_on {
            pad * 2 + bar_h + key_h * rows.len() as i32 + gap * (rows.len() as i32 - 1)
        } else {
            0
        };
        let keyboard_rect = Rect::new(0, h - kb_h, w, kb_h);
        let status_h = self.px(44.0) + 2 * self.px(4.0);
        let status = Rect::new(0, keyboard_rect.y - status_h, w, status_h);
        let fb_h = if self.find_open { self.find_bar_h() } else { 0 };
        let find_bar = Rect::new(0, title.h, w, fb_h);
        let page = Rect::new(0, title.h + fb_h, w, status.y - title.h - fb_h);
        let kb_w = self.px(64.0);
        let keys_button = Rect::new(
            w - self.px(12.0) - kb_w,
            status.y + (status_h - self.px(44.0)) / 2,
            kb_w,
            self.px(44.0),
        );
        let (keys, kb_buttons) = if keys_on {
            self.board(keyboard, keyboard_rect, key_h, gap, pad, bar_h)
        } else {
            (Vec::new(), Vec::new())
        };
        let btn = self.px(44.0);
        let by = (title.h - btn) / 2;
        let files_button = Rect::new(w - self.px(12.0) - btn, by, btn, btn);
        let preview_button = Rect::new(files_button.x - self.px(8.0) - btn, by, btn, btn);
        let left_of = |r: Rect| Rect::new(r.x - self.px(8.0) - btn, by, btn, btn);
        let find_button = left_of(preview_button);
        let scratch_button = left_of(find_button);
        let todos_button = left_of(scratch_button);
        let contents_button = left_of(todos_button);
        let graph_button = left_of(contents_button);
        let options_button = left_of(graph_button);
        let name_rect = Rect::new(
            0,
            0,
            (w * 3 / 5).min(options_button.x - self.px(8.0)),
            title.h,
        );
        let column = if self.landscape() && self.cfg.line_on {
            let cw = w.min(h);
            Rect::new(page.x + (page.w - cw) / 2, page.y, cw, page.h)
        } else {
            page
        };
        let okw = self.px(72.0);
        let cw = self.px(96.0);
        let ok_rect = Rect::new(w - self.px(12.0) - okw, by, okw, btn);
        let cancel_rect = Rect::new(ok_rect.x - self.px(8.0) - cw, by, cw, btn);
        Layout {
            title,
            page,
            status,
            keys_button,
            keyboard: keyboard_rect,
            keys,
            files_button,
            preview_button,
            name_rect,
            ok_rect,
            cancel_rect,
            options_button,
            scratch_button,
            todos_button,
            contents_button,
            find_button,
            graph_button,
            find_bar,
            column,
            kb_buttons,
        }
    }

    /// Keys and landscape bar buttons inside the keyboard rectangle.
    ///
    /// Grid units (10 Compact, 16 Full); rows are centred, columns align
    /// because every key starts at its cumulative unit offset. The unit comes
    /// from the short side, so keys keep their portrait size in landscape.
    /// Landscape: **Centre** centres the board; **Split** cuts it in two with
    /// a gap (a key across the middle goes to the side of its centre; Space
    /// is cut in two so both halves have one); **Full** adds a number or
    /// direction pad on the left or right.
    #[allow(clippy::too_many_lines)]
    fn board(
        &self,
        keyboard: &Keyboard,
        kb: Rect,
        key_h: i32,
        gap: i32,
        pad: i32,
        bar_h: i32,
    ) -> Board {
        let w = self.width as i32;
        let short = w.min(self.height as i32);
        let land = self.landscape();
        let rows = keyboard.rows();
        let grid = keyboard.grid_units();
        let mode = if land {
            self.cfg.land_keys
        } else {
            LandKeys::Centre
        };
        // Portrait key size; Full shrinks it slightly if board + pad would
        // not fit the long side.
        let mut unit = (short - 2 * pad + gap) as f32 / grid;
        if mode == LandKeys::Full {
            unit = unit.min((w - 2 * pad + gap) as f32 / (grid + 3.5));
        }
        let board_w = grid * unit;
        let split_gap = if mode == LandKeys::Split {
            (3.0 * unit).min((w - 2 * pad) as f32 - board_w).max(0.0)
        } else {
            0.0
        };
        let (pad_w, pad_gap) = if mode == LandKeys::Full {
            (3.0 * unit, unit / 2.0)
        } else {
            (0.0, 0.0)
        };
        let total = board_w + split_gap + pad_w + pad_gap;
        let x_start = if land {
            ((w as f32 - total) / 2.0).max(0.0)
        } else {
            pad as f32
        };
        let pad_left = mode == LandKeys::Full && self.cfg.pad_side == Side::Left;
        let board_x = x_start + if pad_left { pad_w + pad_gap } else { 0.0 };
        let top = kb.y + pad + bar_h;
        let mid = grid / 2.0;
        let mut keys = Vec::new();
        let mut push = |x: f32, kw: f32, y: i32, h: i32, k: Key| {
            let x0 = x.round() as i32;
            let x1 = (x + kw).round() as i32 - gap;
            keys.push((Rect::new(x0, y, x1 - x0, h), k));
        };
        for (r, row) in rows.iter().enumerate() {
            let row = if split_gap > 0.0 {
                space_across(keyboard, r, row, grid)
            } else {
                row.clone()
            };
            let total_u: f32 = row
                .iter()
                .enumerate()
                .map(|(c, &k)| keyboard.units(k, r, c))
                .sum();
            let mut u = (grid - total_u) / 2.0;
            let y = top + r as i32 * (key_h + gap);
            for (c, &k) in row.iter().enumerate() {
                let ku = keyboard.units(k, r, c);
                let across = u + 1e-3 < mid && u + ku > mid + 1e-3;
                if split_gap > 0.0 && across && k == Key::Space {
                    push(board_x + u * unit, (mid - u) * unit, y, key_h, k);
                    push(
                        board_x + mid * unit + split_gap,
                        (u + ku - mid) * unit,
                        y,
                        key_h,
                        k,
                    );
                } else {
                    let right = if across {
                        u + ku / 2.0 >= mid
                    } else {
                        u + 1e-3 >= mid
                    };
                    let off = if right { split_gap } else { 0.0 };
                    push(board_x + u * unit + off, ku * unit, y, key_h, k);
                }
                u += ku;
            }
        }
        if mode == LandKeys::Full {
            let px0 = if pad_left {
                x_start
            } else {
                board_x + board_w + pad_gap
            };
            let n = |c: char| Some(Key::Char(c));
            let cells: Vec<[Option<Key>; 3]> = match self.cfg.pad {
                Pad::Numbers => vec![
                    [n('7'), n('8'), n('9')],
                    [n('4'), n('5'), n('6')],
                    [n('1'), n('2'), n('3')],
                    [n('0'), n('.'), None],
                ],
                Pad::Arrows => vec![
                    [None, Some(Key::Arrow(Dir::Up)), None],
                    [
                        Some(Key::Arrow(Dir::Left)),
                        None,
                        Some(Key::Arrow(Dir::Right)),
                    ],
                    [None, Some(Key::Arrow(Dir::Down)), None],
                ],
            };
            let nr = rows.len() as i32;
            let span = nr * key_h + (nr - 1) * gap;
            let m = cells.len() as i32;
            let ph = (span - (m - 1) * gap) / m;
            for (i, line) in cells.iter().enumerate() {
                for (c, k) in line.iter().enumerate() {
                    if let Some(k) = k {
                        push(
                            px0 + c as f32 * unit,
                            unit,
                            top + i as i32 * (ph + gap),
                            ph,
                            *k,
                        );
                    }
                }
            }
        }
        let mut buttons = Vec::new();
        if land {
            let bh = self.px(44.0);
            let by = kb.y + pad;
            let bw = self.px(110.0);
            let bg = self.px(8.0);
            let x = (w - (3 * bw + 2 * bg)) / 2;
            for (i, m) in [LandKeys::Split, LandKeys::Full, LandKeys::Centre]
                .into_iter()
                .enumerate()
            {
                buttons.push((
                    Rect::new(x + i as i32 * (bw + bg), by, bw, bh),
                    KbButton::Mode(m),
                ));
            }
            if mode == LandKeys::Full {
                let side = Rect::new(w - pad - bw, by, bw, bh);
                buttons.push((Rect::new(side.x - bg - bw, by, bw, bh), KbButton::PadToggle));
                buttons.push((side, KbButton::PadSide));
            }
        }
        (keys, buttons)
    }

    /// Label of a landscape bar button (the action it takes).
    #[must_use]
    pub fn kb_button_label(&self, b: KbButton) -> &'static str {
        match b {
            KbButton::Mode(LandKeys::Split) => "Split",
            KbButton::Mode(LandKeys::Full) => "Full",
            KbButton::Mode(LandKeys::Centre) => "Centre",
            KbButton::PadToggle => match self.cfg.pad {
                Pad::Numbers => "Arrows",
                Pad::Arrows => "123",
            },
            KbButton::PadSide => match self.cfg.pad_side {
                Side::Left => "Pad right",
                Side::Right => "Pad left",
            },
        }
    }

    fn text_area(&self, page: Rect) -> Rect {
        let page = if self.landscape() && self.cfg.line_on {
            let cw = (self.width.min(self.height) as i32).min(page.w);
            Rect::new(page.x + (page.w - cw) / 2, page.y, cw, page.h)
        } else {
            page
        };
        let m = self.px(self.margin_px);
        Rect::new(
            page.x + m,
            page.y + self.px(16.0),
            page.w - 2 * m,
            page.h - self.px(16.0) * 2,
        )
    }

    fn visible_rows(&self, page: Rect) -> usize {
        (self.text_area(page).h / self.line_h()).max(1) as usize
    }

    #[must_use]
    pub fn rows(&self, buffer: &Buffer, page: Rect) -> Vec<VisualLine> {
        wrap(
            buffer.text(),
            self.text_area(page).w as u32,
            &self.measure(),
        )
    }

    /// Keeps the caret row on screen.
    pub fn follow_caret(&mut self, buffer: &Buffer, page: Rect) {
        let rows = self.rows(buffer, page);
        let (row, _) = caret_cell(buffer.text(), &rows, buffer.caret(), &self.measure());
        let n = self.visible_rows(page);
        if row < self.scroll {
            self.scroll = row;
        } else if row >= self.scroll + n {
            self.scroll = row + 1 - n;
        }
        self.scroll = self.scroll.min(rows.len().saturating_sub(1));
    }

    /// Page-turn buttons: scroll one screen and put the caret at the top row.
    pub fn page_turn(&mut self, buffer: &mut Buffer, page: Rect, forward: bool) {
        self.page_turn_ext(buffer, page, forward, false);
    }

    pub fn page_turn_ext(&mut self, buffer: &mut Buffer, page: Rect, forward: bool, extend: bool) {
        let rows = self.rows(buffer, page);
        let n = self.visible_rows(page).saturating_sub(1).max(1);
        self.scroll = if forward {
            (self.scroll + n).min(rows.len().saturating_sub(1))
        } else {
            self.scroll.saturating_sub(n)
        };
        buffer.move_to(rows[self.scroll].start, extend);
    }

    /// Vertical caret move by one visual row (keyboard arrows later).
    pub fn caret_row(&mut self, buffer: &mut Buffer, page: Rect, down: bool) {
        self.caret_row_ext(buffer, page, down, false);
    }

    /// As [`Self::caret_row`]; with `extend` the selection grows (Shift+↑/↓).
    pub fn caret_row_ext(&mut self, buffer: &mut Buffer, page: Rect, down: bool, extend: bool) {
        let rows = self.rows(buffer, page);
        let m = self.measure();
        let (row, col) = caret_cell(buffer.text(), &rows, buffer.caret(), &m);
        let target = if down { row + 1 } else { row.wrapping_sub(1) };
        if let Some(&r) = rows.get(target) {
            buffer.move_to(offset_at(buffer.text(), r, col, &m), extend);
        } else {
            buffer.move_to(if down { buffer.text().len() } else { 0 }, extend);
        }
    }

    /// Screen point (x, top of row) of a text offset, if it is on screen.
    #[must_use]
    pub fn point_of(&self, buffer: &Buffer, page: Rect, at: usize) -> Option<(i32, i32)> {
        let area = self.text_area(page);
        let rows = self.rows(buffer, page);
        let (row, col) = caret_cell(buffer.text(), &rows, at, &self.measure());
        (row >= self.scroll && row < self.scroll + self.visible_rows(page)).then(|| {
            (
                area.x + col as i32,
                area.y + (row - self.scroll) as i32 * self.line_h(),
            )
        })
    }

    /// Text offset under a screen point on the page (char-snapped).
    #[must_use]
    pub fn offset_at_point(&self, buffer: &Buffer, page: Rect, x: i32, y: i32) -> usize {
        let area = self.text_area(page);
        let rows = self.rows(buffer, page);
        let r = self.scroll + ((y - area.y).max(0) / self.line_h()) as usize;
        let Some(&row) = rows.get(r).or(rows.last()) else {
            return 0;
        };
        offset_at(
            buffer.text(),
            row,
            (x - area.x).max(0) as u32,
            &self.measure(),
        )
    }

    #[must_use]
    pub fn line_px(&self) -> i32 {
        self.line_h()
    }

    /// Vertical distance from a text row's top to its handle's centre.
    #[must_use]
    pub fn handle_drop(&self) -> i32 {
        self.line_h() + self.px(16.0)
    }

    /// Handles under both selection ends; the action bar above the
    /// selection (below it near the top of the page).
    #[must_use]
    pub fn selection_geometry(&self, model: &Model<'_>, l: &Layout) -> SelGeom {
        let Some(ui) = model.sel_ui else {
            return SelGeom::default();
        };
        let b = model.buffer;
        let target = self.px(48.0);
        let mut g = SelGeom::default();
        let (a, e) = b.selection().unwrap_or((b.caret(), b.caret()));
        let pa = self.point_of(b, l.page, a);
        let pe = self.point_of(b, l.page, e);
        if b.selection().is_some() {
            for (i, p) in [pa, pe].into_iter().enumerate() {
                g.handles[i] = p.map(|(x, y)| {
                    let cy = y + self.handle_drop();
                    Rect::new(x - target / 2, cy - target / 2, target, target)
                });
            }
        }
        if !ui.bar {
            return g;
        }
        let actions: &[BarAction] = if b.selection().is_some() {
            &[
                BarAction::Cut,
                BarAction::Copy,
                BarAction::Paste,
                BarAction::SelectAll,
            ]
        } else {
            &[BarAction::Paste, BarAction::SelectAll]
        };
        let ui_px = self.ui_px();
        let pad = self.px(14.0);
        let bh = self.px(52.0);
        let widths: Vec<i32> = actions
            .iter()
            .map(|a| {
                (self.fonts.width(Face::AtkinsonBold, ui_px, a.label()) as i32 + 2 * pad)
                    .max(self.px(72.0))
            })
            .collect();
        let bw: i32 = widths.iter().sum();
        let Some((ax, ay)) = pa.or(pe) else { return g };
        let gap = self.px(10.0);
        let mut by = ay - gap - bh;
        if by < l.page.y + self.px(4.0) {
            // No room above: go below the end handle.
            let ey = pe.map_or(ay, |p| p.1);
            by = ey + self.handle_drop() + target / 2 + gap;
        }
        let bx = (ax - bw / 2).clamp(self.px(8.0), self.width as i32 - bw - self.px(8.0));
        let bar = Rect::new(bx, by, bw, bh);
        let mut x = bx;
        for (w, a) in widths.iter().zip(actions) {
            g.buttons.push((Rect::new(x, by, *w, bh), *a));
            x += w;
        }
        g.bar = Some(bar);
        g
    }

    #[must_use]
    pub fn hit(&self, model: &Model<'_>, x: i32, y: i32) -> Hit {
        let l = self.layout(model.keys_on, model.keyboard);
        if model.sel_ui.is_some() && model.rename.is_none() {
            let g = self.selection_geometry(model, &l);
            if let Some((_, a)) = g.buttons.iter().find(|(r, _)| r.contains(x, y)) {
                return Hit::Bar(*a);
            }
            for (i, h) in g.handles.iter().enumerate() {
                if h.is_some_and(|r| r.inset(-self.px(6.0)).contains(x, y)) {
                    return Hit::Handle(i);
                }
            }
        }
        if model.rename.is_some() {
            if l.ok_rect.contains(x, y) {
                return Hit::RenameOk;
            }
            if l.cancel_rect.contains(x, y) {
                return Hit::RenameCancel;
            }
            if let Some(&(_, k)) = l
                .keys
                .iter()
                .find(|(r, _)| r.inset(-self.px(3.0) / 2).contains(x, y))
            {
                return Hit::Key(k);
            }
            // Brief: blur confirms.
            return if l.title.contains(x, y) {
                Hit::Nothing
            } else {
                Hit::RenameOk
            };
        }
        if l.files_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Files;
        }
        if l.preview_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::TogglePreview;
        }
        if l.options_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Options;
        }
        if l.find_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Find;
        }
        if l.graph_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Graph;
        }
        for (r, p) in [
            (l.contents_button, crate::drawer::Pane::Contents),
            (l.todos_button, crate::drawer::Pane::Todos),
            (l.scratch_button, crate::drawer::Pane::Scratch),
        ] {
            if r.inset(-self.px(4.0)).contains(x, y) {
                return Hit::Drawer(p);
            }
        }
        if let Some((_, b)) = l.kb_buttons.iter().find(|(r, _)| r.contains(x, y)) {
            return Hit::KbButton(*b);
        }
        if l.name_rect.contains(x, y) {
            return Hit::StartRename;
        }
        if l.keys_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::ToggleKeys;
        }
        if let Some(&(_, k)) = l
            .keys
            .iter()
            .find(|(r, _)| r.inset(-self.px(3.0) / 2).contains(x, y))
        {
            return Hit::Key(k);
        }
        if l.page.contains(x, y) {
            let area = self.text_area(l.page);
            let rows = self.rows(model.buffer, l.page);
            let r = self.scroll + ((y - area.y).max(0) / self.line_h()) as usize;
            let Some(&row) = rows.get(r).or(rows.last()) else {
                return Hit::Nothing;
            };
            let col = (x - area.x).max(0) as u32;
            return Hit::Caret(offset_at(model.buffer.text(), row, col, &self.measure()));
        }
        Hit::Nothing
    }

    /// Draws the whole screen; returns the layout used.
    pub fn draw(&mut self, c: &mut Canvas, model: &Model<'_>) -> Layout {
        let t = model.theme;
        let l = self.layout(model.keys_on, model.keyboard);
        self.follow_caret(model.buffer, l.page);
        c.fill(c.bounds(), t.paper);
        draw_gutters(c, &l, t);
        let ui = 17.0 * self.s.0;

        self.draw_title(c, &l, model.file_name, model.rename, t, true);
        if model.rename.is_none() {
            draw_eye_icon(c, l.preview_button, self.s, t.ink);
            self.draw_options_icon(c, l.options_button, t);
            if self.find_open {
                c.fill(l.find_button, t.wash);
                c.stroke(l.find_button, 1, t.ink);
            }
            draw_find_icon(c, l.find_button, self.s, t.ink);
            draw_graph_icon(c, l.graph_button, self.s, t.ink);
            for (i, r) in [l.contents_button, l.todos_button, l.scratch_button]
                .into_iter()
                .enumerate()
            {
                if self.drawer_open[i] {
                    c.fill(r, t.wash);
                    c.stroke(r, 1, t.ink);
                }
                match i {
                    0 => crate::drawer::draw_contents_icon(c, r, self.s, t.ink),
                    1 => crate::drawer::draw_todos_icon(c, r, self.s, t.ink),
                    _ => crate::drawer::draw_scratch_icon(c, r, self.s, t.ink),
                }
            }
        }

        // Page.
        let area = self.text_area(l.page);
        let rows = self.rows(model.buffer, l.page);
        let text = model.buffer.text();
        let lh = self.line_h();
        let asc = self.fonts.ascent(self.face, self.body());
        let top_pad = ((lh as f32 - self.body() * 1.2) / 2.0).max(0.0);
        let sel = model.buffer.selection();
        for (i, r) in rows
            .iter()
            .skip(self.scroll)
            .take(self.visible_rows(l.page))
            .enumerate()
        {
            if let Some((a, b)) = sel {
                let (s0, s1) = (a.max(r.start), b.min(r.end.max(r.start)));
                let through_newline = b > r.end && a <= r.end;
                if s0 < s1 || through_newline {
                    let x0 = self.fonts.width(
                        self.face,
                        self.body(),
                        &text[r.start..s0.min(r.end).max(r.start)],
                    );
                    let x1 = self
                        .fonts
                        .width(self.face, self.body(), &text[r.start..s1.max(s0)])
                        + if through_newline {
                            self.body() * 0.4
                        } else {
                            0.0
                        };
                    c.fill(
                        Rect::new(
                            area.x + x0 as i32,
                            area.y + i as i32 * lh,
                            (x1 - x0).ceil() as i32,
                            lh,
                        ),
                        t.wash,
                    );
                }
            }
            let y = area.y as f32 + (i as i32 * lh) as f32 + top_pad + asc;
            self.fonts.draw(
                c,
                self.face,
                self.body(),
                area.x as f32,
                y,
                &text[r.start..r.end],
                t.ink,
            );
        }
        let (crow, ccol) = caret_cell(text, &rows, model.buffer.caret(), &self.measure());
        if crow >= self.scroll && crow < self.scroll + self.visible_rows(l.page) {
            let cy =
                area.y + (crow - self.scroll) as i32 * lh + (lh - (self.body() * 1.25) as i32) / 2;
            c.fill(
                Rect::new(
                    area.x + ccol as i32,
                    cy,
                    self.px(2.0).max(2),
                    (self.body() * 1.25) as i32,
                ),
                t.ink,
            );
        }

        self.draw_selection_ui(c, model, &l);

        // Status bar.
        c.fill(Rect::new(0, l.status.y, l.status.w, 1), t.rule);
        let sb = l.status.y as f32
            + l.status.h as f32 / 2.0
            + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
        let left = format!(
            "{} words \u{b7} line {}",
            sigma_core::text::word_count(text),
            model.buffer.line_number()
        );
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            self.px(20.0) as f32,
            sb,
            &left,
            t.mute,
        );
        let right = self.status_right(t, model.save);
        let rw = self.fonts.width(Face::Atkinson, ui, &right);
        let rx = l.keys_button.x as f32 - self.px(16.0) as f32 - rw;
        self.fonts
            .draw(c, Face::Atkinson, ui, rx, sb, &right, t.mute);
        let kb = l.keys_button;
        if model.keys_on {
            c.fill(kb, t.wash);
            c.stroke(kb, 1, t.ink);
        } else {
            c.stroke(kb, 1, t.rule);
        }
        let kw = self.fonts.width(Face::Atkinson, ui, "Keys");
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            kb.x as f32 + (kb.w as f32 - kw) / 2.0,
            sb,
            "Keys",
            t.ink,
        );

        if model.keys_on {
            self.draw_keys(c, &l, model.keyboard, t);
        }
        l
    }

    fn draw_selection_ui(&self, c: &mut Canvas, model: &Model<'_>, l: &Layout) {
        if model.rename.is_some() {
            return;
        }
        let t = model.theme;
        let g = self.selection_geometry(model, l);
        let r = self.px(9.0);
        for h in g.handles.iter().flatten() {
            let (cx, cy) = (h.x + h.w / 2, h.y + h.h / 2);
            // Stem from the text row down to a solid dot: no animation,
            // a few hundred pixels per handle for the partial refresh.
            c.fill(
                Rect::new(
                    cx - 1,
                    cy - self.handle_drop() + self.line_h(),
                    2,
                    self.handle_drop() - self.line_h() - r + 1,
                ),
                t.ink,
            );
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        c.blend(cx + dx, cy + dy, t.ink, 255);
                    }
                }
            }
        }
        if let Some(bar) = g.bar {
            let ui = self.ui_px();
            c.fill(bar.inset(-2), t.paper);
            c.fill(bar, t.ink);
            for (i, (br, a)) in g.buttons.iter().enumerate() {
                if i > 0 {
                    c.fill(
                        Rect::new(br.x, br.y + self.px(10.0), 1, br.h - self.px(20.0)),
                        t.mute,
                    );
                }
                let w = self.fonts.width(Face::AtkinsonBold, ui, a.label());
                let base = br.y as f32
                    + br.h as f32 / 2.0
                    + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6;
                self.fonts.draw(
                    c,
                    Face::AtkinsonBold,
                    ui,
                    br.x as f32 + (br.w as f32 - w) / 2.0,
                    base,
                    a.label(),
                    t.paper,
                );
            }
        }
    }

    /// Status bar right side (brief §3): `USB` when on, `Paper` or
    /// `Night`, then the save state.
    fn status_right(&self, t: Theme, save: &SaveState) -> String {
        format!(
            "{}{} \u{b7} {}",
            if self.cfg.usb { "USB \u{b7} " } else { "" },
            if t == Theme::NIGHT { "Night" } else { "Paper" },
            save.label()
        )
    }

    /// "Aa" in a 44 px target: the Options sheet.
    pub fn draw_options_icon(&self, c: &mut Canvas, r: Rect, t: Theme) {
        let size = self.ui_px() * 1.1;
        let w = self.fonts.width(Face::AtkinsonBold, size, "Aa");
        let base =
            r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(Face::AtkinsonBold, size) / 2.6;
        self.fonts.draw(
            c,
            Face::AtkinsonBold,
            size,
            r.x as f32 + (r.w as f32 - w) / 2.0,
            base,
            "Aa",
            t.ink,
        );
    }

    #[must_use]
    pub fn ui_px(&self) -> f32 {
        17.0 * self.s.0
    }

    #[must_use]
    pub fn scale(&self) -> Scale {
        self.s
    }

    /// Title bar: a name (tap to rename) and the Files icon, or the rename
    /// field with Cancel / OK. Hairline under.
    pub fn draw_title(
        &self,
        c: &mut Canvas,
        l: &Layout,
        name: &str,
        rename: Option<&Buffer>,
        t: Theme,
        files_icon: bool,
    ) {
        let ui = self.ui_px();
        c.fill(l.title, t.paper);
        let base = l.title.y as f32
            + l.title.h as f32 / 2.0
            + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6;
        if let Some(field) = rename {
            let fr = Rect::new(
                self.px(12.0),
                l.ok_rect.y,
                l.cancel_rect.x - self.px(24.0),
                l.ok_rect.h,
            );
            c.fill(fr, t.wash);
            c.stroke(fr, 1, t.ink);
            let x0 = fr.x as f32 + self.px(10.0) as f32;
            let end = self
                .fonts
                .draw(c, Face::Atkinson, ui, x0, base, field.text(), t.ink);
            let before = self
                .fonts
                .width(Face::Atkinson, ui, &field.text()[..field.caret()]);
            let _ = end;
            c.fill(
                Rect::new(
                    (x0 + before) as i32,
                    fr.y + self.px(8.0),
                    2,
                    fr.h - self.px(16.0),
                ),
                t.ink,
            );
            for (r, label, primary) in [(l.cancel_rect, "Cancel", false), (l.ok_rect, "OK", true)] {
                c.fill(r, if primary { t.ink } else { t.paper });
                c.stroke(r, 1, t.ink);
                let w = self.fonts.width(Face::AtkinsonBold, ui, label);
                self.fonts.draw(
                    c,
                    Face::AtkinsonBold,
                    ui,
                    r.x as f32 + (r.w as f32 - w) / 2.0,
                    base,
                    label,
                    if primary { t.paper } else { t.ink },
                );
            }
        } else {
            let shown = ellipsize(
                &self.fonts,
                Face::AtkinsonBold,
                ui,
                name,
                l.name_rect.w as f32 - self.px(28.0) as f32,
            );
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                self.px(20.0) as f32,
                base,
                &shown,
                t.ink,
            );
            if files_icon {
                draw_folder_icon(c, l.files_button, self.s, t.ink);
            }
        }
        c.fill(
            Rect::new(0, l.title.y + l.title.h - 1, l.title.w, 1),
            t.rule,
        );
    }

    /// The on-screen keyboard in `l.keyboard`.
    pub fn draw_keys(&self, c: &mut Canvas, l: &Layout, keyboard: &Keyboard, t: Theme) {
        let t = self.key_theme.unwrap_or(t);
        c.fill(l.keyboard, t.paper);
        c.fill(Rect::new(0, l.keyboard.y, l.keyboard.w, 1), t.rule);
        let ks = 20.0 * self.s.0;
        let ui = self.ui_px();
        for &(r, b) in &l.kb_buttons {
            let active = matches!(b, KbButton::Mode(m) if m == self.cfg.land_keys);
            let (bg, fg) = if active {
                (t.ink, t.paper)
            } else {
                (t.paper, t.ink)
            };
            c.fill(r, bg);
            c.stroke(r, 1, if active { t.ink } else { t.rule });
            let label = self.kb_button_label(b);
            let lw = self.fonts.width(Face::AtkinsonBold, ui, label);
            let by =
                r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6;
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                r.x as f32 + (r.w as f32 - lw) / 2.0,
                by,
                label,
                fg,
            );
        }
        for &(r, k) in &l.keys {
            let active = keyboard.is_active(k);
            let (bg, fg) = if active {
                (t.ink, t.paper)
            } else {
                (t.paper, t.ink)
            };
            c.fill(r, bg);
            c.stroke(r, 1, if active { t.ink } else { t.rule });
            if k == Key::Shift && keyboard.shift_lock {
                // Locked: a bar under the label (one-shot has none).
                c.fill(
                    Rect::new(
                        r.x + r.w / 4,
                        r.y + r.h - self.px(10.0),
                        r.w / 2,
                        self.px(3.0),
                    ),
                    fg,
                );
            }
            if let Key::Arrow(d) = k {
                // Atkinson has no arrow glyphs: draw a 1.75-stroke arrow.
                draw_arrow(c, r, d, self.s, fg);
                continue;
            }
            let label = keyboard.label(k);
            let (face, size) = if matches!(k, Key::Char(_)) {
                (Face::Atkinson, ks * 1.15)
            } else {
                (Face::AtkinsonBold, ks * 0.85)
            };
            let lw = self.fonts.width(face, size, &label);
            let by = r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6;
            self.fonts.draw(
                c,
                face,
                size,
                r.x as f32 + (r.w as f32 - lw) / 2.0,
                by,
                &label,
                fg,
            );
        }
    }
}

/// Split: both halves need a Space (brief §7). If the row's Space sits
/// wholly on one side, move it to the slot where it crosses the middle, as
/// evenly as possible; the other keys keep their order.
fn space_across(keyboard: &Keyboard, r: usize, row: &[Key], grid: f32) -> Vec<Key> {
    let Some(at) = row.iter().position(|&k| k == Key::Space) else {
        return row.to_vec();
    };
    let mid = grid / 2.0;
    let others: Vec<Key> = row.iter().copied().filter(|&k| k != Key::Space).collect();
    let mut best = (f32::MAX, at, row.to_vec());
    for i in 0..=others.len() {
        let mut cand = others.clone();
        cand.insert(i, Key::Space);
        let total: f32 = cand
            .iter()
            .enumerate()
            .map(|(c, &k)| keyboard.units(k, r, c))
            .sum();
        let start = (grid - total) / 2.0
            + cand[..i]
                .iter()
                .enumerate()
                .map(|(c, &k)| keyboard.units(k, r, c))
                .sum::<f32>();
        let ku = keyboard.units(Key::Space, r, i);
        let imbalance = ((mid - start) - (start + ku - mid)).abs();
        let crosses = start < mid && start + ku > mid;
        // Keep the original slot when it already crosses.
        let score = if !crosses {
            f32::MAX / 2.0
        } else if i == at {
            -1.0
        } else {
            imbalance
        };
        if score < best.0 {
            best = (score, i, cand);
        }
    }
    best.2
}

/// Landscape with Line on: the strips beside the locked column (brief §4).
pub fn draw_gutters(c: &mut Canvas, l: &Layout, t: Theme) {
    if l.column.w > 0 && l.column.w < l.page.w {
        c.fill(
            Rect::new(l.page.x, l.page.y, l.column.x - l.page.x, l.page.h),
            t.gutter,
        );
        let right = l.column.x + l.column.w;
        c.fill(
            Rect::new(right, l.page.y, l.page.x + l.page.w - right, l.page.h),
            t.gutter,
        );
    }
}

fn draw_arrow(c: &mut Canvas, r: Rect, d: Dir, s: Scale, ink: u8) {
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    let len = s.px(10.0);
    let head = s.px(6.0);
    let st = s.px(1.75).max(2);
    // Points along the shaft and the two head strokes, in arrow space
    // (pointing +x), rotated per direction.
    let rot = |x: i32, y: i32| match d {
        Dir::Right => (cx + x, cy + y),
        Dir::Left => (cx - x, cy + y),
        Dir::Down => (cx + y, cy + x),
        Dir::Up => (cx + y, cy - x),
    };
    let mut dot = |x: i32, y: i32| {
        for a in 0..st {
            for b in 0..st {
                let (px, py) = rot(x, y);
                c.blend(px + a - st / 2, py + b - st / 2, ink, 255);
            }
        }
    };
    for x in -len..=len {
        dot(x, 0);
    }
    for k in 0..=head {
        dot(len - k, k);
        dot(len - k, -k);
    }
}

/// What preview mode shows (v0.3, read-only).
pub struct PreviewModel<'a> {
    pub file_name: &'a str,
    pub doc: &'a Doc,
    pub page: usize,
    pub words: usize,
    pub save: &'a SaveState,
    pub keyboard: &'a Keyboard,
    pub theme: Theme,
}

impl EditorScreen {
    /// The text column of the preview page (the keyboard is hidden).
    #[must_use]
    pub fn preview_area(&self, keyboard: &Keyboard) -> Rect {
        self.text_area(self.layout(false, keyboard).page)
    }

    /// Parses and lays out `text` for preview at the current type settings.
    #[must_use]
    pub fn build_preview(
        &self,
        text: &str,
        keyboard: &Keyboard,
        known: &dyn Fn(&str) -> bool,
    ) -> Doc {
        self.build_preview_with(text, keyboard, known, None)
    }

    /// As [`Self::build_preview`], with a Backlinks section at the end
    /// (brief §4): `(label, target)` per note that links here.
    #[must_use]
    pub fn build_preview_with(
        &self,
        text: &str,
        keyboard: &Keyboard,
        known: &dyn Fn(&str) -> bool,
        backlinks: Option<&[(String, String)]>,
    ) -> Doc {
        use sigma_core::markdown::{Block, BlockKind, LinkKind, Run, Style};
        let area = self.preview_area(keyboard);
        let ty = Type {
            body: self.body(),
            leading: self.leading,
            scale: self.s,
        };
        let mut blocks = sigma_core::markdown::parse(text);
        if let Some(bl) = backlinks {
            // Past every source line, so it never moves the write position.
            let line = text.lines().count() + 1;
            let run = |text: &str, style: Style, target: Option<&str>| Run {
                text: text.to_owned(),
                style,
                target: target.map(str::to_owned),
            };
            let block = |kind, runs| Block { kind, runs, line };
            blocks.push(block(BlockKind::Rule, Vec::new()));
            blocks.push(block(
                BlockKind::Paragraph,
                vec![run(
                    "Backlinks",
                    Style {
                        strong: true,
                        ..Style::default()
                    },
                    None,
                )],
            ));
            if bl.is_empty() {
                blocks.push(block(
                    BlockKind::Paragraph,
                    vec![run(
                        "No other notes link here.",
                        Style {
                            em: true,
                            ..Style::default()
                        },
                        None,
                    )],
                ));
            }
            for (label, target) in bl {
                blocks.push(block(
                    BlockKind::Item {
                        depth: 0,
                        marker: None,
                        task: None,
                    },
                    vec![run(
                        label,
                        Style {
                            link: Some(LinkKind::Wiki),
                            ..Style::default()
                        },
                        Some(target),
                    )],
                ));
            }
        }
        let lines = preview::layout(&blocks, &self.fonts, area.w, ty, known);
        Doc {
            pages: preview::paginate(&lines, area.h),
            lines,
            width: area.w,
        }
    }

    /// Source line at the top of the write-mode page (keeps the place when
    /// entering preview).
    #[must_use]
    pub fn top_source_line(&self, buffer: &Buffer, page: Rect) -> usize {
        let rows = self.rows(buffer, page);
        let at = rows.get(self.scroll).map_or(0, |r| r.start);
        buffer.text()[..at.min(buffer.text().len())]
            .matches('\n')
            .count()
    }

    /// The `[[wiki]]` target under a tap on the shown preview page.
    #[must_use]
    pub fn preview_link_at(
        &self,
        keyboard: &Keyboard,
        doc: &Doc,
        page: usize,
        x: i32,
        y: i32,
    ) -> Option<String> {
        let area = self.text_area(self.layout(false, keyboard).page);
        let r = doc.pages.get(page)?.clone();
        preview::link_at(&self.fonts, area, &doc.lines, r, x, y, self.px(8.0))
    }

    /// Screen point of the first link labelled `label` on the shown page
    /// (emulator scripts).
    #[must_use]
    pub fn preview_link_point(
        &self,
        keyboard: &Keyboard,
        doc: &Doc,
        page: usize,
        label: &str,
    ) -> Option<(i32, i32)> {
        let area = self.text_area(self.layout(false, keyboard).page);
        let mut y = area.y;
        for l in &doc.lines[doc.pages.get(page)?.clone()] {
            for p in &l.pieces {
                if p.target.is_some() && p.text.trim() == label {
                    let w = self.fonts.width(p.face, p.size, &p.text) as i32;
                    return Some((area.x + p.x + w / 2, y + l.h / 2));
                }
            }
            y += l.h;
        }
        None
    }

    /// Taps in preview: only the title-bar buttons do something in v0.3.
    #[must_use]
    pub fn preview_hit(&self, keyboard: &Keyboard, x: i32, y: i32) -> Hit {
        let l = self.layout(false, keyboard);
        if l.files_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Files
        } else if l.preview_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::TogglePreview
        } else if l.options_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Options
        } else if l.find_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Find
        } else if l.graph_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Graph
        } else {
            Hit::Nothing
        }
    }

    /// Draws preview mode: title with the pencil, one page, status bar.
    pub fn draw_preview(&self, c: &mut Canvas, m: &PreviewModel<'_>) -> Layout {
        let t = m.theme;
        let l = self.layout(false, m.keyboard);
        c.fill(c.bounds(), t.paper);
        draw_gutters(c, &l, t);
        self.draw_title(c, &l, m.file_name, None, t, true);
        draw_pencil_icon(c, l.preview_button, self.s, t.ink);
        self.draw_options_icon(c, l.options_button, t);
        draw_find_icon(c, l.find_button, self.s, t.ink);
        draw_graph_icon(c, l.graph_button, self.s, t.ink);
        let area = self.text_area(l.page);
        let page = m.page.min(m.doc.pages.len().saturating_sub(1));
        if let Some(r) = m.doc.pages.get(page) {
            preview::draw_page(c, &self.fonts, area, &m.doc.lines, r.clone(), t, self.s);
        }
        let ui = self.ui_px();
        c.fill(Rect::new(0, l.status.y, l.status.w, 1), t.rule);
        let sb = l.status.y as f32
            + l.status.h as f32 / 2.0
            + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
        let n = m.doc.pages.len();
        let mut left = format!("{} words \u{b7} preview", m.words);
        if n > 1 {
            left.push_str(&format!(" \u{b7} page {} of {n}", page + 1));
        }
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            self.px(20.0) as f32,
            sb,
            &left,
            t.mute,
        );
        let right = self.status_right(t, m.save);
        let rw = self.fonts.width(Face::Atkinson, ui, &right);
        let rx = (l.status.w - self.px(20.0)) as f32 - rw;
        self.fonts
            .draw(c, Face::Atkinson, ui, rx, sb, &right, t.mute);
        l
    }
}

/// Eye (brief §3: "eye in write mode"): an almond outline and a pupil.
pub fn draw_eye_icon(c: &mut Canvas, target: Rect, s: Scale, ink: u8) {
    let (cx, cy) = (target.x + target.w / 2, target.y + target.h / 2);
    let a = s.px(11.0) as f32;
    let b = s.px(6.5) as f32;
    let st = s.px(1.75).max(2) as f32;
    let pr = s.px(3.0) as f32;
    let r = a as i32 + 2;
    for dy in -r..=r {
        for dx in -r..=r {
            let (x, y) = (dx as f32, dy as f32);
            // Two arcs meeting at the corners: |y| <= b * (1 - (x/a)^2).
            let edge = b * (1.0 - (x / a).powi(2));
            let on_outline = x.abs() <= a && (y.abs() - edge).abs() <= st / 2.0;
            let pupil = x * x + y * y <= pr * pr;
            if on_outline || pupil {
                c.blend(cx + dx, cy + dy, ink, 255);
            }
        }
    }
}

/// Pencil (brief §3: "pencil in preview"): a diagonal shaft with a tip.
pub fn draw_pencil_icon(c: &mut Canvas, target: Rect, s: Scale, ink: u8) {
    let (cx, cy) = (target.x + target.w / 2, target.y + target.h / 2);
    let half = s.px(9.0);
    let w = s.px(2.5).max(2);
    for t in -half..=half {
        // Shaft from top-right to bottom-left; the last quarter is the tip.
        let tip = t > half / 2;
        let ww = if tip {
            w * (half - t) / (half / 2).max(1)
        } else {
            w
        };
        for k in -ww..=ww {
            c.blend(cx - t + k, cy + t + k, ink, 255);
            if !tip && k.abs() == ww {
                c.blend(cx - t + k + 1, cy + t + k, ink, 255);
            }
        }
    }
    // The cap line across the top end.
    for k in -w..=w {
        c.blend(
            cx + half - s.px(3.0) + k,
            cy - half + s.px(3.0) + k,
            ink,
            255,
        );
    }
}

/// Magnifying glass: a ring and a handle to the lower right.
pub fn draw_find_icon(c: &mut Canvas, target: Rect, s: Scale, ink: u8) {
    let (cx, cy) = (
        target.x + target.w / 2 - s.px(3.0),
        target.y + target.h / 2 - s.px(3.0),
    );
    let r = s.px(7.5) as f32;
    let st = s.px(1.75).max(2) as f32;
    let k = s.px(10.0);
    for dy in -k..=k {
        for dx in -k..=k {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if (d - r).abs() <= st / 2.0 {
                c.blend(cx + dx, cy + dy, ink, 255);
            }
        }
    }
    let st = st as i32;
    let from = (r * 0.72) as i32 + 1;
    for t in from..from + s.px(8.0) {
        for a in 0..st + 1 {
            for b in 0..st + 1 {
                c.blend(cx + t + a - st / 2, cy + t + b - st / 2, ink, 255);
            }
        }
    }
}

/// Graph: three dots joined by lines.
pub fn draw_graph_icon(c: &mut Canvas, target: Rect, s: Scale, ink: u8) {
    let (cx, cy) = (target.x + target.w / 2, target.y + target.h / 2);
    let pts = [
        (cx - s.px(8.0), cy + s.px(6.0)),
        (cx + s.px(8.0), cy + s.px(7.0)),
        (cx + s.px(1.0), cy - s.px(8.0)),
    ];
    let st = s.px(1.5).max(2);
    for (i, &(x0, y0)) in pts.iter().enumerate() {
        let (x1, y1) = pts[(i + 1) % 3];
        let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
        for k in 0..=n {
            let x = x0 + (x1 - x0) * k / n;
            let y = y0 + (y1 - y0) * k / n;
            for a in 0..st {
                c.blend(x + a - st / 2, y, ink, 255);
                c.blend(x, y + a - st / 2, ink, 255);
            }
        }
    }
    let r = s.px(3.5);
    for &(x, y) in &pts {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    c.blend(x + dx, y + dy, ink, 255);
                }
            }
        }
    }
}

/// A 20 px (brief), 1.75-stroke folder outline centred in a 44 px target.
pub fn draw_folder_icon(c: &mut Canvas, target: Rect, s: Scale, ink: u8) {
    let w = s.px(20.0);
    let h = s.px(16.0);
    let st = s.px(1.75).max(2);
    let x = target.x + (target.w - w) / 2;
    let y = target.y + (target.h - h) / 2 + s.px(1.0);
    c.stroke(Rect::new(x, y + s.px(3.0), w, h - s.px(3.0)), st, ink);
    c.stroke(Rect::new(x, y, w * 2 / 5, s.px(4.0)), st, ink);
}

fn ellipsize(f: &Fonts, face: Face, size: f32, s: &str, max: f32) -> String {
    if f.width(face, size, s) <= max {
        return s.to_owned();
    }
    let mut out: String = s.to_owned();
    while !out.is_empty() && f.width(face, size, &format!("{out}\u{2026}")) > max {
        out.pop();
    }
    format!("{out}\u{2026}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 1440;
    const H: u32 = 1920;

    fn model<'a>(b: &'a Buffer, k: &'a Keyboard, s: &'a SaveState) -> Model<'a> {
        Model {
            file_name: "field-notes.md",
            buffer: b,
            keyboard: k,
            keys_on: true,
            save: s,
            theme: Theme::DAY,
            rename: None,
            sel_ui: None,
        }
    }

    #[test]
    fn layout_fits_forma_and_targets_are_large() {
        let e = EditorScreen::new(W, H);
        let k = Keyboard::with_layout(sigma_core::keyboard::Layout::Compact);
        let l = e.layout(true, &k);
        assert!(l.page.h > 900, "page height {}", l.page.h);
        assert_eq!(l.keys.len(), 10 + 9 + 9 + 6);
        for (r, _) in &l.keys {
            assert!(r.w >= 50 && r.h >= 50, "target {r:?} under 44 brief px");
            assert!(r.x >= 0 && r.x + r.w <= W as i32 && r.y + r.h <= H as i32);
        }
        assert!(l.keys_button.w >= 50 && l.keys_button.h >= 50);
    }

    #[test]
    fn full_layout_fits_portrait_with_big_keys_and_usable_page() {
        let e = EditorScreen::new(W, H);
        let k = Keyboard::with_layout(sigma_core::keyboard::Layout::Full);
        let l = e.layout(true, &k);
        assert_eq!(l.keys.len(), 15 + 15 + 14 + 14 + 9);
        let narrowest = l.keys.iter().map(|(r, _)| r.w).min().unwrap();
        assert!(narrowest >= 80, "1-unit key {narrowest}px");
        for (r, _) in &l.keys {
            assert!(
                r.h >= 60 && r.x >= 0 && r.x + r.w <= W as i32 && r.y + r.h <= H as i32,
                "{r:?}"
            );
        }
        // No overlaps, and ↑ sits above ↓.
        for (i, (a, _)) in l.keys.iter().enumerate() {
            for (b, _) in &l.keys[i + 1..] {
                assert!(
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y,
                    "{a:?} {b:?}"
                );
            }
        }
        let find = |d| {
            l.keys
                .iter()
                .find(|(_, key)| *key == Key::Arrow(d))
                .unwrap()
                .0
        };
        use sigma_core::keyboard::Dir;
        assert_eq!(find(Dir::Up).x, find(Dir::Down).x);
        assert!(l.keyboard.h <= H as i32 / 4, "keyboard {}px", l.keyboard.h);
        assert!(l.page.h >= 1300, "page {}px", l.page.h);
        let c = e.layout(
            true,
            &Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
        );
        let none = e.layout(false, &k);
        println!(
            "METRICS full kb {} page {} key {}x{} | compact kb {} page {} key {}x{} | hidden page {}",
            l.keyboard.h, l.page.h, narrowest, l.keys[0].0.h,
            c.keyboard.h, c.page.h, c.keys[0].0.w, c.keys[0].0.h, none.page.h
        );
        let (b, s) = (Buffer::new(""), SaveState::Saved);
        let m = model(&b, &k, &s);
        for (r, key) in &l.keys {
            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Key(*key));
        }
    }

    #[test]
    fn selection_handles_and_bar_are_placed_and_hittable() {
        let mut e = EditorScreen::new(W, H);
        let k = Keyboard::default();
        let s = SaveState::Saved;
        let mut b = Buffer::new("one\ntwo\nthree\nfour\nsecond line with words\nthird");
        let at = b.text().find("with").unwrap();
        b.select(at, at + 4);
        let mut m = model(&b, &k, &s);
        m.sel_ui = Some(SelUi { bar: true });
        let mut c = Canvas::new(W, H, 255);
        let l = e.draw(&mut c, &m);
        let g = e.selection_geometry(&m, &l);
        let [Some(h0), Some(h1)] = g.handles else {
            panic!("handles {g:?}")
        };
        assert!(h0.w >= 50 && h0.x < h1.x && h0.y == h1.y);
        let labels: Vec<_> = g.buttons.iter().map(|(_, a)| a.label()).collect();
        assert_eq!(labels, ["Cut", "Copy", "Paste", "Select all"]);
        let bar = g.bar.unwrap();
        assert!(bar.y + bar.h <= h0.y && bar.x >= 0 && bar.x + bar.w <= W as i32 && bar.h >= 50);
        for (r, a) in &g.buttons {
            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Bar(*a));
        }
        assert_eq!(e.hit(&m, h1.x + h1.w / 2, h1.y + h1.h / 2), Hit::Handle(1));
        // Offsets round-trip through screen points (char snapping).
        let (x, y) = e.point_of(&b, l.page, at).unwrap();
        assert_eq!(e.offset_at_point(&b, l.page, x + 1, y + 5), at);
        // First line: no room above, bar goes below the handles.
        let mut b2 = Buffer::new("first line of text");
        b2.select(0, 5);
        let mut m2 = model(&b2, &k, &s);
        m2.sel_ui = Some(SelUi { bar: true });
        let g2 = e.selection_geometry(&m2, &l);
        assert!(g2.bar.unwrap().y > g2.handles[1].unwrap().y);
        // Caret only: Paste and Select all, no handles.
        let b3 = Buffer::new("x  y");
        let mut m3 = model(&b3, &k, &s);
        m3.sel_ui = Some(SelUi { bar: true });
        let g3 = e.selection_geometry(&m3, &l);
        assert_eq!(g3.handles, [None, None]);
        assert_eq!(
            g3.buttons.iter().map(|(_, a)| *a).collect::<Vec<_>>(),
            [BarAction::Paste, BarAction::SelectAll]
        );
    }

    #[test]
    fn every_key_is_hittable_at_its_centre() {
        let e = EditorScreen::new(W, H);
        let (b, k, s) = (
            Buffer::new(""),
            Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
            SaveState::Saved,
        );
        let m = model(&b, &k, &s);
        for (r, key) in e.layout(true, &k).keys {
            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Key(key));
        }
    }

    #[test]
    fn tap_on_page_places_caret() {
        let mut e = EditorScreen::new(W, H);
        let (b, k, s) = (
            Buffer::new("first line\nsecond line"),
            Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
            SaveState::Saved,
        );
        let m = model(&b, &k, &s);
        let mut c = Canvas::new(W, H, 255);
        let l = e.draw(&mut c, &m);
        let area = e.text_area(l.page);
        match e.hit(&m, area.x + 1, area.y + e.line_h() + e.line_h() / 2) {
            Hit::Caret(at) => assert_eq!(at, 11),
            other => panic!("{other:?}"),
        }
        match e.hit(&m, area.x + area.w - 1, area.y + 2) {
            Hit::Caret(at) => assert_eq!(at, 10),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn about_twelve_words_per_line_at_defaults() {
        let e = EditorScreen::new(W, H);
        let text =
            "the quick brown fox jumps over the lazy dog and keeps on running far away ".repeat(5);
        let b = Buffer::new(text.clone());
        let rows = e.rows(
            &b,
            e.layout(
                true,
                &Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
            )
            .page,
        );
        let words = text[rows[0].start..rows[0].end].split_whitespace().count();
        // Brief defaults (20 px at 1264, margin 28) with Line off give a long
        // measure on the 8" Forma; Line mode (S4) fits 8–16 words.
        assert!((12..=30).contains(&words), "{words} words on the first row");
    }

    #[test]
    fn long_note_scrolls_with_caret_and_page_turn() {
        let mut e = EditorScreen::new(W, H);
        let mut b = Buffer::new("line\n".repeat(300));
        let k = Keyboard::with_layout(sigma_core::keyboard::Layout::Compact);
        let page = e.layout(true, &k).page;
        e.follow_caret(&b, page);
        assert!(e.scroll > 250);
        let before = e.scroll;
        e.page_turn(&mut b, page, false);
        assert!(e.scroll < before);
        e.page_turn(&mut b, page, true);
        assert_eq!(e.scroll, before.min(e.scroll));
    }

    #[test]
    fn eye_and_pencil_hit_and_preview_draws_without_keys() {
        let b = Buffer::new("# Title\n\nBody *em*");
        let k = Keyboard::default();
        let st = SaveState::Saved;
        let mut e = EditorScreen::new(W, H);
        let m = model(&b, &k, &st);
        let l = e.layout(true, &k);
        assert!(l.preview_button.x + l.preview_button.w < l.files_button.x);
        assert!(l.preview_button.x > l.name_rect.x + l.name_rect.w);
        let (cx, cy) = (l.preview_button.x + 20, l.preview_button.y + 20);
        assert_eq!(e.hit(&m, cx, cy), Hit::TogglePreview);
        assert_eq!(e.preview_hit(&k, cx, cy), Hit::TogglePreview);
        assert_eq!(e.preview_hit(&k, 700, 900), Hit::Nothing);
        let doc = e.build_preview(b.text(), &k, &|_| false);
        assert_eq!(doc.pages.len(), 1);
        let mut c = Canvas::new(W, H, 0);
        let pl = e.draw_preview(
            &mut c,
            &PreviewModel {
                file_name: "a.md",
                doc: &doc,
                page: 0,
                words: 3,
                save: &st,
                keyboard: &k,
                theme: Theme::DAY,
            },
        );
        assert!(pl.keys.is_empty(), "no keyboard in preview");
        // Page and status bar are drawn; nothing below the status bar.
        assert!(c.pixels.iter().any(|&p| p < 80));
        let _ = e.draw(&mut c, &m);
    }

    fn no_overlap(keys: &[(Rect, Key)]) {
        for (i, (a, _)) in keys.iter().enumerate() {
            for (b, _) in &keys[i + 1..] {
                assert!(
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y,
                    "{a:?} {b:?}"
                );
            }
        }
    }

    fn landscape(
        mode: LandKeys,
        pad: Pad,
        side: Side,
        layout: sigma_core::keyboard::Layout,
    ) -> (EditorScreen, Keyboard, Layout) {
        let mut e = EditorScreen::new(H, W);
        let mut st = Settings {
            land_keys: mode,
            pad,
            pad_side: side,
            ..Settings::default()
        };
        e.configure(&mut st, None);
        let k = Keyboard::with_layout(layout);
        let l = e.layout(true, &k);
        (e, k, l)
    }

    #[test]
    fn landscape_keyboards_fit_hit_and_keep_portrait_key_size() {
        use sigma_core::keyboard::Layout as L;
        let portrait = EditorScreen::new(W, H).layout(true, &Keyboard::with_layout(L::Compact));
        let unit_w = portrait.keys[0].0.w;
        for layout in [L::Compact, L::Full] {
            for mode in [LandKeys::Centre, LandKeys::Split, LandKeys::Full] {
                for pad in [Pad::Numbers, Pad::Arrows] {
                    for side in [Side::Left, Side::Right] {
                        let (e, k, l) = landscape(mode, pad, side, layout);
                        no_overlap(&l.keys);
                        let (b, s) = (Buffer::new(""), SaveState::Saved);
                        let m = model(&b, &k, &s);
                        for (r, key) in &l.keys {
                            assert!(
                                r.x >= 5 && r.x + r.w <= H as i32 - 5 && r.y + r.h <= W as i32,
                                "{r:?}"
                            );
                            assert!(r.w >= 50 && r.h >= 50, "{r:?}");
                            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Key(*key));
                        }
                        for (r, b) in &l.kb_buttons {
                            assert!(r.y >= l.keyboard.y && r.x + r.w <= H as i32);
                            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::KbButton(*b));
                        }
                        assert!(l.page.h >= 800, "{mode:?} page {}", l.page.h);
                        if layout == L::Compact {
                            let q = l.keys.iter().find(|(_, k)| *k == Key::Char('q')).unwrap().0;
                            let tol = if mode == LandKeys::Full {
                                unit_w / 20
                            } else {
                                1
                            };
                            assert!((q.w - unit_w).abs() <= tol, "unit {} vs {unit_w}", q.w);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn split_has_a_gap_and_space_on_both_halves() {
        use sigma_core::keyboard::Layout as L;
        let (_, _, l) = landscape(LandKeys::Split, Pad::Numbers, Side::Right, L::Compact);
        let spaces: Vec<_> = l.keys.iter().filter(|(_, k)| *k == Key::Space).collect();
        assert_eq!(spaces.len(), 2);
        let t = l.keys.iter().find(|(_, k)| *k == Key::Char('t')).unwrap().0;
        let y = l.keys.iter().find(|(_, k)| *k == Key::Char('y')).unwrap().0;
        assert!(y.x - (t.x + t.w) > 2 * t.w, "clear gap between halves");
        let shift = l.keys.iter().find(|(_, k)| *k == Key::Shift).unwrap().0;
        let back = l.keys.iter().find(|(_, k)| *k == Key::Back).unwrap().0;
        assert!(shift.x < t.x && back.x > y.x);
    }

    #[test]
    fn full_pad_sits_on_the_chosen_side_with_the_right_keys() {
        use sigma_core::keyboard::Layout as L;
        let (_, _, l) = landscape(LandKeys::Full, Pad::Numbers, Side::Right, L::Compact);
        let seven = l.keys.iter().find(|(_, k)| *k == Key::Char('7')).unwrap().0;
        let p = l.keys.iter().find(|(_, k)| *k == Key::Char('p')).unwrap().0;
        assert!(seven.x > p.x + p.w);
        let (_, _, l) = landscape(LandKeys::Full, Pad::Arrows, Side::Left, L::Compact);
        let up = l
            .keys
            .iter()
            .find(|(_, k)| *k == Key::Arrow(Dir::Up))
            .unwrap()
            .0;
        let down = l
            .keys
            .iter()
            .find(|(_, k)| *k == Key::Arrow(Dir::Down))
            .unwrap()
            .0;
        let q = l.keys.iter().find(|(_, k)| *k == Key::Char('q')).unwrap().0;
        assert!(up.x + up.w < q.x && up.x == down.x && up.y < down.y);
        assert_eq!(l.kb_buttons.len(), 5);
    }

    #[test]
    fn line_mode_holds_n_words_in_both_orientations_with_real_fonts() {
        for face in [Typeface::Literata, Typeface::Atkinson] {
            for n in 8..=16u8 {
                let mut st = Settings {
                    line_on: true,
                    line_words: n,
                    face,
                    ..Settings::default()
                };
                let k = Keyboard::default();
                let mut p = EditorScreen::new(W, H);
                p.configure(&mut st, None);
                let pp = p.layout(true, &k).page;
                assert_eq!(
                    p.sample_words_per_row(pp),
                    usize::from(n),
                    "portrait {face:?} {n}"
                );
                let mut l = EditorScreen::new(H, W);
                l.configure(&mut st, None);
                let lp = l.layout(true, &k);
                assert_eq!(
                    l.sample_words_per_row(lp.page),
                    usize::from(n),
                    "landscape {face:?} {n}"
                );
                assert_eq!(lp.column.w, W as i32, "column locked to the portrait width");
                assert!(lp.column.x > 0, "gutters beside the column");
            }
        }
    }

    #[test]
    fn night_draws_inverted_and_key_tone_can_differ() {
        let mut e = EditorScreen::new(W, H);
        let mut st = Settings {
            paper: sigma_core::settings::Paper::Night,
            key_tone: KeyTone::Light,
            ..Settings::default()
        };
        e.configure(&mut st, None);
        let (b, k, s) = (Buffer::new("night"), Keyboard::default(), SaveState::Saved);
        let mut m = model(&b, &k, &s);
        m.theme = Theme::NIGHT;
        let mut c = Canvas::new(W, H, 128);
        let l = e.draw(&mut c, &m);
        let px = |x: i32, y: i32| c.pixels[(y * W as i32 + x) as usize];
        assert_eq!(
            px(5, l.page.y + 5),
            Theme::NIGHT.paper,
            "night page, not a grey card"
        );
        assert_eq!(px(2, l.keyboard.y + 3), Theme::DAY.paper, "light keyboard");
        assert!(e.status_right(Theme::NIGHT, &s).starts_with("Night"));
        st.usb = true;
        e.configure(&mut st, None);
        assert!(e.status_right(Theme::NIGHT, &s).starts_with("USB"));
    }
}
