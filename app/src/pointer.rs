//! The USB mouse pointer and scroll wheel.

use super::*;

impl App {
    /// Redraw for the pointer, at most every `POINTER_THROTTLE_MS`.
    fn pointer_redraw(&mut self, t: u64) -> Option<Refresh> {
        if t.saturating_sub(self.pointer.last_draw) >= POINTER_THROTTLE_MS {
            self.pointer.last_draw = t;
            self.pointer.pending = false;
            Some(Refresh::Pointer)
        } else {
            self.pointer.pending = true;
            None
        }
    }

    /// Ticks: flush a throttled move, hide after `POINTER_HIDE_MS` idle.
    pub(crate) fn pointer_tick(&mut self, t: u64) -> Option<Refresh> {
        let p = &mut self.pointer;
        p.last_tick = p.last_tick.max(t);
        if !p.visible {
            return None;
        }
        if p.down.is_none() && t.saturating_sub(p.last_move) >= POINTER_HIDE_MS {
            p.visible = false;
            p.pending = false;
            p.cleanup = true;
            return Some(Refresh::Pointer);
        }
        if p.pending {
            return self.pointer_redraw(t);
        }
        None
    }

    pub(crate) fn pointer_move(&mut self, dx: i32, dy: i32, t: u64) -> Option<Refresh> {
        // A report racing an unplug is dropped.
        if !self.pointer.live {
            return None;
        }
        let (w, h) = (self.canvas.width as i32, self.canvas.height as i32);
        let p = &mut self.pointer;
        p.x = (p.x + dx).clamp(0, w - 1);
        p.y = (p.y + dy).clamp(0, h - 1);
        p.last_move = t.max(p.last_tick);
        let was = std::mem::replace(&mut p.visible, true);
        let (x, y) = (p.x, p.y);
        if let Some(d) = &mut self.pointer.down {
            if (x - d.x).abs() + (y - d.y).abs() > DOUBLE_CLICK_PX / 2 {
                d.dragged = true;
            }
            if let (Some(a), true) = (d.anchor, d.dragged) {
                let at = self
                    .screen
                    .offset_at_point(&self.buffer, self.page_rect(), x, y);
                if at == a {
                    self.buffer.set_caret(a);
                } else {
                    self.buffer.select(a, at);
                }
                self.sel_ui = Some(SelUi { bar: false });
            }
        }
        if was {
            self.pointer_redraw(t)
        } else {
            self.pointer.last_draw = t;
            Some(Refresh::Pointer)
        }
    }

    /// Left click = a tap at the pointer; drag on the note selects;
    /// double-click on the note selects the word. Other buttons: nothing.
    pub(crate) fn pointer_button(&mut self, button: Button, down: bool, t: u64) -> Option<Refresh> {
        if button != Button::Left || !self.pointer.live {
            return None;
        }
        let (x, y) = (self.pointer.x, self.pointer.y);
        self.pointer.visible = true;
        self.pointer.last_move = t;
        let on_note = self.in_editor()
            && self.rename.is_none()
            && self.page_rect().contains(x, y)
            && !self.in_drawer(x, y)
            && !self.in_find_bar(x, y);
        if down {
            let double = self.pointer.last_press.is_some_and(|(px, py, pt)| {
                t.saturating_sub(pt) <= DOUBLE_CLICK_MS
                    && (x - px).abs() <= DOUBLE_CLICK_PX
                    && (y - py).abs() <= DOUBLE_CLICK_PX
            });
            if double && on_note {
                self.pointer.last_press = None;
                self.pointer.down = None;
                self.select_word_at(x, y);
                self.sel_ui = Some(SelUi { bar: true });
                return Some(Refresh::Fast);
            }
            self.pointer.last_press = Some((x, y, t));
            let anchor = on_note.then(|| {
                self.screen
                    .offset_at_point(&self.buffer, self.page_rect(), x, y)
            });
            self.pointer.down = Some(MouseDown {
                x,
                y,
                anchor,
                dragged: false,
            });
            return None;
        }
        let d = self.pointer.down.take()?;
        if d.dragged && self.on_plugin_area(d.x, d.y) {
            let r = self.plugin_mouse_drag((d.x, d.y), (x, y));
            return merge(Some(Refresh::Fast), r);
        }
        if d.dragged && self.on_map(d.x, d.y) {
            let r = self.graph_pan(x - d.x, y - d.y);
            return merge(Some(Refresh::Fast), r);
        }
        if d.dragged && d.anchor.is_some() {
            self.sel_ui = self.buffer.selection().map(|_| SelUi { bar: true });
            self.pointer.last_draw = t;
            self.pointer.pending = false;
            return Some(Refresh::Fast);
        }
        let r = self.handle_inner(Event::Tap { x: d.x, y: d.y });
        merge(Some(Refresh::Fast), r)
    }

    /// Wheel: the note moves 3 rows per notch (the caret follows); preview
    /// and the scrolling sheets turn one page per event.
    pub(crate) fn wheel(&mut self, notches: i32) -> Option<Refresh> {
        if notches == 0 || self.options {
            return None;
        }
        // Reverse scroll (v1.0.1): Natural flips every notch, everywhere.
        let notches = if self.settings.natural_scroll {
            -notches
        } else {
            notches
        };
        let forward = notches < 0;
        if self.plugin_screen.is_some() {
            return self.plugin_wheel(notches);
        }
        if self.graph.as_ref().is_some_and(|g| g.map) {
            return self.graph_zoom(notches > 0);
        }
        if self.preview.is_some()
            || self.files.is_some()
            || self.graph.is_some()
            || self.help.is_some()
        {
            return self.handle_inner(Event::PageTurn { forward });
        }
        if !self.in_editor() {
            return None;
        }
        let page = self.page_rect();
        for _ in 0..(notches.unsigned_abs() * 3).min(15) {
            self.screen.caret_row(&mut self.buffer, page, forward);
        }
        self.sel_ui = None;
        Some(Refresh::Partial)
    }
}
