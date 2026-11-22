//! A thin WASM shell that draws one fixture note with `sigma-render`.
//!
//! This is the P3-06 pilot: a static renderer preview, not a product, not a
//! second e-reader, and not a release. The host (a small HTML page, or the
//! CI smoke) calls [`preview_render`] and reads the 8-bit greyscale buffer
//! from WASM memory. There is no `wasm-bindgen` and no `kobo` feature.
//!
//! The crate allows the workspace `unsafe_code` lint so the exports can
//! be `#[unsafe(no_mangle)]`. It has no `unsafe` blocks.

#![warn(missing_docs)]

use std::sync::Mutex;

use sigma_core::markdown;
use sigma_render::canvas::{Canvas, Rect};
use sigma_render::fonts::Fonts;
use sigma_render::preview::{self, Type};
use sigma_render::theme::{Scale, Theme};

/// Wire version of the raw wasm exports. Bumped on an incompatible change.
pub const ABI: u32 = 1;

/// Night paper (bit 0 of [`preview_render`]'s flags).
pub const FLAG_NIGHT: u32 = 1;
/// Landscape panel (bit 1 of [`preview_render`]'s flags).
pub const FLAG_LANDSCAPE: u32 = 2;

/// Portrait panel width, in device pixels (a Forma).
pub const PORTRAIT_WIDTH: u32 = 1440;
/// Portrait panel height, in device pixels (a Forma).
pub const PORTRAIT_HEIGHT: u32 = 1920;

/// The fixture note drawn by the preview.
pub const FIXTURE: &str = include_str!("../fixture.md");

/// Brief-space page margin (the editor's default).
const MARGIN_BRIEF: f32 = 28.0;
/// Brief-space vertical inset of the text column.
const INSET_BRIEF: f32 = 16.0;
/// Body size in brief pixels (the editor's default).
const BODY_BRIEF: f32 = 20.0;
/// Line-height factor (the editor's default).
const LEADING: f32 = 1.5;

struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

static FRAME: Mutex<Option<Frame>> = Mutex::new(None);

fn lock_frame() -> std::sync::MutexGuard<'static, Option<Frame>> {
    FRAME.lock().unwrap_or_else(|e| e.into_inner())
}

/// FNV-1a 64-bit over `bytes`. Used as the smoke digest.
#[must_use]
pub fn digest(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in bytes {
        h ^= u64::from(x);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Panel size for `flags`.
#[must_use]
pub fn panel_size(flags: u32) -> (u32, u32) {
    if flags & FLAG_LANDSCAPE != 0 {
        (PORTRAIT_HEIGHT, PORTRAIT_WIDTH)
    } else {
        (PORTRAIT_WIDTH, PORTRAIT_HEIGHT)
    }
}

/// Theme for `flags`.
#[must_use]
pub fn theme(flags: u32) -> Theme {
    if flags & FLAG_NIGHT != 0 {
        Theme::NIGHT
    } else {
        Theme::DAY
    }
}

/// Lays out and draws the fixture into a new canvas.
///
/// The text column uses the editor's default body size, leading, and
/// margins. There is no title bar, drawer, or keyboard: this is
/// renderer-only.
#[must_use]
pub fn render_fixture(flags: u32) -> Canvas {
    let (width, height) = panel_size(flags);
    let theme = theme(flags);
    let scale = Scale::for_width(width.min(height));
    let fonts = Fonts::new();
    let margin = scale.px(MARGIN_BRIEF);
    let inset = scale.px(INSET_BRIEF);
    let area = Rect::new(
        margin,
        inset,
        (width as i32 - 2 * margin).max(1),
        (height as i32 - 2 * inset).max(1),
    );
    let ty = Type {
        body: BODY_BRIEF * scale.0,
        leading: LEADING,
        scale,
    };
    let blocks = markdown::parse(FIXTURE);
    let lines = preview::layout(&blocks, &fonts, area.w, ty, &|_| false);
    let pages = preview::paginate(&lines, area.h);
    let mut canvas = Canvas::new(width, height, theme.paper);
    if let Some(range) = pages.first() {
        preview::draw_page(
            &mut canvas,
            &fonts,
            area,
            &lines,
            range.clone(),
            theme,
            scale,
        );
    }
    canvas
}

/// ABI version (`1`).
#[unsafe(no_mangle)]
pub extern "C" fn preview_abi() -> u32 {
    ABI
}

/// Draws the fixture. `flags` is a bitmask of [`FLAG_NIGHT`] and
/// [`FLAG_LANDSCAPE`]. Returns `0` on success.
#[unsafe(no_mangle)]
pub extern "C" fn preview_render(flags: u32) -> i32 {
    let canvas = render_fixture(flags);
    *lock_frame() = Some(Frame {
        width: canvas.width,
        height: canvas.height,
        pixels: canvas.pixels,
    });
    0
}

/// Width of the last rendered frame, or `0`.
#[unsafe(no_mangle)]
pub extern "C" fn preview_width() -> u32 {
    lock_frame().as_ref().map_or(0, |f| f.width)
}

/// Height of the last rendered frame, or `0`.
#[unsafe(no_mangle)]
pub extern "C" fn preview_height() -> u32 {
    lock_frame().as_ref().map_or(0, |f| f.height)
}

/// Pointer to the last frame's greyscale pixels in WASM memory, or `0`.
#[unsafe(no_mangle)]
pub extern "C" fn preview_ptr() -> u32 {
    lock_frame()
        .as_ref()
        .map_or(0, |f| f.pixels.as_ptr() as usize as u32)
}

/// FNV-1a digest of the last frame, or `0` if none has been drawn.
#[unsafe(no_mangle)]
pub extern "C" fn preview_digest() -> u64 {
    lock_frame().as_ref().map_or(0, |f| digest(&f.pixels))
}

/// End-to-end smoke: day and night renders of the fixture, with a live
/// buffer in memory. Returns `0` when the checks pass, a negative code
/// otherwise. The CI job and a host test both call this.
#[unsafe(no_mangle)]
pub extern "C" fn preview_smoke() -> i32 {
    if preview_render(0) != 0 {
        return -1;
    }
    let (w, h, day, ink) = {
        let f = lock_frame();
        let Some(f) = f.as_ref() else {
            return -2;
        };
        if f.width != PORTRAIT_WIDTH || f.height != PORTRAIT_HEIGHT {
            return -3;
        }
        if f.pixels.len() != (PORTRAIT_WIDTH * PORTRAIT_HEIGHT) as usize {
            return -4;
        }
        let paper = Theme::DAY.paper;
        if f.pixels.iter().all(|&p| p == paper) {
            return -5;
        }
        let ink = f.pixels.iter().filter(|&&p| p <= Theme::DAY.ink).count();
        if ink == 0 {
            return -6;
        }
        (f.width, f.height, digest(&f.pixels), ink)
    };
    if preview_render(FLAG_NIGHT) != 0 {
        return -7;
    }
    let night = {
        let f = lock_frame();
        let Some(f) = f.as_ref() else {
            return -8;
        };
        if f.width != w || f.height != h {
            return -9;
        }
        if f.pixels[0] != Theme::NIGHT.paper {
            return -10;
        }
        digest(&f.pixels)
    };
    if night == day {
        return -11;
    }
    if preview_render(FLAG_LANDSCAPE) != 0 {
        return -12;
    }
    {
        let f = lock_frame();
        let Some(f) = f.as_ref() else {
            return -13;
        };
        if f.width != PORTRAIT_HEIGHT || f.height != PORTRAIT_WIDTH {
            return -14;
        }
    }
    // Restore the default (day, portrait) frame for a host that blits it.
    if preview_render(0) != 0 {
        return -15;
    }
    let _ = ink;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_passes_natively() {
        assert_eq!(preview_smoke(), 0);
        assert_eq!(preview_width(), PORTRAIT_WIDTH);
        assert_eq!(preview_height(), PORTRAIT_HEIGHT);
        assert_ne!(preview_digest(), 0);
        assert_ne!(preview_ptr(), 0);
        eprintln!("day digest {:#018x}", preview_digest());
    }

    #[test]
    fn the_same_flags_draw_the_same_pixels() {
        let a = render_fixture(0);
        let b = render_fixture(0);
        assert_eq!(a.pixels, b.pixels);
        assert_eq!(digest(&a.pixels), digest(&b.pixels));
    }

    #[test]
    fn the_fixture_has_visible_ink() {
        let c = render_fixture(0);
        let ink = c.pixels.iter().filter(|&&p| p <= Theme::DAY.ink).count();
        assert!(ink > 1_000, "ink pixels {ink}");
        assert!(ink < c.pixels.len() / 2, "too much ink {ink}");
    }
}
