//! Lucide icons (ISC licence, <https://lucide.dev>), baked at build time
//! into alpha masks at every UI-scale size (v1.1, plan decision 35). One
//! blit function draws them; there is no scaling at run time — the nearest
//! baked size not larger than the target is used.

use crate::canvas::{Canvas, Rect};

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// Title-bar and sheet icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Eye,
    Pencil,
    Search,
    Scratch,
    Todos,
    Contents,
    Graph,
    Help,
    Back,
    Folder,
    Close,
    /// Handwriting panel (v1.5.3).
    Swap,
    Backspace,
    Return,
    Globe,
    PrevPage,
    NextPage,
    Space,
    /// Shift on the Kobo style keyboard (v1.13).
    Shift,
}

impl Icon {
    #[must_use]
    pub fn lucide(self) -> &'static str {
        match self {
            Self::Eye => "eye",
            Self::Pencil => "pencil",
            Self::Search => "search",
            Self::Scratch => "sticky-note",
            Self::Todos => "list-todo",
            Self::Contents => "table-of-contents",
            Self::Graph => "share-2",
            Self::Help => "circle-help",
            Self::Back => "arrow-left",
            Self::Folder => "folder",
            Self::Close => "x",
            Self::Swap => "arrow-left-right",
            Self::Backspace => "delete",
            Self::Return => "corner-down-left",
            Self::Globe => "globe",
            Self::PrevPage => "chevron-left",
            Self::NextPage => "chevron-right",
            Self::Space => "space",
            Self::Shift => "arrow-big-up",
        }
    }
}

/// The baked size used for a glyph of about `want` px.
#[must_use]
pub fn baked_size(want: i32) -> u32 {
    let want = want.max(1) as u32;
    SIZES
        .iter()
        .copied()
        .filter(|&s| s <= want)
        .max()
        .unwrap_or(SIZES[0])
}

/// Draws `icon` centred in `target` with ink `ink`; the glyph is about half
/// the target's short side.
pub fn draw_icon(c: &mut Canvas, icon: Icon, target: Rect, ink: u8) {
    let sz = baked_size(target.w.min(target.h) / 2);
    let Some((_, _, mask)) = ICONS
        .iter()
        .find(|(n, s, _)| *n == icon.lucide() && *s == sz)
    else {
        return;
    };
    let s = sz as i32;
    let x0 = target.x + (target.w - s) / 2;
    let y0 = target.y + (target.h - s) / 2;
    for y in 0..s {
        for x in 0..s {
            let a = mask[(y * s + x) as usize];
            if a > 0 {
                c.blend(x0 + x, y0 + y, ink, a);
            }
        }
    }
}

/// Like [`draw_icon`] but 1-bit (alpha ≥ 50 % is ink, the rest is left
/// alone), for the DU-safe handwriting panel (v1.5.3).
pub fn draw_icon_1bit(c: &mut Canvas, icon: Icon, target: Rect, ink: u8) {
    let sz = baked_size(target.w.min(target.h) / 2);
    let Some((_, _, mask)) = ICONS
        .iter()
        .find(|(n, s, _)| *n == icon.lucide() && *s == sz)
    else {
        return;
    };
    let s = sz as i32;
    let x0 = target.x + (target.w - s) / 2;
    let y0 = target.y + (target.h - s) / 2;
    let b = c.bounds();
    for y in 0..s {
        for x in 0..s {
            if mask[(y * s + x) as usize] >= 128 && b.contains(x0 + x, y0 + y) {
                let i = ((y0 + y) * c.width as i32 + x0 + x) as usize;
                c.pixels[i] = ink;
            }
        }
    }
}

/// A back crumb's arrow (v1.4.6): the Lucide arrow-left, about one text
/// height tall, with its left edge at `x` and centred on `mid_y`. Returns
/// where the label starts. Replaces the "←" text glyph, which Atkinson
/// does not have (it drew as an empty box before "Up to Files").
pub fn draw_back_arrow(c: &mut Canvas, x: i32, mid_y: i32, text_px: f32, ink: u8) -> f32 {
    let side = (text_px * 2.0).round() as i32;
    let r = Rect::new(x - side / 4, mid_y - side / 2, side, side);
    draw_icon(c, Icon::Back, r, ink);
    x as f32 + text_px * 1.35
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_baked_at_every_size() {
        for i in [
            Icon::Eye,
            Icon::Pencil,
            Icon::Search,
            Icon::Scratch,
            Icon::Todos,
            Icon::Contents,
            Icon::Graph,
            Icon::Help,
            Icon::Back,
            Icon::Folder,
            Icon::Close,
            Icon::Swap,
            Icon::Backspace,
            Icon::Return,
            Icon::Globe,
            Icon::PrevPage,
            Icon::NextPage,
            Icon::Space,
        ] {
            for &s in SIZES {
                let m = ICONS
                    .iter()
                    .find(|(n, z, _)| *n == i.lucide() && *z == s)
                    .expect("baked");
                assert_eq!(m.2.len(), (s * s) as usize);
                assert!(m.2.iter().any(|&a| a > 200), "{i:?} {s}: has solid stroke");
            }
        }
    }

    #[test]
    fn forma_steps_are_baked() {
        // 44 brief units × 1440/1264 × scale, half of it.
        for p in [100.0f32, 125.0, 150.0, 175.0, 200.0] {
            let want = (44.0 * 1440.0 / 1264.0 * p / 100.0 * 0.5).round() as u32;
            assert!(SIZES.contains(&want), "{p}");
        }
    }

    #[test]
    fn blit_draws_ink_inside_the_target() {
        let mut c = Canvas::new(120, 120, 255);
        draw_icon(&mut c, Icon::Search, Rect::new(10, 10, 100, 100), 0);
        assert!(c.pixels.iter().any(|&v| v < 64));
        assert_eq!(c.pixels[0], 255);
    }
}
