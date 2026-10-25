//! The splash screen, drawn before the library and settings load.

use super::*;

/// The splash screen: drawn the moment the panel is open, before the
/// library, dictionaries and settings are loaded, so the Forma shows the
/// app while it starts. Follows the saved paper and rotation, and shows a
/// progress bar that the start moves along with [`Splash::progress`]. It
/// never fails the start: a panel that cannot be written to is left alone.
pub struct Splash {
    c: Canvas,
    phys: (u32, u32),
    rot: Rotation,
    t: Theme,
    bar: Rect,
}

impl Splash {
    /// Draws the splash at 0 % and shows it (a full refresh).
    pub fn show(dev: &mut dyn Device, root: &std::path::Path) -> Self {
        use sigma_render::fonts::{Face, Fonts};
        use sigma_render::theme::Scale;
        let st = load_settings(root);
        let t = Theme::for_paper(st.paper);
        let phys = dev.size();
        let (lw, lh) = logical_size(phys, st.rotation);
        let mut c = Canvas::new(lw, lh, t.paper);
        let fonts = Fonts::new();
        let s = Scale::for_width(lw.min(lh));
        let centre = |c: &mut Canvas, face: Face, size: f32, y: f32, text: &str, ink: u8| {
            let w = fonts.width(face, size, text);
            fonts.draw(c, face, size, (lw as f32 - w) / 2.0, y, text, ink);
        };
        let mid = lh as f32 * 0.42;
        centre(
            &mut c,
            Face::LiterataSemi,
            s.px(120.0) as f32,
            mid,
            "Σscribe",
            t.ink,
        );
        centre(
            &mut c,
            Face::Atkinson,
            s.px(26.0) as f32,
            mid + s.px(70.0) as f32,
            "SIGMArecon",
            t.mute,
        );
        // Kobo's loading bar: the word in the serif, left-aligned, over a flat
        // bar that is dark where it is done and light for the rest.
        let bw = s.px(520.0);
        let bar = Rect::new(
            (lw as i32 - bw) / 2,
            (lh as f32 * 0.60) as i32,
            bw,
            s.px(8.0).max(4),
        );
        fonts.draw(
            &mut c,
            Face::Literata,
            s.px(32.0) as f32,
            bar.x as f32,
            (bar.y - s.px(22.0)) as f32,
            &sigma_render::i18n::tr("Loading…"),
            t.ink,
        );
        let mut sp = Self {
            c,
            phys,
            rot: st.rotation,
            t,
            bar,
        };
        sp.draw_bar(0.0);
        sp.present(dev, None, Refresh::Full);
        sp
    }

    /// Moves the bar to `done` (0 to 1) and shows just the bar.
    pub fn progress(&mut self, dev: &mut dyn Device, done: f32) {
        self.draw_bar(done);
        let r = self.bar.inset(-4);
        self.present(dev, Some(r), Refresh::Partial);
    }

    /// A flat bar: dark for what is done, the rule tone for the rest.
    fn draw_bar(&mut self, done: f32) {
        let r = self.bar;
        self.c.fill(r, self.t.rule);
        let w = (r.w as f32 * done.clamp(0.0, 1.0)) as i32;
        if w > 0 {
            self.c.fill(Rect::new(r.x, r.y, w, r.h), self.t.ink);
        }
    }

    /// Shows `region` (the whole screen when `None`), turned to the panel.
    fn present(&self, dev: &mut dyn Device, region: Option<Rect>, refresh: Refresh) {
        let (lw, lh) = (self.c.width as i32, self.c.height as i32);
        let r = region.unwrap_or(self.c.bounds());
        let (x0, y0) = (r.x.max(0), r.y.max(0));
        let (x1, y1) = ((r.x + r.w).min(lw), (r.y + r.h).min(lh));
        let mut panel = Canvas::new(self.phys.0, self.phys.1, self.t.paper);
        let pw = panel.width as usize;
        for y in 0..lh {
            for x in 0..lw {
                let (px, py) = match self.rot {
                    Rotation::R0 => (x, y),
                    Rotation::R90 => (lh - 1 - y, x),
                    Rotation::R180 => (lw - 1 - x, lh - 1 - y),
                    Rotation::R270 => (y, lw - 1 - x),
                };
                if let Some(v) = panel.pixels.get_mut(py as usize * pw + px as usize) {
                    *v = self.c.pixels[(y * lw + x) as usize];
                }
            }
        }
        let pr = match self.rot {
            Rotation::R0 => Rect::new(x0, y0, x1 - x0, y1 - y0),
            Rotation::R90 => Rect::new(lh - y1, x0, y1 - y0, x1 - x0),
            Rotation::R180 => Rect::new(lw - x1, lh - y1, x1 - x0, y1 - y0),
            Rotation::R270 => Rect::new(y0, lw - x1, y1 - y0, x1 - x0),
        };
        let _ = dev.present(&panel, pr, refresh);
    }
}
