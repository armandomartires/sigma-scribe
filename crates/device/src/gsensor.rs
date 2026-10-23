//! Accelerometer (g-sensor) for Options → Orientation → Auto (v1.2, S12).
//!
//! Probed on the Forma (fw 4.38, kernel 4.1.15, 2026-10-23): the mma8652 is
//! the input node named "FreescaleAccelerometer" and reports raw `EV_ABS`
//! X/Y/Z at about 20 Hz (16384 = 1 g), **not** the `MSC_RAW` codes KOReader
//! documents for other firmware. Held portrait with the page buttons on the
//! right, gravity reads about -x; each quarter turn clockwise moves it to
//! +y, +x and -y.

/// Reading the sensor on the Kobo.
pub const ENABLED: bool = true;

pub const EV_ABS: u16 = 0x03;
pub const ABS_X: u16 = 0;
pub const ABS_Y: u16 = 1;
pub const ABS_Z: u16 = 2;

/// In-plane gravity below this (0.3 g) is "lying flat": keep the rotation.
const MIN_IN_PLANE: i32 = 5000;

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
    /// A raw accelerometer sample. `None` when the device lies (nearly)
    /// flat or the in-plane direction is ambiguous (about 45 degrees
    /// between two holds), so the last rotation stays.
    #[must_use]
    pub fn from_accel(x: i32, y: i32) -> Option<Self> {
        let (ax, ay) = (x.abs(), y.abs());
        let (major, minor) = (ax.max(ay), ax.min(ay));
        if major < MIN_IN_PLANE || major < minor * 3 / 2 {
            return None;
        }
        Some(if ax >= ay {
            if x < 0 {
                Self::Upright
            } else {
                Self::UpsideDown
            }
        } else if y > 0 {
            Self::Clockwise
        } else {
            Self::CounterClockwise
        })
    }
}

/// The `/dev/input/eventN` handler of the accelerometer, from the text of
/// `/proc/bus/input/devices` (name contains "Accelerometer").
#[must_use]
pub fn find_node(devices: &str) -> Option<String> {
    for block in devices.split("\n\n") {
        let named = block
            .lines()
            .any(|l| l.starts_with("N: Name=") && l.to_ascii_lowercase().contains("accelerometer"));
        if !named {
            continue;
        }
        let handlers = block.lines().find_map(|l| l.strip_prefix("H: Handlers="))?;
        if let Some(ev) = handlers.split_whitespace().find(|h| h.starts_with("event")) {
            return Some(format!("/dev/input/{ev}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_holds_seen_on_the_forma() {
        // Means of the 2026-10-23 probe: portrait buttons right, then three
        // quarter turns clockwise (x, y only; z is ignored).
        assert_eq!(Tilt::from_accel(-11800, 0), Some(Tilt::Upright));
        assert_eq!(Tilt::from_accel(-7000, -3500), Some(Tilt::Upright));
        assert_eq!(Tilt::from_accel(-200, 14800), Some(Tilt::Clockwise));
        assert_eq!(Tilt::from_accel(13600, -600), Some(Tilt::UpsideDown));
        assert_eq!(Tilt::from_accel(-900, -14800), Some(Tilt::CounterClockwise));
    }

    #[test]
    fn flat_and_diagonal_keep_the_last_rotation() {
        assert_eq!(Tilt::from_accel(-300, 500), None);
        assert_eq!(Tilt::from_accel(0, 0), None);
        assert_eq!(Tilt::from_accel(-9000, 8000), None);
        assert_eq!(Tilt::from_accel(4000, 100), None);
    }

    #[test]
    fn finds_the_accelerometer_node_only() {
        let text = "I: Bus=0000\nN: Name=\"cyttsp5_mt\"\nH: Handlers=event1\nB: EV=f\n\n\
                    I: Bus=0018\nN: Name=\"FreescaleAccelerometer\"\nH: Handlers=event2 \nB: EV=9\n\n";
        assert_eq!(find_node(text).as_deref(), Some("/dev/input/event2"));
        assert_eq!(find_node("N: Name=\"x\"\nH: Handlers=event0\n"), None);
        assert_eq!(find_node(""), None);
    }
}
