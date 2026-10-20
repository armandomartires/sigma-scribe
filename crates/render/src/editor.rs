//! The v0.1 screen (brief §3, §4, §7): title bar, page, status bar,
//! on-screen keyboard. Layout, drawing and hit-testing live together so a tap
//! always lands on what was drawn.

use crate::canvas::{Canvas, Rect};
use crate::fonts::{Face, Fonts, Measure};
use crate::theme::{Scale, Theme};
use sigma_core::keyboard::Key;
use sigma_core::wrap::{caret_cell, offset_at, wrap, VisualLine};
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
    /// Tap on the file name: rename in place.
    StartRename,
    RenameOk,
    RenameCancel,
    Nothing,
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
    pub name_rect: Rect,
    pub ok_rect: Rect,
    pub cancel_rect: Rect,
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
        }
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
            face: Face::Literata,
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
        let kb_h = if keys_on {
            pad * 2 + key_h * rows.len() as i32 + gap * (rows.len() as i32 - 1)
        } else {
            0
        };
        let keyboard_rect = Rect::new(0, h - kb_h, w, kb_h);
        let status_h = self.px(44.0) + 2 * self.px(4.0);
        let status = Rect::new(0, keyboard_rect.y - status_h, w, status_h);
        let page = Rect::new(0, title.h, w, status.y - title.h);
        let kb_w = self.px(64.0);
        let keys_button = Rect::new(
            w - self.px(12.0) - kb_w,
            status.y + (status_h - self.px(44.0)) / 2,
            kb_w,
            self.px(44.0),
        );
        let mut keys = Vec::new();
        if keys_on {
            // Grid units (10 Compact, 16 Full); rows are centred, columns
            // align because every key starts at its cumulative unit offset.
            let grid = keyboard.grid_units();
            let unit = (w - 2 * pad + gap) as f32 / grid;
            for (r, row) in rows.iter().enumerate() {
                let total: f32 = row
                    .iter()
                    .enumerate()
                    .map(|(c, &k)| keyboard.units(k, r, c))
                    .sum();
                let mut x = pad as f32 + (grid - total) / 2.0 * unit;
                let y = keyboard_rect.y + pad + r as i32 * (key_h + gap);
                for (c, &k) in row.iter().enumerate() {
                    let kw = keyboard.units(k, r, c) * unit;
                    let x0 = x.round() as i32;
                    let x1 = (x + kw).round() as i32 - gap;
                    keys.push((Rect::new(x0, y, x1 - x0, key_h), k));
                    x += kw;
                }
            }
        }
        let btn = self.px(44.0);
        let by = (title.h - btn) / 2;
        let files_button = Rect::new(w - self.px(12.0) - btn, by, btn, btn);
        let name_rect = Rect::new(0, 0, w * 3 / 5, title.h);
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
            name_rect,
            ok_rect,
            cancel_rect,
        }
    }

    fn text_area(&self, page: Rect) -> Rect {
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
        let ui = 17.0 * self.s.0;

        self.draw_title(c, &l, model.file_name, model.rename, t, true);

        // Page.
        let area = self.text_area(l.page);
        let rows = self.rows(model.buffer, l.page);
        let text = model.buffer.text();
        let lh = self.line_h();
        let asc = self.fonts.ascent(Face::Literata, self.body());
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
                        Face::Literata,
                        self.body(),
                        &text[r.start..s0.min(r.end).max(r.start)],
                    );
                    let x1 =
                        self.fonts
                            .width(Face::Literata, self.body(), &text[r.start..s1.max(s0)])
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
                Face::Literata,
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
        let right = format!(
            "{} \u{b7} {}",
            if t == Theme::NIGHT { "Night" } else { "Paper" },
            model.save.label()
        );
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
        c.fill(l.keyboard, t.paper);
        c.fill(Rect::new(0, l.keyboard.y, l.keyboard.w, 1), t.rule);
        let ks = 20.0 * self.s.0;
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

fn draw_arrow(c: &mut Canvas, r: Rect, d: sigma_core::keyboard::Dir, s: Scale, ink: u8) {
    use sigma_core::keyboard::Dir;
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
}
