//! Touch gesture recognizer (v0.2.2): one finger, timestamps in ms.
//!
//! Input: `down` / `moved` / `up` with positions and a monotonic clock, plus
//! `tick` while a finger rests (so a hold fires without movement).
//! Output: [`Gesture`]s. A press that stays within [`Config::slop`] for
//! [`Config::hold_ms`] is a long-press; moving further first is a drag.
//! Drag updates are throttled to one per [`Config::drag_every_ms`] so the
//! e-ink panel is not flooded; the release always reports the final point.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub hold_ms: u64,
    /// Movement tolerance in px before a press becomes a drag.
    pub slop: i32,
    pub double_tap_ms: u64,
    pub double_tap_slop: i32,
    pub drag_every_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        // Forma: 300 dpi, so 30 px ≈ 2.5 mm of finger wobble.
        Self {
            hold_ms: 500,
            slop: 30,
            double_tap_ms: 350,
            double_tap_slop: 40,
            drag_every_ms: 150,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Finger down (the app can check for a selection handle here).
    Press {
        x: i32,
        y: i32,
    },
    Tap {
        x: i32,
        y: i32,
    },
    DoubleTap {
        x: i32,
        y: i32,
    },
    LongPress {
        x: i32,
        y: i32,
    },
    /// Movement after a long-press (`held`) or beyond the slop.
    Drag {
        x: i32,
        y: i32,
        held: bool,
    },
    /// Finger up after a drag or long-press.
    Release {
        x: i32,
        y: i32,
        held: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Pressed { x0: i32, y0: i32, t0: u64 },
    Held,
    Dragging { held: bool },
}

#[derive(Clone, Debug)]
pub struct Recognizer {
    pub cfg: Config,
    state: State,
    last: (i32, i32),
    last_drag_ms: u64,
    pending_drag: bool,
    last_tap: Option<(i32, i32, u64)>,
}

impl Default for Recognizer {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl Recognizer {
    #[must_use]
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            state: State::Idle,
            last: (0, 0),
            last_drag_ms: 0,
            pending_drag: false,
            last_tap: None,
        }
    }

    #[must_use]
    pub fn finger_down(&self) -> bool {
        self.state != State::Idle
    }

    pub fn down(&mut self, x: i32, y: i32, t: u64) -> Vec<Gesture> {
        self.state = State::Pressed {
            x0: x,
            y0: y,
            t0: t,
        };
        self.last = (x, y);
        self.pending_drag = false;
        vec![Gesture::Press { x, y }]
    }

    pub fn moved(&mut self, x: i32, y: i32, t: u64) -> Vec<Gesture> {
        self.last = (x, y);
        let mut out = self.tick(t);
        match self.state {
            State::Pressed { x0, y0, .. } => {
                if (x - x0).abs() > self.cfg.slop || (y - y0).abs() > self.cfg.slop {
                    self.state = State::Dragging { held: false };
                    out.extend(self.drag(t, true));
                }
            }
            State::Held => {
                self.state = State::Dragging { held: true };
                out.extend(self.drag(t, true));
            }
            State::Dragging { .. } => out.extend(self.drag(t, false)),
            State::Idle => {}
        }
        out
    }

    fn drag(&mut self, t: u64, force: bool) -> Option<Gesture> {
        let State::Dragging { held } = self.state else {
            return None;
        };
        if force || t.saturating_sub(self.last_drag_ms) >= self.cfg.drag_every_ms {
            self.last_drag_ms = t;
            self.pending_drag = false;
            Some(Gesture::Drag {
                x: self.last.0,
                y: self.last.1,
                held,
            })
        } else {
            self.pending_drag = true;
            None
        }
    }

    /// Call periodically (e.g. every 50 ms) while a finger may be down.
    pub fn tick(&mut self, t: u64) -> Vec<Gesture> {
        match self.state {
            State::Pressed { x0, y0, t0 } if t.saturating_sub(t0) >= self.cfg.hold_ms => {
                self.state = State::Held;
                self.last_tap = None;
                vec![Gesture::LongPress { x: x0, y: y0 }]
            }
            State::Dragging { .. }
                if self.pending_drag
                    && t.saturating_sub(self.last_drag_ms) >= self.cfg.drag_every_ms =>
            {
                self.drag(t, true).into_iter().collect()
            }
            _ => Vec::new(),
        }
    }

    pub fn up(&mut self, x: i32, y: i32, t: u64) -> Vec<Gesture> {
        self.last = (x, y);
        // A hold that expired without a tick still counts.
        let mut out = match self.state {
            State::Pressed { .. } => self.tick(t),
            _ => Vec::new(),
        };
        let state = std::mem::replace(&mut self.state, State::Idle);
        match state {
            State::Pressed { x0, y0, .. } => {
                let double = self.last_tap.is_some_and(|(px, py, pt)| {
                    t.saturating_sub(pt) <= self.cfg.double_tap_ms
                        && (px - x0).abs() <= self.cfg.double_tap_slop
                        && (py - y0).abs() <= self.cfg.double_tap_slop
                });
                if double {
                    self.last_tap = None;
                    out.push(Gesture::DoubleTap { x: x0, y: y0 });
                } else {
                    self.last_tap = Some((x0, y0, t));
                    out.push(Gesture::Tap { x: x0, y: y0 });
                }
            }
            State::Held => out.push(Gesture::Release { x, y, held: true }),
            State::Dragging { held } => out.push(Gesture::Release { x, y, held }),
            State::Idle => {}
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Gesture::*;

    #[test]
    fn quick_press_is_a_tap() {
        let mut r = Recognizer::default();
        assert_eq!(r.down(10, 10, 0), vec![Press { x: 10, y: 10 }]);
        assert_eq!(r.moved(15, 12, 50), vec![], "wobble inside the slop");
        assert_eq!(r.up(15, 12, 120), vec![Tap { x: 10, y: 10 }]);
        assert!(!r.finger_down());
    }

    #[test]
    fn hold_fires_on_tick_with_tolerance_then_release() {
        let mut r = Recognizer::default();
        r.down(100, 100, 1000);
        assert_eq!(r.tick(1400), vec![]);
        assert_eq!(
            r.moved(120, 90, 1450),
            vec![],
            "20 px wobble keeps the hold"
        );
        assert_eq!(r.tick(1500), vec![LongPress { x: 100, y: 100 }]);
        assert_eq!(r.tick(1600), vec![], "fires once");
        assert_eq!(
            r.up(120, 90, 1700),
            vec![Release {
                x: 120,
                y: 90,
                held: true
            }]
        );
    }

    #[test]
    fn hold_detected_on_late_up_without_ticks() {
        let mut r = Recognizer::default();
        r.down(5, 5, 0);
        assert_eq!(
            r.up(5, 5, 800),
            vec![
                LongPress { x: 5, y: 5 },
                Release {
                    x: 5,
                    y: 5,
                    held: true
                }
            ]
        );
    }

    #[test]
    fn moving_past_slop_before_hold_is_a_drag_not_a_hold() {
        let mut r = Recognizer::default();
        r.down(0, 0, 0);
        assert_eq!(
            r.moved(50, 0, 100),
            vec![Drag {
                x: 50,
                y: 0,
                held: false
            }]
        );
        assert_eq!(r.tick(900), vec![], "no long-press once dragging");
        assert_eq!(
            r.up(60, 0, 950),
            vec![Release {
                x: 60,
                y: 0,
                held: false
            }]
        );
    }

    #[test]
    fn drag_after_hold_is_throttled_and_flushed() {
        let mut r = Recognizer::default();
        r.down(0, 0, 0);
        r.tick(500);
        assert_eq!(
            r.moved(10, 0, 520),
            vec![Drag {
                x: 10,
                y: 0,
                held: true
            }],
            "first move always reported"
        );
        assert_eq!(r.moved(20, 0, 560), vec![]);
        assert_eq!(r.moved(30, 0, 600), vec![]);
        assert_eq!(r.tick(640), vec![]);
        assert_eq!(
            r.tick(680),
            vec![Drag {
                x: 30,
                y: 0,
                held: true
            }],
            "pending move flushed after 150 ms"
        );
        assert_eq!(r.moved(40, 0, 700), vec![]);
        assert_eq!(
            r.up(45, 0, 710),
            vec![Release {
                x: 45,
                y: 0,
                held: true
            }]
        );
        // 20 moves in 1 s produce at most ~7 redraws.
        let mut r = Recognizer::default();
        r.down(0, 0, 0);
        let mut n = 0;
        for i in 1..=20 {
            n += r.moved(40 + i, 0, i as u64 * 50).len();
        }
        assert!(n <= 8, "{n} drags");
    }

    #[test]
    fn double_tap_within_time_and_distance() {
        let mut r = Recognizer::default();
        r.down(10, 10, 0);
        r.up(10, 10, 80);
        r.down(20, 15, 200);
        assert_eq!(r.up(20, 15, 260), vec![DoubleTap { x: 20, y: 15 }]);
        r.down(20, 15, 300);
        assert_eq!(
            r.up(20, 15, 360),
            vec![Tap { x: 20, y: 15 }],
            "a third tap starts over"
        );
        r.down(20, 15, 1000);
        assert_eq!(r.up(20, 15, 1060), vec![Tap { x: 20, y: 15 }], "too slow");
        r.down(200, 15, 1100);
        assert_eq!(r.up(200, 15, 1150), vec![Tap { x: 200, y: 15 }], "too far");
    }
}
