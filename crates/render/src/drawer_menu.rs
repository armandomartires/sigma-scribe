//! The drawer button and its menu (v1.16). One title-bar icon, the panel
//! on the drawer's side, stands for the drawer; a tap opens a small menu
//! under it with a checkbox for each pane on offer (Contents, To-dos,
//! Scratch, top to bottom as in the drawer). A tap on a row shows or hides
//! that pane and the menu stays open; a tap anywhere else closes it.

use crate::canvas::{Canvas, Rect};
use crate::drawer::{draw_check, Pane};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::icons::{draw_icon, Icon};
use crate::theme::Theme;
use sigma_core::settings::Side;

/// What a tap does while the drawer menu is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuHit {
    /// A row: show or hide this pane.
    Pane(Pane),
    /// The drawer button: close the menu.
    Button,
    /// Anywhere else: close the menu (the tap goes no further).
    Outside,
}

impl Pane {
    /// The pane's title-bar and menu icon.
    #[must_use]
    pub fn icon(self) -> Icon {
        match self {
            Self::Contents => Icon::Contents,
            Self::Todos => Icon::Todos,
            Self::Scratch => Icon::Scratch,
        }
    }
}

impl EditorScreen {
    /// At least one pane is on offer, so the drawer button is shown.
    #[must_use]
    pub fn panes_on(&self) -> bool {
        self.contents_on || self.todos_on || self.scratch_on
    }

    /// The panes on offer, top to bottom.
    #[must_use]
    pub fn menu_panes(&self) -> Vec<Pane> {
        [
            (Pane::Contents, self.contents_on),
            (Pane::Todos, self.todos_on),
            (Pane::Scratch, self.scratch_on),
        ]
        .into_iter()
        .filter(|e| e.1)
        .map(|e| e.0)
        .collect()
    }

    /// The menu's frame and its rows, right-aligned under the drawer
    /// button.
    #[must_use]
    pub fn drawer_menu_rows(&self, l: &Layout) -> (Rect, Vec<(Pane, Rect)>) {
        let panes = self.menu_panes();
        let (row_h, pad, edge) = (self.px(56.0), self.px(16.0), self.px(4.0));
        let ui = self.ui_px();
        let label = panes
            .iter()
            .map(|p| self.fonts.width(Face::Atkinson, ui, &tr(p.title())) as i32)
            .max()
            .unwrap_or(0);
        let w = (pad + self.px(22.0) + self.px(12.0) + self.px(44.0) + label + 2 * pad)
            .max(self.px(220.0));
        let b = l.drawer_button;
        let x = (b.x + b.w - w).max(self.px(8.0));
        let frame = Rect::new(x, l.title.h, w, panes.len() as i32 * row_h + 2 * edge);
        let rows = panes
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                let y = frame.y + edge + i as i32 * row_h;
                (p, Rect::new(frame.x, y, frame.w, row_h))
            })
            .collect();
        (frame, rows)
    }

    /// What a tap at (`x`, `y`) does while the menu is open.
    #[must_use]
    pub fn drawer_menu_hit(&self, l: &Layout, x: i32, y: i32) -> MenuHit {
        if l.drawer_button.inset(-self.px(4.0)).contains(x, y) {
            return MenuHit::Button;
        }
        let (_, rows) = self.drawer_menu_rows(l);
        rows.into_iter()
            .find(|(_, r)| r.contains(x, y))
            .map_or(MenuHit::Outside, |(p, _)| MenuHit::Pane(p))
    }

    /// The drawer button: the panel on the drawer's side, active while a
    /// pane is open or the menu is.
    pub fn draw_drawer_button(&self, c: &mut Canvas, l: &Layout, t: Theme) {
        if !self.panes_on() {
            return;
        }
        let r = l.drawer_button;
        if self.drawer_menu || self.drawer_open.iter().any(|&o| o) {
            c.fill(r, t.wash);
            c.stroke(r, 2, t.ink);
        }
        let icon = match self.cfg.drawer {
            Side::Left => Icon::PanelLeft,
            Side::Right => Icon::PanelRight,
        };
        draw_icon(c, icon, r, t.ink);
    }

    /// Draws the open menu over whatever is below it: a framed list, each
    /// row a checkbox (ticked while the pane is open), the pane's icon and
    /// its name.
    pub fn draw_drawer_menu(&self, c: &mut Canvas, l: &Layout, t: Theme) {
        if !self.drawer_menu {
            return;
        }
        let (frame, rows) = self.drawer_menu_rows(l);
        let line = self.px(1.5).max(2);
        c.fill(frame, t.paper);
        c.stroke(frame, line, t.ink);
        let ui = self.ui_px();
        let pad = self.px(16.0);
        let side = self.px(22.0);
        for (i, (p, r)) in rows.iter().enumerate() {
            if i > 0 {
                c.fill(
                    Rect::new(r.x + pad, r.y, r.w - 2 * pad, 1.max(self.px(1.0))),
                    t.rule,
                );
            }
            let b = Rect::new(r.x + pad, r.y + (r.h - side) / 2, side, side);
            draw_check(c, b, self.drawer_open[p.index()], line, t);
            let icon = Rect::new(
                b.x + side + self.px(12.0),
                r.y + (r.h - self.px(44.0)) / 2,
                self.px(44.0),
                self.px(44.0),
            );
            draw_icon(c, p.icon(), icon, t.ink);
            let base = r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
            let x = (icon.x + icon.w + self.px(8.0)) as f32;
            self.fonts
                .draw(c, Face::Atkinson, ui, x, base, &tr(p.title()), t.ink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_core::Keyboard;

    fn get(c: &Canvas, x: i32, y: i32) -> u8 {
        c.pixels[(y * c.width as i32 + x) as usize]
    }

    #[test]
    fn rows_are_the_panes_on_offer_under_the_button_and_every_place_answers() {
        let k = Keyboard::default();
        for (w, h) in [(1440, 1920), (1920, 1440)] {
            let mut e = EditorScreen::new(w, h);
            let l = e.layout(true, &k);
            let (frame, rows) = e.drawer_menu_rows(&l);
            let order: Vec<Pane> = rows.iter().map(|r| r.0).collect();
            assert_eq!(order, [Pane::Contents, Pane::Todos, Pane::Scratch]);
            assert_eq!(frame.y, l.title.h, "{w}: just under the title bar");
            assert_eq!(
                frame.x + frame.w,
                l.drawer_button.x + l.drawer_button.w,
                "{w}: right-aligned with the button"
            );
            assert!(frame.x >= 0 && frame.x + frame.w <= w as i32);
            for (p, r) in &rows {
                assert!(r.h >= e.px(44.0), "a full touch target");
                let (x, y) = (r.x + r.w / 2, r.y + r.h / 2);
                assert_eq!(e.drawer_menu_hit(&l, x, y), MenuHit::Pane(*p));
            }
            let b = l.drawer_button;
            assert_eq!(
                e.drawer_menu_hit(&l, b.x + b.w / 2, b.y + b.h / 2),
                MenuHit::Button
            );
            assert_eq!(e.drawer_menu_hit(&l, 10, h as i32 - 10), MenuHit::Outside);
            assert_eq!(
                e.drawer_menu_hit(&l, frame.x + frame.w / 2, frame.y + frame.h + 4),
                MenuHit::Outside,
                "just below the menu"
            );
            // A pane with no plugin is not offered.
            e.todos_on = false;
            let (_, rows) = e.drawer_menu_rows(&e.layout(true, &k));
            assert_eq!(
                rows.iter().map(|r| r.0).collect::<Vec<_>>(),
                [Pane::Contents, Pane::Scratch]
            );
        }
    }

    #[test]
    fn with_no_pane_on_offer_the_button_goes_and_its_place_closes_up() {
        let k = Keyboard::default();
        let mut e = EditorScreen::new(1440, 1920);
        let with = e.layout(true, &k);
        (e.contents_on, e.todos_on, e.scratch_on) = (false, false, false);
        let l = e.layout(true, &k);
        assert!(!e.panes_on());
        assert_eq!(l.drawer_button, Rect::default());
        assert!(l.find_button.x > with.find_button.x, "the rest close up");
        assert_eq!(l.options_button, with.options_button);
        let b = with.drawer_button;
        assert_ne!(
            e.preview_hit(&k, b.x + b.w / 2, b.y + b.h / 2),
            crate::Hit::DrawerMenu
        );
    }

    #[test]
    fn the_menu_ticks_open_panes_and_the_button_follows_the_drawer_side() {
        let k = Keyboard::default();
        let mut e = EditorScreen::new(1440, 1920);
        let t = Theme::DAY;
        let l = e.layout(true, &k);
        let draw = |e: &EditorScreen| {
            let mut c = Canvas::new(1440, 1920, t.paper);
            e.draw_drawer_button(&mut c, &l, t);
            e.draw_drawer_menu(&mut c, &l, t);
            c
        };
        let closed = draw(&e);
        let (frame, rows) = e.drawer_menu_rows(&l);
        assert_eq!(
            get(&closed, frame.x + 1, frame.y + frame.h / 2),
            t.paper,
            "nothing while closed"
        );
        e.drawer_menu = true;
        let open = draw(&e);
        assert_eq!(get(&open, frame.x, frame.y + frame.h / 2), t.ink, "framed");
        // The checkbox of a row: an outline while the pane is closed, solid
        // ink while it is open.
        let r = rows[1].1;
        let side = e.px(22.0);
        let centre = (r.x + e.px(16.0) + side / 4, r.y + r.h / 2 - side / 4);
        assert_eq!(get(&open, centre.0, centre.1), t.paper);
        e.drawer_open[Pane::Todos.index()] = true;
        assert_eq!(get(&draw(&e), centre.0, centre.1), t.ink);
        // The icon is the panel on the drawer's side.
        let b = l.drawer_button;
        let right = draw(&e);
        e.cfg.drawer = Side::Left;
        let left = draw(&e);
        let differs = (b.y..b.y + b.h)
            .flat_map(|y| (b.x..b.x + b.w).map(move |x| (x, y)))
            .any(|(x, y)| get(&right, x, y) != get(&left, x, y));
        assert!(differs, "panel-left is not panel-right");
    }
}
