//! The Find bar (brief §3 "Find and replace", S6): under the title bar, two
//! fields (Find, Replace) and a close mark, then the scope control (This
//! note | All notes, S22), then a tool row with the count, Prev, Next,
//! Replace, All and Aa (match case).
//!
//! The scope has its own row (S22-T11): in the count rect's place the two
//! segments do not fit at 175–200 % portrait in French and Portuguese
//! (626 px needed against 387 px free at 200 % portrait in French).
//! In `Scope::All` the Replace field and the Replace and All keys are
//! greyed and do not hit (S22-D3), and the count rect shows nothing: the
//! results list carries the count.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::theme::Theme;
use sigma_core::Buffer;

/// One of the two text fields of the Find bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindField {
    /// The search text.
    Find,
    /// The replacement text.
    Replace,
}

/// Where the Find bar searches (S22-D1): the open note or the whole
/// library. Session state, not a setting (S22-D2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    /// The open note; the matches are selected on the page.
    #[default]
    Note,
    /// Every note in the library; a results list takes the page's place.
    All,
}

/// What the Find bar shows when it is drawn.
pub struct FindModel<'a> {
    /// Text of the Find field.
    pub query: &'a Buffer,
    /// Text of the Replace field.
    pub replace: &'a Buffer,
    /// Field the keyboard types into (`None`: the page has the caret).
    pub focus: Option<FindField>,
    /// Match case is on (the Aa key is drawn inverted).
    pub case: bool,
    /// `Find`, `None`, or `i of n`. Not drawn in [`Scope::All`].
    pub label: &'a str,
    /// Where the search runs; selects a segment of the scope control.
    pub scope: Scope,
    /// Colours to draw with.
    pub theme: Theme,
}

/// Where the Find bar's controls are, in screen pixels. All empty when the
/// bar is closed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindLayout {
    /// The Find field.
    pub find: Rect,
    /// The Replace field.
    pub replace: Rect,
    /// The close mark.
    pub close: Rect,
    /// The "This note" segment of the scope control.
    pub scope_note: Rect,
    /// The "All notes" segment of the scope control.
    pub scope_all: Rect,
    /// The match count label, left of the tool keys.
    pub count: Rect,
    /// The Prev key.
    pub prev: Rect,
    /// The Next key.
    pub next: Rect,
    /// The Replace key (replaces the current match).
    pub replace_one: Rect,
    /// The All key (replaces every match).
    pub all: Rect,
    /// The Aa (match case) key.
    pub case: Rect,
}

/// What a touch on the Find bar landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindHit {
    /// One of the two text fields.
    Field(FindField),
    /// The close mark.
    Close,
    /// A segment of the scope control.
    Scope(Scope),
    /// The Prev key.
    Prev,
    /// The Next key.
    Next,
    /// The Replace key.
    Replace,
    /// The All key.
    All,
    /// The Aa (match case) key.
    Case,
    /// Inside the bar but on nothing.
    Bar,
    /// Not on the Find bar.
    Outside,
}

const TOOLS: [&str; 5] = ["Prev", "Next", "Replace", "All", "Aa"];
const SCOPES: [(&str, Scope); 2] = [("This note", Scope::Note), ("All notes", Scope::All)];

impl EditorScreen {
    /// Height of the Find bar: three 44 px rows with 8 px around them. The
    /// same in both scopes: greyed controls keep their place.
    #[must_use]
    pub fn find_bar_h(&self) -> i32 {
        let s = self.scale();
        s.px(44.0) * 3 + s.px(8.0) * 4
    }

    /// Where the controls of the Find bar go, given the screen layout
    /// `base`; all empty when the bar is closed.
    #[must_use]
    pub fn find_layout(&self, base: &Layout) -> FindLayout {
        let s = self.scale();
        let bar = base.find_bar;
        if bar.h == 0 {
            return FindLayout::default();
        }
        let (pad, gap, btn) = (s.px(20.0), s.px(8.0), s.px(44.0));
        let y1 = bar.y + s.px(8.0);
        let y_scope = y1 + btn + s.px(8.0);
        let y2 = y_scope + btn + s.px(8.0);
        let close = Rect::new(bar.w - s.px(12.0) - btn, y1, btn, btn);
        let fw = (close.x - gap - pad - gap) / 2;
        let find = Rect::new(pad, y1, fw, btn);
        let replace = Rect::new(pad + fw + gap, y1, fw, btn);
        let ui = self.ui_px();
        // Scope control: two segments side by side, each as wide as its
        // label needs and at least 60 px, sharing a border.
        let seg_w = |label: &str| {
            (self.fonts.width(Face::AtkinsonBold, ui, &tr(label)) as i32 + 2 * s.px(14.0))
                .max(s.px(60.0))
        };
        let note_w = seg_w(SCOPES[0].0);
        let scope_note = Rect::new(pad, y_scope, note_w, btn);
        let scope_all = Rect::new(pad + note_w, y_scope, seg_w(SCOPES[1].0), btn);
        let widths: Vec<i32> = TOOLS
            .iter()
            .map(|t| {
                (self.fonts.width(Face::AtkinsonBold, ui, &tr(t)) as i32 + 2 * s.px(14.0))
                    .max(s.px(60.0))
            })
            .collect();
        let total: i32 = widths.iter().sum::<i32>() + gap * (widths.len() as i32 - 1);
        let mut x = bar.w - s.px(12.0) - total;
        let mut cells = widths.iter().map(|&w| {
            let r = Rect::new(x, y2, w, btn);
            x += w + gap;
            r
        });
        let mut next = || cells.next().unwrap_or_default();
        let (prev, nxt, replace_one, all, case) = (next(), next(), next(), next(), next());
        FindLayout {
            find,
            replace,
            close,
            scope_note,
            scope_all,
            count: Rect::new(pad, y2, (prev.x - gap - pad).max(1), btn),
            prev,
            next: nxt,
            replace_one,
            all,
            case,
        }
    }

    /// What the point (x, y) hits on the Find bar in `scope`. Each control
    /// takes a few pixels of slop around it. In [`Scope::All`] the Replace
    /// field and the Replace and All keys are greyed and hit nothing
    /// ([`FindHit::Bar`]).
    #[must_use]
    pub fn find_hit(&self, base: &Layout, scope: Scope, x: i32, y: i32) -> FindHit {
        if !base.find_bar.contains(x, y) {
            return FindHit::Outside;
        }
        let l = self.find_layout(base);
        let slop = -self.scale().px(3.0);
        let hits = [
            (l.close, FindHit::Close),
            (l.find, FindHit::Field(FindField::Find)),
            (l.replace, FindHit::Field(FindField::Replace)),
            (l.scope_note, FindHit::Scope(Scope::Note)),
            (l.scope_all, FindHit::Scope(Scope::All)),
            (l.prev, FindHit::Prev),
            (l.next, FindHit::Next),
            (l.replace_one, FindHit::Replace),
            (l.all, FindHit::All),
            (l.case, FindHit::Case),
        ];
        let hit = hits
            .iter()
            .find(|(r, _)| r.inset(slop).contains(x, y))
            .map_or(FindHit::Bar, |(_, h)| *h);
        if scope == Scope::All && Self::replaces(hit) {
            return FindHit::Bar;
        }
        hit
    }

    /// Whether a hit is one of the Replace controls, off in [`Scope::All`]
    /// (S22-D3).
    fn replaces(h: FindHit) -> bool {
        matches!(
            h,
            FindHit::Field(FindField::Replace) | FindHit::Replace | FindHit::All
        )
    }

    /// Centre of a Find-bar control by name (emulator scripts): `FindField`,
    /// `ReplaceField`, `FindClose`, `This note`, `All notes`, `Prev`,
    /// `Next`, `Replace`, `All`, `Aa`.
    #[must_use]
    pub fn find_control(&self, base: &Layout, name: &str) -> Option<(i32, i32)> {
        if base.find_bar.h == 0 {
            return None;
        }
        let l = self.find_layout(base);
        let r = match name {
            "FindField" => l.find,
            "ReplaceField" => l.replace,
            "FindClose" => l.close,
            "This note" => l.scope_note,
            "All notes" => l.scope_all,
            "Prev" => l.prev,
            "Next" => l.next,
            "Replace" => l.replace_one,
            "All" => l.all,
            "Aa" | "Case" => l.case,
            _ => return None,
        };
        Some((r.x + r.w / 2, r.y + r.h / 2))
    }

    /// Draws the Find bar: the two fields (with the caret in the focused
    /// one), the close mark, the scope control, the count label and the
    /// tool keys. Does nothing when the bar is closed.
    pub fn draw_find(&self, c: &mut Canvas, base: &Layout, m: &FindModel<'_>) {
        let bar = base.find_bar;
        if bar.h == 0 {
            return;
        }
        let t = m.theme;
        let all = m.scope == Scope::All;
        let s = self.scale();
        let ui = self.ui_px();
        let l = self.find_layout(base);
        c.fill(bar, t.paper);
        c.fill(Rect::new(0, bar.y + bar.h - 1, bar.w, 1), t.rule);
        let base_y =
            |r: Rect, face| r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, ui) / 2.6;
        for (r, buf, field, ph) in [
            (l.find, m.query, FindField::Find, "Find"),
            (l.replace, m.replace, FindField::Replace, "Replace"),
        ] {
            // The Replace field is greyed in All notes: paper fill, mute text.
            let off = all && field == FindField::Replace;
            let focused = m.focus == Some(field) && !off;
            c.fill(r, if off { t.paper } else { t.wash });
            c.stroke(r, 1, if focused { t.ink } else { t.rule });
            let x0 = (r.x + s.px(10.0)) as f32;
            let by = base_y(r, Face::Atkinson);
            let max = (r.w - s.px(20.0)) as f32;
            // Long text: show its end, where the caret usually is.
            let mut shown = buf.text().to_owned();
            while !shown.is_empty() && self.fonts.width(Face::Atkinson, ui, &shown) > max {
                shown.remove(0);
            }
            if buf.text().is_empty() {
                self.fonts
                    .draw(c, Face::Atkinson, ui, x0, by, &tr(ph), t.mute);
            } else {
                let ink = if off { t.mute } else { t.ink };
                self.fonts.draw(c, Face::Atkinson, ui, x0, by, &shown, ink);
            }
            if focused {
                let cut = shown
                    .len()
                    .saturating_sub(buf.text().len() - buf.caret().min(buf.text().len()));
                let before = self
                    .fonts
                    .width(Face::Atkinson, ui, shown.get(..cut).unwrap_or(""));
                c.fill(
                    Rect::new((x0 + before) as i32, r.y + s.px(8.0), 2, r.h - s.px(16.0)),
                    t.ink,
                );
            }
        }
        // Close mark.
        crate::icons::draw_icon(c, crate::icons::Icon::Close, l.close, t.ink);
        // Scope control: the selected segment is inverted, like Aa.
        for (r, (label, scope)) in [(l.scope_note, SCOPES[0]), (l.scope_all, SCOPES[1])] {
            self.draw_key(c, r, &tr(label), m.scope == scope, false, t);
        }
        if !all {
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui,
                l.count.x as f32,
                base_y(l.count, Face::Atkinson),
                &tr(m.label),
                t.mute,
            );
        }
        for (r, label) in [
            (l.prev, TOOLS[0]),
            (l.next, TOOLS[1]),
            (l.replace_one, TOOLS[2]),
            (l.all, TOOLS[3]),
            (l.case, TOOLS[4]),
        ] {
            let active = label == "Aa" && m.case;
            let off = all && (label == "Replace" || label == "All");
            self.draw_key(c, r, &tr(label), active, off, t);
        }
    }

    /// One key of the Find bar: inverted when `active`, mute when `off`.
    fn draw_key(&self, c: &mut Canvas, r: Rect, label: &str, active: bool, off: bool, t: Theme) {
        let ui = self.ui_px();
        c.fill(r, if active { t.ink } else { t.paper });
        c.stroke(r, 1, if active { t.ink } else { t.rule });
        let w = self.fonts.width(Face::AtkinsonBold, ui, label);
        let ink = match (active, off) {
            (true, _) => t.paper,
            (false, true) => t.mute,
            (false, false) => t.ink,
        };
        self.fonts.draw(
            c,
            Face::AtkinsonBold,
            ui,
            r.x as f32 + (r.w as f32 - w) / 2.0,
            r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6,
            label,
            ink,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{set_lang, Lang};
    use sigma_core::Keyboard;

    /// Every control, as the emulator names it, and what a tap on its
    /// centre hits in This note.
    const CONTROLS: [(&str, FindHit); 10] = [
        ("FindField", FindHit::Field(FindField::Find)),
        ("ReplaceField", FindHit::Field(FindField::Replace)),
        ("FindClose", FindHit::Close),
        ("This note", FindHit::Scope(Scope::Note)),
        ("All notes", FindHit::Scope(Scope::All)),
        ("Prev", FindHit::Prev),
        ("Next", FindHit::Next),
        ("Replace", FindHit::Replace),
        ("All", FindHit::All),
        ("Aa", FindHit::Case),
    ];

    /// The bar open at `ui` % on a panel `w` by `h`.
    fn open(w: u32, h: u32, ui: u16) -> (EditorScreen, Layout) {
        let mut e = EditorScreen::new(w, h);
        e.cfg.ui_scale = ui;
        e.resize(w, h);
        e.find_open = true;
        let l = e.layout(true, &Keyboard::default());
        (e, l)
    }

    #[test]
    fn bar_pushes_the_page_down_and_every_control_hits() {
        let mut e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let closed = e.layout(true, &k);
        assert_eq!(closed.find_bar.h, 0);
        e.find_open = true;
        let l = e.layout(true, &k);
        assert_eq!(l.page.y, closed.page.y + e.find_bar_h());
        assert_eq!(l.page.y + l.page.h, closed.page.y + closed.page.h);
        for (name, hit) in CONTROLS {
            let (x, y) = e.find_control(&l, name).unwrap();
            assert_eq!(e.find_hit(&l, Scope::Note, x, y), hit, "{name}");
        }
        assert_eq!(
            e.find_hit(&l, Scope::Note, 5, l.page.y + 50),
            FindHit::Outside
        );
        let fl = e.find_layout(&l);
        assert!(fl.find.w >= 300 && fl.prev.h >= 44, "{fl:?}");
        assert!(fl.count.x + fl.count.w <= fl.prev.x);
    }

    /// Both segments hit, keep the 44 px minimum and stay on the bar, at
    /// every UI scale, both orientations and in the languages with the
    /// longest labels. The scope row sits between the fields and the tools.
    #[test]
    fn scope_segments_hit_at_every_scale_orientation_and_language() {
        for lang in [Lang::En, Lang::Fr, Lang::Pt, Lang::PseudoLong] {
            set_lang(lang);
            for (w, h) in [(1440u32, 1920u32), (1920, 1440)] {
                for ui in sigma_core::settings::UI_SCALES {
                    let (e, l) = open(w, h, ui);
                    let fl = e.find_layout(&l);
                    let min = e.scale().px(44.0);
                    let tag = format!("{lang:?} {w}x{h} {ui}%");
                    for (r, scope) in [(fl.scope_note, Scope::Note), (fl.scope_all, Scope::All)] {
                        assert!(r.w >= min && r.h >= min, "{tag}: {r:?}");
                        assert!(r.x >= 0 && r.x + r.w <= w as i32, "{tag}: {r:?} off screen");
                        assert!(
                            r.y > fl.find.y + fl.find.h && r.y + r.h < fl.prev.y,
                            "{tag}"
                        );
                        let (x, y) = (r.x + r.w / 2, r.y + r.h / 2);
                        for s in [Scope::Note, Scope::All] {
                            assert_eq!(e.find_hit(&l, s, x, y), FindHit::Scope(scope), "{tag}");
                        }
                    }
                    assert_eq!(fl.scope_all.x, fl.scope_note.x + fl.scope_note.w, "{tag}");
                    // The tool row still fits beside the left padding.
                    assert!(fl.prev.x > fl.count.x, "{tag}: {fl:?}");
                }
            }
        }
        set_lang(Lang::En);
    }

    /// S22-D3: in All notes the Replace field and the Replace and All keys
    /// hit nothing; everything else still hits.
    #[test]
    fn replace_does_not_hit_in_all_notes() {
        for (w, h) in [(1440u32, 1920u32), (1920, 1440)] {
            let (e, l) = open(w, h, 200);
            for (name, hit) in CONTROLS {
                let (x, y) = e.find_control(&l, name).unwrap();
                let want = if matches!(name, "ReplaceField" | "Replace" | "All") {
                    FindHit::Bar
                } else {
                    hit
                };
                assert_eq!(e.find_hit(&l, Scope::All, x, y), want, "{name}");
                assert_eq!(e.find_hit(&l, Scope::Note, x, y), hit, "{name}");
            }
        }
    }

    /// Three rows in both scopes: greyed controls keep their place, so the
    /// page does not jump when the scope changes.
    #[test]
    fn layout_height_is_three_rows_in_each_scope() {
        for ui in sigma_core::settings::UI_SCALES {
            let (e, l) = open(1440, 1920, ui);
            let s = e.scale();
            let fl = e.find_layout(&l);
            assert_eq!(l.find_bar.h, e.find_bar_h());
            assert_eq!(e.find_bar_h(), 3 * s.px(44.0) + 4 * s.px(8.0), "{ui}%");
            assert_eq!(
                fl.prev.y + fl.prev.h + s.px(8.0),
                l.find_bar.y + l.find_bar.h
            );
            // Both scopes draw into the same bar.
            let mut c = Canvas::new(1440, 1920, 0xff);
            let q = sigma_core::Buffer::new("");
            for scope in [Scope::Note, Scope::All] {
                let m = FindModel {
                    query: &q,
                    replace: &q,
                    focus: Some(FindField::Find),
                    case: false,
                    label: "Find",
                    scope,
                    theme: Theme::DAY,
                };
                e.draw_find(&mut c, &l, &m);
            }
        }
    }

    /// The scope control is drawn: the selected segment is inverted (ink
    /// fill) and the other is not; in All notes the count rect stays paper.
    #[test]
    fn selected_segment_is_inverted_and_count_is_blank_in_all() {
        let (e, l) = open(1440, 1920, 200);
        let fl = e.find_layout(&l);
        let q = sigma_core::Buffer::new("");
        let px = |c: &Canvas, x: i32, y: i32| c.pixels[(y as u32 * c.width + x as u32) as usize];
        for scope in [Scope::Note, Scope::All] {
            let mut c = Canvas::new(1440, 1920, 0x80);
            let m = FindModel {
                query: &q,
                replace: &q,
                focus: None,
                case: false,
                label: "None",
                scope,
                theme: Theme::DAY,
            };
            e.draw_find(&mut c, &l, &m);
            let (on, off) = match scope {
                Scope::Note => (fl.scope_note, fl.scope_all),
                Scope::All => (fl.scope_all, fl.scope_note),
            };
            // The middle of a segment may be on a glyph; probe a corner inside the border.
            let corner = |r: Rect| (r.x + 3, r.y + 3);
            let (ox, oy) = corner(on);
            assert_eq!(px(&c, ox, oy), Theme::DAY.ink, "{scope:?} selected");
            let (fx, fy) = corner(off);
            assert_eq!(px(&c, fx, fy), Theme::DAY.paper, "{scope:?} other");
            let count_ink = (0..fl.count.w)
                .flat_map(|dx| (0..fl.count.h).map(move |dy| (dx, dy)))
                .filter(|&(dx, dy)| px(&c, fl.count.x + dx, fl.count.y + dy) != Theme::DAY.paper)
                .count();
            match scope {
                Scope::All => assert_eq!(count_ink, 0, "count rect blank in All"),
                Scope::Note => assert!(count_ink > 0, "label drawn in This note"),
            }
        }
    }
}
