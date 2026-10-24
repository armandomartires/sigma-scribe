//! Live desktop window (minifb/X11): the 1440×1920 panel scaled to fit,
//! mouse = touch (press / hold / drag), PageUp/PageDown = page-turn buttons, the real keyboard =
//! a USB keyboard, Escape = power (save and quit). `SIGMA_EINK=1` adds an
//! e-ink look: greyscale only, a black flash on full refreshes, and faint
//! ghosting left behind by fast/partial refreshes.

use minifb::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use sigma_device::{Device, Event, HwKey, Mods, Phase, Refresh};
use sigma_render::{Canvas, Rect};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Chars(Rc<RefCell<VecDeque<char>>>);

impl InputCallback for Chars {
    fn add_char(&mut self, c: u32) {
        if let Some(ch) = char::from_u32(c) {
            if !ch.is_control() {
                self.0.borrow_mut().push_back(ch);
            }
        }
    }
}

pub struct WindowDevice {
    window: Window,
    panel: Canvas,
    scale: f32,
    out: Vec<u32>,
    out_w: usize,
    out_h: usize,
    eink: bool,
    chars: Rc<RefCell<VecDeque<char>>>,
    was_down: bool,
    last_pos: (i32, i32),
    epoch: Instant,
    pending: VecDeque<Event>,
}

impl WindowDevice {
    /// # Errors
    /// No display, or the window could not be created.
    pub fn open(width: u32, height: u32) -> io::Result<Self> {
        let max_h: f32 = std::env::var("SIGMA_SIM_HEIGHT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(960.0);
        let scale = (max_h / height as f32).min(1.0);
        let (out_w, out_h) = (
            (width as f32 * scale) as usize,
            (height as f32 * scale) as usize,
        );
        let mut window = Window::new(
            "Σscribe — Kobo Forma 1440×1920",
            out_w,
            out_h,
            WindowOptions::default(),
        )
        .map_err(|e| io::Error::other(e.to_string()))?;
        window.set_target_fps(30);
        let chars = Rc::new(RefCell::new(VecDeque::new()));
        window.set_input_callback(Box::new(Chars(chars.clone())));
        Ok(Self {
            window,
            panel: Canvas::new(width, height, 255),
            scale,
            out: vec![0; out_w * out_h],
            out_w,
            out_h,
            eink: std::env::var("SIGMA_EINK").is_ok_and(|v| v != "0"),
            chars,
            was_down: false,
            last_pos: (i32::MIN, i32::MIN),
            epoch: Instant::now(),
            pending: VecDeque::new(),
        })
    }

    fn blit(&mut self) {
        // Box-filter downscale of the panel into the window buffer.
        let (pw, ph) = (self.panel.width as usize, self.panel.height as usize);
        let step = (1.0 / self.scale).max(1.0);
        for oy in 0..self.out_h {
            let y0 = ((oy as f32) * step) as usize;
            let y1 = (((oy + 1) as f32) * step).ceil().min(ph as f32) as usize;
            for ox in 0..self.out_w {
                let x0 = ((ox as f32) * step) as usize;
                let x1 = (((ox + 1) as f32) * step).ceil().min(pw as f32) as usize;
                let (mut sum, mut n) = (0u32, 0u32);
                for y in y0..y1.max(y0 + 1) {
                    for x in x0..x1.max(x0 + 1) {
                        sum += u32::from(self.panel.pixels[y * pw + x]);
                        n += 1;
                    }
                }
                let v = sum / n.max(1);
                self.out[oy * self.out_w + ox] = (v << 16) | (v << 8) | v;
            }
        }
        let _ = self
            .window
            .update_with_buffer(&self.out, self.out_w, self.out_h);
    }

    fn poll(&mut self) {
        if !self.window.is_open() {
            self.pending.push_back(Event::Quit);
            return;
        }
        // Mouse = one finger: press, hold still for a long-press, drag.
        let down = self.window.get_mouse_down(MouseButton::Left);
        let t = self.epoch.elapsed().as_millis() as u64;
        if let Some((x, y)) = self.window.get_mouse_pos(MouseMode::Clamp) {
            let (x, y) = ((x / self.scale) as i32, (y / self.scale) as i32);
            let phase = match (self.was_down, down) {
                (false, true) => Some(Phase::Down),
                (true, false) => Some(Phase::Up),
                (true, true) if (x, y) != self.last_pos => Some(Phase::Move),
                _ => None,
            };
            if let Some(phase) = phase {
                self.last_pos = (x, y);
                self.pending.push_back(Event::Touch { phase, x, y, t });
            }
        }
        self.was_down = down;
        // Modifiers (v0.8): the same chords a USB keyboard sends.
        let w = &self.window;
        let mods = Mods {
            ctrl: w.is_key_down(Key::LeftCtrl) || w.is_key_down(Key::RightCtrl),
            shift: w.is_key_down(Key::LeftShift) || w.is_key_down(Key::RightShift),
            alt: w.is_key_down(Key::LeftAlt) || w.is_key_down(Key::RightAlt),
        };
        let chorded = mods.ctrl || mods.alt;
        for k in self.window.get_keys_pressed(KeyRepeat::Yes) {
            let letter = (k as u32).wrapping_sub(Key::A as u32);
            if chorded && letter < 26 {
                let c = char::from(b'a' + letter as u8);
                self.pending.push_back(Event::Chord {
                    key: HwKey::Char(c),
                    mods,
                });
                continue;
            }
            let nav = match k {
                Key::Left => Some(HwKey::Left),
                Key::Right => Some(HwKey::Right),
                Key::Up => Some(HwKey::Up),
                Key::Down => Some(HwKey::Down),
                Key::Home => Some(HwKey::Home),
                Key::End => Some(HwKey::End),
                _ => None,
            };
            if let Some(key) = nav {
                self.pending.push_back(if mods == Mods::default() {
                    Event::Key(key)
                } else {
                    Event::Chord { key, mods }
                });
                continue;
            }
            let e = match k {
                Key::PageDown => Event::PageTurn { forward: true },
                Key::PageUp => Event::PageTurn { forward: false },
                Key::Escape => Event::Power,
                Key::Backspace => Event::Key(HwKey::Backspace),
                Key::Delete => Event::Key(HwKey::Delete),
                Key::Enter | Key::NumPadEnter => Event::Key(HwKey::Return),
                Key::Tab => Event::Key(HwKey::Tab),
                Key::Left => Event::Key(HwKey::Left),
                Key::Right => Event::Key(HwKey::Right),
                Key::Up => Event::Key(HwKey::Up),
                Key::Down => Event::Key(HwKey::Down),
                _ => continue,
            };
            self.pending.push_back(e);
        }
        while let Some(c) = self.chars.borrow_mut().pop_front() {
            if chorded || c.is_control() {
                continue;
            }
            self.pending.push_back(Event::Key(HwKey::Char(c)));
        }
    }
}

impl Device for WindowDevice {
    fn size(&self) -> (u32, u32) {
        (self.panel.width, self.panel.height)
    }

    fn present(&mut self, canvas: &Canvas, region: Rect, refresh: Refresh) -> io::Result<()> {
        let (r, px) = canvas.crop(region);
        let w = self.panel.width as usize;
        if self.eink && refresh == Refresh::Full {
            self.panel.fill(r, 0);
            self.blit();
            std::thread::sleep(Duration::from_millis(120));
        }
        for row in 0..r.h as usize {
            for col in 0..r.w as usize {
                let i = (r.y as usize + row) * w + r.x as usize + col;
                let new = px[row * r.w as usize + col];
                self.panel.pixels[i] = if self.eink && refresh != Refresh::Full {
                    // ~8 % residue of the previous image.
                    ((u16::from(new) * 92 + u16::from(self.panel.pixels[i]) * 8) / 100) as u8
                } else {
                    new
                };
            }
        }
        self.blit();
        Ok(())
    }

    fn next_event(&mut self) -> Option<Event> {
        loop {
            if let Some(e) = self.pending.pop_front() {
                return Some(e);
            }
            self.poll();
            if self.pending.is_empty() {
                self.window.update();
                return Some(Event::Tick {
                    t: self.epoch.elapsed().as_millis() as u64,
                });
            }
        }
    }
}
