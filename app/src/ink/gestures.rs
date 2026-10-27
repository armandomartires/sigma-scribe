//! Touch on the ink panel: strokes, taps, the caret tap and free-area gestures.

use crate::App;
use sigma_device::{Event, Phase, Refresh};
use sigma_render::{Hit, Rect};

/// A stroke's points in panel pixels (v1.5.4 gestures).
fn stroke_pts(s: &sigma_core::ink::Stroke) -> Vec<(f32, f32)> {
    s.points.iter().map(|p| (p.x as f32, p.y as f32)).collect()
}

/// Bounding box `(x0, y0, x1, y1)` of points.
fn pts_box(p: &[(f32, f32)]) -> (f32, f32, f32, f32) {
    p.iter()
        .fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, &(x, y)| {
            (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y))
        })
}

impl App {
    /// Touches while the panel is open: the panel owns every touch
    /// (`Some`); outside it they do nothing.
    pub(crate) fn ink_touch(
        &mut self,
        phase: Phase,
        x: i32,
        y: i32,
        t: u64,
    ) -> Option<Option<Refresh>> {
        use sigma_core::ink::Point;
        use sigma_render::ink::{draw_segment, pen_px};
        let mode = self.ink.as_ref()?.mode;
        let l = self.panel_layout()?;
        // v1.5.4 caret tap: a tap on the note above the panel moves the
        // caret there (pending ink is read first); the panel stays open.
        if !l.panel.contains(x, y) && self.ink.as_ref()?.ink.live.is_none() {
            return Some(self.caret_tap(phase, x, y));
        }
        if mode != sigma_render::ink::InkMode::Draw {
            return Some(self.letters_touch(phase, x, y, t, &l));
        }
        let width = pen_px(self.settings.ink_pen);
        let refresh = self.ink_refresh();
        // Contact size when the panel reports it (v1.5.1); else None.
        let major = sigma_device::evdev::contact_major();
        let p = Point { x, y, t, major };
        let ink = self.ink.as_mut()?;
        let add = |r: Option<Rect>, n: Rect| Some(r.map_or(n, |o| o.union(&n)));
        match phase {
            Phase::Down => {
                if l.area.contains(x, y) {
                    ink.ink.down(p);
                    let r = draw_segment(&mut self.canvas, p, p, width, l.area);
                    ink.pending = add(ink.pending, r);
                    ink.dirty = add(ink.dirty, r);
                    ink.fast = true;
                    ink.last_up = None;
                    return Some(Some(refresh));
                }
                ink.press = self.screen.ink_hit(&l, x, y);
                Some(None)
            }
            Phase::Move => {
                if ink.ink.live.is_none() {
                    return Some(None);
                }
                // Clamp to the area: a stroke that leaves it keeps going
                // along the edge instead of ending.
                let q = Point {
                    x: x.clamp(l.area.x, l.area.x + l.area.w - 1),
                    y: y.clamp(l.area.y, l.area.y + l.area.h - 1),
                    t,
                    major,
                };
                let Some((a, b)) = ink.ink.moved(q) else {
                    return Some(None);
                };
                let r = draw_segment(&mut self.canvas, a, b, width, l.area);
                ink.pending = add(ink.pending, r);
                ink.dirty = add(ink.dirty, r);
                ink.fast = true;
                Some(Some(refresh))
            }
            Phase::Up => {
                if ink.ink.live.is_some() {
                    ink.ink.up();
                    ink.last_up = Some(t);
                    // v1.5.4: a scratch-out erases the strokes it crosses.
                    let size = l.area.h as f32;
                    let last = ink.ink.strokes.last().map(stroke_pts).unwrap_or_default();
                    if sigma_hwr::gesture::classify(&last, size)
                        == Some(sigma_hwr::gesture::Gesture::ScratchOut)
                    {
                        ink.ink.strokes.pop();
                        let bb = pts_box(&last);
                        ink.ink
                            .strokes
                            .retain(|s| !sigma_hwr::gesture::overlaps(pts_box(&stroke_pts(s)), bb));
                        ink.last_up = None;
                        return Some(Some(Refresh::Partial));
                    }
                    return Some(None);
                }
                let pressed = ink.press.take();
                let b = self.screen.ink_hit(&l, x, y);
                if pressed.is_none() || pressed != b {
                    return Some(None);
                }
                Some(self.ink_button(b?))
            }
        }
    }

    /// Letters / Train touches (v1.5.2): ink goes in one box at a time;
    /// starting in another box reads the pending one first.
    pub(super) fn letters_touch(
        &mut self,
        phase: Phase,
        x: i32,
        y: i32,
        t: u64,
        l: &sigma_render::ink::InkLayout,
    ) -> Option<Refresh> {
        use sigma_core::ink::Point;
        use sigma_render::ink::{draw_segment, pen_px};
        let width = pen_px(self.settings.ink_pen);
        let refresh = self.ink_refresh();
        let major = sigma_device::evdev::contact_major();
        let add = |r: Option<Rect>, n: Rect| Some(r.map_or(n, |o| o.union(&n)));
        let mut out = None;
        match phase {
            Phase::Down => {
                if let Some(i) = l.boxes.iter().position(|b| b.contains(x, y)) {
                    let other = self
                        .ink
                        .as_ref()
                        .and_then(|k| k.boxi)
                        .is_some_and(|b| b != i);
                    if other {
                        out = self.read_letter();
                    }
                    let ink = self.ink.as_mut()?;
                    let p = Point { x, y, t, major };
                    ink.ink.down(p);
                    ink.boxi = Some(i);
                    ink.last_up = None;
                    if out.is_none() {
                        let r = draw_segment(&mut self.canvas, p, p, width, l.boxes[i]);
                        ink.pending = add(ink.pending, r);
                        ink.dirty = add(ink.dirty, r);
                        ink.fast = true;
                        out = Some(refresh);
                    }
                    return out;
                }
                self.ink.as_mut()?.press = self.screen.ink_hit(l, x, y);
                None
            }
            Phase::Move => {
                let ink = self.ink.as_mut()?;
                let (Some(i), Some(_)) = (ink.boxi, ink.ink.live.as_ref()) else {
                    return None;
                };
                let b = l.boxes[i];
                // A report outside the box is not this stroke (clamping it
                // drew lines to the box corners); leave it out.
                if !b.contains(x, y) {
                    sigma_device::evdev::log_anomaly(format_args!(
                        "dropped move outside box ({x},{y}) box {},{} {}x{}",
                        b.x, b.y, b.w, b.h
                    ));
                    return None;
                }
                let q = Point { x, y, t, major };
                let (a, bb) = ink.ink.moved(q)?;
                let r = draw_segment(&mut self.canvas, a, bb, width, b);
                ink.pending = add(ink.pending, r);
                ink.dirty = add(ink.dirty, r);
                ink.fast = true;
                Some(refresh)
            }
            Phase::Up => {
                let ink = self.ink.as_mut()?;
                if ink.ink.live.is_some() {
                    ink.ink.up();
                    ink.last_up = Some(t);
                    if ink.mode == sigma_render::ink::InkMode::Free {
                        return self.free_gesture(l);
                    }
                    return None;
                }
                let pressed = ink.press.take();
                let b = self.screen.ink_hit(l, x, y);
                if pressed.is_none() || pressed != b {
                    return None;
                }
                self.ink_button(b?)
            }
        }
    }

    /// v1.5.4: a tap above the panel places the caret (Down remembers the
    /// spot; Up within 24 px of it acts).
    pub(super) fn caret_tap(&mut self, phase: Phase, x: i32, y: i32) -> Option<Refresh> {
        match phase {
            Phase::Down => {
                self.ink.as_mut()?.tap_at = Some((x, y));
                None
            }
            Phase::Move => None,
            Phase::Up => {
                let (dx, dy) = self.ink.as_mut()?.tap_at.take()?;
                if (dx - x).abs() > 24 || (dy - y).abs() > 24 {
                    return None;
                }
                self.place_caret(x, y)
            }
        }
    }

    /// Caret tap: pending ink is read, then the caret goes to the tapped
    /// spot of the note.
    pub(crate) fn place_caret(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let at = match self.editor_hit(x, y) {
            Hit::Caret(at) => at,
            // The title bar stays live while the panel is open: reading
            // the pending ink, the panel closes and the tap goes on to it.
            Hit::Back
            | Hit::Files
            | Hit::TogglePreview
            | Hit::Options
            | Hit::Find
            | Hit::Graph
            | Hit::Drawer(_) => {
                let _ = self.read_letter();
                self.ink = None;
                return self.handle_inner(Event::Tap { x, y });
            }
            _ => return None,
        };
        let _ = self.read_letter();
        self.buffer.set_caret(at);
        let ink = self.ink.as_mut()?;
        ink.replace_ok = false;
        ink.cands.clear();
        Some(Refresh::Partial)
    }

    /// v1.5.4 gestures in the free area, on the stroke that just lifted:
    /// scratch-out over pending ink erases what it crosses, over nothing
    /// deletes the last reading (or the word before the caret); a long
    /// flat flick left is Backspace (joins lines at a line start), right
    /// is Space.
    pub(super) fn free_gesture(&mut self, l: &sigma_render::ink::InkLayout) -> Option<Refresh> {
        use sigma_hwr::gesture::{classify, overlaps, Gesture};
        let ink = self.ink.as_mut()?;
        let last = stroke_pts(ink.ink.strokes.last()?);
        let g = classify(&last, l.area.h as f32)?;
        ink.ink.strokes.pop();
        let others = !ink.ink.strokes.is_empty();
        match g {
            Gesture::ScratchOut if others => {
                // The free area holds one line: everything in the scratch's
                // x-range goes (dots and descenders too).
                let bb = pts_box(&last);
                ink.ink.strokes.retain(|s| {
                    let b = pts_box(&stroke_pts(s));
                    !overlaps((b.0, 0.0, b.2, 0.0), (bb.0, 0.0, bb.2, 0.0))
                });
                if ink.ink.strokes.is_empty() {
                    ink.last_up = None;
                    ink.boxi = None;
                }
                Some(Refresh::Partial)
            }
            Gesture::ScratchOut => {
                ink.last_up = None;
                let n = if ink.replace_ok { ink.last_len } else { 0 };
                ink.replace_ok = false;
                ink.cands.clear();
                if n > 0 {
                    for _ in 0..n {
                        self.buffer.backspace();
                    }
                } else {
                    let c = self.buffer.caret();
                    let text = self.buffer.text();
                    let trimmed = text[..c].trim_end_matches([' ', '\t']).len();
                    let start = self.buffer.word_left(trimmed);
                    if start == c {
                        return Some(Refresh::Partial);
                    }
                    self.buffer.select(start, c);
                    self.buffer.backspace();
                }
                self.save();
                Some(Refresh::Partial)
            }
            Gesture::FlickLeft => {
                if !others {
                    ink.last_up = None;
                }
                self.letters_type("", true)
            }
            Gesture::FlickRight => {
                if !others {
                    ink.last_up = None;
                }
                self.letters_type(" ", false)
            }
        }
    }
}
