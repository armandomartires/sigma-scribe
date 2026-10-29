//! The event loop: dispatching device events and the run loop.

use crate::{merge, App, RenameTarget};
use sigma_device::{Device, Event, HwKey, Mods, Phase, Refresh};
use sigma_render::editor::Model;
use sigma_render::Hit;

impl App {
    /// Applies one event; returns the refresh it needs (or `None`).
    pub fn handle(&mut self, e: Event) -> Option<Refresh> {
        self.sync_pane_plugins();
        self.sync_drawer_open();
        match e {
            Event::Tilt { tilt, t } => {
                self.now = self.now.max(t);
                self.pointer_fast = false;
                return self.tilt_event(tilt, t);
            }
            Event::Key(_) | Event::Chord { .. } => self.last_typed = Some(self.now),
            // Touching (on-screen keys included) also pauses Auto for 1 s.
            Event::Touch { t, .. } => {
                self.now = self.now.max(t);
                self.last_typed = Some(self.now);
            }
            Event::Tap { .. } => self.last_typed = Some(self.now),
            _ => {}
        }
        if let Event::Tick { t } = e {
            self.now = self.now.max(t);
            if let Some(r) = self.tilt_tick(t) {
                self.pointer_fast = false;
                let rest = self.handle_inner(e);
                return merge(Some(r), rest);
            }
        }
        match e {
            Event::Tick { t } => {
                let p = self.pointer_tick(t);
                let r = self.handle_inner(e);
                if r.is_some() {
                    self.pointer_fast = false;
                } else if p == Some(Refresh::Pointer) && self.pointer.down.is_none() {
                    self.pointer_fast = true;
                }
                return merge(p, r);
            }
            Event::PointerMove { .. } => {
                let r = self.handle_inner(e);
                if r == Some(Refresh::Pointer) && self.pointer.down.is_none() {
                    self.pointer_fast = true;
                } else if r.is_some() {
                    self.pointer_fast = false;
                }
                return r;
            }
            // Typing or touching hides the pointer (v0.8.1).
            _ => self.pointer_fast = false,
        }
        match e {
            Event::Key(_) | Event::Chord { .. } | Event::Touch { .. } | Event::Tap { .. }
                if self.pointer.visible =>
            {
                self.pointer.visible = false;
                self.pointer.pending = false;
                self.pointer.cleanup = true;
                let r = self.handle_inner(e);
                return merge(Some(Refresh::Fast), r);
            }
            _ => {}
        }
        self.handle_inner(e)
    }

    pub(crate) fn handle_inner(&mut self, e: Event) -> Option<Refresh> {
        let page = self.screen.layout(self.keys_visible(), &self.keyboard).page;
        match e {
            Event::Tilt { .. } => None,
            // The open light menu takes every tap and touch; a slider moves
            // while the finger is on it.
            Event::Tap { x, y } if self.screen.light_menu && self.drawer_allowed() => {
                self.light_menu_tap(x, y)
            }
            Event::Touch { phase, x, y, t } if self.screen.light_menu && self.drawer_allowed() => {
                if let Some(r) = self.light_menu_touch(phase, x, y) {
                    return r;
                }
                let g = match phase {
                    Phase::Down => self.gestures.down(x, y, t),
                    Phase::Move => self.gestures.moved(x, y, t),
                    Phase::Up => self.gestures.up(x, y, t),
                };
                self.apply(g)
            }
            Event::Key(HwKey::Escape) if self.screen.light_menu && self.drawer_allowed() => {
                self.screen.light_menu = false;
                self.light_touch = None;
                Some(Refresh::Partial)
            }
            // v1.16: the open drawer menu takes every tap and touch.
            Event::Tap { x, y } if self.screen.drawer_menu && self.drawer_allowed() => {
                self.drawer_menu_tap(x, y)
            }
            // v1.18: so does the Wi-Fi menu.
            Event::Tap { x, y } if self.screen.wifi_menu && self.drawer_allowed() => {
                self.wifi_menu_tap(x, y)
            }
            Event::Touch { phase, x, y, t } if self.any_menu_open() && self.drawer_allowed() => {
                let g = match phase {
                    Phase::Down => self.gestures.down(x, y, t),
                    Phase::Move => self.gestures.moved(x, y, t),
                    Phase::Up => self.gestures.up(x, y, t),
                };
                self.apply(g)
            }
            // v1.16: the closed drawer's tab (a mouse click or a script tap).
            Event::Tap { x, y } if self.on_drawer_tab(x, y) => self.reopen_drawer(),
            Event::Key(HwKey::Escape) if self.any_menu_open() && self.drawer_allowed() => {
                self.screen.drawer_menu = false;
                self.screen.wifi_menu = false;
                Some(Refresh::Partial)
            }
            // v1.18: the eye of the Wi-Fi password field.
            Event::Tap { x, y } if self.wifi_eye_at(x, y) => Some(self.wifi_toggle_show()),
            Event::Tap { x, y } if self.paged_bar().is_some_and(|(_, b)| b.zone.contains(x, y)) => {
                self.bar_tap(x, y).flatten()
            }
            Event::Tap { x, y } if self.ink.is_some() => {
                let l = self.panel_layout()?;
                if !l.panel.contains(x, y) {
                    return self.place_caret(x, y);
                }
                let b = self.screen.ink_hit(&l, x, y)?;
                self.ink_button(b)
            }
            Event::Touch { phase, x, y, t } => {
                if let Some(r) = self.ink_touch(phase, x, y, t) {
                    return r;
                }
                if let Some(r) = self.handle_touch(phase, x, y, t) {
                    return r;
                }
                if let Some(r) = self.bar_touch(phase, x, y) {
                    return r;
                }
                if self.options {
                    if let Some(r) = self.options_touch(phase, x, y) {
                        return r;
                    }
                }
                let g = match phase {
                    Phase::Down => self.gestures.down(x, y, t),
                    Phase::Move => self.gestures.moved(x, y, t),
                    Phase::Up => self.gestures.up(x, y, t),
                };
                self.apply(g)
            }
            Event::Tick { t } => {
                if let Some(r) = self.ink_tick(t) {
                    return Some(r);
                }
                // A plugin screen with more to do (loading, or building a
                // layout) takes its next step while the app is idle.
                if let Some(r) = self.plugin_step() {
                    return Some(r);
                }
                // Syncthing's news, and notes changed on disk (v1.16).
                if let Some(r) = self.sync_tick(t) {
                    return Some(r);
                }
                // The radio's news (v1.18).
                if let Some(r) = self.wifi_tick() {
                    return Some(r);
                }
                if let Some(items) = self.indexer.poll() {
                    let changed = items != self.disk_todos;
                    self.disk_todos = items;
                    if changed && self.drawer[1] && self.drawer_visible() {
                        return Some(Refresh::Partial);
                    }
                }
                if !self.gestures.finger_down() {
                    return None;
                }
                let g = self.gestures.tick(t);
                self.apply(g)
            }
            Event::Usb(on) => {
                self.usb_live = on;
                self.screen.usb_live = on || self.pointer.live;
                Some(Refresh::Partial)
            }
            Event::PointerPlug(on) => {
                self.pointer.live = on;
                if !on {
                    self.pointer.visible = false;
                    self.pointer.down = None;
                    self.pointer.pending = false;
                } else if self.pointer.x == 0 && self.pointer.y == 0 {
                    self.pointer.x = self.canvas.width as i32 / 2;
                    self.pointer.y = self.canvas.height as i32 / 2;
                }
                self.screen.usb_live = self.usb_live || on;
                Some(Refresh::Partial)
            }
            Event::PointerMove { dx, dy, t } => self.pointer_move(dx, dy, t),
            Event::PointerButton { button, down, t } => self.pointer_button(button, down, t),
            Event::Wheel { notches } => self.wheel(notches),
            Event::Chord { key, mods } => self.chord(key, mods),
            // Nav keys only a USB keyboard has: the on-screen key path.
            Event::Key(
                k @ (HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown | HwKey::Insert),
            ) => self.chord(k, Mods::default()),
            Event::Quit | Event::Power => {
                self.save();
                self.save_scratch();
                self.running = false;
                None
            }
            Event::Tap { x, y } if self.options => self.options_tap(x, y),
            Event::Key(HwKey::Escape) if self.options => {
                self.settings_focus = None;
                Some(self.settings_back())
            }
            Event::Key(k @ (HwKey::Left | HwKey::Right))
                if self.options && self.settings_focus.is_some() =>
            {
                let lab = self.settings_focus?;
                let d = if k == HwKey::Left { -1 } else { 1 };
                let c = self
                    .screen
                    .options_slider_step(&self.options_model(), lab, d)?;
                Some(self.change(c))
            }
            Event::PageTurn { forward } if self.options => {
                let n = self
                    .screen
                    .options_layout(&self.options_model())
                    .pages
                    .max(1);
                self.settings_sub = if forward {
                    (self.settings_sub + 1).min(n - 1)
                } else {
                    self.settings_sub.saturating_sub(1)
                };
                Some(Refresh::Full)
            }
            Event::Key(_) | Event::PageTurn { .. } if self.options => None,
            Event::Tap { x, y } if self.help.is_some() => self.help_tap(x, y),
            Event::PageTurn { forward } if self.help.is_some() => self.help_page(forward),
            Event::Key(HwKey::Escape) if self.help.is_some() => self.help_back(),
            Event::Key(_) if self.help.is_some() => None,
            Event::Tap { x, y } if self.plugin_screen.is_some() => self.plugin_tap(x, y),
            Event::PageTurn { forward } if self.plugin_screen.is_some() => {
                self.plugin_page(forward)
            }
            Event::Key(HwKey::Escape) if self.plugin_screen.is_some() => {
                self.close_graph();
                Some(Refresh::Full)
            }
            Event::Key(_) if self.plugin_screen.is_some() => None,
            Event::Tap { x, y } if self.graph.is_some() => self.graph_tap(x, y),
            Event::PageTurn { forward } if self.graph.is_some() => self.graph_page(forward),
            Event::Key(HwKey::Escape) if self.graph.is_some() => {
                self.graph = None;
                Some(Refresh::Full)
            }
            Event::Key(_) if self.graph.is_some() => None,
            e @ (Event::Tap { .. } | Event::PageTurn { .. } | Event::Key(_))
                if self.preview.is_some() && self.files.is_none() =>
            {
                self.preview_event(e)
            }
            Event::Key(k) if self.rename.is_some() => self.hw_into_field(k),
            Event::Tap { x, y } if self.files.is_some() => self.files_tap(x, y),
            Event::PageTurn { forward } if self.files.is_some() => {
                let rows = self.rows();
                let (first, visible) = self.files_model(&rows).map_or((0, 1), |m| {
                    let l = self.screen.files_layout(&m);
                    (l.first, l.visible.max(1))
                });
                let f = self.files.as_mut()?;
                let n = rows.len();
                f.first = if forward {
                    (first + visible).min(n.saturating_sub(1))
                } else {
                    first.saturating_sub(visible)
                };
                // The open row and the armed delete may have left the page,
                // and so may the Trash's selection: Restore only acts on
                // what is on screen.
                f.more = None;
                f.armed = None;
                f.selected = None;
                Some(Refresh::Partial)
            }
            Event::Key(HwKey::Escape) if self.files.is_some() => {
                self.files = None;
                Some(Refresh::Full)
            }
            Event::Key(_) if self.files.is_some() => None,
            Event::PageTurn { forward } => {
                self.screen.page_turn(&mut self.buffer, page, forward);
                Some(Refresh::Partial)
            }
            Event::Key(k) if self.find_typing() => self.find_hw(k),
            Event::Key(k) if self.scratch_focused() => self.scratch_hw(k),
            Event::Tap { x, y } if self.in_find_bar(x, y) => self.find_tap(x, y),
            Event::Tap { x, y } if self.in_drawer(x, y) => self.drawer_tap(x, y),
            Event::Key(k) => {
                // Backspace, Return and Esc act on a composition themselves.
                // Left and Right choose the phrase while Japanese is converting.
                let phrase = matches!(k, HwKey::Left | HwKey::Right)
                    && self.keyboard.composer.candidates().is_some();
                if phrase {
                    self.keyboard
                        .composer
                        .phrase_arrow(&mut self.buffer, k == HwKey::Right, false);
                    return Some(Refresh::Fast);
                }
                if !matches!(
                    k,
                    HwKey::Char(_) | HwKey::Backspace | HwKey::Return | HwKey::Escape
                ) {
                    let t = self.keyboard.composer.flush();
                    if !t.is_empty() {
                        self.buffer.insert(&t);
                    }
                }
                match k {
                    HwKey::Char(c) => {
                        // Overwrite is for plain typing, not for composing layouts.
                        let over = self.buffer.overwrite();
                        if self.settings.kb_layout.composes() {
                            self.buffer.set_overwrite(false);
                        }
                        self.keyboard.composer.type_char(
                            &mut self.buffer,
                            self.settings.kb_layout,
                            c,
                        );
                        self.buffer.set_overwrite(over);
                    }
                    HwKey::Backspace => self
                        .keyboard
                        .composer
                        .backspace(&mut self.buffer, self.settings.kb_layout),
                    HwKey::Delete => self.buffer.delete_forward(),
                    HwKey::Return => {
                        if !self.keyboard.composer.enter(&mut self.buffer) {
                            self.buffer.newline();
                        }
                    }
                    HwKey::Tab => self.buffer.insert_tab(self.settings.tab_kind()),
                    HwKey::Left | HwKey::Right => {
                        // Flipped in a right-to-left row (v1.8).
                        if (k == HwKey::Left) != self.caret_rtl() {
                            self.buffer.left();
                        } else {
                            self.buffer.right();
                        }
                    }
                    HwKey::Up => self.screen.caret_row(&mut self.buffer, page, false),
                    HwKey::Down => self.screen.caret_row(&mut self.buffer, page, true),
                    HwKey::Home | HwKey::End | HwKey::PageUp | HwKey::PageDown | HwKey::Insert => {
                        return None
                    }
                    HwKey::Escape => {
                        if self.keyboard.composer.escape(&mut self.buffer) {
                            self.save();
                            return Some(Refresh::Fast);
                        }
                        let had = self.sel_ui.take().is_some();
                        if !had && !self.history.is_empty() {
                            return self.go_back();
                        }
                        return had.then_some(Refresh::Fast);
                    }
                }
                self.sel_ui = None;
                self.save();
                Some(Refresh::Fast)
            }
            Event::Tap { x, y } => {
                let renaming = matches!(
                    &self.rename,
                    Some((RenameTarget::Current | RenameTarget::WifiPassword(_), _))
                );
                let model = Model {
                    file_name: self.current.name(),
                    buffer: &self.buffer,
                    keyboard: &self.keyboard,
                    keys_on: self.keys_visible(),
                    save: &self.autosave.state,
                    theme: self.theme(),
                    sel_ui: if self.rename.is_some() {
                        None
                    } else {
                        self.sel_ui
                    },
                    rename: if renaming {
                        self.rename.as_ref().map(|(_, b)| b)
                    } else {
                        None
                    },
                };
                if self.on_scroll_bar(x, y) {
                    let page = self.page_rect();
                    self.screen.scroll_bar_step(&mut self.buffer, page, y);
                    return Some(Refresh::Partial);
                }
                let hit = self.screen.hit(&model, x, y);
                if !matches!(hit, Hit::Bar(_) | Hit::Handle(_)) {
                    // Tapping elsewhere dismisses the selection chrome.
                    self.sel_ui = None;
                }
                match hit {
                    Hit::Back => self.go_back(),
                    Hit::Bar(a) => self.bar_action(a),
                    Hit::Handle(_) => None,
                    Hit::Key(k) if self.rename.is_some() => self.type_into_field(k),
                    Hit::Key(k) if self.find_typing() => self.find_key(k),
                    Hit::Key(k) if self.scratch_focused() => self.scratch_key(k),
                    Hit::Find => Some(if self.find.is_some() {
                        self.close_find();
                        Refresh::Full
                    } else {
                        self.open_find()
                    }),
                    Hit::Graph => {
                        self.open_graph();
                        Some(Refresh::Full)
                    }
                    Hit::DrawerMenu => Some(self.toggle_drawer_menu()),
                    Hit::Wifi => Some(self.toggle_wifi_menu()),
                    Hit::LightMenu => Some(self.toggle_light_menu()),
                    Hit::Candidate(i) => {
                        self.keyboard.composer.pick(&mut self.buffer, i);
                        self.save();
                        Some(Refresh::Partial)
                    }
                    Hit::Key(k) => {
                        let tab = self.settings.tab_kind();
                        self.keyboard.rtl = self.caret_rtl();
                        let a = self.keyboard.press(k, &mut self.buffer, tab);
                        self.after_key(a)
                    }
                    Hit::ToggleKeys => {
                        self.keys_on = !self.keys_on;
                        Some(Refresh::Partial)
                    }
                    Hit::Ink => {
                        self.open_ink();
                        Some(Refresh::Partial)
                    }
                    Hit::Caret(at) => {
                        self.scratch_focus = false;
                        if let Some(f) = &mut self.find {
                            f.focus = None;
                        }
                        self.buffer.set_caret(at);
                        Some(Refresh::Fast)
                    }
                    Hit::Files => {
                        self.open_files();
                        Some(Refresh::Full)
                    }
                    Hit::TogglePreview => {
                        self.enter_preview();
                        Some(Refresh::Full)
                    }
                    Hit::Options => {
                        self.open_options();
                        Some(Refresh::Full)
                    }
                    Hit::KbButton(b) => self.kb_button(b),
                    Hit::StartRename => {
                        self.start_rename(RenameTarget::Current);
                        Some(Refresh::Partial)
                    }
                    Hit::RenameOk => {
                        self.finish_rename(true);
                        Some(Refresh::Partial)
                    }
                    Hit::RenameCancel => {
                        self.finish_rename(false);
                        Some(Refresh::Partial)
                    }
                    Hit::Nothing => None,
                }
            }
        }
    }

    /// Main loop until power/quit or end of input.
    ///
    /// # Errors
    /// The device failed to present a frame.
    pub fn run(&mut self, dev: &mut dyn Device) -> std::io::Result<()> {
        self.set_gsensor(dev.has_gsensor());
        // The device opened before the log was on: say what its probe found.
        for line in dev.light_report() {
            sigma_core::info!("light", "{line}");
        }
        sigma_device::usbkbd::set_layout(self.settings.kb_layout);
        // The light before the first frame: the title bar's light button is
        // there only once the probe has found a light.
        self.sync_light(dev);
        self.sync_warmth(dev);
        self.render(dev, Refresh::Full)?;
        self.apply_sync();
        while self.running {
            self.sync_usb_host(dev);
            self.sync_light(dev);
            self.sync_warmth(dev);
            let Some(e) = dev.next_event() else { break };
            let t0 = std::time::Instant::now();
            let e = self.to_logical(e);
            let typing = self.is_key_tap(&e);
            if let Some(r) = self.handle(e) {
                self.render(dev, r)?;
                let us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
                if typing {
                    self.perf.samples.push(us);
                } else if r == Refresh::Pointer {
                    self.perf.pointer.push(us);
                }
            }
        }
        self.save();
        self.save_scratch();
        // After the notes are written: ask Syncthing to shut down (docs/dev/sync-design.md §3).
        self.stop_sync();
        if self.usb_host {
            let _ = dev.usb_host(false);
            self.usb_host = false;
        }
        if let Some((n, p50, p95, max)) = self.perf.summary() {
            sigma_core::info!(
                "perf",
                "{n} keystrokes, keystroke-to-refresh p50 {p50:.1} ms, p95 {p95:.1} ms, max {max:.1} ms"
            );
        }
        if let Some((n, p50, p95, max)) = self.perf.pointer_summary() {
            sigma_core::info!(
                "perf pointer",
                "{n} du updates, move-to-submit p50 {p50:.1} ms, p95 {p95:.1} ms, max {max:.1} ms"
            );
        }
        Ok(())
    }

    /// A key event for the perf sample: hardware keys, and taps (or
    /// lifts) on the on-screen keyboard.
    fn is_key_tap(&self, e: &Event) -> bool {
        let on_keys =
            |x: i32, y: i32| self.in_editor() && self.page_layout().keyboard.contains(x, y);
        match *e {
            Event::Key(_) => true,
            Event::Tap { x, y } => on_keys(x, y),
            Event::Touch {
                phase: Phase::Up,
                x,
                y,
                ..
            } => on_keys(x, y),
            _ => false,
        }
    }
}
