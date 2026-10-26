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

/// Linux `EV_SYN`: synchronisation events (`SYN_REPORT` ends a frame).
pub const EV_SYN: u16 = 0x00;
/// Linux `EV_KEY`: key and button events.
pub const EV_KEY: u16 = 0x01;
/// Linux `EV_REL`: relative axis events (mice).
pub const EV_REL: u16 = 0x02;
/// Linux `EV_ABS`: absolute axis events (touch).
pub const EV_ABS: u16 = 0x03;
/// Linux `ABS_X`: single-touch X position.
pub const ABS_X: u16 = 0x00;
/// Linux `ABS_Y`: single-touch Y position.
pub const ABS_Y: u16 = 0x01;
/// Linux `ABS_MT_POSITION_X`: multi-touch X position.
pub const ABS_MT_POSITION_X: u16 = 0x35;
/// Linux `ABS_MT_POSITION_Y`: multi-touch Y position.
pub const ABS_MT_POSITION_Y: u16 = 0x36;
/// Linux `ABS_MT_TRACKING_ID`: contact id; -1 means the contact lifted.
pub const ABS_MT_TRACKING_ID: u16 = 0x39;
/// Linux `ABS_MT_SLOT`: selects the multi-touch slot that follows.
pub const ABS_MT_SLOT: u16 = 0x2f;
/// Linux `ABS_MT_TOUCH_MAJOR`: contact size.
pub const ABS_MT_TOUCH_MAJOR: u16 = 0x30;
/// Linux `ABS_MT_PRESSURE`: contact pressure.
pub const ABS_MT_PRESSURE: u16 = 0x3a;
/// Moves smaller than this (screen px) are not reported.
pub const MOVE_EPS: i32 = 3;
/// Linux `BTN_TOUCH`: finger on (1) or off (0) the panel.
pub const BTN_TOUCH: u16 = 0x14a;
/// Linux `KEY_POWER`: the power button.
pub const KEY_POWER: u16 = 116;
/// Kobo page-turn buttons (Libra 2 / Forma mapping: 193 back, 194 forward).
pub const KEY_PAGE_BACK: u16 = 193;
/// Kobo page-forward button (see [`KEY_PAGE_BACK`]).
pub const KEY_PAGE_FWD: u16 = 194;

/// `struct input_event` size on 32-bit ARM (two 32-bit timeval fields).
pub const EVENT_SIZE_ARM32: usize = 16;

/// One evdev `input_event` without its timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawEvent {
    /// Event type (`EV_*`).
    pub kind: u16,
    /// Event code within the type (`ABS_*`, `KEY_*`, ...).
    pub code: u16,
    /// Event value: position, key state (0 up, 1 down, 2 repeat), etc.
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
    /// Swap the raw X and Y axes first.
    pub swap_xy: bool,
    /// Mirror X (after swapping).
    pub mirror_x: bool,
    /// Mirror Y (after swapping).
    pub mirror_y: bool,
    /// Raw axis maxima after swapping (from EVIOCGABS), and screen size.
    pub raw_max: (i32, i32),
    /// Screen size in pixels (width, height).
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

    /// Maps a raw touch position to screen pixels, clamped to the screen.
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

/// Contact size (`ABS_MT_TOUCH_MAJOR`) of the primary finger at the last
/// touch Down / Move the decoder sent (v1.5.1), or -1 when the input does
/// not report it. One touchscreen per process, so one value.
static CONTACT_MAJOR: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

/// Odd touch input goes to the log (WARNING) so a stray-line report can be
/// traced; capped so a noisy panel cannot fill it.
pub fn log_anomaly(msg: std::fmt::Arguments) {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 400 {
        sigma_core::warn!("touch", "{msg}");
    }
}

/// Dropped corner reports, logged apart from the other anomalies so they
/// cannot use up their cap (and the other way round).
fn log_ghost(msg: std::fmt::Arguments) {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    match N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) {
        0..=99 => sigma_core::warn!("touch", "{msg}"),
        100 => sigma_core::warn!("touch", "more corner reports ignored (not logged)"),
        _ => {}
    }
}

/// A single report this far (px) from the last one is not a finger.
const JUMP_LOG_PX: i32 = 300;

/// The contact size at the latest touch event, if the panel reports one.
#[must_use]
pub fn contact_major() -> Option<u16> {
    u16::try_from(CONTACT_MAJOR.load(std::sync::atomic::Ordering::Relaxed)).ok()
}

/// Turns a stream of raw events into touches and keys.
#[derive(Clone, Debug)]
pub struct Decoder {
    /// Raw-to-screen coordinate mapping.
    pub transform: Transform,
    x: i32,
    y: i32,
    down: bool,
    was_down: bool,
    slot: i32,
    /// Raw position of the last frame that was not a corner ghost.
    acc: (i32, i32),
    /// Tracking id of the primary contact (the panel repeats it each frame).
    track: i32,
    sent: (i32, i32),
    /// Last `ABS_MT_TOUCH_MAJOR` of the primary contact this touch.
    major: Option<i32>,
    /// What the last touch event published to `contact_major`.
    sent_major: i32,
}

impl Decoder {
    /// A decoder with no finger down.
    #[must_use]
    pub fn new(transform: Transform) -> Self {
        Self {
            transform,
            x: 0,
            y: 0,
            down: false,
            was_down: false,
            slot: 0,
            acc: (0, 0),
            track: -1,
            sent: (i32::MIN, i32::MIN),
            major: None,
            sent_major: -1,
        }
    }

    /// A report on one of the four corner pixels (screen coordinates).
    fn is_corner(&self, x: i32, y: i32) -> bool {
        let (w, h) = self.transform.screen;
        let edge = |v: i32, max: i32| v <= 1 || v >= max - 2;
        edge(x, w) && edge(y, h)
    }

    /// The contact size this decoder published with its last touch event.
    #[must_use]
    pub fn contact(&self) -> Option<u16> {
        u16::try_from(self.sent_major).ok()
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
                if self.was_down && e.value != self.slot {
                    log_anomaly(format_args!(
                        "slot {} -> {} while down at {:?}",
                        self.slot, e.value, self.sent
                    ));
                }
                self.slot = e.value;
                None
            }
            (EV_ABS, ABS_MT_TRACKING_ID) if primary => {
                if self.was_down && e.value >= 0 && e.value != self.track {
                    log_anomaly(format_args!(
                        "tracking id {} -> {} while down",
                        self.track, e.value
                    ));
                }
                self.track = e.value;
                self.down = e.value >= 0;
                None
            }
            (EV_ABS, ABS_MT_POSITION_X | ABS_MT_POSITION_Y | ABS_X | ABS_Y)
                if !primary && self.was_down =>
            {
                // A second contact (slot != 0) while the first is down.
                log_anomaly(format_args!(
                    "slot {} reports {}={} while slot 0 is down",
                    self.slot,
                    if matches!(e.code, ABS_MT_POSITION_X | ABS_X) {
                        "x"
                    } else {
                        "y"
                    },
                    e.value
                ));
                None
            }
            (EV_ABS, ABS_MT_PRESSURE | ABS_MT_TOUCH_MAJOR) if primary => {
                if e.value == 0 {
                    self.down = false;
                } else if e.code == ABS_MT_TOUCH_MAJOR {
                    self.major = Some(e.value);
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
                let (mut x, mut y) = self.transform.apply(self.x, self.y);
                if self.is_corner(x, y) {
                    // The Forma's panel sends phantom reports at the four
                    // corners (seen in the device log: only corner to
                    // corner jumps). No finger is there: forget this frame's
                    // position, and never start a touch from it.
                    log_ghost(format_args!(
                        "corner report ({x},{y}) ignored, touch {}",
                        if self.was_down { "down" } else { "up" }
                    ));
                    (self.x, self.y) = self.acc;
                    if !self.was_down {
                        self.down = false;
                    }
                    (x, y) = self.transform.apply(self.x, self.y);
                } else {
                    self.acc = (self.x, self.y);
                }
                let phase = match (self.was_down, self.down) {
                    (false, true) => Some(Phase::Down),
                    (true, false) => Some(Phase::Up),
                    (true, true)
                        if (x - self.sent.0).abs() >= MOVE_EPS
                            || (y - self.sent.1).abs() >= MOVE_EPS =>
                    {
                        if (x - self.sent.0).abs().max((y - self.sent.1).abs()) > JUMP_LOG_PX {
                            log_anomaly(format_args!(
                                "jump {:?} -> {:?} on slot {}",
                                self.sent,
                                (x, y),
                                self.slot
                            ));
                        }
                        Some(Phase::Move)
                    }
                    _ => None,
                };
                self.was_down = self.down;
                if phase.is_some() {
                    let m = if self.down {
                        self.major.unwrap_or(-1)
                    } else {
                        -1
                    };
                    self.sent_major = m;
                    CONTACT_MAJOR.store(m, std::sync::atomic::Ordering::Relaxed);
                }
                if !self.down {
                    self.major = None;
                }
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
    fn contact_major_is_kept_per_touch_and_absent_without_it() {
        let mut d = Decoder::new(Transform::from_spec("none", (100, 100), (100, 100)));
        let f = |d: &mut Decoder, evs: &[RawEvent]| {
            for &e in evs {
                d.feed(e, 0);
            }
            d.feed(ev(EV_SYN, 0, 0), 0)
        };
        f(
            &mut d,
            &[
                ev(EV_ABS, ABS_MT_TOUCH_MAJOR, 9),
                ev(EV_ABS, ABS_MT_POSITION_X, 10),
            ],
        );
        assert_eq!(d.contact(), Some(9));
        f(&mut d, &[ev(EV_ABS, ABS_MT_POSITION_X, 90)]);
        assert_eq!(d.contact(), Some(9), "kept while the finger stays down");
        f(&mut d, &[ev(EV_ABS, ABS_MT_TRACKING_ID, -1)]);
        assert_eq!(d.contact(), None);
        // A panel that never reports it.
        f(&mut d, &[ev(EV_ABS, ABS_MT_POSITION_X, 10)]);
        assert_eq!(d.contact(), None);
    }

    /// A decoder on the Forma's 1440x1920 screen, identity transform.
    fn forma() -> Decoder {
        Decoder::new(Transform::from_spec("none", (1439, 1919), (1440, 1920)))
    }

    fn syn_frame(d: &mut Decoder, evs: &[RawEvent]) -> Option<Event> {
        for &e in evs {
            d.feed(e, 0);
        }
        d.feed(ev(EV_SYN, 0, 0), 0)
    }

    fn pos(x: i32, y: i32) -> [RawEvent; 2] {
        [
            ev(EV_ABS, ABS_MT_POSITION_X, x),
            ev(EV_ABS, ABS_MT_POSITION_Y, y),
        ]
    }

    #[test]
    fn corner_reports_never_start_a_touch() {
        let mut d = forma();
        for (x, y) in [(0, 0), (1439, 0), (0, 1919), (1439, 1919)] {
            assert_eq!(syn_frame(&mut d, &pos(x, y)), None, "ghost at {x},{y}");
        }
        // The next real touch is a normal Down.
        let e = syn_frame(&mut d, &pos(700, 900));
        assert!(matches!(
            e,
            Some(Event::Touch {
                phase: Phase::Down,
                x: 700,
                y: 900,
                ..
            })
        ));
    }

    #[test]
    fn corner_reports_inside_a_stroke_are_ignored_and_do_not_pollute_it() {
        let mut d = forma();
        let down = syn_frame(&mut d, &pos(700, 900));
        assert!(matches!(
            down,
            Some(Event::Touch {
                phase: Phase::Down,
                ..
            })
        ));
        // Ghost frames between real ones.
        assert_eq!(syn_frame(&mut d, &pos(0, 0)), None);
        assert_eq!(syn_frame(&mut d, &pos(1439, 0)), None);
        // The finger moves in x only: y must be the finger's, not the ghost's.
        let e = syn_frame(&mut d, &[ev(EV_ABS, ABS_MT_POSITION_X, 760)]);
        assert!(matches!(
            e,
            Some(Event::Touch {
                phase: Phase::Move,
                x: 760,
                y: 900,
                ..
            })
        ));
        // And it lifts where it was.
        let up = syn_frame(&mut d, &[ev(EV_ABS, ABS_MT_TRACKING_ID, -1)]);
        assert!(matches!(
            up,
            Some(Event::Touch {
                phase: Phase::Up,
                x: 760,
                y: 900,
                ..
            })
        ));
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
