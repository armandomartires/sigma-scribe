//! The device layer. The app draws into a `Canvas` and hands it here with a
//! dirty rectangle and a refresh class; input comes back as `Event`s.

pub mod desktop;
pub mod evdev;
#[cfg(target_arch = "arm")]
pub mod fbink;

use sigma_render::{Canvas, Rect};
use std::io;

/// E-ink refresh class (brief §9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    /// Keystrokes and caret: smallest region, fastest waveform.
    Fast,
    /// Larger partial changes (keyboard toggled, scrolled page).
    Partial,
    /// Theme, orientation, opening a sheet, first paint: flashing full refresh.
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Tap {
        x: i32,
        y: i32,
    },
    PageTurn {
        forward: bool,
    },
    /// Power button: save and leave.
    Power,
    /// A physical keyboard key (desktop keyboard now, USB-OTG keyboard in S8).
    Key(HwKey),
    Quit,
}

pub trait Device {
    fn size(&self) -> (u32, u32);
    /// # Errors
    /// The panel could not be written.
    fn present(&mut self, canvas: &Canvas, region: Rect, refresh: Refresh) -> io::Result<()>;
    /// Blocks for the next input; `None` means no more input (emulator end).
    fn next_event(&mut self) -> Option<Event>;
}

/// Keys from a physical keyboard, already mapped to characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HwKey {
    Char(char),
    Backspace,
    Delete,
    Return,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Escape,
}
