//! Accelerometer (g-sensor) for Options → Orientation → Auto (v1.2, S12).
//!
//! **Stubbed behind a safe default.** On the Forma (fw 4.38) the mma8x5x
//! reports `EV_MSC / MSC_RAW` events (KOReader `handleMiscEvNTX`):
//! 0x17 upright, 0x18 upside down, 0x19 / 0x1a the two landscapes;
//! 0x1b / 0x1c (face up / down) are ignored. Which landscape code is which
//! rotation, and which input node carries them, has **not been checked on
//! the device** (no device access during v1.2), so [`ENABLED`] is `false`:
//! the Kobo build reports no sensor and Auto keeps the last rotation.
//! Turning it on is a one-line change after a read-only device probe.

/// Reading the sensor on the Kobo. `false` until checked on the device.
pub const ENABLED: bool = false;

pub const EV_MSC: u16 = 0x04;
pub const MSC_RAW: u16 = 0x03;

/// Which way the device is held, as the sensor reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tilt {
    Upright,
    /// Turned a quarter clockwise: the picture is drawn landscape.
    Clockwise,
    UpsideDown,
    CounterClockwise,
}

impl Tilt {
    /// A raw `MSC_RAW` value; `None` for face up / down and unknown codes.
    /// The landscape pair is a guess pending the device check.
    #[must_use]
    pub fn from_msc_raw(v: i32) -> Option<Self> {
        match v {
            0x17 => Some(Self::Upright),
            0x18 => Some(Self::UpsideDown),
            0x19 => Some(Self::Clockwise),
            0x1a => Some(Self::CounterClockwise),
            _ => None,
        }
    }
}

/// The `/dev/input/eventN` handler of an input device that reports
/// `MSC_RAW`, from the text of `/proc/bus/input/devices`.
#[must_use]
pub fn find_node(devices: &str) -> Option<String> {
    find_nodes(devices).into_iter().next()
}

/// Every input node whose device reports `MSC_RAW` (gpio-keys can too, so
/// the probe logs all of them instead of trusting the first).
#[must_use]
pub fn find_nodes(devices: &str) -> Vec<String> {
    let mut out = Vec::new();
    for block in devices.split("\n\n") {
        let msc_raw = block.lines().any(|l| {
            l.strip_prefix("B: MSC=")
                .and_then(|m| u64::from_str_radix(m.trim(), 16).ok())
                .is_some_and(|bits| bits & (1 << MSC_RAW) != 0)
        });
        if !msc_raw {
            continue;
        }
        let Some(handlers) = block.lines().find_map(|l| l.strip_prefix("H: Handlers=")) else {
            continue;
        };
        if let Some(ev) = handlers.split_whitespace().find(|h| h.starts_with("event")) {
            out.push(format!("/dev/input/{ev}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_map_and_face_up_down_are_ignored() {
        assert_eq!(Tilt::from_msc_raw(0x17), Some(Tilt::Upright));
        assert_eq!(Tilt::from_msc_raw(0x18), Some(Tilt::UpsideDown));
        assert_eq!(Tilt::from_msc_raw(0x19), Some(Tilt::Clockwise));
        assert_eq!(Tilt::from_msc_raw(0x1a), Some(Tilt::CounterClockwise));
        assert_eq!(Tilt::from_msc_raw(0x1b), None);
        assert_eq!(Tilt::from_msc_raw(0x1c), None);
        assert_eq!(Tilt::from_msc_raw(0), None);
    }

    #[test]
    fn finds_the_msc_raw_node_only() {
        let text = "I: Bus=0018\nN: Name=\"cyttsp5_mt\"\nH: Handlers=event1\nB: EV=b\n\n\
                    I: Bus=0018\nN: Name=\"mma8x5x\"\nH: Handlers=event2\nB: EV=11\nB: MSC=8\n\n";
        assert_eq!(find_node(text).as_deref(), Some("/dev/input/event2"));
        assert_eq!(
            find_node("N: Name=\"x\"\nH: Handlers=event0\nB: MSC=10\n"),
            None
        );
        assert_eq!(find_node(""), None);
    }

    #[test]
    fn the_kobo_build_ships_with_the_sensor_off() {
        const { assert!(!ENABLED, "turn on only after the device check (S12)") };
    }
}
