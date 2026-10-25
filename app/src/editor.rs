//! Touch gestures in the editor, its scroll bar and its action bar.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Drag {
    /// Dragging a handle; the other end stays put.
    Handle { fixed: usize },
    /// Moving after a long-press on a word: extend word by word.
    Word { a: usize, b: usize },
    /// Holding the scroll bar (v1.13.1): the thumb follows the finger.
    Bar,
}

impl App {
    pub(crate) fn in_editor(&self) -> bool {
        self.files.is_none()
            && self.rename.is_none()
            && self.preview.is_none()
            && !self.options
            && self.graph.is_none()
            && self.plugin_screen.is_none()
            && self.help.is_none()
    }

    pub(crate) fn apply(&mut self, gestures: Vec<Gesture>) -> Option<Refresh> {
        let mut out: Option<Refresh> = None;
        for g in gestures {
            if let Some(r) = self.gesture(g) {
                out = Some(match out {
                    Some(o) if rank(o) >= rank(r) => o,
                    _ => r,
                });
            }
        }
        out
    }

    pub(crate) fn select_word_at(&mut self, x: i32, y: i32) -> Option<(usize, usize)> {
        let page = self.page_rect();
        let o = self.screen.offset_at_point(&self.buffer, page, x, y);
        match self.buffer.word_at(o) {
            Some((a, b)) => {
                self.buffer.select(a, b);
                Some((a, b))
            }
            None => {
                self.buffer.set_caret(o);
                None
            }
        }
    }

    pub(crate) fn gesture(&mut self, g: Gesture) -> Option<Refresh> {
        let page = self.page_rect();
        let on_page = |x: i32, y: i32| page.contains(x, y);
        match g {
            Gesture::Press { x, y } if self.on_plugin_area(x, y) => {
                self.drag = None;
                self.plugin_drag_start(x, y);
                None
            }
            Gesture::Drag { x, y, .. } if self.plugin_drag_active() => self.plugin_drag_move(x, y),
            Gesture::Release { x, y, .. } if self.plugin_dragging() => {
                self.plugin_drag_end(x, y).or(Some(Refresh::Partial))
            }
            Gesture::Press { x, y } if self.on_map(x, y) => {
                self.drag = None;
                self.map_drag = Some((x, y, false));
                None
            }
            Gesture::Drag { x, y, .. } if self.map_drag.is_some() => {
                let (x0, y0, _) = self.map_drag?;
                self.map_drag = Some((x, y, true));
                self.graph_pan(x - x0, y - y0)
            }
            Gesture::Release { x, y, .. } if self.map_drag.is_some_and(|d| d.2) => {
                let (x0, y0, _) = self.map_drag.take()?;
                self.graph_pan(x - x0, y - y0).or(Some(Refresh::Partial))
            }
            Gesture::Press { x, y } => {
                self.drag = None;
                self.map_drag = None;
                if self.on_scroll_bar(x, y) {
                    self.drag = Some(Drag::Bar);
                    return None;
                }
                if self.in_editor() && self.sel_ui.is_some() {
                    if let (Hit::Handle(i), Some((a, b))) =
                        (self.editor_hit(x, y), self.buffer.selection())
                    {
                        self.drag = Some(Drag::Handle {
                            fixed: if i == 0 { b } else { a },
                        });
                        self.sel_ui = Some(SelUi { bar: false });
                        return Some(Refresh::Fast);
                    }
                }
                None
            }
            Gesture::Tap { x, y } => {
                if let Some(d) = self.drag.take() {
                    // A tap on the scroll bar goes on below; a handle or
                    // word drag ends with its selection bar.
                    if d != Drag::Bar {
                        self.sel_ui = Some(SelUi { bar: true });
                        return Some(Refresh::Fast);
                    }
                }
                self.handle(Event::Tap { x, y })
            }
            Gesture::DoubleTap { x, y }
                if self.in_editor()
                    && on_page(x, y)
                    && !self.in_drawer(x, y)
                    && self.sel_ui.is_none() =>
            {
                self.select_word_at(x, y);
                self.sel_ui = Some(SelUi { bar: true });
                Some(Refresh::Fast)
            }
            Gesture::DoubleTap { x, y } => self.handle(Event::Tap { x, y }),
            Gesture::LongPress { .. } if self.drag.is_some() => None,
            Gesture::LongPress { x, y }
                if self.in_editor() && on_page(x, y) && !self.in_drawer(x, y) =>
            {
                self.drag = self.select_word_at(x, y).map(|(a, b)| Drag::Word { a, b });
                self.sel_ui = Some(SelUi { bar: true });
                Some(Refresh::Fast)
            }
            // A held key or row still counts as one tap.
            Gesture::LongPress { x, y } => self.handle(Event::Tap { x, y }),
            Gesture::Drag { y, .. } if self.drag == Some(Drag::Bar) => {
                let page = self.page_rect();
                self.screen.scroll_bar_to(&mut self.buffer, page, y);
                Some(Refresh::Fast)
            }
            Gesture::Drag { x, y, .. } => {
                let drag = self.drag?;
                let o = |app: &Self, yy: i32| app.screen.offset_at_point(&app.buffer, page, x, yy);
                match drag {
                    Drag::Handle { fixed } => {
                        // The finger is below the handle; aim at the text row.
                        let at = o(
                            self,
                            y - self.screen.handle_drop() + self.screen.line_px() / 2,
                        );
                        if at != fixed {
                            self.buffer.select(fixed, at);
                        }
                    }
                    Drag::Bar => return None,
                    Drag::Word { a, b } => {
                        let at = o(self, y);
                        let (s0, e0) = if at >= b {
                            (a, self.buffer.word_at(at).map_or(at, |w| w.1).max(b))
                        } else if at < a {
                            (self.buffer.word_at(at).map_or(at, |w| w.0), b)
                        } else {
                            (a, b)
                        };
                        if at < a {
                            self.buffer.select(e0, s0);
                        } else {
                            self.buffer.select(s0, e0);
                        }
                    }
                }
                self.sel_ui = Some(SelUi { bar: false });
                Some(Refresh::Fast)
            }
            Gesture::Release { y, .. } if self.drag == Some(Drag::Bar) => {
                self.drag = None;
                let page = self.page_rect();
                self.screen.scroll_bar_to(&mut self.buffer, page, y);
                Some(Refresh::Partial)
            }
            Gesture::Release { x, y, held } => {
                // Throttled drags: the lift position is always applied.
                if self.drag.is_some() {
                    self.gesture(Gesture::Drag { x, y, held });
                }
                if self.drag.take().is_some() || (held && self.sel_ui.is_some()) {
                    self.sel_ui = Some(SelUi { bar: true });
                    Some(Refresh::Fast)
                } else if held {
                    None
                } else {
                    // A slide that never held: a tap where the finger lifted
                    // (keeps fast typing as forgiving as v0.2.1).
                    self.handle(Event::Tap { x, y })
                }
            }
        }
    }

    /// Is (x, y) on the editor's scroll bar (v1.13.1)?
    pub(crate) fn on_scroll_bar(&self, x: i32, y: i32) -> bool {
        self.in_editor()
            && self.ink.is_none()
            && !self.options
            && !self.in_drawer(x, y)
            && self
                .screen
                .scroll_bar_zone(&self.buffer, self.page_rect())
                .is_some_and(|(z, _)| z.contains(x, y))
    }

    pub(crate) fn editor_hit(&self, x: i32, y: i32) -> Hit {
        let model = Model {
            file_name: self.current.name(),
            buffer: &self.buffer,
            keyboard: &self.keyboard,
            keys_on: self.keys_visible(),
            save: &self.autosave.state,
            theme: self.theme(),
            sel_ui: self.sel_ui,
            rename: None,
        };
        self.screen.hit(&model, x, y)
    }

    pub(crate) fn bar_action(&mut self, a: BarAction) -> Option<Refresh> {
        let sel = self.buffer.selected_text().map(str::to_owned);
        match a {
            BarAction::Cut => {
                if let Some(t) = sel {
                    self.keyboard.clipboard = t;
                    self.buffer.backspace();
                    self.save();
                }
                self.sel_ui = None;
            }
            BarAction::Copy => {
                if let Some(t) = sel {
                    self.keyboard.clipboard = t;
                }
                let end = self
                    .buffer
                    .selection()
                    .map_or(self.buffer.caret(), |(_, b)| b);
                self.buffer.set_caret(end);
                self.sel_ui = None;
            }
            BarAction::Paste => {
                if !self.keyboard.clipboard.is_empty() {
                    let t = self.keyboard.clipboard.clone();
                    self.buffer.insert(&t);
                    self.save();
                }
                self.sel_ui = None;
            }
            BarAction::SelectAll => {
                self.buffer.select_all();
                self.sel_ui = Some(SelUi { bar: true });
            }
        }
        Some(Refresh::Fast)
    }
}
