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
    pub events: VecDeque<Event>,
    pub refreshes: Vec<(Rect, Refresh)>,
}

impl DesktopDevice {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            panel: Canvas::new(width, height, 255),
            events: VecDeque::new(),
            refreshes: Vec::new(),
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

impl Device for DesktopDevice {
    fn size(&self) -> (u32, u32) {
        (self.panel.width, self.panel.height)
    }

    fn present(&mut self, canvas: &Canvas, region: Rect, refresh: Refresh) -> io::Result<()> {
        let (r, px) = canvas.crop(region);
        let w = self.panel.width as usize;
        for row in 0..r.h as usize {
            let dst = (r.y as usize + row) * w + r.x as usize;
            self.panel.pixels[dst..dst + r.w as usize]
                .copy_from_slice(&px[row * r.w as usize..(row + 1) * r.w as usize]);
        }
        self.refreshes.push((r, refresh));
        Ok(())
    }

    fn next_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }
}
