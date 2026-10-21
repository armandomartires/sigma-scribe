//! The device layer. The app draws into a `Canvas` and hands it here with a
//! dirty rectangle and a refresh class; input comes back as `Event`s.

pub mod desktop;
pub mod evdev;
#[cfg(target_arch = "arm")]
pub mod fbink;
pub mod otg;
pub mod pointer;
pub mod usbkbd;

use sigma_render::{Canvas, Rect};
use std::io;

/// E-ink refresh class (brief §9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    /// Keystrokes and caret: smallest region, fastest waveform.
    Fast,
    /// The mouse pointer (v1.0.1): one merged old+new region with the
    /// grey-accurate, non-flashing GL16 waveform (DU left trails on greys).
    Pointer,
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
    /// A physical key with Ctrl / Alt held, or Shift with a non-character
    /// key (v0.8, USB keyboard). Routed through the on-screen keyboard's
    /// shortcut code.
    Chord {
        key: HwKey,
        mods: Mods,
    },
    /// A USB keyboard appeared (`true`) or went away (`false`).
    Usb(bool),
    /// A USB mouse / touchpad appeared or went away (v0.8.1).
    PointerPlug(bool),
    /// Relative pointer motion in screen pixels (v0.8.1).
    PointerMove {
        dx: i32,
        dy: i32,
        t: u64,
    },
    PointerButton {
        button: Button,
        down: bool,
        t: u64,
    },
    /// Wheel / two-finger scroll; positive = away from the user (up).
    Wheel {
        notches: i32,
    },
    Quit,
    /// Raw finger contact with a monotonic timestamp (ms). The app turns
    /// these into taps / holds / drags with `sigma_core::gesture`.
    Touch {
        phase: Phase,
        x: i32,
        y: i32,
        t: u64,
    },
    /// Clock tick while idle (≈ every 50 ms when a finger may be down), so
    /// holds fire without movement.
    Tick {
        t: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Down,
    Move,
    Up,
}

pub trait Device {
    fn size(&self) -> (u32, u32);
    /// # Errors
    /// The panel could not be written.
    fn present(&mut self, canvas: &Canvas, region: Rect, refresh: Refresh) -> io::Result<()>;
    /// Blocks for the next input; `None` means no more input (emulator end).
    fn next_event(&mut self) -> Option<Event>;
    /// USB host mode for a keyboard (v0.8, Options → USB keyboard). Devices
    /// without an OTG port do nothing.
    ///
    /// # Errors
    /// The port could not be switched (the app carries on without it).
    fn usb_host(&mut self, _on: bool) -> io::Result<()> {
        Ok(())
    }
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
    Home,
    End,
    PageUp,
    PageDown,
}

/// Mouse buttons (v0.8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// Modifiers held with a physical key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}
