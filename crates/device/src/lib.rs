//! The device layer. The app draws into a `Canvas` and hands it here with a
//! dirty rectangle and a refresh class; input comes back as `Event`s.
//!
//! Backends implement [`Device`]:
//! - [`desktop`]: an in-memory panel that writes PNGs, used by the
//!   emulator and every test; the live window lives in the app.
//! - `fbink` (only when `target_arch = "arm"`): the Kobo panel through
//!   FBInk, with evdev input. It is the one module allowed `unsafe`, for
//!   the FFI.

#![warn(missing_docs)]

pub mod desktop;
pub mod evdev;
#[cfg(target_arch = "arm")]
pub mod fbink;
pub mod gsensor;
pub mod light;
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
    /// The moving mouse pointer: one merged old+new region with the fast
    /// DU waveform, never waited for (v1.0.3, S10.3-D2; 1.0.1 used GL16,
    /// which lagged and held up typing).
    Pointer,
    /// When the pointer hides: one non-flashing GL16 pass over its path,
    /// clearing the DU trails (v1.0.3). Also the Graph selection pill
    /// (v1.0.4): one merged, non-flashing GL16 region.
    PointerCleanup,
    /// Live ink in the writing panel (v1.5.0): the new segments, pure
    /// black, DU, never waited for. The pause after a stroke sends one
    /// `PointerCleanup` (GL16) pass over the panel.
    Ink,
    /// Live ink with the 1-bit A2 waveform (Settings → Fast ink (A2), off
    /// by default; plan §12.9 decision 6).
    InkA2,
    /// Larger partial changes (keyboard toggled, scrolled page).
    Partial,
    /// Theme, orientation, opening a sheet, first paint: flashing full refresh.
    Full,
}

impl Refresh {
    /// The `sg_refresh` mode code: 0 fast (AUTO), 1 partial (GC16_FAST),
    /// 2 full (flashing GC16), 3 pointer / ink (DU), 4 pointer cleanup
    /// (GL16), 5 ink A2.
    #[must_use]
    pub fn fbink_mode(self) -> i32 {
        match self {
            Self::Fast => 0,
            Self::Partial => 1,
            Self::Full => 2,
            Self::Pointer => 3,
            Self::PointerCleanup => 4,
            Self::Ink => 3,
            Self::InkA2 => 5,
        }
    }

    /// Whether the device waits for this refresh to complete. Never: the
    /// input loop must not block on the panel (v1.0.3, S10.3-D2).
    #[must_use]
    pub fn waits_for_completion(self) -> bool {
        false
    }

    /// Short name for the device log.
    #[must_use]
    pub fn perf_name(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Partial => "partial",
            Self::Full => "full",
            Self::Pointer => "du",
            Self::PointerCleanup => "gl16-cleanup",
            Self::Ink => "ink-du",
            Self::InkA2 => "ink-a2",
        }
    }
}

#[cfg(test)]
mod refresh_tests {
    use super::Refresh;

    #[test]
    fn pointer_is_du_cleanup_is_gl16_and_nothing_waits() {
        assert_eq!(Refresh::Pointer.fbink_mode(), 3);
        assert_eq!(Refresh::PointerCleanup.fbink_mode(), 4);
        assert_eq!(Refresh::Ink.fbink_mode(), 3);
        assert_eq!(Refresh::InkA2.fbink_mode(), 5);
        for r in [
            Refresh::Fast,
            Refresh::Pointer,
            Refresh::PointerCleanup,
            Refresh::Ink,
            Refresh::InkA2,
            Refresh::Partial,
            Refresh::Full,
        ] {
            assert!(!r.waits_for_completion(), "{r:?}");
        }
        assert_eq!(Refresh::Pointer.perf_name(), "du");
    }
}

/// One input from the device, in screen coordinates (pixels, after
/// rotation) and with millisecond timestamps `t` from a monotonic clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)] // Fields `x`, `y`, `t` are described above; the rest by their variant.
pub enum Event {
    /// A tap at a point (emulator scripts and the desktop window; the Kobo
    /// sends [`Event::Touch`] instead).
    Tap {
        /// Horizontal position.
        x: i32,
        /// Vertical position.
        y: i32,
    },
    /// A page-turn button.
    PageTurn {
        /// `true` for the next page.
        forward: bool,
    },
    /// Power button: save and leave.
    Power,
    /// A physical keyboard key (desktop keyboard now, USB-OTG keyboard in S8).
    Key(HwKey),
    /// A physical key with Ctrl / Alt held, or Shift with a non-character
    /// key (v0.8, USB keyboard). Routed through the on-screen keyboard's
    /// shortcut code.
    Chord { key: HwKey, mods: Mods },
    /// A USB keyboard appeared (`true`) or went away (`false`).
    Usb(bool),
    /// A USB mouse / touchpad appeared or went away (v0.8.1).
    PointerPlug(bool),
    /// Relative pointer motion in screen pixels (v0.8.1).
    PointerMove { dx: i32, dy: i32, t: u64 },
    /// A mouse button went down or up.
    PointerButton { button: Button, down: bool, t: u64 },
    /// Wheel / two-finger scroll; positive = away from the user (up).
    Wheel {
        /// Wheel notches.
        notches: i32,
    },
    /// The desktop window was closed. The app treats it like
    /// [`Event::Power`]: save and leave.
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
    Tick { t: u64 },
    /// The accelerometer reports a new hold (v1.2, Auto orientation).
    Tilt { tilt: gsensor::Tilt, t: u64 },
}

/// The stage of a raw finger contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The finger touched the panel.
    Down,
    /// It moved while down.
    Move,
    /// It lifted.
    Up,
}

/// A display and its input: what the app needs from a Kobo or a stand-in.
///
/// The app owns the event loop: it calls [`Device::next_event`], updates
/// its state, draws into a canvas and calls [`Device::present`] with only
/// the region that changed and the cheapest [`Refresh`] that looks right.
pub trait Device {
    /// Panel size in pixels as `(width, height)`, in the current rotation.
    fn size(&self) -> (u32, u32);
    /// Shows `region` of `canvas` on the panel with the given refresh. It
    /// must not wait for the panel to finish (see
    /// [`Refresh::waits_for_completion`]).
    ///
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
    /// An accelerometer sends [`Event::Tilt`] (v1.2). Default: none, so
    /// Auto orientation keeps the last rotation.
    fn has_gsensor(&self) -> bool {
        false
    }
    /// What the start-up probe of the front light and the warmth mixer found,
    /// one line each, handed over once. The device opens before the log is
    /// switched on, so the app logs these itself when it starts. Default:
    /// nothing to say.
    fn light_report(&mut self) -> Vec<String> {
        Vec::new()
    }
    /// The front light's step, 0 (off) to 8 (full), when the device has
    /// one (v1.12). Default: no light, so Display shows no Backlight row.
    fn light_step(&mut self) -> Option<u8> {
        None
    }
    /// Sets the front light to a step, 0 to 8 (v1.12).
    ///
    /// # Errors
    /// The light could not be written (the app carries on without it).
    fn set_light(&mut self, _step: u8) -> io::Result<()> {
        Ok(())
    }
    /// Whether the front light has a warm-to-cool mixer (v1.17). Default:
    /// no, so Display shows no Warmth row.
    fn has_warmth(&self) -> bool {
        false
    }
    /// The mixer's step, 0 (cool) to 10 (warm), if it can be read. The
    /// stock reader never reads it back, so this may be `None` on a device
    /// that has a mixer; the app then keeps its own record (v1.17).
    fn warmth_step(&mut self) -> Option<u8> {
        None
    }
    /// Sets the mixer to a step, 0 (cool) to 10 (warm) (v1.17).
    ///
    /// # Errors
    /// The mixer could not be written (the app carries on without it).
    fn set_warmth(&mut self, _step: u8) -> io::Result<()> {
        Ok(())
    }
}

/// Keys from a physical keyboard, already mapped to characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)] // Named after the keys they are.
pub enum HwKey {
    /// A key that types a character, after layout and dead keys.
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
    Insert,
}

/// Mouse buttons (v0.8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)] // Named after the buttons they are.
pub enum Button {
    Left,
    Right,
    Middle,
}

/// Modifiers held with a physical key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    /// Ctrl is held.
    pub ctrl: bool,
    /// Shift is held.
    pub shift: bool,
    /// Alt is held. Right Alt counts only on the US layout; elsewhere it is
    /// AltGr and already folded into the character.
    pub alt: bool,
}
