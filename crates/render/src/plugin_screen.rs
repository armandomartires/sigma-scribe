//! A screen drawn by a plugin: the host paints the title bar and close
//! button, the plugin's draw list fills the rest.
//!
//! The draw list is the plugin's whole picture (`sigma_plugin_api::Op`):
//! rectangles, strokes, pills, thick lines and text, in the host theme's
//! four tones. The title bar is painted after it, so a plugin can never
//! cover the host's controls.

use crate::canvas::{Canvas, Rect};
use crate::editor::EditorScreen;
use crate::fonts::Face;
use crate::graphmap::{fill_round, thick_line};
use crate::i18n::tr;
use crate::theme::Theme;
use sigma_core::Keyboard;
use sigma_plugin_api::{Frame, Op, Tone};

fn tone(t: Theme, c: Tone) -> u8 {
    match c {
        Tone::Paper => t.paper,
        Tone::Ink => t.ink,
        Tone::Mute => t.mute,
        Tone::Rule => t.rule,
    }
}

/// The host's face for a plugin face.
#[must_use]
pub fn face(f: sigma_plugin_api::Face) -> Face {
    match f {
        sigma_plugin_api::Face::Regular => Face::Atkinson,
        sigma_plugin_api::Face::Bold => Face::AtkinsonBold,
    }
}

fn rect(r: sigma_plugin_api::Rect) -> Rect {
    Rect::new(r.x, r.y, r.w, r.h)
}

impl EditorScreen {
    /// Height of the title bar the host paints; plugins draw below it.
    #[must_use]
    pub fn plugin_screen_top(&self, keyboard: &Keyboard) -> i32 {
        self.layout(false, keyboard).title.h
    }

    /// The picture shown while a plugin's module loads, before it can draw
    /// anything: a title and an empty bar, laid out like the plugin's own
    /// progress picture so the two follow each other without a jump.
    #[must_use]
    pub fn plugin_loading_frame(&self, keyboard: &Keyboard) -> Frame {
        let s = self.scale();
        let ui = self.ui_px();
        let (w, h) = (self.width as i32, self.height as i32);
        let top = self.plugin_screen_top(keyboard);
        let title = tr("Opening the graph\u{2026}").into_owned();
        let size = ui * 1.3;
        let y = top as f32 + (h - top) as f32 * 0.4;
        let tw = self.fonts.width(Face::AtkinsonBold, size, &title);
        let bar_w = (w - 2 * s.px(120.0)).clamp(s.px(200.0), s.px(640.0));
        let bar =
            sigma_plugin_api::Rect::new((w - bar_w) / 2, y as i32 + s.px(36.0), bar_w, s.px(28.0));
        Frame {
            ops: vec![
                Op::Fill {
                    r: sigma_plugin_api::Rect::new(0, 0, w, h),
                    c: Tone::Paper,
                },
                Op::Text {
                    face: sigma_plugin_api::Face::Bold,
                    size,
                    x: (w as f32 - tw) / 2.0,
                    y,
                    s: title,
                    c: Tone::Ink,
                },
                Op::Stroke {
                    r: bar,
                    t: s.px(2.0).max(2),
                    c: Tone::Ink,
                },
            ],
            ..Frame::default()
        }
    }

    /// The close button's box.
    #[must_use]
    pub fn plugin_close_rect(&self, keyboard: &Keyboard) -> Rect {
        self.layout(false, keyboard).close_button
    }

    /// Whether a tap lands on the close button.
    #[must_use]
    pub fn plugin_close_hit(&self, keyboard: &Keyboard, x: i32, y: i32) -> bool {
        self.layout(false, keyboard)
            .close_button
            .inset(-self.scale().px(4.0))
            .contains(x, y)
    }

    /// Paints a draw list onto `c`.
    fn paint_ops(&self, c: &mut Canvas, ops: &[Op], t: Theme) {
        for op in ops {
            match op {
                Op::Fill { r, c: tn } => c.fill(rect(*r), tone(t, *tn)),
                Op::Stroke { r, t: th, c: tn } => c.stroke(rect(*r), *th, tone(t, *tn)),
                Op::Pill { r, c: tn } => fill_round(c, rect(*r), r.h / 2, tone(t, *tn)),
                Op::Line { a, b, w, c: tn } => thick_line(c, *a, *b, *w, tone(t, *tn)),
                Op::Text {
                    face: f,
                    size,
                    x,
                    y,
                    s,
                    c: tn,
                } => {
                    self.fonts.draw(c, face(*f), *size, *x, *y, s, tone(t, *tn));
                }
            }
        }
    }

    /// Paints a pane plugin's frame inside `area` and nowhere else: the
    /// plugin draws in screen coordinates, so a pane-sized canvas catches
    /// the draw list (shifted to its corner) and is copied back.
    pub fn draw_plugin_pane(&self, c: &mut Canvas, frame: &Frame, area: Rect, t: Theme) {
        if area.w <= 0 || area.h <= 0 {
            return;
        }
        let (dx, dy) = (area.x, area.y);
        let shift =
            |r: sigma_plugin_api::Rect| sigma_plugin_api::Rect::new(r.x - dx, r.y - dy, r.w, r.h);
        let ops: Vec<Op> = frame
            .ops
            .iter()
            .map(|op| match op {
                Op::Fill { r, c } => Op::Fill {
                    r: shift(*r),
                    c: *c,
                },
                Op::Stroke { r, t, c } => Op::Stroke {
                    r: shift(*r),
                    t: *t,
                    c: *c,
                },
                Op::Pill { r, c } => Op::Pill {
                    r: shift(*r),
                    c: *c,
                },
                Op::Line { a, b, w, c } => Op::Line {
                    a: (a.0 - dx, a.1 - dy),
                    b: (b.0 - dx, b.1 - dy),
                    w: *w,
                    c: *c,
                },
                Op::Text {
                    face,
                    size,
                    x,
                    y,
                    s,
                    c,
                } => Op::Text {
                    face: *face,
                    size: *size,
                    x: x - dx as f32,
                    y: y - dy as f32,
                    s: s.clone(),
                    c: *c,
                },
            })
            .collect();
        let mut pane = Canvas::new(area.w as u32, area.h as u32, t.paper);
        self.paint_ops(&mut pane, &ops, t);
        c.blit(&pane, dx, dy);
    }

    /// Paints a plugin's frame, then the host's title bar over it.
    pub fn draw_plugin_screen(
        &self,
        c: &mut Canvas,
        frame: &Frame,
        title: &str,
        t: Theme,
        keyboard: &Keyboard,
    ) {
        c.fill(c.bounds(), t.paper);
        self.paint_ops(c, &frame.ops, t);
        let base = self.layout(false, keyboard);
        self.draw_title(c, &base, title, None, t, false);
        crate::icons::draw_icon(c, crate::icons::Icon::Close, base.close_button, t.ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_plugin_api::Rect as R;

    fn px(c: &Canvas, x: i32, y: i32) -> u8 {
        c.crop(Rect::new(x, y, 1, 1)).1[0]
    }

    #[test]
    fn ops_draw_in_theme_tones_and_the_title_bar_wins() {
        let e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let top = e.plugin_screen_top(&k);
        assert!(top > 0);
        let t = Theme::DAY;
        let frame = Frame {
            ops: vec![
                Op::Fill {
                    r: R::new(0, 0, 1440, 1920),
                    c: Tone::Rule,
                },
                Op::Fill {
                    r: R::new(100, top + 100, 50, 50),
                    c: Tone::Ink,
                },
                Op::Pill {
                    r: R::new(300, top + 100, 60, 60),
                    c: Tone::Mute,
                },
                Op::Line {
                    a: (500, top + 100),
                    b: (600, top + 100),
                    w: 4,
                    c: Tone::Ink,
                },
                Op::Stroke {
                    r: R::new(700, top + 100, 40, 40),
                    t: 2,
                    c: Tone::Ink,
                },
            ],
            ..Frame::default()
        };
        let mut c = Canvas::new(1440, 1920, 0);
        e.draw_plugin_screen(&mut c, &frame, "Graph", t, &k);
        assert_eq!(px(&c, 120, top + 120), t.ink);
        assert_eq!(px(&c, 330, top + 130), t.mute);
        assert_eq!(px(&c, 550, top + 100), t.ink);
        assert_eq!(px(&c, 700, top + 120), t.ink, "stroke edge");
        assert_eq!(
            px(&c, 720, top + 120),
            t.rule,
            "stroke interior is untouched"
        );
        assert_eq!(px(&c, 1400, top + 500), t.rule, "plugin background");
        // The plugin tried to cover the title bar; the host's paper is there.
        assert_eq!(px(&c, 1, 1), t.paper);
    }

    #[test]
    fn text_draws_and_close_button_hits() {
        let e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let top = e.plugin_screen_top(&k);
        let frame = Frame {
            ops: vec![Op::Text {
                face: sigma_plugin_api::Face::Bold,
                size: 40.0,
                x: 100.0,
                y: top as f32 + 100.0,
                s: "Hello".into(),
                c: Tone::Ink,
            }],
            ..Frame::default()
        };
        let mut c = Canvas::new(1440, 1920, 0);
        e.draw_plugin_screen(&mut c, &frame, "Graph", Theme::DAY, &k);
        let (_, pixels) = c.crop(Rect::new(100, top + 60, 200, 50));
        assert!(
            pixels.iter().any(|&p| p < Theme::DAY.paper),
            "ink was drawn"
        );
        let close = e.layout(false, &k).close_button;
        assert!(e.plugin_close_hit(&k, close.x + 2, close.y + 2));
        assert!(!e.plugin_close_hit(&k, 700, top + 400));
    }
}
