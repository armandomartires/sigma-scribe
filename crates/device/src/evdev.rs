//! Linux evdev decoding for the Kobo: touch and page-turn / power keys.
//! Pure logic, tested on the desktop.
//!
//! Touch (v0.2.2): one finger, reported as `Touch { Down | Move | Up }` with
//! the caller's monotonic clock, one event per `SYN_REPORT`. Multi-touch
//! protocol B is followed on slot 0 only (other fingers are ignored). Lift
//! is recognised from any of: `ABS_MT_TRACKING_ID = -1`, `BTN_TOUCH = 0`,
//! `ABS_MT_PRESSURE = 0` or `ABS_MT_TOUCH_MAJOR = 0`, because Kobo models
//! differ in which one they send.

use crate::{Event, Phase};

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_MT_POSITION_X: u16 = 0x35;
pub const ABS_MT_POSITION_Y: u16 = 0x36;
pub const ABS_MT_TRACKING_ID: u16 = 0x39;
pub const ABS_MT_SLOT: u16 = 0x2f;
pub const ABS_MT_TOUCH_MAJOR: u16 = 0x30;
pub const ABS_MT_PRESSURE: u16 = 0x3a;
/// Moves smaller than this (screen px) are not reported.
pub const MOVE_EPS: i32 = 3;
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

/// Turns a stream of raw events into touches and keys.
#[derive(Clone, Debug)]
pub struct Decoder {
    pub transform: Transform,
    x: i32,
    y: i32,
    down: bool,
    was_down: bool,
    slot: i32,
    sent: (i32, i32),
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
            slot: 0,
            sent: (i32::MIN, i32::MIN),
        }
    }

    /// Feeds one raw event; `now` is a monotonic clock in ms.
    pub fn feed(&mut self, e: RawEvent, now: u64) -> Option<Event> {
        let primary = self.slot == 0;
        match (e.kind, e.code) {
            (EV_KEY, KEY_PAGE_BACK) if e.value == 1 => Some(Event::PageTurn { forward: false }),
            (EV_KEY, KEY_PAGE_FWD) if e.value == 1 => Some(Event::PageTurn { forward: true }),
            (EV_KEY, KEY_POWER) if e.value == 1 => Some(Event::Power),
            (EV_KEY, BTN_TOUCH) => {
                self.down = e.value != 0;
                None
            }
            (EV_ABS, ABS_MT_SLOT) => {
                self.slot = e.value;
                None
            }
            (EV_ABS, ABS_MT_TRACKING_ID) if primary => {
                self.down = e.value >= 0;
                None
            }
            (EV_ABS, ABS_MT_PRESSURE | ABS_MT_TOUCH_MAJOR) if primary => {
                if e.value == 0 {
                    self.down = false;
                }
                None
            }
            (EV_ABS, ABS_MT_POSITION_X | ABS_X) if primary => {
                self.x = e.value;
                self.down = true;
                None
            }
            (EV_ABS, ABS_MT_POSITION_Y | ABS_Y) if primary => {
                self.y = e.value;
                self.down = true;
                None
            }
            (EV_SYN, 0) => {
                let (x, y) = self.transform.apply(self.x, self.y);
                let phase = match (self.was_down, self.down) {
                    (false, true) => Some(Phase::Down),
                    (true, false) => Some(Phase::Up),
                    (true, true)
                        if (x - self.sent.0).abs() >= MOVE_EPS
                            || (y - self.sent.1).abs() >= MOVE_EPS =>
                    {
                        Some(Phase::Move)
                    }
                    _ => None,
                };
                self.was_down = self.down;
                phase.map(|phase| {
                    self.sent = (x, y);
                    Event::Touch {
                        phase,
                        x,
                        y,
                        t: now,
                    }
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
    fn down_and_up_with_transform() {
        let t = Transform::from_spec("swap,mirror_x", (1439, 1919), (1440, 1920));
        let mut d = Decoder::new(t);
        let seq = [
            ev(EV_ABS, ABS_MT_TRACKING_ID, 7),
            ev(EV_ABS, ABS_MT_POSITION_X, 100), // raw x → screen y
            ev(EV_ABS, ABS_MT_POSITION_Y, 1339), // raw y → mirrored screen x
            ev(EV_SYN, 0, 0),
            ev(EV_ABS, ABS_MT_TRACKING_ID, -1),
        ];
        let out: Vec<_> = seq.into_iter().filter_map(|e| d.feed(e, 5)).collect();
        assert_eq!(
            out,
            vec![Event::Touch {
                phase: Phase::Down,
                x: 100,
                y: 100,
                t: 5
            }]
        );
        assert_eq!(
            d.feed(ev(EV_SYN, 0, 0), 90),
            Some(Event::Touch {
                phase: Phase::Up,
                x: 100,
                y: 100,
                t: 90
            })
        );
    }

    #[test]
    fn keys() {
        let mut d = Decoder::new(Transform::from_spec("none", (1, 1), (1, 1)));
        assert_eq!(
            d.feed(ev(EV_KEY, KEY_PAGE_BACK, 1), 0),
            Some(Event::PageTurn { forward: false })
        );
        assert_eq!(d.feed(ev(EV_KEY, KEY_PAGE_BACK, 0), 0), None);
        assert_eq!(d.feed(ev(EV_KEY, KEY_POWER, 1), 0), Some(Event::Power));
    }

    fn none() -> Decoder {
        Decoder::new(Transform::from_spec("none", (1439, 1919), (1440, 1920)))
    }

    fn frame(d: &mut Decoder, evs: &[RawEvent], t: u64) -> Option<Event> {
        let mut out = None;
        for &e in evs {
            out = out.or(d.feed(e, t));
        }
        out.or_else(|| d.feed(ev(EV_SYN, 0, 0), t))
    }

    #[test]
    fn protocol_b_hold_and_drag_with_moves_and_timestamps() {
        let mut d = none();
        let touch = |phase, x, y, t| Some(Event::Touch { phase, x, y, t });
        assert_eq!(
            frame(
                &mut d,
                &[
                    ev(EV_ABS, ABS_MT_SLOT, 0),
                    ev(EV_ABS, ABS_MT_TRACKING_ID, 3),
                    ev(EV_ABS, ABS_MT_POSITION_X, 200),
                    ev(EV_ABS, ABS_MT_POSITION_Y, 300)
                ],
                0
            ),
            touch(Phase::Down, 200, 300, 0)
        );
        // Resting finger: frames with no movement report nothing.
        assert_eq!(frame(&mut d, &[ev(EV_ABS, ABS_MT_PRESSURE, 40)], 100), None);
        assert_eq!(
            frame(&mut d, &[ev(EV_ABS, ABS_MT_POSITION_X, 201)], 200),
            None,
            "sub-pixel jitter"
        );
        assert_eq!(
            frame(&mut d, &[ev(EV_ABS, ABS_MT_POSITION_X, 260)], 600),
            touch(Phase::Move, 260, 300, 600)
        );
        // A second finger on slot 1 is ignored.
        assert_eq!(
            frame(
                &mut d,
                &[
                    ev(EV_ABS, ABS_MT_SLOT, 1),
                    ev(EV_ABS, ABS_MT_TRACKING_ID, 4),
                    ev(EV_ABS, ABS_MT_POSITION_X, 900)
                ],
                650
            ),
            None
        );
        assert_eq!(
            frame(&mut d, &[ev(EV_ABS, ABS_MT_TRACKING_ID, -1)], 660),
            None,
            "slot 1 lift is not our lift"
        );
        assert_eq!(
            frame(
                &mut d,
                &[
                    ev(EV_ABS, ABS_MT_SLOT, 0),
                    ev(EV_ABS, ABS_MT_TRACKING_ID, -1)
                ],
                700
            ),
            touch(Phase::Up, 260, 300, 700)
        );
    }

    #[test]
    fn lift_by_pressure_or_btn_touch() {
        let mut d = none();
        frame(
            &mut d,
            &[
                ev(EV_ABS, ABS_MT_POSITION_X, 10),
                ev(EV_ABS, ABS_MT_POSITION_Y, 10),
            ],
            0,
        );
        assert!(matches!(
            frame(&mut d, &[ev(EV_ABS, ABS_MT_PRESSURE, 0)], 50),
            Some(Event::Touch {
                phase: Phase::Up,
                ..
            })
        ));
        frame(
            &mut d,
            &[
                ev(EV_KEY, BTN_TOUCH, 1),
                ev(EV_ABS, ABS_X, 10),
                ev(EV_ABS, ABS_Y, 10),
            ],
            100,
        );
        assert!(matches!(
            frame(&mut d, &[ev(EV_KEY, BTN_TOUCH, 0)], 150),
            Some(Event::Touch {
                phase: Phase::Up,
                ..
            })
        ));
    }
}
