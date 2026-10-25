//! The side drawer (brief §5): one panel on the left or right of the page,
//! about 48% wide and at most 320 brief px. It covers the page, does not
//! push it, and does not animate. Up to three panes, top to bottom:
//! Contents, To-dos, Scratch; open panes share the height equally with a
//! 1 px ink rule between them.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::{Face, Measure};
use crate::i18n::tr;
use crate::theme::Theme;
use sigma_core::outline::{Filter, Heading, TodoItem};
use sigma_core::settings::Side;
use sigma_core::wrap::{caret_cell, offset_at, wrap, VisualLine};
use sigma_core::Buffer;
use sigma_plugin_api::Frame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Contents,
    Todos,
    Scratch,
}

impl Pane {
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Contents => "Contents",
            Self::Todos => "To-dos",
            Self::Scratch => "Scratch",
        }
    }
}

pub const EMPTY_CONTENTS: &str = "No headings in this note. Start a line with #.";
pub const EMPTY_TODOS: &str = "No to-dos. Start a line with - [ ].";
pub const SCRATCH_PLACEHOLDER: &str = "A side note. It stays off the page.";

pub struct DrawerModel<'a> {
    pub side: Side,
    pub contents: Option<&'a [Heading]>,
    /// All to-dos and the filter (the layout applies it).
    pub todos: Option<(&'a [TodoItem], Filter)>,
    /// The scratch buffer and whether it has the keyboard.
    pub scratch: Option<(&'a Buffer, bool)>,
    /// Panes drawn by a plugin, indexed Contents, To-dos, Scratch: the
    /// plugin's latest frame takes over the pane (header included) and the
    /// host keeps the drawer's background, edge and rules. A Scratch frame
    /// places the text field the host still draws and edits.
    pub plugin: [Option<&'a Frame>; 3],
    pub theme: Theme,
}

impl Pane {
    /// Position in [`DrawerModel::plugin`] and the title bar's pane list.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Contents => 0,
            Self::Todos => 1,
            Self::Scratch => 2,
        }
    }
}

impl DrawerModel<'_> {
    #[must_use]
    pub fn panes(&self) -> Vec<Pane> {
        let mut v = Vec::new();
        if self.contents.is_some() {
            v.push(Pane::Contents);
        }
        if self.todos.is_some() {
            v.push(Pane::Todos);
        }
        if self.scratch.is_some() {
            v.push(Pane::Scratch);
        }
        v
    }
}

#[derive(Clone, Debug, Default)]
pub struct DrawerLayout {
    pub rect: Rect,
    /// Pane, header band, body.
    pub panes: Vec<(Pane, Rect, Rect)>,
    /// Heading rows and their source line.
    pub headings: Vec<(Rect, usize)>,
    /// To-do rows: box target, text target, index into the full list.
    pub todos: Vec<(Rect, Rect, usize)>,
    pub filter: Option<Rect>,
    /// Rows that did not fit (Contents, To-dos).
    pub hidden: [usize; 2],
    /// Scratch text area.
    pub scratch: Option<Rect>,
    /// Panes a plugin draws, with their whole rectangle (header included).
    pub plugin: Vec<(Pane, Rect)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawerHit {
    Heading(usize),
    TodoBox(usize),
    TodoOpen(usize),
    Filter,
    /// Byte offset in the scratch buffer.
    Scratch(usize),
    /// A tap inside a pane a plugin draws (not in its Scratch field): the
    /// plugin decides what it means.
    Plugin(Pane),
    /// Inside the drawer but on nothing (swallowed, not the page's).
    Inside,
    Outside,
}

impl EditorScreen {
    fn d(&self, v: f32) -> i32 {
        self.scale().px(v)
    }

    fn scratch_measure(&self) -> Measure<'_> {
        Measure {
            fonts: &self.fonts,
            face: Face::Atkinson,
            size: self.ui_px(),
        }
    }

    /// The drawer rectangle over the page of `base`.
    #[must_use]
    pub fn drawer_rect(&self, base: &Layout, side: Side) -> Rect {
        let p = base.page;
        let w = ((p.w as f32 * 0.48) as i32).min(self.d(320.0));
        let x = match side {
            Side::Left => p.x,
            Side::Right => p.x + p.w - w,
        };
        Rect::new(x, p.y, w, p.h)
    }

    /// The whole rectangle of each open pane (header included), top to
    /// bottom: what a pane plugin is told it may draw in.
    #[must_use]
    pub fn drawer_pane_rects(
        &self,
        base: &Layout,
        side: Side,
        open: [bool; 3],
    ) -> Vec<(Pane, Rect)> {
        let rect = self.drawer_rect(base, side);
        let panes: Vec<Pane> = [Pane::Contents, Pane::Todos, Pane::Scratch]
            .into_iter()
            .filter(|p| open[p.index()])
            .collect();
        let n = panes.len() as i32;
        panes
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                let y0 = rect.y + rect.h * i as i32 / n;
                let y1 = rect.y + rect.h * (i as i32 + 1) / n;
                (p, Rect::new(rect.x, y0, rect.w, y1 - y0))
            })
            .collect()
    }

    #[must_use]
    pub fn drawer_layout(&self, base: &Layout, m: &DrawerModel<'_>) -> DrawerLayout {
        let rect = self.drawer_rect(base, m.side);
        let panes = m.panes();
        let mut l = DrawerLayout {
            rect,
            ..DrawerLayout::default()
        };
        if panes.is_empty() {
            return l;
        }
        let n = panes.len() as i32;
        let pad = self.d(12.0);
        let head_h = self.d(44.0);
        for (i, pane) in panes.iter().enumerate() {
            let y0 = rect.y + rect.h * i as i32 / n;
            let y1 = rect.y + rect.h * (i as i32 + 1) / n;
            let header = Rect::new(rect.x, y0, rect.w, head_h);
            let body = Rect::new(
                rect.x + pad,
                y0 + head_h,
                rect.w - 2 * pad,
                y1 - y0 - head_h,
            );
            l.panes.push((*pane, header, body));
            let frame = m.plugin[pane.index()];
            if let Some(f) = frame {
                let whole = Rect::new(rect.x, y0, rect.w, y1 - y0);
                l.plugin.push((*pane, whole));
                if *pane == Pane::Scratch {
                    l.scratch = f
                        .field
                        .as_ref()
                        .map(|fl| {
                            let r = fl.rect;
                            whole.intersect(&Rect::new(r.x, r.y, r.w, r.h))
                        })
                        .filter(|r| r.w > 0 && r.h > 0);
                }
                continue;
            }
            match pane {
                Pane::Contents => {
                    // 44 px rows: the touch-target minimum (S7 audit).
                    let row_h = self.d(44.0);
                    let hs = m.contents.unwrap_or_default();
                    let fit = (body.h / row_h).max(0) as usize;
                    for (k, h) in hs.iter().take(fit).enumerate() {
                        l.headings.push((
                            Rect::new(body.x, body.y + k as i32 * row_h, body.w, row_h),
                            h.line,
                        ));
                    }
                    l.hidden[0] = hs.len().saturating_sub(fit);
                }
                Pane::Todos => {
                    let (items, filter) = m.todos.unwrap_or((&[], Filter::All));
                    let fw = self.d(72.0);
                    l.filter = Some(Rect::new(rect.x + rect.w - pad - fw, y0, fw, head_h));
                    let row_h = self.d(58.0);
                    let fit = (body.h / row_h).max(0) as usize;
                    let shown: Vec<usize> = (0..items.len())
                        .filter(|&i| filter.keeps(items[i].todo.done))
                        .collect();
                    for (k, &i) in shown.iter().take(fit).enumerate() {
                        let y = body.y + k as i32 * row_h;
                        let bx = self.d(44.0);
                        l.todos.push((
                            Rect::new(body.x, y, bx, row_h),
                            Rect::new(body.x + bx, y, body.w - bx, row_h),
                            i,
                        ));
                    }
                    l.hidden[1] = shown.len().saturating_sub(fit);
                }
                Pane::Scratch => l.scratch = Some(body),
            }
        }
        l
    }

    fn scratch_rows(&self, b: &Buffer, area: Rect) -> (Vec<VisualLine>, usize) {
        let m = self.scratch_measure();
        let rows = wrap(b.text(), area.w.max(1) as u32, &m);
        let lh = self.scratch_line_h();
        let fit = (area.h / lh).max(1) as usize;
        let (crow, _) = caret_cell(b.text(), &rows, b.caret(), &m);
        let first = (crow + 1).saturating_sub(fit);
        (rows, first)
    }

    fn scratch_line_h(&self) -> i32 {
        (self.ui_px() * 1.45).round() as i32
    }

    #[must_use]
    pub fn drawer_hit(&self, base: &Layout, m: &DrawerModel<'_>, x: i32, y: i32) -> DrawerHit {
        let l = self.drawer_layout(base, m);
        if l.panes.is_empty() || !l.rect.contains(x, y) {
            return DrawerHit::Outside;
        }
        if l.filter
            .is_some_and(|r| r.inset(-self.d(4.0)).contains(x, y))
        {
            return DrawerHit::Filter;
        }
        if let Some((_, line)) = l.headings.iter().find(|(r, _)| r.contains(x, y)) {
            return DrawerHit::Heading(*line);
        }
        for (b, t, i) in &l.todos {
            if b.contains(x, y) {
                return DrawerHit::TodoBox(*i);
            }
            if t.contains(x, y) {
                return DrawerHit::TodoOpen(*i);
            }
        }
        if let (Some(area), Some((b, _))) = (l.scratch, m.scratch) {
            if area.contains(x, y) {
                let (rows, first) = self.scratch_rows(b, area);
                let r = first + ((y - area.y) / self.scratch_line_h()) as usize;
                let at = rows.get(r).or(rows.last()).map_or(0, |row| {
                    offset_at(
                        b.text(),
                        *row,
                        (x - area.x).max(0) as u32,
                        &self.scratch_measure(),
                    )
                });
                return DrawerHit::Scratch(at);
            }
        }
        if let Some((p, _)) = l.plugin.iter().find(|(_, r)| r.contains(x, y)) {
            return DrawerHit::Plugin(*p);
        }
        DrawerHit::Inside
    }

    #[allow(clippy::too_many_lines)]
    pub fn draw_drawer(&self, c: &mut Canvas, base: &Layout, m: &DrawerModel<'_>) -> DrawerLayout {
        let l = self.drawer_layout(base, m);
        if l.panes.is_empty() {
            return l;
        }
        let t = m.theme;
        let ui = self.ui_px();
        let small = ui * 0.82;
        let r = l.rect;
        c.fill(r, t.paper);
        // The edge facing the page.
        let edge = match m.side {
            Side::Left => r.x + r.w - 1,
            Side::Right => r.x,
        };
        c.fill(Rect::new(edge, r.y, 1, r.h), t.ink);
        let edge_rect = Rect::new(edge, r.y, 1, r.h);
        let base_of = |rr: Rect, face: Face, size: f32| {
            rr.y as f32 + rr.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6
        };
        let fit_text = |s: &str, face: Face, size: f32, max: f32| -> String {
            if self.fonts.width(face, size, s) <= max {
                return s.to_owned();
            }
            let mut o = s.to_owned();
            while !o.is_empty() && self.fonts.width(face, size, &format!("{o}\u{2026}")) > max {
                o.pop();
            }
            format!("{o}\u{2026}")
        };
        for (i, (pane, header, body)) in l.panes.iter().enumerate() {
            if i > 0 {
                c.fill(Rect::new(r.x, header.y, r.w, 1), t.ink);
            }
            let plugin = l.plugin.iter().find(|(p, _)| p == pane).map(|(_, r)| *r);
            let body = &match (plugin, pane) {
                (Some(_), Pane::Scratch) => l.scratch.unwrap_or_default(),
                _ => *body,
            };
            if let (Some(whole), Some(f)) = (plugin, m.plugin[pane.index()]) {
                // The plugin draws the pane, header included, below the rule.
                let rule = i32::from(i > 0);
                let below = Rect::new(whole.x, whole.y + rule, whole.w, whole.h - rule);
                self.draw_plugin_pane(c, f, below, t);
                if *pane != Pane::Scratch || l.scratch.is_none() {
                    continue;
                }
            } else {
                self.fonts.draw(
                    c,
                    Face::AtkinsonBold,
                    ui,
                    (r.x + self.d(12.0)) as f32,
                    base_of(*header, Face::AtkinsonBold, ui),
                    &tr(pane.title()),
                    t.ink,
                );
                c.fill(
                    Rect::new(
                        r.x + self.d(12.0),
                        header.y + header.h - 1,
                        r.w - self.d(24.0),
                        1,
                    ),
                    t.rule,
                );
            }
            let empty = |c: &mut Canvas, text: &str| {
                // Wrapped muted hint.
                let m = Measure {
                    fonts: &self.fonts,
                    face: Face::Atkinson,
                    size: small,
                };
                let text: &str = &tr(text);
                let rows = wrap(text, body.w as u32, &m);
                for (k, row) in rows.iter().enumerate() {
                    self.fonts.draw(
                        c,
                        Face::Atkinson,
                        small,
                        body.x as f32,
                        body.y as f32 + self.d(24.0) as f32 + k as f32 * small * 1.4,
                        &text[row.start..row.end],
                        t.mute,
                    );
                }
            };
            match pane {
                Pane::Contents => {
                    let hs = m.contents.unwrap_or_default();
                    if hs.is_empty() {
                        empty(c, EMPTY_CONTENTS);
                    }
                    for ((rr, _), h) in l.headings.iter().zip(hs) {
                        let indent = self.d(14.0) * (i32::from(h.level) - 1).min(3);
                        let face = if h.level == 1 {
                            Face::AtkinsonBold
                        } else {
                            Face::Atkinson
                        };
                        let s = fit_text(&h.text, face, ui, (rr.w - indent) as f32);
                        self.fonts.draw(
                            c,
                            face,
                            ui,
                            (rr.x + indent) as f32,
                            base_of(*rr, face, ui),
                            &s,
                            t.ink,
                        );
                    }
                    if l.hidden[0] > 0 {
                        self.more_line(c, *body, l.hidden[0], t);
                    }
                }
                Pane::Todos => {
                    let (items, filter) = m.todos.unwrap_or((&[], Filter::All));
                    if let Some(f) = l.filter {
                        c.stroke(f, 1, t.ink);
                        let w = self
                            .fonts
                            .width(Face::AtkinsonBold, small, &tr(filter.label()));
                        self.fonts.draw(
                            c,
                            Face::AtkinsonBold,
                            small,
                            f.x as f32 + (f.w as f32 - w) / 2.0,
                            base_of(f, Face::AtkinsonBold, small),
                            &tr(filter.label()),
                            t.ink,
                        );
                    }
                    if l.todos.is_empty() {
                        empty(c, EMPTY_TODOS);
                    }
                    for (bx, tx, idx) in &l.todos {
                        let it = &items[*idx];
                        let side = self.d(18.0);
                        let b = Rect::new(bx.x + self.d(4.0), tx.y + self.d(10.0), side, side);
                        if it.todo.done {
                            c.fill(b, t.ink);
                            // Check mark in paper.
                            for k in 0..side / 2 {
                                let (x0, y0) = (b.x + side / 5 + k / 2, b.y + side / 2 + k / 2);
                                c.fill(Rect::new(x0, y0, 2, 2), t.paper);
                                c.fill(
                                    Rect::new(b.x + side / 2 - 2 + k, b.y + side * 3 / 4 - k, 2, 2),
                                    t.paper,
                                );
                            }
                        } else {
                            c.stroke(b, self.d(1.75).max(2), t.ink);
                        }
                        let (ink, line) = if it.todo.done {
                            (t.mute, true)
                        } else {
                            (t.ink, false)
                        };
                        let y1 = tx.y as f32 + self.d(26.0) as f32;
                        let s = fit_text(&it.todo.text, Face::Atkinson, ui, tx.w as f32);
                        let w = self
                            .fonts
                            .draw(c, Face::Atkinson, ui, tx.x as f32, y1, &s, ink);
                        if line {
                            c.fill(
                                Rect::new(
                                    tx.x,
                                    (y1 - ui * 0.3) as i32,
                                    (w - tx.x as f32) as i32,
                                    1.max(self.d(1.0)),
                                ),
                                t.mute,
                            );
                        }
                        let file = fit_text(it.note.name(), Face::Atkinson, small, tx.w as f32);
                        self.fonts.draw(
                            c,
                            Face::Atkinson,
                            small,
                            tx.x as f32,
                            y1 + small * 1.3,
                            &file,
                            t.mute,
                        );
                    }
                    if l.hidden[1] > 0 {
                        self.more_line(c, *body, l.hidden[1], t);
                    }
                }
                Pane::Scratch => {
                    let Some((b, focused)) = m.scratch else {
                        continue;
                    };
                    let lh = self.scratch_line_h();
                    if b.text().is_empty() {
                        let own = m.plugin[2]
                            .and_then(|f| f.field.as_ref())
                            .map(|f| f.placeholder.as_str());
                        empty(c, own.unwrap_or(SCRATCH_PLACEHOLDER));
                    }
                    let (rows, first) = self.scratch_rows(b, *body);
                    let fit = (body.h / lh).max(1) as usize;
                    let asc = self.fonts.ascent(Face::Atkinson, ui);
                    for (k, row) in rows.iter().skip(first).take(fit).enumerate() {
                        self.fonts.draw(
                            c,
                            Face::Atkinson,
                            ui,
                            body.x as f32,
                            body.y as f32
                                + k as f32 * lh as f32
                                + asc
                                + (lh as f32 - ui * 1.2) / 2.0,
                            &b.text()[row.start..row.end],
                            t.ink,
                        );
                    }
                    if focused {
                        let (crow, ccol) =
                            caret_cell(b.text(), &rows, b.caret(), &self.scratch_measure());
                        if crow >= first && crow < first + fit {
                            c.fill(
                                Rect::new(
                                    body.x + ccol as i32,
                                    body.y + (crow - first) as i32 * lh + self.d(3.0),
                                    2,
                                    lh - self.d(6.0),
                                ),
                                t.ink,
                            );
                        }
                    }
                }
            }
        }
        // A plugin's pane is painted over the whole width; the edge stays.
        if !l.plugin.is_empty() {
            c.fill(edge_rect, t.ink);
        }
        l
    }

    fn more_line(&self, c: &mut Canvas, body: Rect, n: usize, t: Theme) {
        let small = self.ui_px() * 0.82;
        let s = tr(&format!("{n} more\u{2026}")).into_owned();
        self.fonts.draw(
            c,
            Face::Atkinson,
            small,
            body.x as f32,
            (body.y + body.h) as f32 - small * 0.5,
            &s,
            t.mute,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::outline::{headings, Todo};
    use sigma_core::{Keyboard, NotePath};

    fn items() -> Vec<TodoItem> {
        (0..30)
            .map(|i| TodoItem {
                note: NotePath::parse("a.md").unwrap(),
                todo: Todo {
                    line: i,
                    done: i % 2 == 1,
                    text: format!("task {i}"),
                },
            })
            .collect()
    }

    #[test]
    fn panes_split_the_height_and_width_follows_the_brief() {
        for (w, h) in [(1440u32, 1920u32), (1920, 1440)] {
            let e = EditorScreen::new(w, h);
            let base = e.layout(true, &Keyboard::default());
            let hs = headings("# A\n## B\n### C");
            let it = items();
            let b = Buffer::new("note");
            for mask in 1..8u8 {
                for side in [Side::Left, Side::Right] {
                    let m = DrawerModel {
                        plugin: [None; 3],
                        side,
                        contents: (mask & 1 != 0).then_some(hs.as_slice()),
                        todos: (mask & 2 != 0).then_some((it.as_slice(), Filter::All)),
                        scratch: (mask & 4 != 0).then_some((&b, true)),
                        theme: Theme::DAY,
                    };
                    let l = e.drawer_layout(&base, &m);
                    assert_eq!(
                        l.rect.w,
                        ((base.page.w as f32 * 0.48) as i32).min(e.d(320.0))
                    );
                    assert_eq!(l.rect.x == base.page.x, side == Side::Left);
                    let n = mask.count_ones() as i32;
                    assert_eq!(l.panes.len() as i32, n);
                    let total: i32 = l.panes.iter().map(|(_, hd, bd)| hd.h + bd.h).sum();
                    assert!((total - base.page.h).abs() <= n);
                    for w2 in l.panes.windows(2) {
                        assert!((w2[0].2.h - w2[1].2.h).abs() <= 1, "equal shares");
                    }
                    let mut c = Canvas::new(w, h, 255);
                    let _ = e.draw_drawer(&mut c, &base, &m);
                    for (r, line) in &l.headings {
                        assert_eq!(
                            e.drawer_hit(&base, &m, r.x + 5, r.y + r.h / 2),
                            DrawerHit::Heading(*line)
                        );
                    }
                    for (bx, tx, i) in &l.todos {
                        assert_eq!(
                            e.drawer_hit(&base, &m, bx.x + 5, bx.y + 5),
                            DrawerHit::TodoBox(*i)
                        );
                        assert_eq!(
                            e.drawer_hit(&base, &m, tx.x + 20, tx.y + 5),
                            DrawerHit::TodoOpen(*i)
                        );
                    }
                    let outside = if side == Side::Left {
                        l.rect.x + l.rect.w + 5
                    } else {
                        l.rect.x - 5
                    };
                    assert_eq!(
                        e.drawer_hit(&base, &m, outside, l.rect.y + 50),
                        DrawerHit::Outside
                    );
                }
            }
        }
    }

    #[test]
    fn filter_hides_rows_and_overflow_is_counted() {
        let e = EditorScreen::new(1440, 1920);
        let base = e.layout(true, &Keyboard::default());
        let it = items();
        let m = |f| DrawerModel {
            plugin: [None; 3],
            side: Side::Right,
            contents: None,
            todos: Some((it.as_slice(), f)),
            scratch: None,
            theme: Theme::DAY,
        };
        let open = e.drawer_layout(&base, &m(Filter::Open));
        assert!(open.todos.iter().all(|(_, _, i)| !it[*i].todo.done));
        let all = e.drawer_layout(&base, &m(Filter::All));
        assert_eq!(all.todos.len() + all.hidden[1], 30);
        let f = all.filter.unwrap();
        assert_eq!(
            e.drawer_hit(&base, &m(Filter::All), f.x + 3, f.y + 3),
            DrawerHit::Filter
        );
    }

    #[test]
    fn scratch_tap_places_the_caret() {
        let e = EditorScreen::new(1440, 1920);
        let base = e.layout(true, &Keyboard::default());
        let b = Buffer::new("one\ntwo");
        let m = DrawerModel {
            plugin: [None; 3],
            side: Side::Left,
            contents: None,
            todos: None,
            scratch: Some((&b, true)),
            theme: Theme::DAY,
        };
        let a = e.drawer_layout(&base, &m).scratch.unwrap();
        assert_eq!(
            e.drawer_hit(&base, &m, a.x + 1, a.y + e.scratch_line_h() + 3),
            DrawerHit::Scratch(4)
        );
    }
}
