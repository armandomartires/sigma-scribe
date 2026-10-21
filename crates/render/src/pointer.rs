//! The on-screen mouse pointer (v0.8.1): a small outlined arrow, drawn last
//! over the frame. Its tip is at (x, y). Ink fill with a paper outline, so
//! it reads on Paper and Night and on text.

use crate::{Canvas, Rect, Theme};

/// 12 × 19 arrow: `#` outline (paper), `o` fill (ink).
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

/// Draws the arrow, clipped to the canvas.
pub fn draw_pointer(c: &mut Canvas, x: i32, y: i32, cell: i32, t: Theme) {
    let (w, h) = (c.width as i32, c.height as i32);
    for (r, row) in ARROW.iter().enumerate() {
        for (col, ch) in row.bytes().enumerate() {
            let v = match ch {
                b'#' => t.paper,
                b'o' => t.ink,
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
        assert_eq!(c.pixels[0], Theme::DAY.paper, "tip outline");
        assert_eq!(c.pixels[(6 * 40 + 2) as usize], Theme::DAY.ink, "fill");
        assert_eq!(c.pixels[(39 * 40 + 39) as usize], 128, "outside untouched");
        draw_pointer(&mut c, 35, 35, 2, Theme::NIGHT);
        let r = pointer_rect(0, 0, pointer_cell(2.0));
        assert!(r.w >= 16 && r.h >= 16);
        assert!(ARROW.iter().all(|r| r.len() == 12));
    }
}
