//! The search results list (S22.1): with the Find bar in All notes scope,
//! a paged list of matching lines is drawn in place of the note page,
//! inside the editor layout's page rect, under the Find bar and above the
//! on-screen keyboard. One row per match: `path · line n` in the small
//! face over the matched line in the body face with the hit in bold,
//! trimmed around the hit so it fits one line. Page arrows and a status
//! line sit at the bottom. The row and paging arithmetic follow the Graph
//! list (`graph.rs`), but this is not a sheet: no title, no close button.
//!
//! The row model is plain data so the renderer does not depend on the
//! core search type; the app maps its results into [`SearchRow`]s.

use crate::canvas::{Canvas, Rect};
use crate::editor::EditorScreen;
use crate::fonts::Face;
use crate::i18n::tr;
use crate::icons::{draw_icon, Icon};
use crate::theme::Theme;

/// One matching line of one note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRow {
    /// Path of the note from the library root.
    pub path: String,
    /// Line number in the note, 1-based as shown.
    pub line: usize,
    /// The matched line, as it is in the note.
    pub text: String,
    /// Byte range of the hit inside `text` (`start..end`).
    pub hit: (usize, usize),
}

/// Where the search stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SearchState {
    /// No query yet: the list invites typing.
    #[default]
    Idle,
    /// A request is running on the worker.
    Searching,
    /// The rows are the answer to the current query.
    Done,
}

/// What the results list shows.
pub struct SearchListModel<'a> {
    /// Every match, grouped by note in library order.
    pub rows: &'a [SearchRow],
    /// Index of the first row on the shown page.
    pub first: usize,
    /// The selected row (Up/Down, Enter), drawn inverted.
    pub selected: Option<usize>,
    /// Idle, searching or done.
    pub state: SearchState,
    /// How many notes the library has (the idle line).
    pub notes_total: usize,
    /// How many notes have at least one match.
    pub notes_with_hits: usize,
    /// The result was cut at the cap: "first n shown" is appended.
    pub capped: bool,
    /// Colours to draw with.
    pub theme: Theme,
}

/// Where the list's parts were placed, in device pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchLayout {
    /// The page rect the list was laid out in.
    pub page: Rect,
    /// Row index and band of each row on the shown page.
    pub rows: Vec<(usize, Rect)>,
    /// How many rows fit on one page (0 when the page is too short).
    pub visible: usize,
    /// Height of one row.
    pub row_h: i32,
    /// The ‹ (previous page) button.
    pub prev: Rect,
    /// The › (next page) button.
    pub next: Rect,
    /// The status line, left of the arrows.
    pub status: Rect,
    /// The whole bottom band (status and arrows).
    pub foot: Rect,
}

/// What a tap on the results list landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchListHit {
    /// A row: index into the rows.
    Row(usize),
    /// The ‹ button.
    Prev,
    /// The › button.
    Next,
    /// Nothing that responds to a tap.
    None,
}

/// The pieces of a matched line trimmed to one row: the text before the
/// hit, the hit, the text after it. A trimmed side starts or ends with an
/// ellipsis.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrimmedLine {
    /// Text before the hit (may start with `…`).
    pub before: String,
    /// The hit, drawn bold (may end with `…` if it alone is too wide).
    pub hit: String,
    /// Text after the hit (may end with `…`).
    pub after: String,
}

pub(crate) const ELLIPSIS: char = '\u{2026}';
/// The small face (path line) relative to `ui_px`.
pub(crate) const SMALL: f32 = 0.85;

impl EditorScreen {
    /// Height of one results row: two lines of text with padding, never
    /// under the 44 px minimum target.
    #[must_use]
    pub fn search_row_h(&self) -> i32 {
        let s = self.scale();
        let ui = self.ui_px();
        let text = (ui * SMALL * 1.3 + ui * 1.35).ceil() as i32;
        (s.px(8.0) * 2 + text)
            .max(s.px(44.0))
            .max(self.page_scale().px(44.0))
    }

    /// Lays out the results list inside `page` (the editor layout's page
    /// rect) for `m`.
    #[must_use]
    pub fn search_layout(&self, page: Rect, m: &SearchListModel<'_>) -> SearchLayout {
        let s = self.scale();
        let row_h = self.search_row_h();
        let btn = s.px(44.0);
        let gap = s.px(8.0);
        let pad = s.px(12.0);
        let foot_h = (btn + 2 * s.px(6.0)).min(page.h.max(0));
        let foot = Rect::new(page.x, page.y + page.h - foot_h, page.w, foot_h);
        let by = foot.y + (foot.h - btn) / 2;
        let next = Rect::new(page.x + page.w - pad - btn, by, btn, btn);
        let prev = Rect::new(next.x - gap - btn, by, btn, btn);
        let tx = page.x + s.px(20.0);
        let status = Rect::new(tx, foot.y, (prev.x - gap - tx).max(1), foot.h);
        let avail = page.h - foot_h;
        let visible = if avail >= row_h {
            (avail / row_h) as usize
        } else {
            0
        };
        let rows = (m.first..m.rows.len().min(m.first + visible))
            .enumerate()
            .map(|(k, i)| {
                (
                    i,
                    Rect::new(page.x, page.y + k as i32 * row_h, page.w, row_h),
                )
            })
            .collect();
        SearchLayout {
            page,
            rows,
            visible,
            row_h,
            prev,
            next,
            status,
            foot,
        }
    }

    /// What the results list has at (`x`, `y`), in device pixels. The
    /// arrows take a few pixels of slop around them.
    #[must_use]
    pub fn search_hit(&self, l: &SearchLayout, x: i32, y: i32) -> SearchListHit {
        let slop = -self.scale().px(4.0);
        if l.prev.inset(slop).contains(x, y) {
            return SearchListHit::Prev;
        }
        if l.next.inset(slop).contains(x, y) {
            return SearchListHit::Next;
        }
        l.rows
            .iter()
            .find(|(_, r)| r.contains(x, y))
            .map_or(SearchListHit::None, |(i, _)| SearchListHit::Row(*i))
    }

    /// Number of pages for `m` with `visible` rows per page, at least one.
    #[must_use]
    pub fn search_pages(m: &SearchListModel<'_>, visible: usize) -> usize {
        m.rows.len().div_ceil(visible.max(1)).max(1)
    }

    /// The status line for `m`, translated: "Type to search n notes",
    /// "Searching…", "No matches", or "n matches in m notes · page i of k",
    /// with "first n shown" appended when the result was capped.
    #[must_use]
    pub fn search_status(m: &SearchListModel<'_>, visible: usize) -> String {
        match m.state {
            SearchState::Idle => {
                let s = if m.notes_total == 1 {
                    "Type to search 1 note".to_owned()
                } else {
                    format!("Type to search {} notes", m.notes_total)
                };
                return tr(&s).into_owned();
            }
            SearchState::Searching => return tr("Searching\u{2026}").into_owned(),
            SearchState::Done => {}
        }
        if m.rows.is_empty() {
            return tr("No matches").into_owned();
        }
        let n = m.rows.len();
        let notes = m.notes_with_hits;
        let mut out = match (n, notes) {
            (1, _) => "1 match in 1 note".to_owned(),
            (_, 1) => format!("{n} matches in 1 note"),
            _ => format!("{n} matches in {notes} notes"),
        };
        let pages = Self::search_pages(m, visible);
        if pages > 1 {
            let page = m.first / visible.max(1) + 1;
            out.push_str(&format!(" \u{b7} page {page} of {pages}"));
        }
        if m.capped {
            out.push_str(&format!(" \u{b7} first {n} shown"));
        }
        tr(&out).into_owned()
    }

    /// `path · line n` for a row, with the start of the path cut (an
    /// ellipsis first) so the whole fits `max` pixels in the small face.
    /// The file name at the end is what matters.
    #[must_use]
    pub fn search_path_line(&self, row: &SearchRow, max: f32) -> String {
        let small = self.ui_px() * SMALL;
        let tail = format!(" \u{b7} {}", tr(&format!("line {}", row.line)));
        let tail_w = self.fonts.width(Face::Atkinson, small, &tail);
        let room = max - tail_w;
        let mut path = row.path.clone();
        if self.fonts.width(Face::Atkinson, small, &path) > room {
            let ell = self
                .fonts
                .width(Face::Atkinson, small, &ELLIPSIS.to_string());
            path = self.cut_start(&path, Face::Atkinson, small, room - ell);
            path.insert(0, ELLIPSIS);
        }
        path.push_str(&tail);
        path
    }

    /// The matched line of `row` trimmed around its hit so it fits `max`
    /// pixels in the body face, the hit in bold. Leading blanks are
    /// dropped; a hit wider than the row is cut at its end.
    #[must_use]
    pub fn search_trim(&self, row: &SearchRow, max: f32) -> TrimmedLine {
        let ui = self.ui_px();
        let text = &row.text;
        let len = text.len();
        let (a, b) = row.hit;
        let a = a.min(len);
        let b = b.clamp(a, len);
        // Only char boundaries make valid pieces; a bad range shows the
        // whole line unmarked rather than panicking.
        let (a, b) = if text.is_char_boundary(a) && text.is_char_boundary(b) {
            (a, b)
        } else {
            (0, 0)
        };
        let lead = text.len() - text.trim_start().len();
        let a = a.max(lead);
        let b = b.max(a);
        let tidy = |s: &str| s.replace('\t', " ");
        let (mut before, mut hit, mut after) =
            (tidy(&text[lead..a]), tidy(&text[a..b]), tidy(&text[b..]));
        let w = |face: Face, s: &str| self.fonts.width(face, ui, s);
        if w(Face::Atkinson, &before) + w(Face::AtkinsonBold, &hit) + w(Face::Atkinson, &after)
            <= max
        {
            return TrimmedLine { before, hit, after };
        }
        let ell_b = w(Face::AtkinsonBold, &ELLIPSIS.to_string());
        let ell = w(Face::Atkinson, &ELLIPSIS.to_string());
        if w(Face::AtkinsonBold, &hit) > max {
            hit = self.cut_end(&hit, Face::AtkinsonBold, ui, max - ell_b);
            hit.push(ELLIPSIS);
            return TrimmedLine {
                before: String::new(),
                hit,
                after: String::new(),
            };
        }
        let budget = max - w(Face::AtkinsonBold, &hit);
        let before_w = w(Face::Atkinson, &before);
        let after_w = w(Face::Atkinson, &after);
        // Some context before the hit (a third of the room, or more when
        // the text after it is short), the rest after it.
        let want_before = (budget / 3.0).max(budget - after_w).min(before_w);
        if before_w > want_before {
            before = self.cut_start(&before, Face::Atkinson, ui, want_before - ell);
            before.insert(0, ELLIPSIS);
        }
        let room_after = budget - w(Face::Atkinson, &before);
        if after_w > room_after {
            after = self.cut_end(&after, Face::Atkinson, ui, room_after - ell);
            after.push(ELLIPSIS);
        }
        TrimmedLine { before, hit, after }
    }

    /// `s` with characters dropped from its end until it is at most `max`
    /// pixels wide.
    pub(crate) fn cut_end(&self, s: &str, face: Face, size: f32, max: f32) -> String {
        let mut out = s.to_owned();
        while !out.is_empty() && self.fonts.width(face, size, &out) > max {
            out.pop();
        }
        out
    }

    /// `s` with characters dropped from its start until it is at most
    /// `max` pixels wide.
    pub(crate) fn cut_start(&self, s: &str, face: Face, size: f32, max: f32) -> String {
        let mut out = s;
        while !out.is_empty() && self.fonts.width(face, size, out) > max {
            let n = out.chars().next().map_or(0, char::len_utf8);
            out = &out[n..];
        }
        out.to_owned()
    }

    /// Draws the results list into `l` (from [`Self::search_layout`]): the
    /// rows, the selected one inverted, then the status line and the page
    /// arrows.
    pub fn draw_search(&self, c: &mut Canvas, l: &SearchLayout, m: &SearchListModel<'_>) {
        let t = m.theme;
        let s = self.scale();
        let ui = self.ui_px();
        let small = ui * SMALL;
        c.fill(l.page, t.paper);
        let tx = (l.page.x + s.px(20.0)) as f32;
        let max = (l.page.w - s.px(40.0)) as f32;
        let pad = s.px(8.0) as f32;
        for &(i, r) in &l.rows {
            let Some(row) = m.rows.get(i) else {
                continue;
            };
            let sel = m.selected == Some(i);
            let (ink, mute) = if sel {
                c.fill(r, t.ink);
                (t.paper, t.paper)
            } else {
                (t.ink, t.mute)
            };
            let b1 = r.y as f32 + pad + self.fonts.ascent(Face::Atkinson, small);
            let b2 = r.y as f32 + pad + small * 1.3 + self.fonts.ascent(Face::Atkinson, ui);
            let path = self.search_path_line(row, max);
            self.fonts
                .draw(c, Face::Atkinson, small, tx, b1, &path, mute);
            let line = self.search_trim(row, max);
            let x = self
                .fonts
                .draw(c, Face::Atkinson, ui, tx, b2, &line.before, ink);
            let x = self
                .fonts
                .draw(c, Face::AtkinsonBold, ui, x, b2, &line.hit, ink);
            self.fonts
                .draw(c, Face::Atkinson, ui, x, b2, &line.after, ink);
            if !sel {
                c.fill(Rect::new(r.x, r.y + r.h - 1, r.w, 1), t.rule);
            }
        }
        if l.foot.h <= 0 {
            return;
        }
        c.fill(Rect::new(l.foot.x, l.foot.y, l.foot.w, 1), t.rule);
        let mut status = Self::search_status(m, l.visible);
        let room = l.status.w as f32;
        if self.fonts.width(Face::Atkinson, ui, &status) > room {
            let ell = self.fonts.width(Face::Atkinson, ui, &ELLIPSIS.to_string());
            status = self.cut_end(&status, Face::Atkinson, ui, room - ell);
            status.push(ELLIPSIS);
        }
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            l.status.x as f32,
            l.foot.y as f32 + l.foot.h as f32 / 2.0 + self.fonts.ascent(Face::Atkinson, ui) / 2.6,
            &status,
            t.mute,
        );
        let pages = Self::search_pages(m, l.visible);
        let page = m.first / l.visible.max(1);
        let tone = |on: bool| if on { t.ink } else { t.mute };
        draw_icon(c, Icon::PrevPage, l.prev, tone(page > 0));
        draw_icon(c, Icon::NextPage, l.next, tone(page + 1 < pages));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::settings::UI_SCALES;
    use sigma_core::{Keyboard, Settings};

    fn rows(n: usize) -> Vec<SearchRow> {
        (0..n)
            .map(|i| SearchRow {
                path: format!("notes/day {i}.md"),
                line: i + 1,
                text: format!("Line {i} says hello to the world."),
                hit: (13, 18),
            })
            .collect()
    }

    fn model<'a>(rows: &'a [SearchRow], theme: Theme) -> SearchListModel<'a> {
        SearchListModel {
            rows,
            first: 0,
            selected: None,
            state: SearchState::Done,
            notes_total: 12,
            notes_with_hits: 3,
            capped: false,
            theme,
        }
    }

    /// The editor with the Find bar open at `ui_scale`, and its page rect
    /// with the keyboard up.
    fn screen(w: u32, h: u32, ui_scale: u16) -> (EditorScreen, Rect) {
        let mut e = EditorScreen::new(w, h);
        let mut st = Settings {
            ui_scale,
            ..Settings::default()
        };
        e.configure(&mut st, None);
        e.find_open = true;
        let page = e.layout(true, &Keyboard::default()).page;
        (e, page)
    }

    fn px(c: &Canvas, x: i32, y: i32) -> u8 {
        c.pixels[y as usize * c.width as usize + x as usize]
    }

    fn inside(outer: Rect, r: Rect) -> bool {
        r.x >= outer.x
            && r.y >= outer.y
            && r.x + r.w <= outer.x + outer.w
            && r.y + r.h <= outer.y + outer.h
    }

    #[test]
    fn fits_the_page_at_every_scale_and_orientation() {
        let rs = rows(60);
        for &scale in &UI_SCALES {
            for (w, h) in [(1440u32, 1920u32), (1920, 1440)] {
                let (e, page) = screen(w, h, scale);
                let m = model(&rs, Theme::DAY);
                let l = e.search_layout(page, &m);
                let tag = format!("{scale}% {w}x{h}");
                // Landscape at 200 % with the keyboard up leaves the
                // three-row bar no room for a row: only the status line.
                assert!(
                    l.visible >= 1 || page.h < l.row_h + l.foot.h,
                    "{tag}: no row fits, page {page:?}"
                );
                assert!(l.row_h >= 44 && l.row_h >= e.scale().px(44.0), "{tag}");
                assert!(l.prev.w >= 44 && l.next.w >= 44 && l.prev.h >= 44, "{tag}");
                for r in [l.prev, l.next, l.foot, l.status] {
                    assert!(inside(page, r), "{tag}: {r:?} outside {page:?}");
                }
                assert_eq!(l.rows.len(), l.visible, "{tag}");
                for (_, r) in &l.rows {
                    assert!(inside(page, *r), "{tag}: row {r:?} outside {page:?}");
                    assert!(
                        r.y + r.h <= l.foot.y,
                        "{tag}: row {r:?} over foot {:?}",
                        l.foot
                    );
                }
                assert!(l.status.x + l.status.w <= l.prev.x, "{tag}");
                // Nothing is drawn outside the page.
                let mut c = Canvas::new(w, h, 128);
                e.draw_search(&mut c, &l, &m);
                for (x, y) in [
                    (page.x + 10, page.y - 1),
                    (page.x + 10, page.y + page.h),
                    (page.x + page.w / 2, page.y - 1),
                    (page.x + page.w / 2, page.y + page.h),
                ] {
                    assert_eq!(px(&c, x, y), 128, "{tag}: ink outside the page at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn hits_map_to_rows_and_arrows() {
        let rs = rows(60);
        let (e, page) = screen(1440, 1920, 200);
        let mut m = model(&rs, Theme::DAY);
        let l = e.search_layout(page, &m);
        for (k, (i, r)) in l.rows.iter().enumerate() {
            assert_eq!(*i, k);
            assert_eq!(
                e.search_hit(&l, r.x + 50, r.y + r.h / 2),
                SearchListHit::Row(k)
            );
        }
        let c = |r: Rect| (r.x + r.w / 2, r.y + r.h / 2);
        let (x, y) = c(l.prev);
        assert_eq!(e.search_hit(&l, x, y), SearchListHit::Prev);
        let (x, y) = c(l.next);
        assert_eq!(e.search_hit(&l, x, y), SearchListHit::Next);
        assert_eq!(
            e.search_hit(&l, l.status.x + 5, l.foot.y + l.foot.h / 2),
            SearchListHit::None
        );
        assert_eq!(
            e.search_hit(&l, page.x + 5, page.y - 5),
            SearchListHit::None
        );
        // The second page starts where the first ended.
        m.first = l.visible;
        let l2 = e.search_layout(page, &m);
        assert_eq!(l2.rows[0].0, l.visible);
        assert_eq!(l2.rows[0].1, l.rows[0].1);
        // A short last page has fewer rows.
        m.first = 60 - 2;
        let l3 = e.search_layout(page, &m);
        assert_eq!(l3.rows.len(), 2);
    }

    #[test]
    fn status_strings_for_every_state() {
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let rs = rows(60);
        let (e, page) = screen(1440, 1920, 200);
        let mut m = model(&rs, Theme::DAY);
        let l = e.search_layout(page, &m);
        let pages = 60usize.div_ceil(l.visible);
        assert!(pages > 1);
        assert_eq!(
            EditorScreen::search_status(&m, l.visible),
            format!("60 matches in 3 notes \u{b7} page 1 of {pages}")
        );
        m.first = l.visible;
        assert_eq!(
            EditorScreen::search_status(&m, l.visible),
            format!("60 matches in 3 notes \u{b7} page 2 of {pages}")
        );
        m.first = 0;
        m.capped = true;
        assert_eq!(
            EditorScreen::search_status(&m, l.visible),
            format!("60 matches in 3 notes \u{b7} page 1 of {pages} \u{b7} first 60 shown")
        );
        m.capped = false;
        m.state = SearchState::Searching;
        assert_eq!(
            EditorScreen::search_status(&m, l.visible),
            "Searching\u{2026}"
        );
        m.state = SearchState::Idle;
        assert_eq!(
            EditorScreen::search_status(&m, l.visible),
            "Type to search 12 notes"
        );
        m.notes_total = 1;
        assert_eq!(
            EditorScreen::search_status(&m, l.visible),
            "Type to search 1 note"
        );
        // Singular forms and the empty answer.
        let one = rows(1);
        let mut m1 = model(&one, Theme::DAY);
        m1.notes_with_hits = 1;
        assert_eq!(
            EditorScreen::search_status(&m1, l.visible),
            "1 match in 1 note"
        );
        let three = rows(3);
        let mut m3 = model(&three, Theme::DAY);
        m3.notes_with_hits = 1;
        assert_eq!(
            EditorScreen::search_status(&m3, l.visible),
            "3 matches in 1 note"
        );
        let none: Vec<SearchRow> = Vec::new();
        let m0 = model(&none, Theme::DAY);
        assert_eq!(EditorScreen::search_status(&m0, l.visible), "No matches");
        // Each state draws.
        for m in [&m, &m1, &m3, &m0] {
            let mut c = Canvas::new(1440, 1920, 255);
            e.draw_search(&mut c, &e.search_layout(page, m), m);
        }
    }

    #[test]
    fn long_path_and_long_line_are_trimmed_on_char_boundaries() {
        let (e, page) = screen(1440, 1920, 200);
        let max = (page.w - e.scale().px(40.0)) as f32;
        let ui = e.ui_px();
        let before = "éàü ".repeat(60);
        let after = " ñçø".repeat(60);
        let row = SearchRow {
            path: format!("{}/notas/último día.md", "carpeta muy larga/".repeat(8)),
            line: 17,
            text: format!("\t  {before}needle{after}"),
            hit: (3 + before.len(), 3 + before.len() + 6),
        };
        let path = e.search_path_line(&row, max);
        assert!(path.starts_with('\u{2026}'), "{path}");
        assert!(
            path.ends_with("/notas/último día.md \u{b7} line 17"),
            "{path}"
        );
        assert!(e.fonts.width(Face::Atkinson, ui * SMALL, &path) <= max);
        let t = e.search_trim(&row, max);
        assert_eq!(t.hit, "needle");
        assert!(
            t.before.starts_with('\u{2026}') && t.before.ends_with("éàü "),
            "{t:?}"
        );
        assert!(
            t.after.ends_with('\u{2026}') && t.after.starts_with(" ñçø"),
            "{t:?}"
        );
        let w = e.fonts.width(Face::Atkinson, ui, &t.before)
            + e.fonts.width(Face::AtkinsonBold, ui, &t.hit)
            + e.fonts.width(Face::Atkinson, ui, &t.after);
        assert!(w <= max, "{w} > {max}");
        // A short line is shown whole, its indentation dropped.
        let short = SearchRow {
            path: "a.md".into(),
            line: 1,
            text: "   say hello there".into(),
            hit: (7, 12),
        };
        assert_eq!(
            e.search_trim(&short, max),
            TrimmedLine {
                before: "say ".into(),
                hit: "hello".into(),
                after: " there".into(),
            }
        );
        // A hit wider than the row is cut at its end; a bad range does not panic.
        let wide = SearchRow {
            path: "a.md".into(),
            line: 1,
            text: "x".repeat(400),
            hit: (0, 400),
        };
        let t = e.search_trim(&wide, max);
        assert!(t.hit.ends_with('\u{2026}') && t.before.is_empty() && t.after.is_empty());
        let bad = SearchRow {
            path: "a.md".into(),
            line: 1,
            text: "éé".into(),
            hit: (1, 3),
        };
        assert_eq!(e.search_trim(&bad, max).after, "éé");
    }

    #[test]
    fn selected_row_is_inverted_in_both_themes() {
        let rs = rows(10);
        let (e, page) = screen(1440, 1920, 200);
        for theme in [Theme::DAY, Theme::NIGHT] {
            let mut m = model(&rs, theme);
            m.selected = Some(2);
            let l = e.search_layout(page, &m);
            let mut c = Canvas::new(1440, 1920, 128);
            e.draw_search(&mut c, &l, &m);
            let (_, r) = l.rows[2];
            assert_eq!(
                px(&c, r.x + r.w - 3, r.y + 3),
                theme.ink,
                "selected row background"
            );
            let (_, r0) = l.rows[0];
            assert_eq!(
                px(&c, r0.x + r0.w - 3, r0.y + 3),
                theme.paper,
                "plain row background"
            );
        }
    }
}
