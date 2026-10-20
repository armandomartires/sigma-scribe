//! Linux evdev decoding for the Kobo: touch (single finger, tap on lift) and
//! page-turn / power keys. Pure logic, tested on the desktop.

use crate::Event;

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_MT_POSITION_X: u16 = 0x35;
pub const ABS_MT_POSITION_Y: u16 = 0x36;
pub const ABS_MT_TRACKING_ID: u16 = 0x39;
pub const BTN_TOUCH: u16 = 0x14a;
pub const KEY_POWER: u16 = 116;
/// Kobo page-turn buttons (Libra 2 / Forma mapping: 193 back, 194 forward).
pub const KEY_PAGE_BACK: u16 = 193;
pub const KEY_PAGE_FWD: u16 = 194;

/// `struct input_event` size on 32-bit ARM (two 32-bit timeval fields).
pub const EVENT_SIZE_ARM32: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawEvent {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

/// Decodes one 16-byte ARM32 `input_event`.
#[must_use]
pub fn decode_arm32(b: &[u8; EVENT_SIZE_ARM32]) -> RawEvent {
    RawEvent {
        kind: u16::from_le_bytes([b[8], b[9]]),
        code: u16::from_le_bytes([b[10], b[11]]),
        value: i32::from_le_bytes([b[12], b[13], b[14], b[15]]),
    }
}

/// Raw touch → screen coordinates. Kobo touch panels report in the panel's
/// native (landscape) frame; Forma defaults follow KOReader's Kobo defaults
/// (swap XY, mirror X) and must be confirmed on the device (checklist §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform {
    pub swap_xy: bool,
    pub mirror_x: bool,
    pub mirror_y: bool,
    /// Raw axis maxima after swapping (from EVIOCGABS), and screen size.
    pub raw_max: (i32, i32),
    pub screen: (i32, i32),
}

impl Transform {
    /// Parses `SIGMA_TOUCH`, e.g. `swap,mirror_x` or `none`.
    #[must_use]
    pub fn from_spec(spec: &str, raw_max: (i32, i32), screen: (i32, i32)) -> Self {
        let has = |k: &str| spec.split(',').any(|p| p.trim() == k);
        Self {
            swap_xy: has("swap"),
            mirror_x: has("mirror_x"),
            mirror_y: has("mirror_y"),
            raw_max,
            screen,
        }
    }

    #[must_use]
    pub fn apply(&self, x: i32, y: i32) -> (i32, i32) {
        let (mut x, mut y) = if self.swap_xy { (y, x) } else { (x, y) };
        let (mx, my) = (self.raw_max.0.max(1), self.raw_max.1.max(1));
        if self.mirror_x {
            x = mx - x;
        }
        if self.mirror_y {
            y = my - y;
        }
        let sx = (i64::from(x) * i64::from(self.screen.0 - 1) / i64::from(mx)) as i32;
        let sy = (i64::from(y) * i64::from(self.screen.1 - 1) / i64::from(my)) as i32;
        (
            sx.clamp(0, self.screen.0 - 1),
            sy.clamp(0, self.screen.1 - 1),
        )
    }
}

/// Turns a stream of raw events into taps and keys.
#[derive(Clone, Debug)]
pub struct Decoder {
    pub transform: Transform,
    x: i32,
    y: i32,
    down: bool,
    was_down: bool,
}

impl Decoder {
    #[must_use]
    pub fn new(transform: Transform) -> Self {
        Self {
            transform,
            x: 0,
            y: 0,
            down: false,
            was_down: false,
        }
    }

    pub fn feed(&mut self, e: RawEvent) -> Option<Event> {
        match (e.kind, e.code) {
            (EV_KEY, KEY_PAGE_BACK) if e.value == 1 => Some(Event::PageTurn { forward: false }),
            (EV_KEY, KEY_PAGE_FWD) if e.value == 1 => Some(Event::PageTurn { forward: true }),
            (EV_KEY, KEY_POWER) if e.value == 1 => Some(Event::Power),
            (EV_KEY, BTN_TOUCH) => {
                self.down = e.value != 0;
                None
            }
            (EV_ABS, ABS_MT_TRACKING_ID) => {
                self.down = e.value >= 0;
                None
            }
            (EV_ABS, ABS_MT_POSITION_X | ABS_X) => {
                self.x = e.value;
                self.down = true;
                None
            }
            (EV_ABS, ABS_MT_POSITION_Y | ABS_Y) => {
                self.y = e.value;
                self.down = true;
                None
            }
            (EV_SYN, 0) => {
                let lifted = self.was_down && !self.down;
                self.was_down = self.down;
                lifted.then(|| {
                    let (x, y) = self.transform.apply(self.x, self.y);
                    Event::Tap { x, y }
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: u16, code: u16, value: i32) -> RawEvent {
        RawEvent { kind, code, value }
    }

    #[test]
    fn decodes_arm32_layout() {
        let mut b = [0u8; 16];
        b[8..10].copy_from_slice(&EV_KEY.to_le_bytes());
        b[10..12].copy_from_slice(&KEY_PAGE_FWD.to_le_bytes());
        b[12..16].copy_from_slice(&1i32.to_le_bytes());
        assert_eq!(decode_arm32(&b), ev(EV_KEY, KEY_PAGE_FWD, 1));
    }

    #[test]
    fn tap_on_lift_with_transform() {
        let t = Transform::from_spec("swap,mirror_x", (1439, 1919), (1440, 1920));
        let mut d = Decoder::new(t);
        let seq = [
            ev(EV_ABS, ABS_MT_TRACKING_ID, 7),
            ev(EV_ABS, ABS_MT_POSITION_X, 100), // raw x → screen y
            ev(EV_ABS, ABS_MT_POSITION_Y, 1339), // raw y → mirrored screen x
            ev(EV_SYN, 0, 0),
            ev(EV_ABS, ABS_MT_TRACKING_ID, -1),
        ];
        for e in seq {
            assert_eq!(d.feed(e), None);
        }
        assert_eq!(
            d.feed(ev(EV_SYN, 0, 0)),
            Some(Event::Tap { x: 100, y: 100 })
        );
    }

    #[test]
    fn keys() {
        let mut d = Decoder::new(Transform::from_spec("none", (1, 1), (1, 1)));
        assert_eq!(
            d.feed(ev(EV_KEY, KEY_PAGE_BACK, 1)),
            Some(Event::PageTurn { forward: false })
        );
        assert_eq!(d.feed(ev(EV_KEY, KEY_PAGE_BACK, 0)), None);
        assert_eq!(d.feed(ev(EV_KEY, KEY_POWER, 1)), Some(Event::Power));
    }
}
