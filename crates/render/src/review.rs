//! The review sheet of "Replace in all notes" (S24.1): with the Find bar in
//! All notes scope and a replacement typed, Replace all draws this paged
//! sheet in place of the results list, inside the editor layout's page rect,
//! under the Find bar and above the on-screen keyboard (S24-D1: no blind
//! replace across notes). It is modelled on the search results list
//! (`search.rs`): a model the app fills, a layout, a draw and a hit-test.
//!
//! Top to bottom: a header line ("Replace “q” with “w”: 52 matches in 3
//! notes", or "Delete “q”: …" for an empty replacement); then rows, one line
//! high each, paged. A **note row** (a wash band) has a tick box, the note
//! path and "k of n" ticked; under it one **line row** per matched line has
//! a tick box, the line number and the line with every ticked match shown as
//! the old text struck through followed by the new text in bold, an unticked
//! match left as it is but underlined. A match inside a `[[link]]` carries a
//! small "link" marker (S24-D6: unticked by default). At the bottom: the
//! buttons **Replace in N notes** (ink, greyed while nothing is ticked) and
//! **Cancel**, "page i of n" and the page arrows. While the apply runs the
//! footer is replaced by "Replacing… i of n notes" and a flat progress bar
//! and nothing answers a tap (S24.1 step 5).
//!
//! The page shows a whole row at a time; a note whose lines continue on the
//! next page repeats its note row on top of that page. A page never ends on
//! a note row alone.
//!
//! Everything is pure ink, paper, rule and wash tokens; no row is drawn with
//! a grey of its own (UI rulebook §1, §3).

use crate::canvas::{Canvas, Rect};
use crate::drawer::draw_check;
use crate::editor::EditorScreen;
use crate::fonts::Face;
use crate::i18n::tr;
use crate::icons::{draw_icon, Icon};
use crate::search::{ELLIPSIS, SMALL};
use crate::theme::Theme;
use sigma_core::replace::{NotePlan, Plan, PreviewLine, Selection};

/// What the review sheet shows.
pub struct ReviewModel<'a> {
    /// What the replace would change (from `sigma_core::replace::plan`).
    pub plan: &'a Plan,
    /// Which matches are ticked.
    pub selection: &'a Selection,
    /// The shown page, 0-based. Clamped by [`EditorScreen::review_layout`],
    /// whose `index` is the page really shown.
    pub page: usize,
    /// `Some((done, total))` while the apply runs on the worker: the footer
    /// becomes "Replacing… done of total notes" and the sheet stops
    /// answering taps.
    pub progress: Option<(usize, usize)>,
    /// Colours to draw with.
    pub theme: Theme,
}

/// One row of the sheet, before paging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewRow {
    /// A note: index into `Plan::notes`.
    Note(usize),
    /// A matched line: note index, index into `NotePlan::lines`.
    Line(usize, usize),
}

/// A row placed on the shown page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewRowBox {
    /// What the row is.
    pub row: ReviewRow,
    /// The whole row band; a tap anywhere on it ticks or unticks.
    pub rect: Rect,
    /// A note row repeated at the top of a page because the note's lines
    /// continue from the page before.
    pub continued: bool,
    /// For a line row, the index (into `NotePlan::matches`) of the first
    /// match on the line: what [`ReviewHit::MatchTick`] carries.
    pub first: usize,
}

/// Where the sheet's parts were placed, in device pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReviewLayout {
    /// The page rect the sheet was laid out in.
    pub area: Rect,
    /// The header line.
    pub header: Rect,
    /// The rows of the shown page, top to bottom.
    pub rows: Vec<ReviewRowBox>,
    /// How many rows fit on one page (0 when the page is too short).
    pub visible: usize,
    /// Height of one row.
    pub row_h: i32,
    /// The shown page, 0-based, after clamping the model's.
    pub index: usize,
    /// Number of pages, at least one.
    pub pages: usize,
    /// The whole bottom band.
    pub foot: Rect,
    /// The ‹ button (empty with one page or while applying).
    pub prev: Rect,
    /// The › button (empty with one page or while applying).
    pub next: Rect,
    /// "Cancel" (empty while applying).
    pub cancel: Rect,
    /// "Replace in N notes" (empty while applying).
    pub apply: Rect,
    /// Whether Apply is enabled (something is ticked).
    pub apply_on: bool,
    /// "page i of n" between the buttons and the arrows; while applying,
    /// the room of the "Replacing…" line.
    pub status: Rect,
    /// Whether the apply is running (no button answers).
    pub busy: bool,
}

/// What a tap on the review sheet landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewHit {
    /// A note row: index into `Plan::notes`. Toggle it with
    /// `Selection::set_note`.
    NoteTick(usize),
    /// A line row: the note index and the index of the **first** match on
    /// the tapped line. A line can hold several matches; its plain ones are
    /// toggled together, the wiki-link ones not ([`review_line_matches`]
    /// lists what is toggled).
    MatchTick(usize, usize),
    /// The ‹ button (only when there is a previous page).
    PagePrev,
    /// The › button (only when there is a next page).
    PageNext,
    /// "Replace in N notes" (only when something is ticked).
    Apply,
    /// "Cancel".
    Cancel,
    /// Nothing that responds to a tap (everything, while applying).
    None,
}

/// How a span of a review line is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanStyle {
    /// Text outside any match.
    Plain,
    /// A ticked match: the old text, struck through.
    Struck,
    /// The new text after a struck match, bold.
    New,
    /// An unticked match: unchanged text, underlined.
    Unticked,
}

/// A piece of a review line drawn in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewSpan {
    /// The text (tabs and line breaks already turned into blanks).
    pub text: String,
    /// How it is drawn.
    pub style: SpanStyle,
}

/// The rows of `plan` before paging: each note, then its matched lines.
#[must_use]
pub fn review_rows(plan: &Plan) -> Vec<ReviewRow> {
    let mut out = Vec::new();
    for (i, n) in plan.notes.iter().enumerate() {
        out.push(ReviewRow::Note(i));
        out.extend((0..n.lines.len()).map(|l| ReviewRow::Line(i, l)));
    }
    out
}

/// The matches (indices into `NotePlan::matches`) that the tick box of a
/// preview line stands for: its matches outside a `[[wiki link]]`. Matches in
/// a link start unticked (S24-D6) and a line tick never turns them on. Only
/// on a line that holds nothing but link matches is the box theirs, since
/// there is no other control to tick them with.
#[must_use]
pub fn line_tick_targets(line: &PreviewLine) -> Vec<usize> {
    let plain: Vec<usize> = line
        .ranges
        .iter()
        .filter(|r| !r.in_link)
        .map(|r| r.index)
        .collect();
    if plain.is_empty() {
        line.ranges.iter().map(|r| r.index).collect()
    } else {
        plain
    }
}

/// The indices (into `NotePlan::matches`) that a tap on the tick box of the
/// preview line whose first match is `first` toggles together
/// ([`line_tick_targets`]). Empty if no line starts with `first`.
#[must_use]
pub fn review_line_matches(note: &NotePlan, first: usize) -> Vec<usize> {
    note.lines
        .iter()
        .find(|l| l.ranges.first().is_some_and(|r| r.index == first))
        .map(line_tick_targets)
        .unwrap_or_default()
}

/// A page of rows: `start..end` of the flat list, plus whether its note row
/// is repeated on top.
struct PageSpan {
    start: usize,
    end: usize,
    cont: bool,
}

/// Splits `rows` into pages of at most `visible` rows. A page that starts in
/// the middle of a note gives one of its rows to the note's repeated header;
/// a page does not end on a note row unless that is all it holds.
fn page_spans(rows: &[ReviewRow], visible: usize) -> Vec<PageSpan> {
    if visible == 0 || rows.is_empty() {
        return vec![PageSpan {
            start: 0,
            end: 0,
            cont: false,
        }];
    }
    let mut out = Vec::new();
    let mut start = 0;
    while start < rows.len() {
        let cont = visible >= 2 && matches!(rows[start], ReviewRow::Line(..));
        let cap = (visible - usize::from(cont)).max(1);
        let mut end = (start + cap).min(rows.len());
        if end < rows.len() && end - start > 1 && matches!(rows[end - 1], ReviewRow::Note(_)) {
            end -= 1;
        }
        out.push(PageSpan { start, end, cont });
        start = end;
    }
    out
}

/// `s` with blanks for tabs and line breaks, cut to `n` characters with an
/// ellipsis at the end.
fn clip_chars(s: &str, n: usize) -> String {
    let t: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if t.chars().count() <= n {
        return t;
    }
    let mut out: String = t.chars().take(n).collect();
    out.push(ELLIPSIS);
    out
}

/// `template` with each `{key}` replaced by its value in one pass, so a
/// value that looks like a placeholder is never substituted again.
fn fill(template: &str, vals: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut rest = template;
    'scan: while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        for (k, v) in vals {
            if let Some(r) = rest.strip_prefix(k) {
                out.push_str(v);
                rest = r;
                continue 'scan;
            }
        }
        out.push('{');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

/// The spans of one preview line: leading blanks dropped, each ticked match
/// as its old text (struck) followed by `with` (new, bold, none when `with`
/// is empty), each unticked one as unchanged text, the rest plain.
/// Ranges that are out of order, overlap or fall inside a character are
/// skipped rather than panicking.
#[must_use]
pub fn review_spans(
    line: &PreviewLine,
    with: &str,
    ticked: impl Fn(usize) -> bool,
) -> Vec<ReviewSpan> {
    let text = &line.text;
    let tidy = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect()
    };
    let lead = text.len() - text.trim_start().len();
    let mut out: Vec<ReviewSpan> = Vec::new();
    let mut push = |s: &str, style: SpanStyle| {
        if !s.is_empty() {
            out.push(ReviewSpan {
                text: tidy(s),
                style,
            });
        }
    };
    let mut pos = lead;
    for r in &line.ranges {
        let (a, b) = (r.from, r.to.min(text.len()));
        if a < pos || b < a || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
            continue;
        }
        push(&text[pos..a], SpanStyle::Plain);
        if ticked(r.index) {
            push(&text[a..b], SpanStyle::Struck);
            push(with, SpanStyle::New);
        } else {
            push(&text[a..b], SpanStyle::Unticked);
        }
        pos = b;
    }
    push(&text[pos.min(text.len())..], SpanStyle::Plain);
    out
}

impl EditorScreen {
    /// Height of one row of the review sheet: one line of text with padding,
    /// never under the 44 px minimum target.
    #[must_use]
    pub fn review_row_h(&self) -> i32 {
        let s = self.scale();
        let text = (self.ui_px() * 1.35).ceil() as i32;
        (s.px(8.0) * 2 + text)
            .max(s.px(44.0))
            .max(self.page_scale().px(44.0))
    }

    /// The label of the Apply button, translated: "Replace in N notes" with
    /// N the notes that have a ticked match.
    #[must_use]
    pub fn review_apply_label(m: &ReviewModel<'_>) -> String {
        let n = m.selection.notes();
        let s = if n == 1 {
            "Replace in 1 note".to_owned()
        } else {
            format!("Replace in {n} notes")
        };
        tr(&s).into_owned()
    }

    /// The header line, translated: "Replace “q” with “w”: N matches in M
    /// notes" ("Delete “q”: …" when the replacement is empty). The query and
    /// the replacement are shortened (32, then 20, 12, 6, 3 characters, an
    /// ellipsis at the cut) until the line fits `max` pixels in bold, so the
    /// counts stay visible; if it still does not fit its end is cut.
    #[must_use]
    pub fn review_header(&self, plan: &Plan, max: f32) -> String {
        let n = plan.total_matches();
        let notes = plan.notes.len();
        let counts = match (n, notes) {
            (0, _) => "No matches".to_owned(),
            (1, _) => "1 match in 1 note".to_owned(),
            (_, 1) => format!("{n} matches in 1 note"),
            _ => format!("{n} matches in {notes} notes"),
        };
        let counts = tr(&counts).into_owned();
        let ui = self.ui_px();
        let mut out = String::new();
        for cut in [32, 20, 12, 6, 3] {
            let q = clip_chars(&plan.query, cut);
            let head = if plan.with.is_empty() {
                fill(&tr("Delete \u{201c}{q}\u{201d}"), &[("{q}", &q)])
            } else {
                let w = clip_chars(&plan.with, cut);
                fill(
                    &tr("Replace \u{201c}{q}\u{201d} with \u{201c}{w}\u{201d}"),
                    &[("{q}", &q), ("{w}", &w)],
                )
            };
            out = format!("{head}: {counts}");
            if self.fonts.width(Face::AtkinsonBold, ui, &out) <= max {
                return out;
            }
        }
        let ell = self
            .fonts
            .width(Face::AtkinsonBold, ui, &ELLIPSIS.to_string());
        let mut cut = self.cut_end(&out, Face::AtkinsonBold, ui, max - ell);
        cut.push(ELLIPSIS);
        cut
    }

    /// Lays out the sheet inside `page` (the editor layout's page rect) for
    /// `m`.
    #[must_use]
    pub fn review_layout(&self, page: Rect, m: &ReviewModel<'_>) -> ReviewLayout {
        let s = self.scale();
        let ui = self.ui_px();
        let row_h = self.review_row_h();
        let btn = s.px(44.0);
        let gap = s.px(8.0);
        let pad = s.px(12.0);
        let side = s.px(20.0);
        let foot_h = (btn + 2 * s.px(6.0)).min(page.h.max(0));
        // The header is dropped when it would leave no room for a row: the
        // buttons and one row matter more (landscape, keyboard up).
        let head_h = if page.h - foot_h >= 2 * row_h {
            row_h
        } else {
            0
        };
        let header = Rect::new(page.x, page.y, page.w, head_h);
        let foot = Rect::new(page.x, page.y + page.h - foot_h, page.w, foot_h);
        let avail = page.h - head_h - foot_h;
        let visible = if avail >= row_h {
            (avail / row_h) as usize
        } else {
            0
        };
        let flat = review_rows(m.plan);
        let spans = page_spans(&flat, visible);
        let pages = spans.len();
        let index = m.page.min(pages - 1);
        let sp = &spans[index];
        let mut rows = Vec::new();
        let mut y = page.y + head_h;
        let mut put = |row: ReviewRow, continued: bool| {
            let first = match row {
                ReviewRow::Line(n, l) => m
                    .plan
                    .notes
                    .get(n)
                    .and_then(|np| np.lines.get(l))
                    .and_then(|pl| pl.ranges.first())
                    .map_or(0, |r| r.index),
                ReviewRow::Note(_) => 0,
            };
            rows.push(ReviewRowBox {
                row,
                rect: Rect::new(page.x, y, page.w, row_h),
                continued,
                first,
            });
            y += row_h;
        };
        if sp.cont {
            if let ReviewRow::Line(n, _) = flat[sp.start] {
                put(ReviewRow::Note(n), true);
            }
        }
        for &row in &flat[sp.start..sp.end] {
            put(row, false);
        }
        let apply_on = m.selection.total() > 0;
        let busy = m.progress.is_some();
        let mut l = ReviewLayout {
            area: page,
            header,
            rows,
            visible,
            row_h,
            index,
            pages,
            foot,
            apply_on,
            busy,
            ..ReviewLayout::default()
        };
        if foot.h <= 0 {
            return l;
        }
        if busy {
            l.status = Rect::new(page.x + side, foot.y, (page.w - 2 * side).max(1), foot.h);
            return l;
        }
        let by = foot.y + (foot.h - btn) / 2;
        let mut right = page.x + page.w - pad;
        if pages > 1 {
            l.next = Rect::new(right - btn, by, btn, btn);
            l.prev = Rect::new(l.next.x - gap - btn, by, btn, btn);
            right = l.prev.x - gap;
        }
        let label_w = |face: Face, text: &str| {
            (self.fonts.width(face, ui, text).ceil() as i32 + 2 * s.px(16.0)).max(btn)
        };
        let cw = label_w(Face::LiterataItalic, &tr("Cancel"));
        let aw = label_w(Face::LiterataItalic, &Self::review_apply_label(m));
        let cx = page.x + side;
        l.cancel = Rect::new(cx, by, cw.min((right - cx).max(1)), btn);
        let ax = l.cancel.x + l.cancel.w + gap;
        l.apply = Rect::new(ax, by, aw.min((right - ax).max(1)), btn);
        let sx = l.apply.x + l.apply.w + gap;
        l.status = Rect::new(sx, foot.y, (right - sx).max(0), foot.h);
        l
    }

    /// What the sheet has at (`x`, `y`), in device pixels. The arrows take a
    /// few pixels of slop around them. A row answers anywhere on its band.
    /// While applying, and for an arrow with no page behind it or Apply with
    /// nothing ticked, the answer is [`ReviewHit::None`].
    #[must_use]
    pub fn review_hit(&self, l: &ReviewLayout, x: i32, y: i32) -> ReviewHit {
        if l.busy {
            return ReviewHit::None;
        }
        let slop = -self.scale().px(4.0);
        if l.pages > 1 {
            if l.index > 0 && l.prev.inset(slop).contains(x, y) {
                return ReviewHit::PagePrev;
            }
            if l.index + 1 < l.pages && l.next.inset(slop).contains(x, y) {
                return ReviewHit::PageNext;
            }
        }
        if l.cancel.w > 0 && l.cancel.contains(x, y) {
            return ReviewHit::Cancel;
        }
        if l.apply_on && l.apply.w > 0 && l.apply.contains(x, y) {
            return ReviewHit::Apply;
        }
        l.rows
            .iter()
            .find(|b| b.rect.contains(x, y))
            .map_or(ReviewHit::None, |b| match b.row {
                ReviewRow::Note(i) => ReviewHit::NoteTick(i),
                ReviewRow::Line(i, _) => ReviewHit::MatchTick(i, b.first),
            })
    }

    /// The gap between a struck match and the new text after it.
    fn new_gap(&self) -> f32 {
        self.ui_px() * 0.25
    }

    /// Width of `span` in pixels at the body size, with the gap before new
    /// text.
    fn span_w(&self, span: &ReviewSpan) -> f32 {
        let face = span_face(span.style);
        let w = self.fonts.width(face, self.ui_px(), &span.text);
        if span.style == SpanStyle::New {
            w + self.new_gap()
        } else {
            w
        }
    }

    /// `spans` cut from the end, an ellipsis last, until they fit `max`.
    fn cut_spans_end(&self, spans: &[ReviewSpan], max: f32) -> Vec<ReviewSpan> {
        let ui = self.ui_px();
        let mut out = Vec::new();
        let mut used = 0.0;
        for sp in spans {
            let w = self.span_w(sp);
            if used + w <= max {
                used += w;
                out.push(sp.clone());
                continue;
            }
            let face = span_face(sp.style);
            let ell = self.fonts.width(face, ui, &ELLIPSIS.to_string());
            let gap = if sp.style == SpanStyle::New {
                self.new_gap()
            } else {
                0.0
            };
            let mut text = self.cut_end(&sp.text, face, ui, max - used - ell - gap);
            text.push(ELLIPSIS);
            if used + ell + gap <= max {
                out.push(ReviewSpan {
                    text,
                    style: sp.style,
                });
            }
            break;
        }
        out
    }

    /// `spans` (from [`review_spans`]) trimmed to fit `max` pixels: the
    /// stretch from the first to the last match stays whole, with some text
    /// before and after it, each cut side marked with an ellipsis. If even
    /// that stretch is wider than `max` it is cut at its end, so the old
    /// text of the first match stays visible.
    #[must_use]
    pub fn review_trim(&self, spans: &[ReviewSpan], max: f32) -> Vec<ReviewSpan> {
        let ui = self.ui_px();
        let total: f32 = spans.iter().map(|s| self.span_w(s)).sum();
        if total <= max {
            return spans.to_vec();
        }
        let a = spans.iter().position(|s| s.style != SpanStyle::Plain);
        let b = spans.iter().rposition(|s| s.style != SpanStyle::Plain);
        let (Some(a), Some(b)) = (a, b) else {
            return self.cut_spans_end(spans, max);
        };
        let core = &spans[a..=b];
        let core_w: f32 = core.iter().map(|s| self.span_w(s)).sum();
        if core_w > max {
            return self.cut_spans_end(core, max);
        }
        let plain = |v: &[ReviewSpan]| -> String { v.iter().map(|s| s.text.as_str()).collect() };
        let (mut before, mut after) = (plain(&spans[..a]), plain(&spans[b + 1..]));
        let w = |s: &str| self.fonts.width(Face::Atkinson, ui, s);
        let ell = w(&ELLIPSIS.to_string());
        let budget = max - core_w;
        let (before_w, after_w) = (w(&before), w(&after));
        // Some context before the match (a third of the room, or more when
        // the text after it is short), the rest after it.
        let want_before = (budget / 3.0).max(budget - after_w).min(before_w);
        if before_w > want_before {
            if want_before < ell {
                before.clear();
            } else {
                before = self.cut_start(&before, Face::Atkinson, ui, want_before - ell);
                before.insert(0, ELLIPSIS);
            }
        }
        let room_after = budget - w(&before);
        if after_w > room_after {
            if room_after < ell {
                after.clear();
            } else {
                after = self.cut_end(&after, Face::Atkinson, ui, room_after - ell);
                after.push(ELLIPSIS);
            }
        }
        let mut out = Vec::new();
        if !before.is_empty() {
            out.push(ReviewSpan {
                text: before,
                style: SpanStyle::Plain,
            });
        }
        out.extend_from_slice(core);
        if !after.is_empty() {
            out.push(ReviewSpan {
                text: after,
                style: SpanStyle::Plain,
            });
        }
        out
    }

    /// Draws the review sheet into `l` (from [`Self::review_layout`]): the
    /// header, the rows, then the footer.
    pub fn draw_review(&self, c: &mut Canvas, l: &ReviewLayout, m: &ReviewModel<'_>) {
        let t = m.theme;
        let s = self.scale();
        let ui = self.ui_px();
        let small = ui * SMALL;
        let side = s.px(20.0);
        c.fill(l.area, t.paper);
        // Header.
        if l.header.h > 0 {
            let max = (l.area.w - 2 * side) as f32;
            let head = self.review_header(m.plan, max);
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                (l.area.x + side) as f32,
                l.header.y as f32 + l.header.h as f32 / 2.0 + ui * 0.35,
                &head,
                t.ink,
            );
            c.fill(
                Rect::new(l.header.x, l.header.y + l.header.h - 1, l.header.w, 1),
                t.rule,
            );
        }
        for b in &l.rows {
            match b.row {
                ReviewRow::Note(i) => self.draw_note_row(c, b, i, m, small),
                ReviewRow::Line(i, li) => self.draw_line_row(c, b, i, li, m, small),
            }
        }
        if l.foot.h <= 0 {
            return;
        }
        c.fill(Rect::new(l.foot.x, l.foot.y, l.foot.w, 1), t.rule);
        if l.busy {
            self.draw_progress(c, l, m);
            return;
        }
        self.draw_button(c, l.cancel, &tr("Cancel"), false, true, t);
        self.draw_button(
            c,
            l.apply,
            &Self::review_apply_label(m),
            true,
            l.apply_on,
            t,
        );
        if l.pages > 1 {
            let tone = |on: bool| if on { t.ink } else { t.mute };
            draw_icon(c, Icon::PrevPage, l.prev, tone(l.index > 0));
            draw_icon(c, Icon::NextPage, l.next, tone(l.index + 1 < l.pages));
            let mut text = tr(&format!("page {} of {}", l.index + 1, l.pages)).into_owned();
            let room = l.status.w as f32;
            let ell = self.fonts.width(Face::Atkinson, ui, &ELLIPSIS.to_string());
            if self.fonts.width(Face::Atkinson, ui, &text) > room {
                text = self.cut_end(&text, Face::Atkinson, ui, room - ell);
                text.push(ELLIPSIS);
            }
            if room >= ell {
                self.fonts.draw(
                    c,
                    Face::Atkinson,
                    ui,
                    l.status.x as f32,
                    l.foot.y as f32 + l.foot.h as f32 / 2.0 + ui * 0.35,
                    &text,
                    t.mute,
                );
            }
        }
    }

    /// A wash button with an italic label: ink with a paper label when it is
    /// the enabled primary one, wash with a mute label when disabled.
    fn draw_button(&self, c: &mut Canvas, r: Rect, label: &str, primary: bool, on: bool, t: Theme) {
        if r.w <= 0 {
            return;
        }
        let ui = self.ui_px();
        let (bg, fg) = match (primary, on) {
            (true, true) => (t.ink, t.paper),
            (_, false) => (t.wash, t.mute),
            _ => (t.wash, t.ink),
        };
        c.fill(r, bg);
        let room = (r.w - 2 * self.scale().px(8.0)) as f32;
        let mut text = label.to_owned();
        let mut w = self.fonts.width(Face::LiterataItalic, ui, &text);
        if w > room {
            let ell = self
                .fonts
                .width(Face::LiterataItalic, ui, &ELLIPSIS.to_string());
            text = self.cut_end(&text, Face::LiterataItalic, ui, room - ell);
            text.push(ELLIPSIS);
            w = self.fonts.width(Face::LiterataItalic, ui, &text);
        }
        self.fonts.draw(
            c,
            Face::LiterataItalic,
            ui,
            r.x as f32 + (r.w as f32 - w) / 2.0,
            r.y as f32 + r.h as f32 / 2.0 + ui * 0.35,
            &text,
            fg,
        );
    }

    /// The footer while applying: "Replacing… i of n notes" over a flat bar,
    /// dark where done and the rule tone for the rest.
    fn draw_progress(&self, c: &mut Canvas, l: &ReviewLayout, m: &ReviewModel<'_>) {
        let t = m.theme;
        let ui = self.ui_px();
        let s = self.scale();
        let (done, total) = m.progress.unwrap_or((0, 0));
        let mut text = tr(&format!("Replacing\u{2026} {done} of {total} notes")).into_owned();
        let room = l.status.w as f32;
        let ell = self.fonts.width(Face::Atkinson, ui, &ELLIPSIS.to_string());
        if self.fonts.width(Face::Atkinson, ui, &text) > room {
            text = self.cut_end(&text, Face::Atkinson, ui, room - ell);
            text.push(ELLIPSIS);
        }
        let bar_h = s.px(4.0).max(2);
        let bar = Rect::new(
            l.status.x,
            l.foot.y + l.foot.h - s.px(8.0) - bar_h,
            l.status.w,
            bar_h,
        );
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            l.status.x as f32,
            l.foot.y as f32 + (bar.y - l.foot.y) as f32 / 2.0 + ui * 0.35,
            &text,
            t.ink,
        );
        c.fill(bar, t.rule);
        let filled = if total == 0 {
            0
        } else {
            (i64::from(bar.w) * done.min(total) as i64 / total as i64) as i32
        };
        c.fill(Rect::new(bar.x, bar.y, filled, bar.h), t.ink);
    }

    /// The box of a tick, centred in the 44 px target at `x`.
    fn tick_box(&self, r: Rect, x: i32) -> Rect {
        let side = self.scale().px(24.0);
        Rect::new(x, r.y + (r.h - side) / 2, side, side)
    }

    fn draw_note_row(
        &self,
        c: &mut Canvas,
        b: &ReviewRowBox,
        i: usize,
        m: &ReviewModel<'_>,
        small: f32,
    ) {
        let t = m.theme;
        let s = self.scale();
        let ui = self.ui_px();
        let r = b.rect;
        let Some(np) = m.plan.notes.get(i) else {
            return;
        };
        c.fill(r, t.wash);
        let side = s.px(20.0);
        let on = m.selection.ticked_in(i);
        let bx = self.tick_box(r, r.x + side);
        draw_check(c, bx, on > 0, s.px(2.0).max(2), t);
        let count = tr(&format!("{} of {}", on, np.matches.len())).into_owned();
        let cw = self.fonts.width(Face::Atkinson, small, &count);
        let cx = (r.x + r.w - side) as f32 - cw;
        self.fonts.draw(
            c,
            Face::Atkinson,
            small,
            cx,
            r.y as f32 + r.h as f32 / 2.0 + small * 0.35,
            &count,
            t.ink,
        );
        let nx = bx.x + bx.w + s.px(12.0);
        let room = cx - (s.px(12.0) + nx) as f32;
        let mut path = np.path.as_str().to_owned();
        if self.fonts.width(Face::AtkinsonBold, ui, &path) > room {
            let ell = self
                .fonts
                .width(Face::AtkinsonBold, ui, &ELLIPSIS.to_string());
            path = self.cut_start(&path, Face::AtkinsonBold, ui, room - ell);
            path.insert(0, ELLIPSIS);
        }
        self.fonts.draw(
            c,
            Face::AtkinsonBold,
            ui,
            nx as f32,
            r.y as f32 + r.h as f32 / 2.0 + ui * 0.35,
            &path,
            t.ink,
        );
        if b.continued {
            // The note's lines carry on from the page before.
            c.fill(Rect::new(r.x, r.y, r.w, 1), t.rule);
        }
    }

    fn draw_line_row(
        &self,
        c: &mut Canvas,
        b: &ReviewRowBox,
        i: usize,
        li: usize,
        m: &ReviewModel<'_>,
        small: f32,
    ) {
        let t = m.theme;
        let s = self.scale();
        let ui = self.ui_px();
        let r = b.rect;
        let Some(line) = m.plan.notes.get(i).and_then(|n| n.lines.get(li)) else {
            return;
        };
        let side = s.px(20.0);
        // The box shows the matches it toggles (not the wiki-link ones on a
        // line that also has others).
        let targets = line_tick_targets(line);
        let n_on = targets
            .iter()
            .filter(|&&k| m.selection.is_ticked(i, k))
            .count();
        let bx = self.tick_box(r, r.x + side + s.px(24.0));
        let stroke = s.px(2.0).max(2);
        draw_check(
            c,
            bx,
            !targets.is_empty() && n_on == targets.len(),
            stroke,
            t,
        );
        if n_on > 0 && n_on < targets.len() {
            // Some of the line's matches: a dash across the empty box.
            c.fill(
                Rect::new(
                    bx.x + bx.w / 4,
                    bx.y + (bx.h - stroke) / 2,
                    bx.w / 2,
                    stroke,
                ),
                t.ink,
            );
        }
        let base_small = r.y as f32 + r.h as f32 / 2.0 + small * 0.35;
        let base = r.y as f32 + r.h as f32 / 2.0 + ui * 0.35;
        // Line number, right-aligned in a four-digit column.
        let col = self.fonts.width(Face::Atkinson, small, "8888");
        let num = line.line.to_string();
        let nw = self.fonts.width(Face::Atkinson, small, &num);
        let nx = (bx.x + bx.w + s.px(10.0)) as f32;
        self.fonts.draw(
            c,
            Face::Atkinson,
            small,
            nx + col - nw,
            base_small,
            &num,
            t.mute,
        );
        // The "link" marker at the right edge.
        let mut right = (r.x + r.w - side) as f32;
        if line.ranges.iter().any(|g| g.in_link) {
            let word = tr("link").into_owned();
            let ww = self.fonts.width(Face::Atkinson, small, &word);
            let padx = s.px(6.0);
            let tag_h = (small * 1.5).ceil() as i32;
            let tag = Rect::new(
                (right - ww) as i32 - padx,
                r.y + (r.h - tag_h) / 2,
                ww.ceil() as i32 + 2 * padx,
                tag_h,
            );
            c.stroke(tag, 1, t.ink);
            self.fonts.draw(
                c,
                Face::Atkinson,
                small,
                (tag.x + padx) as f32,
                base_small,
                &word,
                t.ink,
            );
            right = tag.x as f32 - s.px(10.0) as f32;
        }
        let tx = nx + col + s.px(12.0) as f32;
        let max = right - tx;
        let spans = review_spans(line, &m.plan.with, |k| m.selection.is_ticked(i, k));
        let spans = self.review_trim(&spans, max);
        let thick = ((ui / 14.0).round() as i32).max(1);
        let mut x = tx;
        for sp in &spans {
            let face = span_face(sp.style);
            if sp.style == SpanStyle::New {
                x += self.new_gap();
            }
            let x1 = self.fonts.draw(c, face, ui, x, base, &sp.text, t.ink);
            match sp.style {
                SpanStyle::Struck => c.fill(
                    Rect::new(
                        x as i32,
                        (base - ui * 0.28) as i32,
                        (x1 - x).ceil() as i32,
                        thick,
                    ),
                    t.ink,
                ),
                SpanStyle::Unticked => c.fill(
                    Rect::new(
                        x as i32,
                        (base + ui * 0.12) as i32,
                        (x1 - x).ceil() as i32,
                        thick,
                    ),
                    t.ink,
                ),
                _ => {}
            }
            x = x1;
        }
        c.fill(Rect::new(r.x, r.y + r.h - 1, r.w, 1), t.rule);
    }
}

fn span_face(style: SpanStyle) -> Face {
    if style == SpanStyle::New {
        Face::AtkinsonBold
    } else {
        Face::Atkinson
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::replace::{LineMatch, Match};
    use sigma_core::settings::UI_SCALES;
    use sigma_core::{Keyboard, NotePath, Settings};
    use std::path::PathBuf;

    /// A note of `lines` matched lines, one match each on `needle` at byte
    /// 6 of "Line N needle here." (the `in_link` one is the second line).
    fn note(path: &str, lines: usize) -> NotePlan {
        let mut matches = Vec::new();
        let mut pl = Vec::new();
        for k in 0..lines {
            let text = format!("Line {k} needle here, and more words after it.");
            let from = text.find("needle").unwrap();
            let in_link = k == 1;
            matches.push(Match {
                from: k * 100 + from,
                to: k * 100 + from + 6,
                in_link,
            });
            pl.push(PreviewLine {
                line: k + 1,
                text,
                ranges: vec![LineMatch {
                    index: k,
                    from,
                    to: from + 6,
                    in_link,
                }],
            });
        }
        NotePlan {
            path: NotePath::parse(path).unwrap(),
            matches,
            lines: pl,
            open: false,
        }
    }

    fn plan(notes: &[(&str, usize)], with: &str) -> Plan {
        Plan {
            root: PathBuf::from("/tmp/none"),
            query: "needle".into(),
            case: false,
            with: with.into(),
            notes: notes.iter().map(|(p, n)| note(p, *n)).collect(),
            skipped: 0,
            scanned: notes.len(),
            elapsed_ms: 0,
        }
    }

    fn model<'a>(p: &'a Plan, sel: &'a Selection, theme: Theme) -> ReviewModel<'a> {
        ReviewModel {
            plan: p,
            selection: sel,
            page: 0,
            progress: None,
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

    fn centre(r: Rect) -> (i32, i32) {
        (r.x + r.w / 2, r.y + r.h / 2)
    }

    #[test]
    fn fits_the_page_at_every_scale_and_orientation() {
        let p = plan(
            &[("a/one.md", 6), ("two.md", 3), ("deep/er/three.md", 9)],
            "pin",
        );
        let sel = Selection::defaults(&p);
        for &scale in &UI_SCALES {
            for (w, h) in [(1440u32, 1920u32), (1920, 1440)] {
                let (e, page) = screen(w, h, scale);
                let m = model(&p, &sel, Theme::DAY);
                let l = e.review_layout(page, &m);
                let tag = format!("{scale}% {w}x{h}");
                // Landscape at 200 % with the keyboard up leaves no room for
                // a row: only the footer.
                assert!(
                    l.visible >= 1 || l.area.h < l.row_h + l.foot.h,
                    "{tag}: no row fits, page {page:?}"
                );
                assert!(l.row_h >= 44 && l.row_h >= e.scale().px(44.0), "{tag}");
                for r in [l.header, l.foot, l.cancel, l.apply, l.status] {
                    assert!(inside(page, r), "{tag}: {r:?} outside {page:?}");
                }
                for r in [l.cancel, l.apply] {
                    assert!(r.w >= 44 && r.h >= 44, "{tag}: button {r:?}");
                }
                if l.pages > 1 {
                    for r in [l.prev, l.next] {
                        assert!(inside(page, r) && r.w >= 44 && r.h >= 44, "{tag}: {r:?}");
                    }
                    assert!(l.apply.x + l.apply.w <= l.prev.x, "{tag}: apply over prev");
                    assert!(l.status.x + l.status.w <= l.prev.x, "{tag}");
                }
                assert!(
                    l.cancel.x + l.cancel.w <= l.apply.x,
                    "{tag}: buttons overlap"
                );
                assert!(l.apply.x + l.apply.w <= page.x + page.w, "{tag}");
                assert!(l.rows.len() <= l.visible, "{tag}");
                let mut y = l.header.y + l.header.h;
                for b in &l.rows {
                    assert!(inside(page, b.rect), "{tag}: row {:?} outside", b.rect);
                    assert_eq!(b.rect.y, y, "{tag}: rows are stacked");
                    assert!(b.rect.y + b.rect.h <= l.foot.y, "{tag}: row over the foot");
                    y += b.rect.h;
                }
                // Nothing is drawn outside the page.
                let mut c = Canvas::new(w, h, 128);
                e.draw_review(&mut c, &l, &m);
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
    fn pages_hold_whole_rows_and_repeat_the_note_header() {
        // 3 notes: rows = [N0, L0 L1 L2 L3 L4, N1, L0, N2, L0 L1].
        let p = plan(&[("a.md", 5), ("b.md", 1), ("c.md", 2)], "x");
        let flat = review_rows(&p);
        assert_eq!(flat.len(), 3 + 8);
        // Four rows a page: [N0 L0 L1 L2] [N0* L3 L4 -] [N1 L0 N2 -]...
        let sp = page_spans(&flat, 4);
        let spans: Vec<(usize, usize, bool)> =
            sp.iter().map(|s| (s.start, s.end, s.cont)).collect();
        assert_eq!(spans[0], (0, 4, false));
        // The second page starts mid-note: its header is repeated.
        assert!(spans[1].2 && spans[1].0 == 4);
        // No page ends on a note row (but a one-row page may be one).
        for s in &sp {
            if s.end - s.start > 1 {
                assert!(
                    !matches!(flat[s.end - 1], ReviewRow::Note(_)),
                    "page {s_start}..{s_end} ends on a note row",
                    s_start = s.start,
                    s_end = s.end
                );
            }
        }
        // Every row is on exactly one page, in order.
        let mut next = 0;
        for s in &sp {
            assert_eq!(s.start, next);
            next = s.end;
        }
        assert_eq!(next, flat.len());
        // No room for a row: one empty page.
        assert_eq!(page_spans(&flat, 0).len(), 1);
        assert_eq!(page_spans(&[], 5).len(), 1);
    }

    #[test]
    fn layout_pages_clamp_and_continue() {
        let p = plan(&[("a.md", 30), ("b.md", 4)], "x");
        let sel = Selection::defaults(&p);
        let (e, page) = screen(1440, 1920, 100);
        let mut m = model(&p, &sel, Theme::DAY);
        let l0 = e.review_layout(page, &m);
        assert!(l0.pages > 1, "{} rows visible", l0.visible);
        assert_eq!(l0.index, 0);
        assert_eq!(l0.rows.len(), l0.visible);
        assert_eq!(l0.rows[0].row, ReviewRow::Note(0));
        assert!(!l0.rows[0].continued);
        m.page = 1;
        let l1 = e.review_layout(page, &m);
        assert_eq!(l1.index, 1);
        // The note's row is repeated, then its lines go on.
        assert_eq!(l1.rows[0].row, ReviewRow::Note(0));
        assert!(l1.rows[0].continued);
        let ReviewRow::Line(0, ln) = l1.rows[1].row else {
            panic!("{:?}", l1.rows[1]);
        };
        assert_eq!(
            ln,
            l0.rows.len() - 1,
            "the next line follows the last one shown"
        );
        // A page past the end shows the last page.
        m.page = 99;
        let le = e.review_layout(page, &m);
        assert_eq!(le.index, le.pages - 1);
        assert!(!le.rows.is_empty());
        // One page: no arrows and more room.
        let small = plan(&[("a.md", 2)], "x");
        let ss = Selection::defaults(&small);
        let ls = e.review_layout(page, &model(&small, &ss, Theme::DAY));
        assert_eq!((ls.pages, ls.rows.len()), (1, 3));
        assert_eq!(ls.prev.w, 0);
        assert_eq!(ls.next.w, 0);
    }

    #[test]
    fn hits_map_to_ticks_pages_and_buttons() {
        let p = plan(&[("a.md", 30), ("b.md", 4)], "x");
        let mut sel = Selection::defaults(&p);
        let (e, page) = screen(1440, 1920, 100);
        let mut m = model(&p, &sel, Theme::DAY);
        let l = e.review_layout(page, &m);
        assert!(l.pages > 1);
        // Rows: the note row, then its lines (the first match on the line).
        let (x, y) = centre(l.rows[0].rect);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::NoteTick(0));
        for (k, b) in l.rows.iter().enumerate().skip(1) {
            let (x, y) = (b.rect.x + 5, b.rect.y + b.rect.h / 2);
            assert_eq!(e.review_hit(&l, x, y), ReviewHit::MatchTick(0, k - 1));
        }
        // The buttons.
        let (x, y) = centre(l.cancel);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::Cancel);
        let (x, y) = centre(l.apply);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::Apply);
        // First page: no previous page, a next one.
        let (x, y) = centre(l.prev);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::None);
        let (x, y) = centre(l.next);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::PageNext);
        // The last page has the opposite.
        m.page = l.pages - 1;
        let ll = e.review_layout(page, &m);
        let (x, y) = centre(ll.prev);
        assert_eq!(e.review_hit(&ll, x, y), ReviewHit::PagePrev);
        let (x, y) = centre(ll.next);
        assert_eq!(e.review_hit(&ll, x, y), ReviewHit::None);
        // The header and outside the page answer nothing.
        let (x, y) = centre(l.header);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::None);
        assert_eq!(e.review_hit(&l, page.x + 5, page.y - 5), ReviewHit::None);
        // Apply answers nothing while nothing is ticked.
        for (i, n) in p.notes.iter().enumerate() {
            for k in 0..n.matches.len() {
                sel.set(i, k, false);
            }
        }
        let m0 = model(&p, &sel, Theme::DAY);
        let l0 = e.review_layout(page, &m0);
        assert!(!l0.apply_on);
        let (x, y) = centre(l0.apply);
        assert_eq!(e.review_hit(&l0, x, y), ReviewHit::None);
        let (x, y) = centre(l0.cancel);
        assert_eq!(e.review_hit(&l0, x, y), ReviewHit::Cancel);
        // A continued note row still ticks its note.
        let sel2 = Selection::defaults(&p);
        let mut m2 = model(&p, &sel2, Theme::DAY);
        m2.page = 1;
        let l2 = e.review_layout(page, &m2);
        assert!(l2.rows[0].continued);
        let (x, y) = centre(l2.rows[0].rect);
        assert_eq!(e.review_hit(&l2, x, y), ReviewHit::NoteTick(0));
    }

    /// A line tick never turns on a wiki-link match (S24-D6), unless the
    /// line has nothing else; the box shows the matches it toggles.
    #[test]
    fn a_line_tick_leaves_link_matches_alone() {
        let lm = |index, in_link| LineMatch {
            index,
            from: 0,
            to: 1,
            in_link,
        };
        let m = |in_link| Match {
            from: 0,
            to: 1,
            in_link,
        };
        let np = NotePlan {
            path: NotePath::parse("m.md").unwrap(),
            matches: vec![m(false), m(true), m(true), m(false)],
            lines: vec![
                PreviewLine {
                    line: 1,
                    text: "n [[n]]".into(),
                    ranges: vec![lm(0, false), lm(1, true)],
                },
                PreviewLine {
                    line: 2,
                    text: "[[n]]".into(),
                    ranges: vec![lm(2, true)],
                },
                PreviewLine {
                    line: 3,
                    text: "n".into(),
                    ranges: vec![lm(3, false)],
                },
            ],
            open: false,
        };
        assert_eq!(review_line_matches(&np, 0), vec![0], "plain only");
        assert_eq!(review_line_matches(&np, 2), vec![2], "a link-only line");
        assert_eq!(review_line_matches(&np, 3), vec![3]);
        let p = Plan {
            root: PathBuf::from("/tmp/none"),
            query: "n".into(),
            case: true,
            with: "x".into(),
            notes: vec![np],
            skipped: 0,
            scanned: 1,
            elapsed_ms: 0,
        };
        // The defaults tick the plain matches; a link match is ticked only by
        // its own line, and drawing the mixed states does not panic.
        let mut sel = Selection::defaults(&p);
        assert!(sel.is_ticked(0, 0) && !sel.is_ticked(0, 1));
        let (e, page) = screen(1440, 1920, 100);
        let l = e.review_layout(page, &model(&p, &sel, Theme::DAY));
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_review(&mut c, &l, &model(&p, &sel, Theme::DAY));
        sel.set(0, 0, false);
        e.draw_review(&mut c, &l, &model(&p, &sel, Theme::DAY));
    }

    #[test]
    fn a_line_with_several_matches_ticks_them_together() {
        let text = "aa needle bb needle cc".to_owned();
        let np = NotePlan {
            path: NotePath::parse("m.md").unwrap(),
            matches: vec![
                Match {
                    from: 3,
                    to: 9,
                    in_link: false,
                },
                Match {
                    from: 13,
                    to: 19,
                    in_link: false,
                },
                Match {
                    from: 40,
                    to: 46,
                    in_link: false,
                },
            ],
            lines: vec![
                PreviewLine {
                    line: 1,
                    text,
                    ranges: vec![
                        LineMatch {
                            index: 0,
                            from: 3,
                            to: 9,
                            in_link: false,
                        },
                        LineMatch {
                            index: 1,
                            from: 13,
                            to: 19,
                            in_link: false,
                        },
                    ],
                },
                PreviewLine {
                    line: 4,
                    text: "needle".into(),
                    ranges: vec![LineMatch {
                        index: 2,
                        from: 0,
                        to: 6,
                        in_link: false,
                    }],
                },
            ],
            open: false,
        };
        assert_eq!(review_line_matches(&np, 0), vec![0, 1]);
        assert_eq!(review_line_matches(&np, 2), vec![2]);
        assert!(review_line_matches(&np, 1).is_empty(), "1 starts no line");
        let p = Plan {
            root: PathBuf::from("/tmp/none"),
            query: "needle".into(),
            case: true,
            with: "pin".into(),
            notes: vec![np],
            skipped: 0,
            scanned: 1,
            elapsed_ms: 0,
        };
        let (e, page) = screen(1440, 1920, 100);
        let sel = Selection::defaults(&p);
        let l = e.review_layout(page, &model(&p, &sel, Theme::DAY));
        assert_eq!(l.rows[1].first, 0);
        assert_eq!(l.rows[2].first, 2);
        let (x, y) = centre(l.rows[1].rect);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::MatchTick(0, 0));
        // Mixed state draws without panicking.
        let mut half = Selection::defaults(&p);
        half.set(0, 1, false);
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_review(&mut c, &l, &model(&p, &half, Theme::DAY));
    }

    #[test]
    fn spans_show_old_struck_new_bold_and_unticked_plain() {
        let line = PreviewLine {
            line: 3,
            text: "\t  a needle b needle c".into(),
            ranges: vec![
                LineMatch {
                    index: 0,
                    from: 5,
                    to: 11,
                    in_link: false,
                },
                LineMatch {
                    index: 1,
                    from: 14,
                    to: 20,
                    in_link: true,
                },
            ],
        };
        let sp = review_spans(&line, "pin", |k| k == 0);
        let got: Vec<(&str, SpanStyle)> = sp.iter().map(|s| (s.text.as_str(), s.style)).collect();
        assert_eq!(
            got,
            vec![
                ("a ", SpanStyle::Plain),
                ("needle", SpanStyle::Struck),
                ("pin", SpanStyle::New),
                (" b ", SpanStyle::Plain),
                ("needle", SpanStyle::Unticked),
                (" c", SpanStyle::Plain),
            ]
        );
        // An empty replacement only strikes.
        let del = review_spans(&line, "", |_| true);
        assert!(del.iter().all(|s| s.style != SpanStyle::New));
        assert_eq!(
            del.iter().filter(|s| s.style == SpanStyle::Struck).count(),
            2
        );
        // Bad ranges (inside a character, backwards, past the end) are skipped.
        let bad = PreviewLine {
            line: 1,
            text: "éé needle".into(),
            ranges: vec![
                LineMatch {
                    index: 0,
                    from: 1,
                    to: 3,
                    in_link: false,
                },
                LineMatch {
                    index: 1,
                    from: 9,
                    to: 4,
                    in_link: false,
                },
                LineMatch {
                    index: 2,
                    from: 5,
                    to: 99,
                    in_link: false,
                },
            ],
        };
        let sp = review_spans(&bad, "x", |_| true);
        let all: String = sp.iter().map(|s| s.text.as_str()).collect();
        assert!(all.contains("éé "), "{sp:?}");
        // Line breaks in the replacement are blanks.
        let nl = review_spans(&line, "a\nb", |_| true);
        assert!(nl
            .iter()
            .any(|s| s.style == SpanStyle::New && s.text == "a b"));
    }

    #[test]
    fn drawing_strikes_the_old_text_and_marks_links() {
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let p = plan(&[("one.md", 2)], "pin");
        let sel = Selection::defaults(&p);
        let (e, page) = screen(1440, 1920, 100);
        let m = model(&p, &sel, Theme::DAY);
        let l = e.review_layout(page, &m);
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_review(&mut c, &l, &m);
        // Line 0 is ticked: a one-pixel-class strike crosses its row; line 1
        // (a link match) is unticked and has none but is underlined. Count
        // rows of full ink along the row's text area to tell them apart.
        let ink_run = |r: Rect, y: i32| -> i32 {
            let mut best = 0;
            let mut run = 0;
            for x in r.x..r.x + r.w {
                if px(&c, x, y) == Theme::DAY.ink {
                    run += 1;
                    best = best.max(run);
                } else {
                    run = 0;
                }
            }
            best
        };
        let ui = e.ui_px();
        let rows: Vec<&ReviewRowBox> = l
            .rows
            .iter()
            .filter(|b| matches!(b.row, ReviewRow::Line(..)))
            .collect();
        let base = |b: &ReviewRowBox| b.rect.y as f32 + b.rect.h as f32 / 2.0 + ui * 0.35;
        let strike_y = |b: &ReviewRowBox| (base(b) - ui * 0.28) as i32;
        let under_y = |b: &ReviewRowBox| (base(b) + ui * 0.12) as i32;
        let word_w = e.fonts.width(Face::Atkinson, ui, "needle") as i32;
        assert!(
            ink_run(rows[0].rect, strike_y(rows[0])) >= word_w - 2,
            "ticked line struck"
        );
        assert!(
            ink_run(rows[0].rect, under_y(rows[0])) < word_w / 2,
            "ticked line not underlined"
        );
        assert!(
            ink_run(rows[1].rect, under_y(rows[1])) >= word_w - 2,
            "unticked link match underlined"
        );
        assert!(
            ink_run(rows[1].rect, strike_y(rows[1])) < word_w / 2,
            "unticked line not struck"
        );
        // The link marker: an outlined box at the right edge of line 1 only.
        let right = rows[1].rect.x + rows[1].rect.w - e.scale().px(20.0);
        let probe = |b: &ReviewRowBox| {
            (right - 4..right)
                .any(|x| (b.rect.y..b.rect.y + b.rect.h).any(|y| px(&c, x, y) == Theme::DAY.ink))
        };
        assert!(probe(rows[1]), "link marker drawn");
        assert!(!probe(rows[0]), "no link marker on a plain match");
        // Only theme tokens are used: ink, paper, rule, wash, mute and the
        // anti-aliased text between them; the tick and strike are exact ink.
        assert_eq!(
            px(&c, l.rows[0].rect.x + 1, l.rows[0].rect.y + 3),
            Theme::DAY.wash
        );
    }

    #[test]
    fn trimming_keeps_the_match_visible_on_char_boundaries() {
        let (e, page) = screen(1440, 1920, 200);
        let max = (page.w - e.scale().px(120.0)) as f32;
        let ui = e.ui_px();
        let before = "éàü ".repeat(60);
        let after = " ñçø".repeat(60);
        let text = format!("{before}needle{after}");
        let from = before.len();
        let line = PreviewLine {
            line: 9,
            text,
            ranges: vec![LineMatch {
                index: 0,
                from,
                to: from + 6,
                in_link: false,
            }],
        };
        let spans = review_spans(&line, "pin", |_| true);
        let t = e.review_trim(&spans, max);
        let texts: Vec<&str> = t.iter().map(|s| s.text.as_str()).collect();
        assert!(
            texts.contains(&"needle") && texts.contains(&"pin"),
            "{texts:?}"
        );
        assert_eq!(t[0].style, SpanStyle::Plain);
        assert!(
            t[0].text.starts_with('\u{2026}') && t[0].text.ends_with("éàü "),
            "{t:?}"
        );
        let last = t.last().unwrap();
        assert!(
            last.text.ends_with('\u{2026}') && last.text.starts_with(" ñçø"),
            "{t:?}"
        );
        let w: f32 = t.iter().map(|s| e.span_w(s)).sum();
        assert!(w <= max, "{w} > {max}");
        // A line that fits is returned as it is.
        let short = PreviewLine {
            line: 1,
            text: "say needle there".into(),
            ranges: vec![LineMatch {
                index: 0,
                from: 4,
                to: 10,
                in_link: false,
            }],
        };
        let sp = review_spans(&short, "pin", |_| true);
        assert_eq!(e.review_trim(&sp, max), sp);
        // A match wider than the row is cut at its end, the ellipsis last.
        let wide = PreviewLine {
            line: 1,
            text: "x".repeat(400),
            ranges: vec![LineMatch {
                index: 0,
                from: 0,
                to: 400,
                in_link: false,
            }],
        };
        let sp = review_spans(&wide, "pin", |_| true);
        let t = e.review_trim(&sp, max);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].style, SpanStyle::Struck);
        assert!(t[0].text.ends_with('\u{2026}'));
        assert!(e.span_w(&t[0]) <= max);
        // Several matches: the stretch between the first and last stays whole.
        let multi = PreviewLine {
            line: 1,
            text: format!("{}a needle b needle c{}", "w ".repeat(80), " z".repeat(80)),
            ranges: {
                let a = 160 + 2;
                let b = a + 6 + 3;
                vec![
                    LineMatch {
                        index: 0,
                        from: a,
                        to: a + 6,
                        in_link: false,
                    },
                    LineMatch {
                        index: 1,
                        from: b,
                        to: b + 6,
                        in_link: false,
                    },
                ]
            },
        };
        let sp = review_spans(&multi, "pin", |_| true);
        let t = e.review_trim(&sp, max);
        assert_eq!(t.iter().filter(|s| s.style == SpanStyle::Struck).count(), 2);
        let w: f32 = t.iter().map(|s| e.span_w(s)).sum();
        assert!(w <= max, "{w} > {max} at {ui}");
        // Under a tiny width nothing panics and the result fits.
        for tiny in [1.0, 10.0, 40.0, 90.0] {
            let t = e.review_trim(&sp, tiny);
            let w: f32 = t.iter().map(|s| e.span_w(s)).sum();
            assert!(w <= tiny + 0.01 || tiny < 10.0, "{w} > {tiny}");
        }
    }

    #[test]
    fn header_and_labels_in_english() {
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let (e, _) = screen(1440, 1920, 100);
        let p = plan(&[("a.md", 4), ("b.md", 3)], "pin");
        assert_eq!(
            e.review_header(&p, 5000.0),
            "Replace \u{201c}needle\u{201d} with \u{201c}pin\u{201d}: 7 matches in 2 notes"
        );
        let del = plan(&[("a.md", 4)], "");
        assert_eq!(
            e.review_header(&del, 5000.0),
            "Delete \u{201c}needle\u{201d}: 4 matches in 1 note"
        );
        let one = plan(&[("a.md", 1)], "x");
        assert_eq!(
            e.review_header(&one, 5000.0),
            "Replace \u{201c}needle\u{201d} with \u{201c}x\u{201d}: 1 match in 1 note"
        );
        // A long query and replacement are shortened so the counts stay.
        let mut long = plan(&[("a.md", 4), ("b.md", 3)], &"w".repeat(80));
        long.query = "q".repeat(80);
        let max = (e
            .fonts
            .width(Face::AtkinsonBold, e.ui_px(), &e.review_header(&p, 5000.0))
            * 1.2)
            .ceil();
        let h = e.review_header(&long, max);
        assert!(h.ends_with(": 7 matches in 2 notes"), "{h}");
        assert!(h.contains('\u{2026}'), "{h}");
        assert!(e.fonts.width(Face::AtkinsonBold, e.ui_px(), &h) <= max);
        // A query that looks like a placeholder is not substituted twice.
        let mut tricky = plan(&[("a.md", 1)], "{q}");
        tricky.query = "{w}".into();
        assert_eq!(
            e.review_header(&tricky, 5000.0),
            "Replace \u{201c}{w}\u{201d} with \u{201c}{q}\u{201d}: 1 match in 1 note"
        );
        // Apply label counts notes with a ticked match.
        let mut sel = Selection::defaults(&p);
        let m = model(&p, &sel, Theme::DAY);
        assert_eq!(EditorScreen::review_apply_label(&m), "Replace in 2 notes");
        sel.set_note(&p, 1, false);
        let m = model(&p, &sel, Theme::DAY);
        assert_eq!(EditorScreen::review_apply_label(&m), "Replace in 1 note");
        let none = Selection::none(&p);
        let m = model(&p, &none, Theme::DAY);
        assert_eq!(EditorScreen::review_apply_label(&m), "Replace in 0 notes");
        // No matches at all.
        let empty = plan(&[], "x");
        assert!(e.review_header(&empty, 5000.0).ends_with(": No matches"));
    }

    #[test]
    fn progress_replaces_the_footer_and_stops_taps() {
        crate::i18n::set_lang(crate::i18n::Lang::En);
        let p = plan(&[("a.md", 3), ("b.md", 2)], "pin");
        let sel = Selection::defaults(&p);
        let (e, page) = screen(1440, 1920, 100);
        let mut m = model(&p, &sel, Theme::DAY);
        m.progress = Some((1, 2));
        let l = e.review_layout(page, &m);
        assert!(l.busy);
        assert_eq!((l.cancel.w, l.apply.w, l.prev.w, l.next.w), (0, 0, 0, 0));
        assert!(inside(page, l.status) && l.status.w > page.w / 2);
        for b in &l.rows {
            let (x, y) = centre(b.rect);
            assert_eq!(e.review_hit(&l, x, y), ReviewHit::None);
        }
        let (x, y) = centre(l.status);
        assert_eq!(e.review_hit(&l, x, y), ReviewHit::None);
        // The bar is dark for the share done.
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_review(&mut c, &l, &m);
        let s = e.scale();
        let bar_y = l.foot.y + l.foot.h - s.px(8.0) - s.px(4.0).max(2) + 1;
        assert_eq!(px(&c, l.status.x + 2, bar_y), Theme::DAY.ink);
        assert_eq!(px(&c, l.status.x + l.status.w - 3, bar_y), Theme::DAY.rule);
        assert_eq!(
            px(&c, l.status.x + l.status.w / 2 - 3, bar_y),
            Theme::DAY.ink
        );
        assert_eq!(
            px(&c, l.status.x + l.status.w / 2 + 3, bar_y),
            Theme::DAY.rule
        );
        // 0 of 0 and a finished run draw too.
        for pr in [(0, 0), (2, 2), (5, 2)] {
            m.progress = Some(pr);
            let l = e.review_layout(page, &m);
            e.draw_review(&mut c, &l, &m);
        }
    }

    #[test]
    fn buttons_and_ticks_use_the_theme_in_both_themes() {
        let p = plan(&[("a.md", 3)], "pin");
        let (e, page) = screen(1440, 1920, 100);
        for theme in [Theme::DAY, Theme::NIGHT] {
            let sel = Selection::defaults(&p);
            let m = model(&p, &sel, theme);
            let l = e.review_layout(page, &m);
            let mut c = Canvas::new(1440, 1920, 128);
            e.draw_review(&mut c, &l, &m);
            // The enabled Apply is ink, Cancel is a wash, the note row a wash.
            assert_eq!(px(&c, l.apply.x + 2, l.apply.y + 2), theme.ink);
            assert_eq!(px(&c, l.cancel.x + 2, l.cancel.y + 2), theme.wash);
            assert_eq!(
                px(&c, l.rows[0].rect.x + 1, l.rows[0].rect.y + 2),
                theme.wash
            );
            assert_eq!(
                px(&c, l.rows[1].rect.x + 1, l.rows[1].rect.y + 2),
                theme.paper
            );
            // With nothing ticked Apply is a wash too.
            let none = Selection::none(&p);
            let m0 = model(&p, &none, theme);
            let l0 = e.review_layout(page, &m0);
            let mut c0 = Canvas::new(1440, 1920, 128);
            e.draw_review(&mut c0, &l0, &m0);
            assert_eq!(px(&c0, l0.apply.x + 2, l0.apply.y + 2), theme.wash);
            // A ticked box is solid ink, an unticked one an ink outline.
            let side = e.scale().px(20.0);
            let bx = e.tick_box(l.rows[0].rect, l.rows[0].rect.x + side);
            assert_eq!(px(&c, bx.x + 1, bx.y + 1), theme.ink);
            assert_eq!(px(&c0, bx.x + 1, bx.y + 1), theme.ink);
            assert_eq!(px(&c0, bx.x + bx.w / 2, bx.y + bx.h / 2), theme.wash);
        }
    }

    #[test]
    fn long_note_path_is_cut_at_the_start() {
        let long = format!("{}/último día.md", ["carpeta muy larga"; 10].join("/"));
        let p = plan(&[(&long, 2)], "pin");
        let sel = Selection::defaults(&p);
        let (e, page) = screen(1440, 1920, 200);
        let m = model(&p, &sel, Theme::DAY);
        let l = e.review_layout(page, &m);
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_review(&mut c, &l, &m);
        // The count stays at the right edge, clear of the name.
        let r = l.rows[0].rect;
        let side = e.scale().px(20.0);
        assert_ne!(
            px(&c, r.x + r.w - side - 2, r.y + r.h / 2),
            255,
            "count drawn"
        );
    }
}
