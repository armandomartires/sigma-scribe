//! Desktop emulator backend: a framebuffer in memory (1440×1920 by default),
//! a queue of scripted events, PNG snapshots and a log of refreshes.

use crate::{Device, Event, Refresh};

use sigma_render::{Canvas, Rect};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;

pub struct DesktopDevice {
    pub panel: Canvas,
    /// `SIGMA_EINK=1`: 16 grey levels and ~8 % ghosting from fast/partial
    /// refreshes (a full refresh clears it), like `scripts/sim.sh --eink`.
    pub eink: bool,
    pub events: VecDeque<Event>,
    pub refreshes: Vec<(Rect, Refresh)>,
    /// Every `usb_host` call (v0.8), for tests.
    pub usb_host: Vec<bool>,
}

impl DesktopDevice {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            panel: Canvas::new(width, height, 255),
            eink: std::env::var("SIGMA_EINK").is_ok_and(|v| v != "0"),
            events: VecDeque::new(),
            refreshes: Vec::new(),
            usb_host: Vec::new(),
        }
    }

    /// # Errors
    /// The PNG could not be written.
    pub fn shot(&self, path: &Path) -> io::Result<()> {
        write_png(&self.panel, path)
    }
}

/// # Errors
/// The PNG could not be written.
pub fn write_png(c: &Canvas, path: &Path) -> io::Result<()> {
    let w = BufWriter::new(File::create(path)?);
    let mut enc = png::Encoder::new(w, c.width, c.height);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(io::Error::other)?;
    writer.write_image_data(&c.pixels).map_err(io::Error::other)
}

/// One e-ink pixel: 16 grey levels (the panel's 4 bits); fast and partial
/// refreshes keep ~8 % of the previous image.
#[must_use]
pub fn eink_pixel(old: u8, new: u8, refresh: Refresh) -> u8 {
    let v = if refresh == Refresh::Full {
        u16::from(new)
    } else if refresh == Refresh::Fast && new > 32 && new < 224 {
        // DU-like (v1.0.1 model): small Fast updates cannot reach a grey
        // exactly, so some of the old pixel stays (the pointer trails).
        (u16::from(new) * 70 + u16::from(old) * 30) / 100
    } else {
        (u16::from(new) * 92 + u16::from(old) * 8) / 100
    };
    ((v + 8) / 17 * 17).min(255) as u8
}

impl Device for DesktopDevice {
    fn size(&self) -> (u32, u32) {
        (self.panel.width, self.panel.height)
    }

    fn present(&mut self, canvas: &Canvas, region: Rect, refresh: Refresh) -> io::Result<()> {
        let (r, px) = canvas.crop(region);
        let w = self.panel.width as usize;
        for row in 0..r.h as usize {
            let dst = (r.y as usize + row) * w + r.x as usize;
            let src = &px[row * r.w as usize..(row + 1) * r.w as usize];
            if self.eink {
                for (i, &new) in src.iter().enumerate() {
                    let old = self.panel.pixels[dst + i];
                    self.panel.pixels[dst + i] = eink_pixel(old, new, refresh);
                }
            } else {
                self.panel.pixels[dst..dst + r.w as usize].copy_from_slice(src);
            }
        }
        self.refreshes.push((r, refresh));
        Ok(())
    }

    fn next_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    fn usb_host(&mut self, on: bool) -> io::Result<()> {
        self.usb_host.push(on);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eink_quantises_and_ghosts_until_a_full_refresh() {
        assert_eq!(eink_pixel(0, 255, Refresh::Full), 255);
        assert_eq!(eink_pixel(0, 100, Refresh::Full) % 17, 0);
        let ghost = eink_pixel(0, 255, Refresh::Fast);
        assert!(ghost < 255 && ghost > 200, "{ghost}");
    }
}
