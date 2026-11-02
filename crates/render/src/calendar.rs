//! The Calendar pane of the side drawer (the journal): a date line
//! (`2 | November | 2026`, three fields that open a day, month or year
//! picker in place of the grid), the navigation row (`‹ Today ›`), the
//! weekday initials, six rows of seven days and, while a journal note is
//! open, a footer that steps to the previous and next existing entry.
//!
//! Drawing: today is outlined, days with an entry are bold with a small dot,
//! the open entry is filled (ink background, paper text), days of other
//! months are left empty.

use std::collections::BTreeSet;

use crate::canvas::{Canvas, Rect};
use crate::drawer::DrawerHit;
use crate::editor::EditorScreen;
use crate::fonts::Face;
use crate::graphmap::fill_round;
use crate::i18n::tr;
use crate::icons::{draw_icon, Icon};
use crate::theme::Theme;
use sigma_core::date::{month_grid, Date, WeekStart};

/// Month names, in English (translated with `tr` when drawn).
pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Weekday names, Monday first.
pub const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Weekday initials for the grid's header row, Monday first.
pub const WEEKDAY_INITIALS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Shown in the pane while the clock is not set (no "today").
pub const CLOCK_NOT_SET: &str = "Clock not set";

/// The translated name of month `m` (1 to 12).
#[must_use]
pub fn month_name(m: u8) -> String {
    let i = usize::from(m.clamp(1, 12)) - 1;
    tr(MONTHS[i]).into_owned()
}

/// The long form of a date in the current language, as a journal note's
/// heading: `Monday, 2 November 2026`.
#[must_use]
pub fn long_date(d: Date) -> String {
    format!(
        "{}, {} {} {}",
        tr(WEEKDAYS[usize::from(d.weekday())]),
        d.d,
        month_name(d.m),
        d.y
    )
}

/// One of the three fields of the Calendar's date line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalField {
    /// The day of the month.
    Day,
    /// The month.
    Month,
    /// The year.
    Year,
}

/// The picker open in the Calendar pane, in place of the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalPicker {
    /// The days of the shown month, 1 to the month's length.
    Day,
    /// The twelve months.
    Month,
    /// A page of twelve years.
    Year {
        /// The first year of the page.
        first_year: i32,
    },
}

impl CalPicker {
    /// The date line field that opens this picker.
    #[must_use]
    pub fn field(self) -> CalField {
        match self {
            Self::Day => CalField::Day,
            Self::Month => CalField::Month,
            Self::Year { .. } => CalField::Year,
        }
    }
}

/// The first year of the page of twelve years that holds `y`.
#[must_use]
pub fn year_page(y: i32) -> i32 {
    (y - y.rem_euclid(12)).max(1)
}

/// What the Calendar pane shows.
#[derive(Clone, Copy, Debug)]
pub struct CalendarModel<'a> {
    /// The month on show (any day of it; the first is used).
    pub month: Date,
    /// Today, or `None` while the clock is not set.
    pub today: Option<Date>,
    /// The days that have a journal entry.
    pub marked: &'a BTreeSet<Date>,
    /// The open note's date, when it is a journal entry.
    pub open: Option<Date>,
    /// Which day the weeks start on.
    pub week_start: WeekStart,
    /// The open picker, if any.
    pub picker: Option<CalPicker>,
}

impl CalendarModel<'_> {
    /// The day the date line shows: the open entry's when it lies in the
    /// shown month, else today's when it does, else the 1st.
    #[must_use]
    pub fn day_field(&self) -> u8 {
        let in_month = |d: Option<Date>| {
            d.filter(|d| d.y == self.month.y && d.m == self.month.m)
                .map(|d| d.d)
        };
        in_month(self.open)
            .or_else(|| in_month(self.today))
            .unwrap_or(1)
    }

    /// The previous entry before the open one, if any.
    #[must_use]
    pub fn prev_entry(&self) -> Option<Date> {
        let o = self.open?;
        self.marked.range(..o).next_back().copied()
    }

    /// The next entry after the open one, if any.
    #[must_use]
    pub fn next_entry(&self) -> Option<Date> {
        let o = self.open?;
        self.marked.range(o.add_days(1)..).next().copied()
    }
}

/// Where the Calendar pane's parts were placed, in device pixels.
#[derive(Clone, Debug, Default)]
pub struct CalendarLayout {
    /// The pane's whole slice of the drawer.
    pub whole: Rect,
    /// The date line's day, month and year fields.
    pub fields: [Rect; 3],
    /// Navigation row: previous month (or page of years).
    pub prev: Rect,
    /// Navigation row: next month (or page of years).
    pub next: Rect,
    /// Navigation row: the Today button.
    pub today: Rect,
    /// The open picker, when the grid is replaced by one.
    pub picker: Option<CalPicker>,
    /// The picker's cells with their values (day, month 1 to 12, or year).
    pub picks: Vec<(Rect, u16)>,
    /// The weekday initials' row.
    pub weekdays: Rect,
    /// The days of the month with their cells.
    pub cells: Vec<(Rect, Date)>,
    /// Footer: previous entry and next entry (only with a journal note open).
    pub entries: Option<(Rect, Rect)>,
    /// The "Clock not set" line.
    pub hint: Option<Rect>,
}

/// Cuts `s` with an ellipsis to fit `max` px.
fn fit(e: &EditorScreen, s: &str, face: Face, size: f32, max: f32) -> String {
    if e.fonts.width(face, size, s) <= max {
        return s.to_owned();
    }
    let mut o = s.to_owned();
    while !o.is_empty() && e.fonts.width(face, size, &format!("{o}\u{2026}")) > max {
        o.pop();
    }
    format!("{o}\u{2026}")
}

impl EditorScreen {
    /// Lays the Calendar pane out in `slice`, the pane's whole share of the
    /// drawer (the date line first as the 44 px header band like the other
    /// panes, then the navigation row).
    #[must_use]
    pub fn calendar_layout(&self, slice: Rect, m: &CalendarModel<'_>) -> CalendarLayout {
        let d = |v: f32| self.scale().px(v);
        let head_h = d(44.0);
        let pad = d(8.0);
        let small = self.ui_px() * 0.82;
        let mut l = CalendarLayout {
            whole: slice,
            ..CalendarLayout::default()
        };
        // Date line: day | month | year.
        let gap = d(6.0);
        let total = (slice.w - 2 * pad - 2 * gap).max(0);
        let dw = total * 19 / 100;
        let mw = total * 47 / 100;
        let yw = total - dw - mw;
        let fx = slice.x + pad;
        l.fields = [
            Rect::new(fx, slice.y, dw, head_h),
            Rect::new(fx + dw + gap, slice.y, mw, head_h),
            Rect::new(fx + dw + mw + 2 * gap, slice.y, yw, head_h),
        ];
        l.picker = m.picker;
        // Below the date line: navigation row, weekday row, six week rows,
        // footer, hint. In a short slice the hint goes first, then the
        // footer, then the weekday row shrinks, then the navigation row;
        // the date line and the six week rows always stay (every day must
        // remain tappable) at whatever height is left.
        let avail = (slice.h - head_h).max(0);
        let min_row = d(22.0);
        let mut nav_h = d(40.0);
        let mut wd_h = d(26.0);
        let mut hint_h = if m.today.is_none() { d(24.0) } else { 0 };
        let mut foot_h = if m.open.is_some() { d(44.0) } else { 0 };
        let tight = |nav: i32, wd: i32, hint: i32, foot: i32| (avail - nav - wd - hint - foot) / 6;
        if foot_h > 0 && tight(nav_h, wd_h, hint_h, foot_h) < d(36.0) {
            foot_h = d(36.0);
        }
        if tight(nav_h, wd_h, hint_h, foot_h) < min_row {
            hint_h = 0;
        }
        if tight(nav_h, wd_h, hint_h, foot_h) < min_row {
            foot_h = 0;
        }
        if tight(nav_h, wd_h, hint_h, foot_h) < min_row {
            wd_h = d(16.0).min(avail / 8);
        }
        if tight(nav_h, wd_h, hint_h, foot_h) < min_row {
            nav_h = d(30.0).min(avail / 8);
        }
        let nav_w = d(40.0);
        let tw = (self.fonts.width(Face::AtkinsonBold, small, &tr("Today")) as i32 + d(20.0))
            .max(d(56.0));
        let ny = slice.y + head_h;
        l.prev = Rect::new(slice.x + pad, ny, nav_w, nav_h);
        l.next = Rect::new(slice.x + slice.w - pad - nav_w, ny, nav_w, nav_h);
        l.today = Rect::new(slice.x + (slice.w - tw) / 2, ny, tw, nav_h);
        let row_h = tight(nav_h, wd_h, hint_h, foot_h).clamp(1, d(44.0));
        let (gx, gw) = (slice.x + pad, slice.w - 2 * pad);
        let cw = gw / 7;
        let gx = gx + (gw - 7 * cw) / 2;
        let by = ny + nav_h;
        let gy = by + wd_h;
        if let Some(p) = m.picker {
            // The picker takes the place of the weekday row, the grid and
            // the footer; the hint keeps its line below.
            let body = Rect::new(gx, by, 7 * cw, wd_h + 6 * row_h + foot_h).intersect(&slice);
            let (cols, values): (i32, Vec<u16>) = match p {
                CalPicker::Day => (
                    7,
                    (1..=u16::from(sigma_core::date::days_in_month(m.month.y, m.month.m)))
                        .collect(),
                ),
                CalPicker::Month => (3, (1..=12).collect()),
                CalPicker::Year { first_year } => {
                    let f = first_year.max(1);
                    (
                        3,
                        (f..f + 12).filter_map(|y| u16::try_from(y).ok()).collect(),
                    )
                }
            };
            let rows = (values.len() as i32 + cols - 1) / cols;
            let pw = body.w / cols;
            let ph = (body.h / rows.max(1)).min(d(64.0));
            let px = body.x + (body.w - cols * pw) / 2;
            for (i, v) in values.iter().enumerate() {
                let (r, c) = (i as i32 / cols, i as i32 % cols);
                let rect = Rect::new(px + c * pw, body.y + r * ph, pw, ph).intersect(&slice);
                if rect.w > 0 && rect.h > 0 {
                    l.picks.push((rect, *v));
                }
            }
            if hint_h > 0 {
                l.hint = Some(Rect::new(gx, body.y + body.h, 7 * cw, hint_h));
            }
            return l;
        }
        l.weekdays = Rect::new(gx, by, 7 * cw, wd_h);
        let grid = month_grid(m.month.y, m.month.m, m.week_start);
        for (r, week) in grid.iter().enumerate() {
            for (c, day) in week.iter().enumerate() {
                if let Some(day) = day {
                    let rect = Rect::new(gx + c as i32 * cw, gy + r as i32 * row_h, cw, row_h)
                        .intersect(&slice);
                    if rect.w > 0 && rect.h > 0 {
                        l.cells.push((rect, *day));
                    }
                }
            }
        }
        let mut y = gy + 6 * row_h;
        if foot_h > 0 {
            let half = (7 * cw) / 2;
            l.entries = Some((
                Rect::new(gx, y, half, foot_h),
                Rect::new(gx + half, y, 7 * cw - half, foot_h),
            ));
            y += foot_h;
        }
        if hint_h > 0 {
            l.hint = Some(Rect::new(gx, y, 7 * cw, hint_h));
        }
        l
    }

    /// What a tap at (`x`, `y`) means in the Calendar pane, if anything.
    #[must_use]
    pub fn calendar_hit(&self, l: &CalendarLayout, x: i32, y: i32) -> Option<DrawerHit> {
        if !l.whole.contains(x, y) {
            return None;
        }
        if let Some((_, v)) = l.picks.iter().find(|(r, _)| r.contains(x, y)) {
            return Some(DrawerHit::CalPick(*v));
        }
        if let Some((_, day)) = l.cells.iter().find(|(r, _)| r.contains(x, y)) {
            return Some(DrawerHit::CalDay(*day));
        }
        for (r, f) in l
            .fields
            .iter()
            .zip([CalField::Day, CalField::Month, CalField::Year])
        {
            if r.contains(x, y) {
                return Some(DrawerHit::CalField(f));
            }
        }
        if l.prev.contains(x, y) {
            return Some(DrawerHit::CalPrevMonth);
        }
        if l.next.contains(x, y) {
            return Some(DrawerHit::CalNextMonth);
        }
        if l.today.contains(x, y) {
            return Some(DrawerHit::CalToday);
        }
        if let Some((p, n)) = l.entries {
            if p.contains(x, y) {
                return Some(DrawerHit::CalPrevEntry);
            }
            if n.contains(x, y) {
                return Some(DrawerHit::CalNextEntry);
            }
        }
        None
    }

    /// Draws the Calendar pane (the drawer has drawn its rule and edge).
    pub fn draw_calendar(
        &self,
        c: &mut Canvas,
        l: &CalendarLayout,
        m: &CalendarModel<'_>,
        t: Theme,
    ) {
        let d = |v: f32| self.scale().px(v);
        let ui = self.ui_px();
        let small = ui * 0.82;
        let mid = |r: Rect, face: Face, size: f32| {
            r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6
        };
        let centred =
            |c: &mut Canvas, r: Rect, face: Face, size: f32, s: &str, ink: u8, dy: f32| {
                let w = self.fonts.width(face, size, s);
                self.fonts.draw(
                    c,
                    face,
                    size,
                    r.x as f32 + (r.w as f32 - w) / 2.0,
                    mid(r, face, size) + dy,
                    s,
                    ink,
                );
            };
        // Date line: day | month | year, each a button with a chevron; the
        // open one is filled.
        let head = Rect::new(l.whole.x, l.whole.y, l.whole.w, l.fields[0].h);
        let values = [
            m.day_field().to_string(),
            month_name(m.month.m),
            m.month.y.to_string(),
        ];
        let open_field = l.picker.map(CalPicker::field);
        for ((r, f), text) in l
            .fields
            .iter()
            .zip([CalField::Day, CalField::Month, CalField::Year])
            .zip(&values)
        {
            let open = open_field == Some(f);
            let b = Rect::new(r.x, r.y + d(4.0), r.w, r.h - d(8.0));
            let ink = if open { t.paper } else { t.ink };
            if open {
                fill_round(c, b, d(6.0), t.ink);
            } else {
                c.stroke(b, 1.max(d(1.5)), t.ink);
            }
            let side = b.h.min(d(30.0));
            let chev = Rect::new(b.x + b.w - side, b.y, side, b.h);
            draw_icon(c, Icon::ChevronDown, chev, ink);
            let tr_ = Rect::new(b.x + d(2.0), b.y, (b.w - side - d(2.0)).max(0), b.h);
            let s = fit(self, text, Face::AtkinsonBold, ui, tr_.w as f32);
            centred(c, tr_, Face::AtkinsonBold, ui, &s, ink, 0.0);
        }
        c.fill(
            Rect::new(
                l.whole.x + d(12.0),
                head.y + head.h - 1,
                l.whole.w - d(24.0),
                1,
            ),
            t.rule,
        );
        // Navigation row: ‹ Today ›.
        draw_icon(c, Icon::PrevPage, l.prev, t.ink);
        draw_icon(c, Icon::NextPage, l.next, t.ink);
        let ink = if m.today.is_some() { t.ink } else { t.mute };
        let inner = l.today.inset(d(4.0));
        c.stroke(inner, 1.max(d(1.5)), ink);
        centred(c, inner, Face::AtkinsonBold, small, &tr("Today"), ink, 0.0);
        // The look of a day in the grid and in the day picker, and of a
        // cell of the month and year pickers.
        let cell_look = |c: &mut Canvas, r: Rect, label: &str, filled, outlined, marked| {
            let cell = r.inset(d(2.0));
            if filled {
                fill_round(c, cell, d(6.0), t.ink);
            }
            if outlined {
                c.stroke(cell, 2.max(d(2.0)), if filled { t.paper } else { t.ink });
                if filled {
                    // A paper ring inside the fill would vanish on the
                    // edge: keep an ink ring outside it too.
                    c.stroke(r.inset(d(1.0)), 1, t.ink);
                }
            }
            let (face, ink) = match (marked, filled) {
                (_, true) => (Face::AtkinsonBold, t.paper),
                (true, false) => (Face::AtkinsonBold, t.ink),
                (false, false) => (Face::Atkinson, t.ink),
            };
            let dy = -(r.h as f32) * 0.08;
            let s = fit(self, label, face, ui * 0.95, (r.w - d(8.0)) as f32);
            centred(c, r, face, ui * 0.95, &s, ink, dy);
            if marked {
                let dot = d(4.0).max(3);
                let dr = Rect::new(r.x + (r.w - dot) / 2, r.y + r.h - d(7.0) - dot, dot, dot);
                fill_round(c, dr, dot / 2, ink);
            }
        };
        if let Some(p) = l.picker {
            let on_month = |d: Option<Date>| d.filter(|d| d.y == m.month.y);
            for (r, v) in &l.picks {
                let (label, filled, outlined, marked) = match p {
                    CalPicker::Day => {
                        let day = Date::new(m.month.y, m.month.m, *v as u8);
                        (
                            v.to_string(),
                            day.is_some() && m.open == day,
                            day.is_some() && m.today == day,
                            day.is_some_and(|d| m.marked.contains(&d)),
                        )
                    }
                    CalPicker::Month => (
                        month_name(*v as u8),
                        *v == u16::from(m.month.m),
                        on_month(m.today).is_some_and(|t| u16::from(t.m) == *v),
                        false,
                    ),
                    CalPicker::Year { .. } => (
                        v.to_string(),
                        i32::from(*v) == m.month.y,
                        m.today.is_some_and(|t| i32::from(*v) == t.y),
                        false,
                    ),
                };
                cell_look(c, *r, &label, filled, outlined, marked);
            }
        }
        // Weekday initials.
        if l.weekdays.w > 0 {
            let cw = l.weekdays.w / 7;
            for i in 0..7 {
                let day = match m.week_start {
                    WeekStart::Monday => i,
                    WeekStart::Sunday => (i + 6) % 7,
                };
                let r = Rect::new(l.weekdays.x + i as i32 * cw, l.weekdays.y, cw, l.weekdays.h);
                let s = tr(WEEKDAY_INITIALS[day]).into_owned();
                centred(c, r, Face::Atkinson, small * 0.9, &s, t.mute, 0.0);
            }
        }
        // Days.
        for (r, day) in &l.cells {
            cell_look(
                c,
                *r,
                &day.d.to_string(),
                m.open == Some(*day),
                m.today == Some(*day),
                m.marked.contains(day),
            );
        }
        // Footer: previous and next entry.
        if let Some((p, n)) = l.entries {
            c.fill(Rect::new(p.x, p.y, p.w + n.w, 1), t.rule);
            let (has_p, has_n) = (m.prev_entry().is_some(), m.next_entry().is_some());
            let side = d(32.0).min(p.h);
            let icon_p = Rect::new(p.x, p.y + (p.h - side) / 2, side, side);
            draw_icon(
                c,
                Icon::PrevPage,
                icon_p,
                if has_p { t.ink } else { t.mute },
            );
            let lp = tr("Previous");
            let pr = Rect::new(icon_p.x + side, p.y, p.w - side, p.h);
            let sp = fit(self, &lp, Face::AtkinsonBold, small, pr.w as f32);
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                small,
                pr.x as f32,
                mid(pr, Face::AtkinsonBold, small),
                &sp,
                if has_p { t.ink } else { t.mute },
            );
            let icon_n = Rect::new(n.x + n.w - side, n.y + (n.h - side) / 2, side, side);
            draw_icon(
                c,
                Icon::NextPage,
                icon_n,
                if has_n { t.ink } else { t.mute },
            );
            let ln = tr("Next");
            let nr = Rect::new(n.x, n.y, n.w - side, n.h);
            let sn = fit(self, &ln, Face::AtkinsonBold, small, nr.w as f32);
            let w = self.fonts.width(Face::AtkinsonBold, small, &sn);
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                small,
                nr.x as f32 + nr.w as f32 - w,
                mid(nr, Face::AtkinsonBold, small),
                &sn,
                if has_n { t.ink } else { t.mute },
            );
        }
        if let Some(h) = l.hint {
            let s = fit(self, &tr(CLOCK_NOT_SET), Face::Atkinson, small, h.w as f32);
            centred(c, h, Face::Atkinson, small, &s, t.mute, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawer::{DrawerModel, Pane};
    use sigma_core::settings::Side;
    use sigma_core::Keyboard;

    fn dt(y: i32, m: u8, d: u8) -> Date {
        Date::new(y, m, d).unwrap()
    }

    fn model<'a>(
        marked: &'a BTreeSet<Date>,
        ws: WeekStart,
        today: Option<Date>,
        open: Option<Date>,
    ) -> CalendarModel<'a> {
        CalendarModel {
            month: dt(2026, 11, 1),
            today,
            marked,
            open,
            week_start: ws,
            picker: None,
        }
    }

    fn drawer<'a>(cm: Option<CalendarModel<'a>>) -> DrawerModel<'a> {
        DrawerModel {
            side: Side::Right,
            contents: None,
            todos: None,
            scratch: None,
            plugin: [None; 4],
            calendar: cm,
            theme: Theme::DAY,
        }
    }

    fn centre(r: Rect) -> (i32, i32) {
        (r.x + r.w / 2, r.y + r.h / 2)
    }

    #[test]
    fn a_known_day_is_hit_and_the_week_start_moves_it() {
        let marked: BTreeSet<Date> = [dt(2026, 11, 2)].into_iter().collect();
        for (w, h) in [(1440u32, 1920u32), (1920, 1440)] {
            let e = EditorScreen::new(w, h);
            let base = e.layout(true, &Keyboard::default());
            let mut xs = Vec::new();
            for ws in [WeekStart::Monday, WeekStart::Sunday] {
                let m = drawer(Some(model(&marked, ws, Some(dt(2026, 11, 2)), None)));
                let l = e.drawer_layout(&base, &m);
                let cl = l.calendar.as_ref().expect("calendar laid out");
                assert_eq!(cl.cells.len(), 30, "every day of November");
                for (r, day) in &cl.cells {
                    let (x, y) = centre(*r);
                    assert_eq!(
                        e.drawer_hit(&base, &m, x, y),
                        DrawerHit::CalDay(*day),
                        "{w}x{h} {ws:?} {day}"
                    );
                    assert!(r.h >= e.scale().px(22.0), "cells stay tappable");
                    assert!(l.rect.contains(x, y));
                }
                let x2 = cl
                    .cells
                    .iter()
                    .find(|c| c.1 == dt(2026, 11, 2))
                    .unwrap()
                    .0
                    .x;
                let x1 = cl
                    .cells
                    .iter()
                    .find(|c| c.1 == dt(2026, 11, 1))
                    .unwrap()
                    .0
                    .x;
                xs.push((x1, x2));
            }
            // 1 November 2026 is a Sunday: last column with Monday weeks,
            // first with Sunday weeks; the 2nd is a Monday: first column
            // with Monday weeks, second with Sunday weeks.
            let (mon, sun) = (xs[0], xs[1]);
            assert!(mon.1 < mon.0, "{w}: Monday start: Mon 2 left of Sun 1");
            assert!(sun.0 < sun.1, "{w}: Sunday start: Sun 1 left of Mon 2");
            assert_eq!(mon.1, sun.0, "both put the first column at the same x");
        }
    }

    #[test]
    fn the_header_buttons_answer_and_the_footer_needs_an_open_entry() {
        let marked: BTreeSet<Date> = [dt(2026, 11, 2), dt(2026, 11, 5)].into_iter().collect();
        let e = EditorScreen::new(1440, 1920);
        let base = e.layout(true, &Keyboard::default());
        let closed = drawer(Some(model(
            &marked,
            WeekStart::Monday,
            Some(dt(2026, 11, 3)),
            None,
        )));
        let cl = e.drawer_layout(&base, &closed).calendar.unwrap();
        assert!(cl.entries.is_none(), "no footer without an open entry");
        for (r, want) in [
            (cl.prev, DrawerHit::CalPrevMonth),
            (cl.next, DrawerHit::CalNextMonth),
            (cl.today, DrawerHit::CalToday),
        ] {
            assert!(r.h >= e.px(40.0) && r.w >= e.px(40.0), "{want:?}: a finger");
            let (x, y) = centre(r);
            assert_eq!(e.drawer_hit(&base, &closed, x, y), want);
        }
        let open = drawer(Some(model(
            &marked,
            WeekStart::Sunday,
            Some(dt(2026, 11, 3)),
            Some(dt(2026, 11, 2)),
        )));
        let cl = e.drawer_layout(&base, &open).calendar.unwrap();
        let (p, n) = cl.entries.expect("footer with an open entry");
        let (x, y) = centre(p);
        assert_eq!(e.drawer_hit(&base, &open, x, y), DrawerHit::CalPrevEntry);
        let (x, y) = centre(n);
        assert_eq!(e.drawer_hit(&base, &open, x, y), DrawerHit::CalNextEntry);
        let m = open.calendar.unwrap();
        assert_eq!(
            (m.prev_entry(), m.next_entry()),
            (None, Some(dt(2026, 11, 5)))
        );
        // The footer sits below the grid, inside the pane.
        let last = cl.cells.iter().map(|c| c.0.y + c.0.h).max().unwrap();
        assert!(p.y >= last - 1 && p.y + p.h <= cl.whole.y + cl.whole.h);
    }

    #[test]
    fn without_a_clock_the_grid_stays_and_a_hint_shows() {
        let marked = BTreeSet::new();
        let e = EditorScreen::new(1440, 1920);
        let base = e.layout(true, &Keyboard::default());
        let m = drawer(Some(model(&marked, WeekStart::Monday, None, None)));
        let cl = e.drawer_layout(&base, &m).calendar.unwrap();
        assert_eq!(cl.cells.len(), 30);
        assert!(cl.hint.is_some());
        let set: BTreeSet<Date> = BTreeSet::new();
        let m = drawer(Some(model(
            &set,
            WeekStart::Monday,
            Some(dt(2026, 11, 3)),
            None,
        )));
        assert!(e.drawer_layout(&base, &m).calendar.unwrap().hint.is_none());
    }

    #[test]
    fn the_calendar_draws_today_outlined_marked_days_bold_and_the_open_one_filled() {
        let t = Theme::DAY;
        let marked: BTreeSet<Date> = [dt(2026, 11, 10), dt(2026, 11, 12)].into_iter().collect();
        let e = EditorScreen::new(1440, 1920);
        let base = e.layout(true, &Keyboard::default());
        let m = drawer(Some(model(
            &marked,
            WeekStart::Monday,
            Some(dt(2026, 11, 5)),
            Some(dt(2026, 11, 10)),
        )));
        let mut c = Canvas::new(1440, 1920, 255);
        let l = e.draw_drawer(&mut c, &base, &m);
        let cl = l.calendar.unwrap();
        let px = |c: &Canvas, x: i32, y: i32| c.pixels[(y * c.width as i32 + x) as usize];
        let cell = |d: Date| cl.cells.iter().find(|c| c.1 == d).unwrap().0;
        // The open day: an ink fill (its corner, inside the rounded fill).
        let o = cell(dt(2026, 11, 10)).inset(e.px(6.0));
        assert_eq!(px(&c, o.x + o.w - 2, o.y + 3), t.ink, "filled");
        // A plain day is untouched at the same spot.
        let p = cell(dt(2026, 11, 17)).inset(e.px(6.0));
        assert_eq!(px(&c, p.x + p.w - 2, p.y + 3), t.paper);
        // Today: an outline at the edge of its cell, paper inside.
        let td = cell(dt(2026, 11, 5));
        let edge = td.inset(e.scale().px(2.0));
        assert_eq!(px(&c, edge.x + edge.w / 2, edge.y), t.ink, "outline");
        assert_eq!(px(&c, edge.x + edge.w / 2, edge.y + 7), t.paper);
        // A marked day carries a dot under its number.
        let mk = cell(dt(2026, 11, 12));
        let dot = (0..mk.w)
            .map(|dx| px(&c, mk.x + dx, mk.y + mk.h - e.px(7.0) - 2))
            .any(|v| v == t.ink);
        assert!(dot, "dot under a day with an entry");
    }

    #[test]
    fn the_four_pane_drawer_keeps_every_calendar_part_inside_its_slice() {
        let marked: BTreeSet<Date> = [dt(2026, 11, 2)].into_iter().collect();
        let hs = sigma_core::outline::headings("# A");
        let b = sigma_core::Buffer::new("x");
        for (w, h, keys) in [
            (1440u32, 1920u32, false),
            (1920, 1440, false),
            (1440, 1920, true),
            (1920, 1440, true),
        ] {
            let e = EditorScreen::new(w, h);
            let base = e.layout(keys, &Keyboard::default());
            let m = DrawerModel {
                side: Side::Left,
                contents: Some(hs.as_slice()),
                todos: Some((&[], sigma_core::outline::Filter::All)),
                scratch: Some((&b, false)),
                plugin: [None; 4],
                calendar: Some(model(
                    &marked,
                    WeekStart::Monday,
                    None,
                    Some(dt(2026, 11, 2)),
                )),
                theme: Theme::DAY,
            };
            let l = e.drawer_layout(&base, &m);
            assert_eq!(l.panes.len(), 4);
            assert_eq!(l.panes[3].0, Pane::Calendar);
            let cl = l.calendar.as_ref().unwrap();
            let inside = |r: &Rect| {
                r.w > 0
                    && r.h > 0
                    && r.x >= cl.whole.x
                    && r.x + r.w <= cl.whole.x + cl.whole.w
                    && r.y >= cl.whole.y
                    && r.y + r.h <= cl.whole.y + cl.whole.h
            };
            for (name, r) in [("prev", &cl.prev), ("next", &cl.next), ("today", &cl.today)] {
                assert!(
                    inside(r),
                    "{w}x{h} kb={keys}: {name} {r:?} in {:?}",
                    cl.whole
                );
            }
            assert!(inside(&cl.weekdays), "{w}x{h} kb={keys}: weekdays");
            if let Some(r) = &cl.hint {
                assert!(inside(r), "{w}x{h} kb={keys}: hint {r:?}");
            }
            if let Some((p, n)) = &cl.entries {
                assert!(inside(p) && inside(n), "{w}x{h} kb={keys}: footer");
            }
            for (r, _) in &cl.cells {
                assert!(inside(r), "{w}x{h} kb={keys}: cell {r:?} in {:?}", cl.whole);
            }
            let days: BTreeSet<u8> = cl.cells.iter().map(|(_, d)| d.d).collect();
            assert_eq!(days.len(), 30, "{w}x{h} kb={keys}: every day of November");
            let mut c = Canvas::new(w, h, 255);
            let _ = e.draw_drawer(&mut c, &base, &m);
        }
    }

    fn with_picker<'a>(
        marked: &'a BTreeSet<Date>,
        today: Option<Date>,
        open: Option<Date>,
        picker: Option<CalPicker>,
    ) -> DrawerModel<'a> {
        let mut m = model(marked, WeekStart::Monday, today, open);
        m.picker = picker;
        drawer(Some(m))
    }

    #[test]
    fn the_date_line_sits_on_top_with_three_fields_the_navigation_row_below() {
        let marked = BTreeSet::new();
        let e = EditorScreen::new(1440, 1920);
        let base = e.layout(true, &Keyboard::default());
        let m = with_picker(&marked, Some(dt(2026, 11, 2)), None, None);
        let l = e.drawer_layout(&base, &m);
        let cl = l.calendar.unwrap();
        let [d, mo, y] = cl.fields;
        assert!(d.x + d.w <= mo.x && mo.x + mo.w <= y.x, "side by side");
        assert!(mo.w > d.w, "the month is the widest");
        for (r, f) in [
            (d, CalField::Day),
            (mo, CalField::Month),
            (y, CalField::Year),
        ] {
            assert_eq!(r.y, cl.whole.y, "{f:?} in the header band");
            assert_eq!(r.h, e.scale().px(44.0), "{f:?}: 44 px band");
            let (x, yy) = centre(r);
            assert_eq!(
                e.drawer_hit(&base, &m, x, yy),
                DrawerHit::CalField(f),
                "{f:?}"
            );
        }
        // The navigation row is below the line: prev left, Today middle,
        // next right.
        let band = cl.fields[0].y + cl.fields[0].h;
        assert!(cl.prev.y >= band && cl.today.y >= band && cl.next.y >= band);
        assert!(cl.prev.x + cl.prev.w <= cl.today.x);
        assert!(cl.today.x + cl.today.w <= cl.next.x);
        let mid = cl.whole.x + cl.whole.w / 2;
        assert!((cl.today.x + cl.today.w / 2 - mid).abs() <= 2, "centred");
        assert!(cl.weekdays.y >= cl.prev.y + cl.prev.h);
    }

    #[test]
    fn the_day_field_is_the_open_day_else_today_else_the_first() {
        let marked = BTreeSet::new();
        let day = |today, open| model(&marked, WeekStart::Monday, today, open).day_field();
        assert_eq!(day(Some(dt(2026, 11, 2)), Some(dt(2026, 11, 9))), 9);
        assert_eq!(day(Some(dt(2026, 11, 2)), Some(dt(2026, 10, 9))), 2);
        assert_eq!(day(Some(dt(2026, 10, 2)), Some(dt(2026, 10, 9))), 1);
        assert_eq!(day(None, None), 1);
    }

    #[test]
    fn each_picker_replaces_the_grid_with_its_own_cells() {
        let marked: BTreeSet<Date> = [dt(2026, 11, 2)].into_iter().collect();
        for (w, h, keys) in [
            (1440u32, 1920u32, false),
            (1920, 1440, false),
            (1440, 1920, true),
            (1920, 1440, true),
        ] {
            let e = EditorScreen::new(w, h);
            let base = e.layout(keys, &Keyboard::default());
            for (picker, want) in [
                (CalPicker::Day, (1..=30).collect::<Vec<u16>>()),
                (CalPicker::Month, (1..=12).collect()),
                (CalPicker::Year { first_year: 2016 }, (2016..2028).collect()),
            ] {
                let m = with_picker(
                    &marked,
                    Some(dt(2026, 11, 2)),
                    Some(dt(2026, 11, 2)),
                    Some(picker),
                );
                let l = e.drawer_layout(&base, &m);
                let cl = l.calendar.as_ref().unwrap();
                let tag = format!("{w}x{h} kb={keys} {picker:?}");
                assert!(cl.cells.is_empty() && cl.entries.is_none(), "{tag}");
                let vals: Vec<u16> = cl.picks.iter().map(|p| p.1).collect();
                assert_eq!(vals, want, "{tag}");
                for (r, v) in &cl.picks {
                    assert!(
                        r.w > 0
                            && r.h >= e.scale().px(22.0)
                            && r.x >= cl.whole.x
                            && r.x + r.w <= cl.whole.x + cl.whole.w
                            && r.y >= cl.whole.y
                            && r.y + r.h <= cl.whole.y + cl.whole.h,
                        "{tag}: cell {v} {r:?} in {:?}",
                        cl.whole
                    );
                    let (x, y) = centre(*r);
                    assert_eq!(e.drawer_hit(&base, &m, x, y), DrawerHit::CalPick(*v));
                }
                // Cells do not overlap.
                for (i, (a, _)) in cl.picks.iter().enumerate() {
                    for (b, _) in &cl.picks[i + 1..] {
                        assert!(a.intersect(b).w == 0 || a.intersect(b).h == 0, "{tag}");
                    }
                }
                // The line and the navigation row stay.
                let (x, y) = centre(cl.fields[1]);
                assert_eq!(
                    e.drawer_hit(&base, &m, x, y),
                    DrawerHit::CalField(CalField::Month)
                );
                let (x, y) = centre(cl.today);
                assert_eq!(e.drawer_hit(&base, &m, x, y), DrawerHit::CalToday);
                let mut c = Canvas::new(w, h, 255);
                let _ = e.draw_drawer(&mut c, &base, &m);
            }
        }
    }

    #[test]
    fn pickers_fit_a_short_four_pane_slice() {
        let marked: BTreeSet<Date> = BTreeSet::new();
        let hs = sigma_core::outline::headings("# A");
        let b = sigma_core::Buffer::new("x");
        for (w, h) in [(1440u32, 1920u32), (1920, 1440)] {
            let e = EditorScreen::new(w, h);
            let base = e.layout(true, &Keyboard::default());
            for picker in [
                CalPicker::Day,
                CalPicker::Month,
                CalPicker::Year { first_year: 2016 },
            ] {
                let mut cm = model(&marked, WeekStart::Monday, None, Some(dt(2026, 11, 2)));
                cm.picker = Some(picker);
                let m = DrawerModel {
                    side: Side::Left,
                    contents: Some(hs.as_slice()),
                    todos: Some((&[], sigma_core::outline::Filter::All)),
                    scratch: Some((&b, false)),
                    plugin: [None; 4],
                    calendar: Some(cm),
                    theme: Theme::DAY,
                };
                let cl = e.drawer_layout(&base, &m).calendar.unwrap();
                assert_eq!(
                    cl.picks.len(),
                    if picker == CalPicker::Day { 30 } else { 12 }
                );
                for r in cl.fields.iter().chain([&cl.prev, &cl.next, &cl.today]) {
                    assert!(
                        r.w > 0
                            && r.h > 0
                            && r.y >= cl.whole.y
                            && r.y + r.h <= cl.whole.y + cl.whole.h,
                        "{w}x{h} {picker:?}: {r:?} in {:?}",
                        cl.whole
                    );
                }
                for (r, _) in &cl.picks {
                    assert!(r.y + r.h <= cl.whole.y + cl.whole.h, "{w}x{h} {picker:?}");
                }
                if let Some(r) = cl.hint {
                    assert!(r.y + r.h <= cl.whole.y + cl.whole.h, "{w}x{h} {picker:?}");
                }
            }
        }
    }

    #[test]
    fn the_open_field_is_filled_and_the_pickers_mark_today_the_open_day_and_entries() {
        let t = Theme::DAY;
        let marked: BTreeSet<Date> = [dt(2026, 11, 10), dt(2026, 11, 12)].into_iter().collect();
        let e = EditorScreen::new(1440, 1920);
        let base = e.layout(true, &Keyboard::default());
        let px = |c: &Canvas, x: i32, y: i32| c.pixels[(y * c.width as i32 + x) as usize];
        // Month picker: the month field is filled, the others are not.
        let m = with_picker(
            &marked,
            Some(dt(2026, 11, 5)),
            Some(dt(2026, 11, 10)),
            Some(CalPicker::Month),
        );
        let mut c = Canvas::new(1440, 1920, 255);
        let cl = e.draw_drawer(&mut c, &base, &m).calendar.unwrap();
        let inner = |r: Rect| r.inset(e.px(10.0));
        let probe = |c: &Canvas, r: Rect| {
            let i = inner(r);
            px(c, i.x + 2, i.y + 2)
        };
        assert_eq!(probe(&c, cl.fields[1]), t.ink, "open field filled");
        assert_eq!(probe(&c, cl.fields[0]), t.paper);
        assert_eq!(probe(&c, cl.fields[2]), t.paper);
        // The current month (today's) is outlined: a cell other than the
        // shown one would be, so check an outline on a different month.
        let mut m2 = m;
        let mut cm = m2.calendar.unwrap();
        cm.month = dt(2026, 3, 1);
        m2.calendar = Some(cm);
        let mut c = Canvas::new(1440, 1920, 255);
        let cl = e.draw_drawer(&mut c, &base, &m2).calendar.unwrap();
        let nov = cl.picks.iter().find(|p| p.1 == 11).unwrap().0;
        let edge = nov.inset(e.px(2.0));
        assert_eq!(px(&c, edge.x + edge.w / 2, edge.y), t.ink, "outlined");
        let mar = cl
            .picks
            .iter()
            .find(|p| p.1 == 3)
            .unwrap()
            .0
            .inset(e.px(6.0));
        assert_eq!(px(&c, mar.x + mar.w - 2, mar.y + 3), t.ink, "shown month");
        // Day picker: the open day is filled, an entry has a dot.
        let m = with_picker(
            &marked,
            Some(dt(2026, 11, 5)),
            Some(dt(2026, 11, 10)),
            Some(CalPicker::Day),
        );
        let mut c = Canvas::new(1440, 1920, 255);
        let cl = e.draw_drawer(&mut c, &base, &m).calendar.unwrap();
        let open = cl
            .picks
            .iter()
            .find(|p| p.1 == 10)
            .unwrap()
            .0
            .inset(e.px(6.0));
        assert_eq!(px(&c, open.x + open.w - 2, open.y + 3), t.ink);
        let mk = cl.picks.iter().find(|p| p.1 == 12).unwrap().0;
        let dot = (0..mk.w)
            .map(|dx| px(&c, mk.x + dx, mk.y + mk.h - e.px(7.0) - 2))
            .any(|v| v == t.ink);
        assert!(dot, "dot under a day with an entry");
        let plain = cl
            .picks
            .iter()
            .find(|p| p.1 == 17)
            .unwrap()
            .0
            .inset(e.px(6.0));
        assert_eq!(px(&c, plain.x + plain.w - 2, plain.y + 3), t.paper);
    }

    #[test]
    fn dates_read_in_the_current_language() {
        assert_eq!(long_date(dt(2026, 11, 2)), "Monday, 2 November 2026");
        assert_eq!(month_name(5), "May");
    }
}
