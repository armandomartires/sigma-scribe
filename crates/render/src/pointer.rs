//! The on-screen mouse pointer (v0.8.1): a small outlined arrow, drawn last
//! over the frame. Its tip is at (x, y).
//!
//! v1.4.1: always a pure black (0x00) arrow with a pure white (0xFF)
//! outline, in Paper and Night. Until 1.4.0 it used the theme's ink and
//! paper: in Night that is a near-white (233) arrow, and in Paper 28 / 240
//! are greys, which the 2-level DU waveform used for pointer moves is not
//! defined for (DU only targets black or white), so the panel could leave
//! it white. Only 0x00 / 0xFF pixels are ever drawn now.

use crate::{Canvas, Rect, Theme};

/// Arrow fill.
pub const FILL: u8 = 0x00;
/// Arrow outline.
pub const OUTLINE: u8 = 0xFF;

/// 12 × 19 arrow: `#` outline (white), `o` fill (black).
const ARROW: [&str; 19] = [
    "#           ",
    "##          ",
    "#o#         ",
    "#oo#        ",
    "#ooo#       ",
    "#oooo#      ",
    "#ooooo#     ",
    "#oooooo#    ",
    "#ooooooo#   ",
    "#oooooooo#  ",
    "#ooooooooo# ",
    "#oooooo#####",
    "#ooo#oo#    ",
    "#oo# #oo#   ",
    "#o#  #oo#   ",
    "##    #oo#  ",
    "#     #oo#  ",
    "       #oo# ",
    "        ##  ",
];

/// Size of one sprite cell for a screen scale (`px(1.0)` of the brief).
#[must_use]
pub fn pointer_cell(unit: f32) -> i32 {
    ((unit * 1.5).round() as i32).max(1)
}

/// Screen rectangle the sprite covers with its tip at (x, y).
#[must_use]
pub fn pointer_rect(x: i32, y: i32, cell: i32) -> Rect {
    Rect::new(x, y, 12 * cell, 19 * cell)
}

/// The arrow's pixel values, `w × h` cells (None = transparent).
#[must_use]
pub fn sprite() -> Vec<Option<u8>> {
    ARROW
        .iter()
        .flat_map(|r| {
            r.bytes().map(|b| match b {
                b'#' => Some(OUTLINE),
                b'o' => Some(FILL),
                _ => None,
            })
        })
        .collect()
}

/// Draws the arrow, clipped to the canvas. The theme is not used for the
/// colours (always black with a white outline, see the module note).
pub fn draw_pointer(c: &mut Canvas, x: i32, y: i32, cell: i32, _t: Theme) {
    let (w, h) = (c.width as i32, c.height as i32);
    for (r, row) in ARROW.iter().enumerate() {
        for (col, ch) in row.bytes().enumerate() {
            let v = match ch {
                b'#' => OUTLINE,
                b'o' => FILL,
                _ => continue,
            };
            let (px, py) = (x + col as i32 * cell, y + r as i32 * cell);
            for dy in 0..cell {
                for dx in 0..cell {
                    let (qx, qy) = (px + dx, py + dy);
                    if qx >= 0 && qy >= 0 && qx < w && qy < h {
                        c.pixels[(qy * w + qx) as usize] = v;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_is_outlined_and_clipped() {
        let mut c = Canvas::new(40, 40, 128);
        draw_pointer(&mut c, 0, 0, 2, Theme::DAY);
        assert_eq!(c.pixels[0], OUTLINE, "tip outline");
        assert_eq!(c.pixels[(6 * 40 + 2) as usize], FILL, "fill");
        assert_eq!(c.pixels[(39 * 40 + 39) as usize], 128, "outside untouched");
        draw_pointer(&mut c, 35, 35, 2, Theme::NIGHT);
        let r = pointer_rect(0, 0, pointer_cell(2.0));
        assert!(r.w >= 16 && r.h >= 16);
        assert!(ARROW.iter().all(|r| r.len() == 12));
    }

    #[test]
    fn sprite_is_only_pure_black_and_white() {
        let sp = sprite();
        assert_eq!(sp.len(), 12 * 19);
        assert!(sp.iter().flatten().all(|&v| v == 0x00 || v == 0xFF));
        assert!(sp.contains(&Some(0x00)) && sp.contains(&Some(0xFF)));
        for t in [Theme::DAY, Theme::NIGHT] {
            let mut c = Canvas::new(60, 60, 128);
            draw_pointer(&mut c, 5, 5, 2, t);
            assert!(c.pixels.iter().all(|&v| v == 0 || v == 255 || v == 128));
        }
    }
}
