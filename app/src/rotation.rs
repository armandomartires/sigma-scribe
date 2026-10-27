//! Tilt sensor, rotation, and mapping between panel and logical coordinates.

use super::*;

impl App {
    #[must_use]
    pub fn is_landscape(&self) -> bool {
        self.settings.rotation.is_landscape()
    }

    /// The rotation the screen is drawn in (v1.2).
    #[must_use]
    pub fn rotation(&self) -> Rotation {
        self.settings.rotation
    }

    /// Auto orientation: a sensor reading becomes pending; it is applied by
    /// [`App::tilt_tick`] after it has been stable for `TILT_STABLE_MS`.
    pub(crate) fn tilt_event(
        &mut self,
        tilt: sigma_device::gsensor::Tilt,
        t: u64,
    ) -> Option<Refresh> {
        use sigma_device::gsensor::Tilt;
        if self.settings.orientation != Orientation::Auto || !self.gsensor {
            return None;
        }
        let r = match tilt {
            Tilt::Upright => Rotation::R0,
            // Checked on the Forma: a quarter turn clockwise needs R270,
            // the screen turns the other way to stay upright.
            Tilt::Clockwise => Rotation::R270,
            Tilt::UpsideDown => Rotation::R180,
            Tilt::CounterClockwise => Rotation::R90,
        };
        self.tilt = match self.tilt {
            _ if r == self.settings.rotation => None,
            Some((p, t0)) if p == r => Some((p, t0)),
            _ => Some((r, t)),
        };
        self.tilt_tick(t)
    }

    /// Applies a stable pending reading unless a finger is down or the
    /// user typed in the last `TILT_TYPING_PAUSE_MS` (S12: no turning
    /// mid-sentence). One full refresh.
    pub(crate) fn tilt_tick(&mut self, t: u64) -> Option<Refresh> {
        let (r, t0) = self.tilt?;
        if self.settings.orientation != Orientation::Auto {
            self.tilt = None;
            return None;
        }
        let typing = self
            .last_typed
            .is_some_and(|k| t.saturating_sub(k) < TILT_TYPING_PAUSE_MS);
        if t.saturating_sub(t0) < TILT_STABLE_MS || self.gestures.finger_down() || typing {
            return None;
        }
        self.tilt = None;
        Some(self.change(Change::Rotate(r)))
    }

    /// An accelerometer is present (from `Device::has_gsensor`).
    pub fn set_gsensor(&mut self, on: bool) {
        self.gsensor = on;
        self.orientation_label = self.orientation_status();
    }

    /// Options → Orientation hint: what Auto is doing (v1.2).
    #[must_use]
    pub fn orientation_status(&self) -> String {
        use sigma_render::i18n::tr;
        let r = tr(self.settings.rotation.name());
        match (self.settings.orientation, self.gsensor) {
            (Orientation::Auto, true) => format!("{} — {r}", tr("Auto")),
            (Orientation::Auto, false) => format!("{} — {r}", tr("Auto (no sensor)")),
            _ => r.into_owned(),
        }
    }

    /// Device coordinates → screen coordinates (landscape turns the
    /// picture 90° clockwise on the portrait panel).
    #[must_use]
    pub fn to_logical(&self, e: Event) -> Event {
        let (lw, lh) = (self.canvas.width as i32, self.canvas.height as i32);
        let rot = self.settings.rotation;
        let m = |x: i32, y: i32| match rot {
            Rotation::R0 => (x, y),
            Rotation::R90 => (y, lh - 1 - x),
            Rotation::R180 => (lw - 1 - x, lh - 1 - y),
            Rotation::R270 => (lw - 1 - y, x),
        };
        match e {
            Event::Tap { x, y } => {
                let (x, y) = m(x, y);
                Event::Tap { x, y }
            }
            Event::Touch { phase, x, y, t } => {
                let (x, y) = m(x, y);
                Event::Touch { phase, x, y, t }
            }
            // Upside down, the page buttons swap ends (v1.2).
            Event::PageTurn { forward } if rot.is_inverted() => {
                Event::PageTurn { forward: !forward }
            }
            e => e,
        }
    }

    /// Screen pixel (x, y) → panel pixel, for the rotation in force.
    pub(crate) fn to_panel(&self, x: i32, y: i32) -> (i32, i32) {
        let (lw, lh) = (self.canvas.width as i32, self.canvas.height as i32);
        match self.settings.rotation {
            Rotation::R0 => (x, y),
            Rotation::R90 => (lh - 1 - y, x),
            Rotation::R180 => (lw - 1 - x, lh - 1 - y),
            Rotation::R270 => (y, lw - 1 - x),
        }
    }

    /// What the panel shows, turned back to the screen's orientation
    /// (emulator screenshots).
    #[must_use]
    pub fn view(&self, panel: &Canvas) -> Canvas {
        if self.settings.rotation == Rotation::R0 {
            return panel.clone();
        }
        let (lw, lh) = (self.canvas.width as i32, self.canvas.height as i32);
        let pw = panel.width as usize;
        let mut out = Canvas::new(lw as u32, lh as u32, 255);
        for y in 0..lh {
            for x in 0..lw {
                let (px, py) = self.to_panel(x, y);
                out.pixels[(y * lw + x) as usize] = panel.pixels[py as usize * pw + px as usize];
            }
        }
        out
    }
}

/// Auto orientation: a reading must hold this long before it is applied.
pub const TILT_STABLE_MS: u64 = 600;
/// Auto orientation waits this long after typing or touching.
pub const TILT_TYPING_PAUSE_MS: u64 = 1000;
