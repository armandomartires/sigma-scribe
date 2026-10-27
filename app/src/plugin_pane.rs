//! The side drawer's panes (Contents, To-dos, Scratch) as WASM plugins.
//!
//! A pane plugin draws one pane of the drawer, header included, inside the
//! rectangle the host gives it, and answers taps with actions (check a
//! to-do, jump to a heading). The host keeps the data (headings, the to-do
//! index, the scratch pad and its keyboard, handwriting and saving), the
//! drawer's background and rules, and the title-bar icons. If a plugin
//! cannot load or fails while running, the built-in pane takes over for the
//! rest of the session.

use crate::plugin_screen::Stamp;
use crate::{merge, App};
use sigma_core::plugins::{self, PaneSlot};
use sigma_device::Refresh;
use sigma_plugin_api::{Action, Event as PEvent, Frame, HeadingRow, Note, Redraw, Scene, TodoRow};
use sigma_plugin_host::Plugin;
use sigma_render::drawer::Pane;

/// The slot of drawer pane number `i` (Contents, To-dos, Scratch).
fn slot(i: usize) -> PaneSlot {
    match i {
        0 => PaneSlot::Contents,
        1 => PaneSlot::Todos,
        _ => PaneSlot::Scratch,
    }
}

/// An enabled pane plugin and where its module is.
pub struct PaneSlotPlugin {
    pub manifest: plugins::Manifest,
    pub wasm: std::path::PathBuf,
}

/// A loaded pane plugin with the scene it was last told and its answer.
pub struct PanePlugin {
    plugin: Plugin,
    stamp: Stamp,
    /// The scene behind `frame`; `None` after a tap changed the frame on
    /// its own (the plugin then knows more than the scene does).
    sent: Option<Scene>,
    pub(crate) frame: Frame,
}

impl App {
    /// Shows the drawer icons and draws the built-in panes even with the
    /// pane plugins off (the reference the plugins are tested against, and
    /// the emulator's drawer). Off in the app.
    pub fn set_builtin_panes(&mut self, on: bool) {
        self.panes_builtin = on;
        self.sync_plugin_icons();
    }

    /// Reads which pane plugins are enabled and offers each pane that has
    /// one in the drawer menu (or all of them with the built-in panes
    /// wanted). A pane that lost its plugin closes; with none left, the
    /// drawer button and its menu go.
    pub(crate) fn sync_pane_slots(&mut self) {
        for i in 0..3 {
            let found = plugins::enabled_pane_in(&self.plugins_root, slot(i))
                .map(|(manifest, wasm)| PaneSlotPlugin { manifest, wasm });
            let on = self.panes_builtin || found.is_some();
            self.pane_slots[i] = found;
            if !on {
                self.drawer[i] = false;
                if let Some((open, _)) = &mut self.drawer_stash {
                    open[i] = false;
                }
                self.pane_plugins[i] = None;
                if i == 2 {
                    self.scratch_focus = false;
                }
            }
            match i {
                0 => self.screen.contents_on = on,
                1 => self.screen.todos_on = on,
                _ => self.screen.scratch_on = on,
            }
        }
        if !self.screen.panes_on() {
            self.screen.drawer_menu = false;
        }
        if self
            .drawer_stash
            .is_some_and(|(open, _)| !open.iter().any(|&o| o))
        {
            self.drawer_stash = None;
        }
    }

    /// Whether drawer pane `i` is drawn by its plugin right now.
    fn pane_uses_plugin(&self, i: usize) -> bool {
        !self.panes_builtin && !self.pane_failed[i] && self.pane_slots[i].is_some()
    }

    /// The frame of pane `i`'s plugin, when the pane is open and drawn by one.
    pub(crate) fn pane_frame(&self, i: usize) -> Option<&Frame> {
        if !self.drawer[i] || !self.pane_uses_plugin(i) {
            return None;
        }
        self.pane_plugins[i].as_ref().map(|p| &p.frame)
    }

    /// Brings every open pane plugin up to date with the page: loads its
    /// module, and tells it the scene again when the data or the pane's
    /// place changed.
    pub(crate) fn sync_pane_plugins(&mut self) {
        if !self.drawer_visible() {
            return;
        }
        let base = self.drawer_base();
        let rects = self
            .screen
            .drawer_pane_rects(&base, self.settings.drawer, self.drawer);
        for (pane, rect) in rects {
            let i = pane.index();
            if self.pane_uses_plugin(i) {
                self.sync_pane(i, rect);
            }
        }
    }

    fn sync_pane(&mut self, i: usize, rect: sigma_render::Rect) {
        let Some(slot) = self.pane_slots[i].as_ref() else {
            return;
        };
        let (manifest, wasm) = (slot.manifest.clone(), slot.wasm.clone());
        let stamp = match Stamp::of(&wasm) {
            Ok(s) => s,
            Err(e) => return self.pane_failed_over(i, &e.to_string()),
        };
        if self.pane_plugins[i]
            .as_ref()
            .is_none_or(|p| p.stamp != stamp)
        {
            let t0 = std::time::Instant::now();
            match self.load_module(&manifest.id, &wasm) {
                Ok(plugin) => {
                    sigma_core::info!(
                        "plugin",
                        "{}: loaded in {} ms",
                        manifest.id,
                        t0.elapsed().as_millis()
                    );
                    self.pane_plugins[i] = Some(PanePlugin {
                        plugin,
                        stamp,
                        sent: None,
                        frame: Frame::default(),
                    });
                }
                Err(e) => return self.pane_failed_over(i, &e),
            }
        }
        let scene = self.pane_scene(&manifest, rect);
        let Some(p) = self.pane_plugins[i].as_mut() else {
            return;
        };
        if p.sent.as_ref() == Some(&scene) {
            return;
        }
        match p.plugin.call(&PEvent::Open(scene.clone())) {
            Ok(frame) => {
                p.frame = frame;
                p.sent = Some(scene);
            }
            Err(e) => self.pane_failed_over(i, &e.to_string()),
        }
    }

    /// A pane plugin failed: drop it; the built-in pane draws instead.
    fn pane_failed_over(&mut self, i: usize, why: &str) {
        let id = self.pane_slots[i]
            .as_ref()
            .map_or_else(String::new, |s| s.manifest.id.clone());
        sigma_core::error!("plugin", "{id}: {why}; the built-in pane draws instead");
        self.pane_failed[i] = true;
        self.pane_plugins[i] = None;
    }

    /// What a pane plugin is told: the pane's rectangle, the open note's
    /// headings and the live to-do list.
    fn pane_scene(&self, manifest: &plugins::Manifest, rect: sigma_render::Rect) -> Scene {
        let chosen = plugins::settings_in(&self.plugins_root, manifest);
        let mut notes: Vec<Note> = Vec::new();
        let todos = self
            .todo_list()
            .iter()
            .map(|it| {
                let at = notes.iter().position(|n| n.name == it.note.name());
                let note = at.unwrap_or_else(|| {
                    notes.push(Note {
                        name: it.note.name().to_owned(),
                        current: it.note == self.current,
                        detail: String::new(),
                    });
                    notes.len() - 1
                });
                TodoRow {
                    note: note as u32,
                    line: it.todo.line as u32,
                    done: it.todo.done,
                    text: it.todo.text.clone(),
                }
            })
            .collect();
        let headings = self
            .drawer_headings()
            .iter()
            .map(|h| HeadingRow {
                level: h.level,
                line: h.line as u32,
                text: h.text.clone(),
            })
            .collect();
        Scene {
            width: rect.w,
            height: self.screen.height as i32,
            scale: self.screen.scale().0,
            ui_px: self.screen.ui_px(),
            top: rect.y,
            bottom: rect.y + rect.h,
            notes,
            edges: Vec::new(),
            links: 0,
            settings: manifest
                .settings
                .iter()
                .zip(chosen)
                .map(|(d, k)| (d.key.clone(), d.options[k].clone()))
                .collect(),
            left: rect.x,
            todos,
            headings,
        }
    }

    /// A tap inside a plugin's pane (outside the Scratch field, which the
    /// host handles): the plugin answers, and its actions run in order.
    pub(crate) fn pane_plugin_tap(&mut self, pane: Pane, x: i32, y: i32) -> Option<Refresh> {
        let i = pane.index();
        let p = self.pane_plugins[i].as_mut()?;
        let frame = match p.plugin.call(&PEvent::Tap { x, y }) {
            Ok(f) => f,
            Err(e) => {
                self.pane_failed_over(i, &e.to_string());
                return Some(Refresh::Partial);
            }
        };
        // The frame is the plugin's own change now; the scene still holds.
        p.frame = frame.clone();
        let mut r = match frame.redraw {
            Redraw::None => None,
            Redraw::Partial => Some(Refresh::Partial),
            Redraw::Clean => Some(Refresh::PointerCleanup),
            Redraw::Full => Some(Refresh::Full),
        };
        for a in &frame.actions {
            let done = match *a {
                Action::ToggleTodo(n) => self.drawer_todo_box(n as usize),
                Action::GotoTodo(n) => self.drawer_todo_open(n as usize),
                Action::GotoHeading(n) => {
                    let line = self.drawer_headings().get(n as usize).map(|h| h.line);
                    line.and_then(|l| self.drawer_heading(l))
                }
                // A pane stays where it is: screens' actions mean nothing here.
                Action::Close | Action::OpenNote(_) | Action::PreviewNote(_) => None,
            };
            r = merge(r, done);
        }
        r
    }

    /// The tappable regions a pane plugin named, with their labels, for
    /// emulator scripts and the touch-target audit.
    pub(crate) fn pane_hits(&self) -> Vec<(String, sigma_render::Rect)> {
        (0..3)
            .filter_map(|i| self.pane_frame(i))
            .flat_map(|f| f.hits.iter())
            .map(|h| {
                (
                    h.label.clone(),
                    sigma_render::Rect::new(h.rect.x, h.rect.y, h.rect.w, h.rect.h),
                )
            })
            .collect()
    }
}
