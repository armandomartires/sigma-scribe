//! The Find bar (brief §3 "Find and replace", S6): under the title bar, two
//! fields (Find, Replace) and a close mark, then a tool row with the count,
//! Prev, Next, Replace, All and Aa (match case).

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::theme::Theme;
use sigma_core::Buffer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindField {
    Find,
    Replace,
}

pub struct FindModel<'a> {
    pub query: &'a Buffer,
    pub replace: &'a Buffer,
    /// Field the keyboard types into (`None`: the page has the caret).
    pub focus: Option<FindField>,
    pub case: bool,
    /// `Find`, `None`, or `i of n`.
    pub label: &'a str,
    pub theme: Theme,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindLayout {
    pub find: Rect,
    pub replace: Rect,
    pub close: Rect,
    pub count: Rect,
    pub prev: Rect,
    pub next: Rect,
    pub replace_one: Rect,
    pub all: Rect,
    pub case: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindHit {
    Field(FindField),
    Close,
    Prev,
    Next,
    Replace,
    All,
    Case,
    /// Inside the bar but on nothing.
    Bar,
    Outside,
}

const TOOLS: [&str; 5] = ["Prev", "Next", "Replace", "All", "Aa"];

impl EditorScreen {
    /// Height of the Find bar: two 44 px rows with 8 px around them.
    #[must_use]
    pub fn find_bar_h(&self) -> i32 {
        let s = self.scale();
        s.px(44.0) * 2 + s.px(8.0) * 3
    }

    #[must_use]
    pub fn find_layout(&self, base: &Layout) -> FindLayout {
        let s = self.scale();
        let bar = base.find_bar;
        if bar.h == 0 {
            return FindLayout::default();
        }
        let (pad, gap, btn) = (s.px(20.0), s.px(8.0), s.px(44.0));
        let y1 = bar.y + s.px(8.0);
        let y2 = y1 + btn + s.px(8.0);
        let close = Rect::new(bar.w - s.px(12.0) - btn, y1, btn, btn);
        let fw = (close.x - gap - pad - gap) / 2;
        let find = Rect::new(pad, y1, fw, btn);
        let replace = Rect::new(pad + fw + gap, y1, fw, btn);
        let ui = self.ui_px();
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
            count: Rect::new(pad, y2, (prev.x - gap - pad).max(1), btn),
            prev,
            next: nxt,
            replace_one,
            all,
            case,
        }
    }

    #[must_use]
    pub fn find_hit(&self, base: &Layout, x: i32, y: i32) -> FindHit {
        if !base.find_bar.contains(x, y) {
            return FindHit::Outside;
        }
        let l = self.find_layout(base);
        let slop = -self.scale().px(3.0);
        let hits = [
            (l.close, FindHit::Close),
            (l.find, FindHit::Field(FindField::Find)),
            (l.replace, FindHit::Field(FindField::Replace)),
            (l.prev, FindHit::Prev),
            (l.next, FindHit::Next),
            (l.replace_one, FindHit::Replace),
            (l.all, FindHit::All),
            (l.case, FindHit::Case),
        ];
        hits.iter()
            .find(|(r, _)| r.inset(slop).contains(x, y))
            .map_or(FindHit::Bar, |(_, h)| *h)
    }

    /// Centre of a Find-bar control by name (emulator scripts): `FindField`,
    /// `ReplaceField`, `FindClose`, `Prev`, `Next`, `Replace`, `All`, `Aa`.
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
            "Prev" => l.prev,
            "Next" => l.next,
            "Replace" => l.replace_one,
            "All" => l.all,
            "Aa" | "Case" => l.case,
            _ => return None,
        };
        Some((r.x + r.w / 2, r.y + r.h / 2))
    }

    pub fn draw_find(&self, c: &mut Canvas, base: &Layout, m: &FindModel<'_>) {
        let bar = base.find_bar;
        if bar.h == 0 {
            return;
        }
        let t = m.theme;
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
            let focused = m.focus == Some(field);
            c.fill(r, t.wash);
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
                self.fonts
                    .draw(c, Face::Atkinson, ui, x0, by, &shown, t.ink);
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
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            l.count.x as f32,
            base_y(l.count, Face::Atkinson),
            &tr(m.label),
            t.mute,
        );
        for (r, label) in [
            (l.prev, TOOLS[0]),
            (l.next, TOOLS[1]),
            (l.replace_one, TOOLS[2]),
            (l.all, TOOLS[3]),
            (l.case, TOOLS[4]),
        ] {
            let active = label == "Aa" && m.case;
            let label: &str = &tr(label);
            c.fill(r, if active { t.ink } else { t.paper });
            c.stroke(r, 1, if active { t.ink } else { t.rule });
            let w = self.fonts.width(Face::AtkinsonBold, ui, label);
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                r.x as f32 + (r.w as f32 - w) / 2.0,
                base_y(r, Face::AtkinsonBold),
                label,
                if active { t.paper } else { t.ink },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::Keyboard;

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
        for (name, hit) in [
            ("FindField", FindHit::Field(FindField::Find)),
            ("ReplaceField", FindHit::Field(FindField::Replace)),
            ("FindClose", FindHit::Close),
            ("Prev", FindHit::Prev),
            ("Next", FindHit::Next),
            ("Replace", FindHit::Replace),
            ("All", FindHit::All),
            ("Aa", FindHit::Case),
        ] {
            let (x, y) = e.find_control(&l, name).unwrap();
            assert_eq!(e.find_hit(&l, x, y), hit, "{name}");
        }
        assert_eq!(e.find_hit(&l, 5, l.page.y + 50), FindHit::Outside);
        let fl = e.find_layout(&l);
        assert!(fl.find.w >= 300 && fl.prev.h >= 44, "{fl:?}");
        assert!(fl.count.x + fl.count.w <= fl.prev.x);
    }
}
