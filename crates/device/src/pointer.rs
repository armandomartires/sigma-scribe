//! USB mouse / touchpad (v0.8.1): relative motion, buttons and wheel from
//! evdev, one batch per `SYN_REPORT`. Pure logic, tested on the desktop.

use crate::evdev::{RawEvent, EV_KEY, EV_REL, EV_SYN};
use crate::{Button, Event};

/// Linux `REL_X`: horizontal motion.
pub const REL_X: u16 = 0x00;
/// Linux `REL_Y`: vertical motion.
pub const REL_Y: u16 = 0x01;
/// Linux `REL_WHEEL`: scroll wheel.
pub const REL_WHEEL: u16 = 0x08;
/// Linux `BTN_LEFT`: left mouse button.
pub const BTN_LEFT: u16 = 0x110;
/// Linux `BTN_RIGHT`: right mouse button.
pub const BTN_RIGHT: u16 = 0x111;
/// Linux `BTN_MIDDLE`: middle mouse button.
pub const BTN_MIDDLE: u16 = 0x112;

/// Collects mouse motion, wheel and buttons until `SYN_REPORT`, then
/// emits them as events.
#[derive(Clone, Debug, Default)]
pub struct PointerDecoder {
    dx: i32,
    dy: i32,
    wheel: i32,
    buttons: Vec<(Button, bool)>,
}

impl PointerDecoder {
    /// Feeds one raw event (`now` = monotonic ms). Keys other than mouse
    /// buttons are left to the keymap.
    pub fn feed(&mut self, e: RawEvent, now: u64) -> Vec<Event> {
        match (e.kind, e.code) {
            (EV_REL, REL_X) => self.dx += e.value,
            (EV_REL, REL_Y) => self.dy += e.value,
            (EV_REL, REL_WHEEL) => self.wheel += e.value,
            (EV_KEY, BTN_LEFT | BTN_RIGHT | BTN_MIDDLE) if e.value != 2 => {
                let b = match e.code {
                    BTN_LEFT => Button::Left,
                    BTN_RIGHT => Button::Right,
                    _ => Button::Middle,
                };
                self.buttons.push((b, e.value != 0));
            }
            (EV_SYN, 0) => return self.flush(now),
            _ => {}
        }
        Vec::new()
    }

    fn flush(&mut self, t: u64) -> Vec<Event> {
        let mut out = Vec::new();
        // Motion first, so a click lands where the pointer ended up.
        if self.dx != 0 || self.dy != 0 {
            out.push(Event::PointerMove {
                dx: self.dx,
                dy: self.dy,
                t,
            });
        }
        for (button, down) in self.buttons.drain(..) {
            out.push(Event::PointerButton { button, down, t });
        }
        if self.wheel != 0 {
            out.push(Event::Wheel {
                notches: self.wheel,
            });
        }
        (self.dx, self.dy, self.wheel) = (0, 0, 0);
        out
    }
}

fn ev(kind: u16, code: u16, value: i32) -> RawEvent {
    RawEvent { kind, code, value }
}

/// Raw events for a relative move (emulator).
#[must_use]
pub fn raw_move(dx: i32, dy: i32) -> Vec<RawEvent> {
    vec![
        ev(EV_REL, REL_X, dx),
        ev(EV_REL, REL_Y, dy),
        ev(EV_SYN, 0, 0),
    ]
}

/// Raw events for a button change (emulator).
#[must_use]
pub fn raw_button(code: u16, down: bool) -> Vec<RawEvent> {
    vec![ev(EV_KEY, code, i32::from(down)), ev(EV_SYN, 0, 0)]
}

/// Raw events for wheel notches; positive = away from the user (up).
#[must_use]
pub fn raw_wheel(notches: i32) -> Vec<RawEvent> {
    vec![ev(EV_REL, REL_WHEEL, notches), ev(EV_SYN, 0, 0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batches_per_report() {
        let mut d = PointerDecoder::default();
        let mut out = Vec::new();
        for e in raw_move(3, -2)
            .into_iter()
            .chain([ev(EV_REL, REL_X, 4)])
            .chain(raw_button(BTN_LEFT, true))
            .chain(raw_wheel(-2))
            .chain(raw_button(BTN_RIGHT, false))
        {
            out.extend(d.feed(e, 7));
        }
        assert_eq!(
            out,
            vec![
                Event::PointerMove {
                    dx: 3,
                    dy: -2,
                    t: 7
                },
                Event::PointerMove { dx: 4, dy: 0, t: 7 },
                Event::PointerButton {
                    button: Button::Left,
                    down: true,
                    t: 7
                },
                Event::Wheel { notches: -2 },
                Event::PointerButton {
                    button: Button::Right,
                    down: false,
                    t: 7
                },
            ]
        );
        // Letters are not the pointer's business.
        assert!(d.feed(ev(EV_KEY, 30, 1), 8).is_empty());
        assert!(d.feed(ev(EV_SYN, 0, 0), 8).is_empty());
    }
}
