//! Touch gesture recognizer (v0.2.2): one finger, timestamps in ms.
//!
//! Input: `down` / `moved` / `up` with positions and a monotonic clock, plus
//! `tick` while a finger rests (so a hold fires without movement).
//! Output: [`Gesture`]s. A press that stays within [`Config::slop`] for
//! [`Config::hold_ms`] is a long-press; moving further first is a drag.
//! Drag updates are throttled to one per [`Config::drag_every_ms`] so the
//! e-ink panel is not flooded; the release always reports the final point.

/// Timing and distance thresholds for a [`Recognizer`].
///
/// The [`Default`] values are tuned for the Kobo Forma (300 dpi).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Time in ms a press must last to become a [`Gesture::LongPress`].
    pub hold_ms: u64,
    /// Movement tolerance in px before a press becomes a drag.
    pub slop: i32,
    /// Most time in ms between the releases of two taps for them to make a
    /// [`Gesture::DoubleTap`].
    pub double_tap_ms: u64,
    /// Most distance in px, on each axis, between the press points of two
    /// taps for them to make a [`Gesture::DoubleTap`].
    pub double_tap_slop: i32,
    /// Least time in ms between two reported [`Gesture::Drag`]s.
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

/// A gesture reported by a [`Recognizer`]. Positions are in px.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Finger down (the app can check for a selection handle here).
    Press {
        /// Where the finger went down.
        x: i32,
        /// Where the finger went down.
        y: i32,
    },
    /// A short press released within the slop, not close enough to the
    /// previous tap to be a double tap.
    Tap {
        /// Where the press started.
        x: i32,
        /// Where the press started.
        y: i32,
    },
    /// A second tap close in time and place to the previous one.
    DoubleTap {
        /// Where the second press started.
        x: i32,
        /// Where the second press started.
        y: i32,
    },
    /// A press held within the slop for [`Config::hold_ms`].
    LongPress {
        /// Where the press started.
        x: i32,
        /// Where the press started.
        y: i32,
    },
    /// Movement after a long-press (`held`) or beyond the slop.
    Drag {
        /// The latest finger position.
        x: i32,
        /// The latest finger position.
        y: i32,
        /// Whether the drag started from a long-press.
        held: bool,
    },
    /// Finger up after a drag or long-press.
    Release {
        /// Where the finger went up.
        x: i32,
        /// Where the finger went up.
        y: i32,
        /// Whether the press had become a long-press.
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

/// One-finger gesture recognizer: turns touch events into [`Gesture`]s.
///
/// Feed it [`down`](Self::down), [`moved`](Self::moved) and
/// [`up`](Self::up), and call [`tick`](Self::tick) while a finger rests.
/// Each call returns the gestures it produced, possibly none.
#[derive(Clone, Debug)]
pub struct Recognizer {
    /// The thresholds in use.
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
    /// Creates an idle recognizer with the given thresholds.
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

    /// Whether a finger is currently down.
    #[must_use]
    pub fn finger_down(&self) -> bool {
        self.state != State::Idle
    }

    /// Handles a finger going down at `(x, y)` at time `t` (ms).
    ///
    /// Starts a new press and returns a [`Gesture::Press`].
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

    /// Handles the finger moving to `(x, y)` at time `t` (ms).
    ///
    /// May return a [`Gesture::LongPress`] that came due, and a
    /// [`Gesture::Drag`] once the press has left the slop or was held.
    /// Drags after the first are throttled by [`Config::drag_every_ms`].
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

    /// Advances the clock to `t` (ms) without new input.
    ///
    /// Call periodically (e.g. every 50 ms) while a finger may be down.
    /// Returns a [`Gesture::LongPress`] when the hold time has passed, or a
    /// throttled [`Gesture::Drag`] that is now due.
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

    /// Handles the finger going up at `(x, y)` at time `t` (ms).
    ///
    /// Returns a [`Gesture::Tap`] or [`Gesture::DoubleTap`] for a short
    /// press, or a [`Gesture::Release`] after a long-press or drag. A hold
    /// that expired without a [`tick`](Self::tick) is reported as a
    /// [`Gesture::LongPress`] first.
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
