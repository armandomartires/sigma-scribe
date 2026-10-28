//! The front-light button and its menu. One title-bar icon, a sun (dim
//! while the light is off), opens a small menu under it with two sliders,
//! Backlight and, when the light has a warm-to-cool mixer, Warmth. They are
//! the sliders of Settings → Display, with the same stops. A tap or a drag
//! on a slider sets the light at once and the menu stays open; a tap
//! anywhere else, or Escape, closes it.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::icons::{draw_icon, Icon};
use crate::slider::{self, Geom, Slider};
use crate::theme::Theme;
use sigma_core::settings::{BACKLIGHT_MAX, WARMTH_MAX};

/// A slider of the light menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightRow {
    /// The brightness, off to full.
    Brightness,
    /// The colour, cool to warm.
    Warmth,
}

impl LightRow {
    /// The row's name, translated by `tr`.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Brightness => "Backlight",
            Self::Warmth => "Warmth",
        }
    }

    /// The number of stops.
    #[must_use]
    pub fn stops(self) -> usize {
        match self {
            Self::Brightness => usize::from(BACKLIGHT_MAX) + 1,
            Self::Warmth => usize::from(WARMTH_MAX) + 1,
        }
    }

    /// The stop labels, as in Settings → Display.
    #[must_use]
    pub fn labels(self) -> Vec<String> {
        let (max, lo, hi) = match self {
            Self::Brightness => (BACKLIGHT_MAX, "Off", "Full"),
            Self::Warmth => (WARMTH_MAX, "Cool", "Warm"),
        };
        (0..=max)
            .map(|i| match i {
                0 => tr(lo).into_owned(),
                i if i == max => tr(hi).into_owned(),
                i => format!("{} %", usize::from(i) * 100 / usize::from(max)),
            })
            .collect()
    }
}

/// What a tap or touch does while the light menu is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightHit {
    /// On a slider: set this row to this step.
    Slider(LightRow, u8),
    /// The light button: close the menu.
    Button,
    /// In the menu but not on a slider: nothing.
    Frame,
    /// Anywhere else: close the menu (the tap goes no further).
    Outside,
}

impl EditorScreen {
    /// The sliders on offer, top to bottom: Warmth only with a mixer.
    #[must_use]
    pub fn light_rows(&self) -> Vec<LightRow> {
        let mut rows = vec![LightRow::Brightness];
        if self.warmth_level.is_some() {
            rows.push(LightRow::Warmth);
        }
        rows
    }

    /// The menu's frame and, per slider, its row and geometry; right-aligned
    /// under the light button and kept on the screen.
    #[must_use]
    pub fn light_menu_rows(&self, l: &Layout) -> (Rect, Vec<(LightRow, Rect, Geom)>) {
        let rows = self.light_rows();
        let (pad, edge) = (self.px(16.0), self.px(4.0));
        let (title_h, slider_h) = (self.px(40.0), self.px(84.0));
        let row_h = title_h + slider_h;
        let screen_w = self.width as i32;
        let most = rows.iter().map(|r| r.stops()).max().unwrap_or(1) as i32;
        // Every stop is a full touch target (one tap per step).
        let min = most * self.page_scale().px(44.0) + 2 * self.px(24.0) + 2 * pad;
        let w = self
            .px(560.0)
            .max(min)
            .min(screen_w - 2 * self.px(8.0))
            .max(1);
        let b = l.light_button;
        let x = (b.x + b.w - w).clamp(self.px(8.0), (screen_w - self.px(8.0) - w).max(0));
        let frame = Rect::new(x, l.title.h, w, rows.len() as i32 * row_h + 2 * edge);
        let rows = rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                let y = frame.y + edge + i as i32 * row_h;
                let area = Rect::new(frame.x + pad, y + title_h, frame.w - 2 * pad, slider_h);
                (
                    r,
                    Rect::new(frame.x, y, frame.w, row_h),
                    slider::geom(area, r.stops(), self.scale()),
                )
            })
            .collect();
        (frame, rows)
    }

    /// What a tap at (`x`, `y`) does while the menu is open.
    #[must_use]
    pub fn light_menu_hit(&self, l: &Layout, x: i32, y: i32) -> LightHit {
        if l.light_button.inset(-self.px(4.0)).contains(x, y) {
            return LightHit::Button;
        }
        let (frame, rows) = self.light_menu_rows(l);
        for (r, _, g) in rows {
            if g.area.contains(x, y) {
                return LightHit::Slider(r, slider::nearest(&g, x) as u8);
            }
        }
        if frame.contains(x, y) {
            LightHit::Frame
        } else {
            LightHit::Outside
        }
    }

    /// Dragging slider `row` to `x`: the step it snaps to (drags outside the
    /// track clamp to the ends).
    #[must_use]
    pub fn light_menu_drag(&self, l: &Layout, row: LightRow, x: i32) -> Option<u8> {
        self.light_menu_rows(l)
            .1
            .into_iter()
            .find(|(r, _, _)| *r == row)
            .map(|(_, _, g)| slider::nearest(&g, x) as u8)
    }

    /// The step a slider shows.
    #[must_use]
    pub fn light_value(&self, row: LightRow) -> u8 {
        match row {
            LightRow::Brightness => self.light_level,
            LightRow::Warmth => self.warmth_level.unwrap_or(0),
        }
    }

    /// The light button: a sun, dim while the light is off, active while
    /// the menu is open. Nothing without a front light.
    pub fn draw_light_button(&self, c: &mut Canvas, l: &Layout, t: Theme) {
        if !self.light_on {
            return;
        }
        let r = l.light_button;
        if self.light_menu {
            c.fill(r, t.wash);
            c.stroke(r, 2, t.ink);
        }
        let icon = if self.light_level == 0 {
            Icon::SunDim
        } else {
            Icon::Sun
        };
        draw_icon(c, icon, r, t.ink);
    }

    /// Draws the open menu over whatever is below it: a framed panel, each
    /// row its name over a slider.
    pub fn draw_light_menu(&self, c: &mut Canvas, l: &Layout, t: Theme) {
        if !self.light_menu || !self.light_on {
            return;
        }
        let (frame, rows) = self.light_menu_rows(l);
        c.fill(frame, t.paper);
        c.stroke(frame, self.px(1.5).max(2), t.ink);
        let ui = self.ui_px();
        let pad = self.px(16.0);
        let ink = if t.paper < 128 { 0xFF } else { 0x00 };
        for (i, (row, r, g)) in rows.iter().enumerate() {
            if i > 0 {
                c.fill(
                    Rect::new(r.x + pad, r.y, r.w - 2 * pad, 1.max(self.px(1.0))),
                    t.rule,
                );
            }
            let base = (g.area.y - self.px(10.0)) as f32;
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui,
                (r.x + pad + self.px(8.0)) as f32,
                base,
                &tr(row.title()),
                t.ink,
            );
            let sl = Slider {
                stops: row.labels(),
                value: usize::from(self.light_value(*row)),
                enabled: true,
                ends: true,
                focused: false,
            };
            slider::draw(c, g, &sl, &self.fonts, ui, self.scale(), (ink, t.paper));
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

    fn at(e: &mut EditorScreen, pct: u16) {
        let mut st = sigma_core::settings::Settings {
            ui_scale: pct,
            ..Default::default()
        };
        e.configure(&mut st, None);
    }

    fn lit(w: u32, h: u32) -> EditorScreen {
        let mut e = EditorScreen::new(w, h);
        e.light_on = true;
        e.light_level = 4;
        e.warmth_level = Some(3);
        e
    }

    #[test]
    fn the_button_takes_a_place_after_find_and_the_bar_does_not_overflow() {
        let k = Keyboard::default();
        for (w, h) in [(1440, 1920), (1920, 1440)] {
            for pct in [100u16, 125, 150, 175, 200] {
                let mut e = lit(w, h);
                at(&mut e, pct);
                let without = {
                    let mut x = EditorScreen::new(w, h);
                    at(&mut x, pct);
                    x.layout(true, &k)
                };
                let l = e.layout(true, &k);
                let order = [
                    l.files_button,
                    l.preview_button,
                    l.find_button,
                    l.light_button,
                    l.drawer_button,
                    l.graph_button,
                    l.options_button,
                ];
                for p in order.windows(2) {
                    assert!(p[0].x + p[0].w <= p[1].x, "{w}x{h} {pct}: in order");
                }
                assert!(l.options_button.x + l.options_button.w <= w as i32);
                assert!(l.light_button.w >= e.page_scale().px(44.0), "a full target");
                assert!(l.name_rect.w > 0, "{w}x{h} {pct}: the title keeps room");
                assert_eq!(
                    l.options_button.x + l.options_button.w,
                    without.options_button.x + without.options_button.w,
                    "Settings stays flush right"
                );
            }
        }
        // No light: no button, and its place closes up.
        let e = EditorScreen::new(1440, 1920);
        assert_eq!(e.layout(true, &k).light_button, Rect::default());
    }

    #[test]
    fn the_icon_hits_in_write_and_preview_and_only_with_a_light() {
        let k = Keyboard::default();
        let mut e = lit(1440, 1920);
        let l = e.layout(true, &k);
        let b = l.light_button;
        assert_eq!(
            e.preview_hit(&k, b.x + b.w / 2, b.y + b.h / 2),
            crate::Hit::LightMenu
        );
        e.light_on = false;
        assert_ne!(
            e.preview_hit(&k, b.x + b.w / 2, b.y + b.h / 2),
            crate::Hit::LightMenu
        );
    }

    #[test]
    fn sliders_have_the_settings_stops_and_warmth_only_with_a_mixer() {
        let k = Keyboard::default();
        let mut e = lit(1440, 1920);
        let l = e.layout(true, &k);
        let (_, rows) = e.light_menu_rows(&l);
        assert_eq!(
            rows.iter().map(|r| r.0).collect::<Vec<_>>(),
            [LightRow::Brightness, LightRow::Warmth]
        );
        assert_eq!(rows[0].2.xs.len(), usize::from(BACKLIGHT_MAX) + 1);
        assert_eq!(rows[1].2.xs.len(), usize::from(WARMTH_MAX) + 1);
        e.warmth_level = None;
        let (_, rows) = e.light_menu_rows(&e.layout(true, &k));
        assert_eq!(rows.len(), 1, "no mixer, no Warmth");
        assert_eq!(
            LightRow::Brightness.labels().first().map(String::as_str),
            Some("Off")
        );
        assert_eq!(
            LightRow::Warmth.labels().last().map(String::as_str),
            Some("Warm")
        );
    }

    #[test]
    fn every_step_is_one_tap_and_the_menu_stays_on_screen() {
        let k = Keyboard::default();
        for (w, h) in [(1440, 1920), (1920, 1440)] {
            for pct in [100u16, 200] {
                let mut e = lit(w, h);
                at(&mut e, pct);
                let l = e.layout(true, &k);
                let (frame, rows) = e.light_menu_rows(&l);
                assert_eq!(frame.y, l.title.h, "just under the title bar");
                assert!(frame.x >= 0 && frame.x + frame.w <= w as i32, "{w} {pct}");
                assert!(frame.y + frame.h <= h as i32);
                for (row, _, g) in &rows {
                    for (i, &x) in g.xs.iter().enumerate() {
                        assert_eq!(
                            e.light_menu_hit(&l, x, g.track_y),
                            LightHit::Slider(*row, i as u8),
                            "{w} {pct} {row:?} step {i}"
                        );
                    }
                    for r in &g.stops {
                        assert!(
                            r.w >= e.page_scale().px(44.0) - 1,
                            "{w} {pct}: a full target"
                        );
                    }
                    // A drag past either end clamps.
                    assert_eq!(e.light_menu_drag(&l, *row, -500), Some(0));
                    assert_eq!(
                        e.light_menu_drag(&l, *row, w as i32 + 500),
                        Some(row.stops() as u8 - 1)
                    );
                }
                let b = l.light_button;
                assert_eq!(
                    e.light_menu_hit(&l, b.x + b.w / 2, b.y + b.h / 2),
                    LightHit::Button
                );
                assert_eq!(
                    e.light_menu_hit(&l, frame.x + 2, frame.y + 2),
                    LightHit::Frame
                );
                assert_eq!(e.light_menu_hit(&l, 10, h as i32 - 10), LightHit::Outside);
                assert_eq!(
                    e.light_menu_hit(&l, frame.x + frame.w / 2, frame.y + frame.h + 4),
                    LightHit::Outside
                );
            }
        }
    }

    #[test]
    fn drawing_follows_the_state() {
        let k = Keyboard::default();
        let t = Theme::DAY;
        let mut e = lit(1440, 1920);
        let l = e.layout(true, &k);
        let draw = |e: &EditorScreen| {
            let mut c = Canvas::new(1440, 1920, t.paper);
            e.draw_light_button(&mut c, &l, t);
            e.draw_light_menu(&mut c, &l, t);
            c
        };
        let (frame, _) = e.light_menu_rows(&l);
        let closed = draw(&e);
        assert_eq!(get(&closed, frame.x, frame.y + frame.h / 2), t.paper);
        e.light_menu = true;
        let open = draw(&e);
        assert_eq!(get(&open, frame.x, frame.y + frame.h / 2), t.ink, "framed");
        // Dim sun while off, sun while on.
        let on = draw(&e).pixels;
        e.light_level = 0;
        assert_ne!(draw(&e).pixels, on);
        // No light: nothing at all.
        e.light_on = false;
        assert_eq!(
            draw(&e).pixels,
            Canvas::new(1440, 1920, t.paper).pixels,
            "no light, no menu"
        );
    }
}
