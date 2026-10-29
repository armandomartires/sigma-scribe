//! Desktop emulator backend: a framebuffer in memory (1440×1920 by default),
//! a queue of scripted events, PNG snapshots and a log of refreshes.

use crate::{Device, Event, Refresh};

use sigma_render::{Canvas, Rect};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;

/// The desktop [`Device`]: an in-memory panel driven by
/// scripted events, used by the emulator and tests.
pub struct DesktopDevice {
    /// The screen contents (8-bit grey).
    pub panel: Canvas,
    /// `SIGMA_EINK=1`: 16 grey levels and ~8 % ghosting from fast/partial
    /// refreshes (a full refresh clears it), like `scripts/sim.sh --eink`.
    pub eink: bool,
    /// Events still to deliver, oldest first.
    pub events: VecDeque<Event>,
    /// Every refresh drawn so far: its rectangle and mode.
    pub refreshes: Vec<(Rect, Refresh)>,
    /// Every `usb_host` call (v0.8), for tests.
    pub usb_host: Vec<bool>,
    /// Pretend an accelerometer is present (v1.2 emulator `sensor on`).
    pub gsensor: bool,
    /// Pretend a front light is present at this step (v1.12). `None`: none.
    pub light: Option<u8>,
    /// Pretend a warm-to-cool mixer is present at this step (v1.17).
    /// `None`: none.
    pub warmth: Option<u8>,
    /// Whether the pretend mixer can be read back (the Forma's may not be).
    pub warmth_readable: bool,
    /// What the light probe "found" at start, handed over once (the Kobo's
    /// is real; tests set this to see the app log it).
    pub light_report: Vec<String>,
}

impl DesktopDevice {
    /// A white panel of `width` × `height`, with no events, no sensor and
    /// no light. E-ink emulation follows `SIGMA_EINK`.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            panel: Canvas::new(width, height, 255),
            eink: std::env::var("SIGMA_EINK").is_ok_and(|v| v != "0"),
            events: VecDeque::new(),
            refreshes: Vec::new(),
            usb_host: Vec::new(),
            gsensor: false,
            light: None,
            warmth: None,
            warmth_readable: true,
            light_report: Vec::new(),
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
    } else if matches!(
        refresh,
        Refresh::Fast | Refresh::Pointer | Refresh::Ink | Refresh::InkA2
    ) && new > 32
        && new < 224
    {
        // DU-like (v1.0.1 model): small Fast updates cannot reach a grey
        // exactly, so some of the old pixel stays (the pointer trails).
        (u16::from(new) * 70 + u16::from(old) * 30) / 100
    } else {
        (u16::from(new) * 92 + u16::from(old) * 8) / 100
    };
    ((v + 8) / 17 * 17).min(255) as u8
}

impl Device for DesktopDevice {
    fn has_gsensor(&self) -> bool {
        self.gsensor
    }

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

    fn light_step(&mut self) -> Option<u8> {
        self.light
    }

    fn set_light(&mut self, step: u8) -> io::Result<()> {
        if self.light.is_some() {
            self.light = Some(step);
        }
        Ok(())
    }

    fn light_report(&mut self) -> Vec<String> {
        std::mem::take(&mut self.light_report)
    }

    fn has_warmth(&self) -> bool {
        self.warmth.is_some()
    }

    fn warmth_step(&mut self) -> Option<u8> {
        self.warmth.filter(|_| self.warmth_readable)
    }

    fn set_warmth(&mut self, step: u8) -> io::Result<()> {
        if self.warmth.is_some() {
            self.warmth = Some(step.min(crate::light::WARMTH_MAX));
        }
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
