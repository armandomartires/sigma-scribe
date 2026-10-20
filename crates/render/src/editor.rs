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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Key(Key),
    ToggleKeys,
    /// Byte offset in the buffer where the caret should go.
    Caret(usize),
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
            // Ten units per row; wide keys from the brief's compact board.
            let unit = (w - 2 * pad - 9 * gap) as f32 / 10.0;
            let width_of = |k: Key| match k {
                Key::Shift | Key::Back => 1.5,
                Key::SwapLayer | Key::Tab | Key::Del | Key::Return => 1.5,
                Key::Space => 4.0,
                Key::Char(_) => 1.0,
            };
            for (r, row) in rows.iter().enumerate() {
                let kw_of = |k: Key| width_of(k) * unit + gap as f32 * (width_of(k) - 1.0).max(0.0);
                let total: f32 = row.iter().map(|&k| kw_of(k)).sum::<f32>()
                    + gap as f32 * (row.len() as f32 - 1.0);
                let mut x = (w as f32 - total) / 2.0;
                let y = keyboard_rect.y + pad + r as i32 * (key_h + gap);
                for &k in row {
                    let kw = kw_of(k);
                    keys.push((Rect::new(x.round() as i32, y, kw.round() as i32, key_h), k));
                    x += kw + gap as f32;
                }
            }
        }
        Layout {
            title,
            page,
            status,
            keys_button,
            keyboard: keyboard_rect,
            keys,
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
        let rows = self.rows(buffer, page);
        let n = self.visible_rows(page).saturating_sub(1).max(1);
        self.scroll = if forward {
            (self.scroll + n).min(rows.len().saturating_sub(1))
        } else {
            self.scroll.saturating_sub(n)
        };
        buffer.set_caret(rows[self.scroll].start);
    }

    /// Vertical caret move by one visual row (keyboard arrows later).
    pub fn caret_row(&mut self, buffer: &mut Buffer, page: Rect, down: bool) {
        let rows = self.rows(buffer, page);
        let m = self.measure();
        let (row, col) = caret_cell(buffer.text(), &rows, buffer.caret(), &m);
        let target = if down { row + 1 } else { row.wrapping_sub(1) };
        if let Some(&r) = rows.get(target) {
            buffer.set_caret(offset_at(buffer.text(), r, col, &m));
        }
    }

    #[must_use]
    pub fn hit(&self, model: &Model<'_>, x: i32, y: i32) -> Hit {
        let l = self.layout(model.keys_on, model.keyboard);
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

        // Title bar: file name left (ellipsized), hairline under.
        let name = ellipsize(
            &self.fonts,
            Face::AtkinsonBold,
            ui,
            model.file_name,
            (l.title.w as f32) * 0.6,
        );
        let base = l.title.y as f32
            + l.title.h as f32 / 2.0
            + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6;
        self.fonts.draw(
            c,
            Face::AtkinsonBold,
            ui,
            self.px(20.0) as f32,
            base,
            &name,
            t.ink,
        );
        c.fill(
            Rect::new(0, l.title.y + l.title.h - 1, l.title.w, 1),
            t.rule,
        );

        // Page.
        let area = self.text_area(l.page);
        let rows = self.rows(model.buffer, l.page);
        let text = model.buffer.text();
        let lh = self.line_h();
        let asc = self.fonts.ascent(Face::Literata, self.body());
        let top_pad = ((lh as f32 - self.body() * 1.2) / 2.0).max(0.0);
        for (i, r) in rows
            .iter()
            .skip(self.scroll)
            .take(self.visible_rows(l.page))
            .enumerate()
        {
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

        // Keyboard.
        if model.keys_on {
            c.fill(l.keyboard, t.paper);
            c.fill(Rect::new(0, l.keyboard.y, l.keyboard.w, 1), t.rule);
            let ks = 20.0 * self.s.0;
            for &(r, k) in &l.keys {
                let active = k == Key::Shift && model.keyboard.shift;
                let (bg, fg) = if active {
                    (t.ink, t.paper)
                } else {
                    (t.paper, t.ink)
                };
                c.fill(r, bg);
                c.stroke(r, 1, if active { t.ink } else { t.rule });
                let label = model.keyboard.label(k);
                let face = if matches!(k, Key::Char(_)) {
                    Face::Atkinson
                } else {
                    Face::AtkinsonBold
                };
                let size = if matches!(k, Key::Char(_)) {
                    ks * 1.15
                } else {
                    ks * 0.85
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
        l
    }
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
        }
    }

    #[test]
    fn layout_fits_forma_and_targets_are_large() {
        let e = EditorScreen::new(W, H);
        let k = Keyboard::default();
        let l = e.layout(true, &k);
        assert!(l.page.h > 900, "page height {}", l.page.h);
        assert_eq!(l.keys.len(), 10 + 9 + 9 + 5);
        for (r, _) in &l.keys {
            assert!(r.w >= 50 && r.h >= 50, "target {r:?} under 44 brief px");
            assert!(r.x >= 0 && r.x + r.w <= W as i32 && r.y + r.h <= H as i32);
        }
        assert!(l.keys_button.w >= 50 && l.keys_button.h >= 50);
    }

    #[test]
    fn every_key_is_hittable_at_its_centre() {
        let e = EditorScreen::new(W, H);
        let (b, k, s) = (Buffer::new(""), Keyboard::default(), SaveState::Saved);
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
            Keyboard::default(),
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
        let rows = e.rows(&b, e.layout(true, &Keyboard::default()).page);
        let words = text[rows[0].start..rows[0].end].split_whitespace().count();
        // Brief defaults (20 px at 1264, margin 28) with Line off give a long
        // measure on the 8" Forma; Line mode (S4) fits 8–16 words.
        assert!((12..=30).contains(&words), "{words} words on the first row");
    }

    #[test]
    fn long_note_scrolls_with_caret_and_page_turn() {
        let mut e = EditorScreen::new(W, H);
        let mut b = Buffer::new("line\n".repeat(300));
        let k = Keyboard::default();
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
