//! A QR code of text (the Kobo's Syncthing device ID), as a module matrix
//! and drawn in whole pixels. Black on white whatever the page theme: phone
//! scanners want the contrast. Encoded by `qrcodegen` at error correction M.

use crate::canvas::{Canvas, Rect};
use qrcodegen::{QrCode, QrCodeEcc};

/// Quiet zone around the code, in modules (the QR specification asks for 4).
pub const QUIET: i32 = 4;

/// The largest module the page draws, in pixels: 14 px is about 1.2 mm on
/// the Forma (300 dpi), so a device ID's 33-module code is about 39 mm
/// across, quiet zone not counted.
pub const MAX_MODULE: i32 = 14;

/// The code for `text`: its side in modules and the modules in rows
/// (`true` = dark). `None` when the text does not fit a QR code.
#[must_use]
pub fn modules(text: &str) -> Option<(usize, Vec<bool>)> {
    let qr = QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let n = usize::try_from(qr.size()).ok()?;
    let mut out = Vec::with_capacity(n * n);
    for y in 0..qr.size() {
        for x in 0..qr.size() {
            out.push(qr.get_module(x, y));
        }
    }
    Some((n, out))
}

/// Pixels per module for a code of `n` modules (plus its quiet zone) that
/// must fit in `w` by `h` pixels; whole pixels, at least 1, at most
/// [`MAX_MODULE`].
#[must_use]
pub fn module_px(n: usize, w: i32, h: i32) -> i32 {
    let span = n as i32 + 2 * QUIET;
    (w.min(h) / span).clamp(1, MAX_MODULE)
}

/// Draws the code of `text` with its quiet zone centred on `x` (the middle
/// of a column) and starting at `y`, `px` pixels to the module. Returns the
/// square it covers, or `None` when the text cannot be encoded.
pub fn draw(c: &mut Canvas, text: &str, cx: i32, y: i32, px: i32) -> Option<Rect> {
    let (n, mods) = modules(text)?;
    let side = (n as i32 + 2 * QUIET) * px;
    let area = Rect::new(cx - side / 2, y, side, side);
    c.fill(area, 255);
    for (i, &dark) in mods.iter().enumerate() {
        if dark {
            let (mx, my) = ((i % n) as i32, (i / n) as i32);
            c.fill(
                Rect::new(
                    area.x + (QUIET + mx) * px,
                    area.y + (QUIET + my) * px,
                    px,
                    px,
                ),
                0,
            );
        }
    }
    Some(area)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "MFZWI3D-BONSGYC-YLTMRWG-C43ENR5-QXGZDMM-FZWI3DP-BONSGYY-LTMRWAD";

    #[test]
    fn a_device_id_is_a_version_4_code_in_alphanumeric_mode() {
        let (n, m) = modules(ID).unwrap();
        assert_eq!(n, 33, "version 4: the ID is upper case, digits and dashes");
        assert_eq!(m.len(), 33 * 33);
        // Finder pattern corner: the top-left 7x7 has a dark ring.
        assert!(m[0] && m[6] && m[6 * 33] && !m[33 + 1]);
    }

    #[test]
    fn modules_are_whole_pixels_and_fit() {
        assert_eq!(module_px(33, 1296, 1500), MAX_MODULE);
        assert_eq!(module_px(33, 450, 1500), 450 / 41);
        assert_eq!(module_px(33, 10, 10), 1);
    }

    #[test]
    fn the_drawn_code_reads_back_as_the_id() {
        let mut c = Canvas::new(700, 700, 200);
        let r = draw(&mut c, ID, 350, 40, MAX_MODULE).unwrap();
        assert_eq!(r.w, (33 + 8) * MAX_MODULE);
        // Quiet zone is white, and the area is square.
        assert_eq!(c.pixels[(r.y as usize) * 700 + r.x as usize], 255);
        let mut img =
            rqrr::PreparedImage::prepare_from_greyscale(700, 700, |x, y| c.pixels[y * 700 + x]);
        let mut grids = img.detect_grids();
        assert_eq!(grids.len(), 1);
        let (_, text) = grids.remove(0).decode().unwrap();
        assert_eq!(text, ID);
    }
}
