//! The Graph screen as a WASM plugin (`graph-map`), when one is installed
//! and enabled. The host keeps the title bar, the library data and the
//! note actions; the plugin draws the body and handles taps, drags and
//! pages. If the plugin cannot load or fails while running, the built-in
//! Graph takes over for the rest of the session.

use crate::{merge, App, GraphRow};
use sigma_core::plugins;
use sigma_core::NotePath;
use sigma_device::Refresh;
use sigma_plugin_api::{Action, Event as PEvent, Frame, Note, Phase as PPhase, Redraw, Scene};
use sigma_plugin_host::{DirCache, Env, Limits, Plugin};
use sigma_render::i18n::tr;
use sigma_render::options::PluginRow;
use sigma_render::plugin_screen::face;
use sigma_render::Fonts;

/// The plugin that replaces the Graph screen.
pub const GRAPH_PLUGIN: &str = "graph-map";

/// Plugin cache limits: a few layouts and slots, about a megabyte.
const CACHE_ENTRIES: usize = 16;
const CACHE_BYTES: u64 = 1 << 20;

struct ScreenEnv {
    fonts: Fonts,
}

impl Env for ScreenEnv {
    fn text_width(&self, f: sigma_plugin_api::Face, size: f32, s: &str) -> f32 {
        self.fonts.width(face(f), size, s)
    }
    fn ascent(&self, f: sigma_plugin_api::Face, size: f32) -> f32 {
        self.fonts.ascent(face(f), size)
    }
    fn tr(&self, s: &str) -> String {
        tr(s).into_owned()
    }
}

/// Identifies a plugin module on disk, so a loaded instance is reused only
/// while the file is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    path: std::path::PathBuf,
    modified: Option<std::time::SystemTime>,
    len: u64,
}

impl Stamp {
    fn of(path: &std::path::Path) -> std::io::Result<Self> {
        let m = std::fs::metadata(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            modified: m.modified().ok(),
            len: m.len(),
        })
    }
}

/// A loaded plugin kept between opens: starting wasmi and validating the
/// module costs more than answering an event.
pub struct IdlePlugin {
    plugin: Plugin,
    stamp: Stamp,
}

/// A plugin screen that is open.
pub struct PluginScreen {
    plugin: Plugin,
    stamp: Stamp,
    frame: Frame,
    /// The notes the plugin was given, so actions can name them.
    notes: Vec<NotePath>,
    title: String,
    /// Press point of a drag in progress and whether it moved.
    drag: Option<(i32, i32, bool)>,
}

impl App {
    /// Whether a plugin screen is open.
    #[must_use]
    pub fn plugin_screen_open(&self) -> bool {
        self.plugin_screen.is_some()
    }

    /// Where installed plugins live (tests point this at a temp folder).
    pub fn set_plugins_root(&mut self, root: impl Into<std::path::PathBuf>) {
        self.plugins_root = root.into();
        self.plugin_failed = false;
        self.plugin_idle = None;
    }

    /// Reads the installed plugins for Settings → Plugins.
    pub(crate) fn refresh_plugin_list(&mut self) {
        let app = plugins::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_default();
        self.plugin_list = plugins::list_in(&self.plugins_root)
            .into_iter()
            .map(|l| PluginRow {
                compatible: l.manifest.compatible_with(app),
                restart: l.manifest.kind != plugins::PluginKind::Screen,
                kind: l.manifest.kind.label(),
                version: l.manifest.version.to_string(),
                id: l.manifest.id,
                name: l.manifest.name,
                enabled: l.enabled,
            })
            .collect();
    }

    /// Settings → Plugins: turns plugin number `i` of the list on or off.
    /// Screen plugins take effect the next time their screen opens; data
    /// packs when the app next starts.
    pub(crate) fn set_plugin(&mut self, i: usize, on: bool) -> Refresh {
        // The switch means the plugin the page showed at that place, even if
        // the folder changed since.
        let id = self.plugin_list.get(i).map(|p| p.id.clone());
        if let Some(id) = &id {
            match plugins::set_enabled_in(&self.plugins_root, id, on) {
                Ok(()) => {
                    if !on {
                        // A switched-off plugin need not stay in memory.
                        self.plugin_idle = None;
                    }
                    // A plugin that failed earlier gets another chance.
                    self.plugin_failed = false;
                }
                Err(e) => eprintln!("sigma-scribe: plugin {id}: {e}"),
            }
        }
        self.refresh_plugin_list();
        Refresh::Full
    }

    /// Closes the Graph screen, built-in or plugin.
    pub(crate) fn close_graph(&mut self) {
        self.graph = None;
        self.map_drag = None;
        // Keep a healthy plugin loaded for the next open.
        if let Some(ps) = self.plugin_screen.take() {
            if !ps.plugin.is_poisoned() {
                self.plugin_idle = Some(IdlePlugin {
                    plugin: ps.plugin,
                    stamp: ps.stamp,
                });
            }
        }
    }

    /// Tries to open the Graph as a plugin screen. False when no usable
    /// plugin is enabled, so the caller opens the built-in one.
    pub(crate) fn open_plugin_graph(
        &mut self,
        notes: &[NotePath],
        rows: &[GraphRow],
        links: &[(usize, usize)],
        link_count: usize,
    ) -> bool {
        if self.plugin_failed {
            return false;
        }
        let Some((manifest, wasm)) = plugins::enabled_screen_in(&self.plugins_root, GRAPH_PLUGIN)
        else {
            return false;
        };
        match self.load_plugin(&manifest.id, &wasm, rows, links, link_count) {
            Ok((plugin, frame, stamp)) => {
                self.plugin_screen = Some(PluginScreen {
                    plugin,
                    stamp,
                    frame,
                    notes: notes.to_vec(),
                    title: manifest.name,
                    drag: None,
                });
                true
            }
            Err(e) => {
                eprintln!("sigma-scribe: plugin {GRAPH_PLUGIN}: {e}");
                self.plugin_failed = true;
                false
            }
        }
    }

    fn load_plugin(
        &mut self,
        id: &str,
        wasm: &std::path::Path,
        rows: &[GraphRow],
        links: &[(usize, usize)],
        link_count: usize,
    ) -> Result<(Plugin, Frame, Stamp), String> {
        let stamp = Stamp::of(wasm).map_err(|e| e.to_string())?;
        let idle = self.plugin_idle.take().filter(|i| i.stamp == stamp);
        let mut plugin = match idle {
            Some(i) => i.plugin,
            None => {
                let bytes = std::fs::read(wasm).map_err(|e| e.to_string())?;
                let cache = DirCache::new(
                    self.plugins_root.join(id).join(".cache"),
                    CACHE_ENTRIES,
                    CACHE_BYTES,
                );
                let env = Box::new(ScreenEnv {
                    fonts: Fonts::new(),
                });
                Plugin::load(&bytes, env, cache, Limits::default()).map_err(|e| e.to_string())?
            }
        };
        let scene = Scene {
            width: self.screen.width as i32,
            height: self.screen.height as i32,
            scale: self.screen.scale().0,
            ui_px: self.screen.ui_px(),
            top: self.screen.plugin_screen_top(&self.keyboard),
            bottom: self.screen.height as i32,
            notes: rows
                .iter()
                .map(|r| Note {
                    name: r.name.clone(),
                    current: r.current,
                    detail: r.detail.clone(),
                })
                .collect(),
            edges: links.iter().map(|&(a, b)| (a as u32, b as u32)).collect(),
            links: link_count as u32,
        };
        let frame = plugin
            .call(&PEvent::Open(scene))
            .map_err(|e| e.to_string())?;
        Ok((plugin, frame, stamp))
    }

    /// Sends an event to the open plugin and applies its answer. A failing
    /// plugin is dropped and the built-in Graph opens instead.
    fn plugin_event(&mut self, ev: &PEvent) -> Option<Refresh> {
        let ps = self.plugin_screen.as_mut()?;
        match ps.plugin.call(ev) {
            Ok(frame) => {
                ps.frame = frame.clone();
                self.plugin_apply(&frame)
            }
            Err(e) => {
                eprintln!("sigma-scribe: plugin {GRAPH_PLUGIN}: {e}");
                self.plugin_failed = true;
                self.plugin_screen = None;
                self.open_graph();
                Some(Refresh::Full)
            }
        }
    }

    fn plugin_apply(&mut self, frame: &Frame) -> Option<Refresh> {
        // Every action leaves the screen, so the first one decides.
        if let Some(a) = frame.actions.first() {
            let note = |app: &Self, n: u32| {
                app.plugin_screen
                    .as_ref()
                    .and_then(|p| p.notes.get(n as usize).cloned())
            };
            match *a {
                Action::Close => {
                    self.close_graph();
                    return Some(Refresh::Full);
                }
                Action::OpenNote(n) => {
                    let p = note(self, n)?;
                    self.close_graph();
                    if p != self.current {
                        self.push_history();
                    }
                    self.open(&p);
                    return Some(Refresh::Full);
                }
                Action::PreviewNote(n) => {
                    let p = note(self, n)?;
                    self.close_graph();
                    if p != self.current {
                        self.push_history();
                        self.open(&p);
                    }
                    self.enter_preview();
                    return Some(Refresh::Full);
                }
            }
        }
        match frame.redraw {
            Redraw::None => None,
            Redraw::Partial => Some(Refresh::Partial),
            Redraw::Clean => Some(Refresh::PointerCleanup),
            Redraw::Full => Some(Refresh::Full),
        }
    }

    pub(crate) fn plugin_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        if self.screen.plugin_close_hit(&self.keyboard, x, y) {
            self.close_graph();
            return Some(Refresh::Full);
        }
        if y < self.screen.plugin_screen_top(&self.keyboard) {
            return None;
        }
        self.plugin_event(&PEvent::Tap { x, y })
    }

    pub(crate) fn plugin_page(&mut self, forward: bool) -> Option<Refresh> {
        self.plugin_event(&PEvent::PageTurn { forward })
    }

    pub(crate) fn plugin_wheel(&mut self, notches: i32) -> Option<Refresh> {
        let (x, y) = (self.screen.width as i32 / 2, self.screen.height as i32 / 2);
        self.plugin_event(&PEvent::Wheel { x, y, dy: notches })
    }

    /// Whether a point is in the plugin's drawing area.
    pub(crate) fn on_plugin_area(&self, x: i32, y: i32) -> bool {
        self.plugin_screen.is_some() && y >= self.screen.plugin_screen_top(&self.keyboard) && x >= 0
    }

    /// Whether a press started a drag that has not ended.
    pub(crate) fn plugin_drag_active(&self) -> bool {
        self.plugin_screen
            .as_ref()
            .is_some_and(|p| p.drag.is_some())
    }

    pub(crate) fn plugin_drag_start(&mut self, x: i32, y: i32) {
        if let Some(p) = &mut self.plugin_screen {
            p.drag = Some((x, y, false));
        }
    }

    /// A finger or button moved: the first move starts the drag.
    pub(crate) fn plugin_drag_move(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let (x0, y0, moved) = self.plugin_screen.as_ref()?.drag?;
        self.plugin_screen.as_mut()?.drag = Some((x0, y0, true));
        if !moved {
            let r = self.plugin_event(&PEvent::Drag {
                phase: PPhase::Start,
                x: x0,
                y: y0,
            });
            let m = self.plugin_event(&PEvent::Drag {
                phase: PPhase::Move,
                x,
                y,
            });
            return merge(r, m);
        }
        self.plugin_event(&PEvent::Drag {
            phase: PPhase::Move,
            x,
            y,
        })
    }

    /// Whether a drag moved past the slop (a release then ends it instead
    /// of being a tap).
    pub(crate) fn plugin_dragging(&self) -> bool {
        self.plugin_screen
            .as_ref()
            .and_then(|p| p.drag)
            .is_some_and(|d| d.2)
    }

    pub(crate) fn plugin_drag_end(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let ps = self.plugin_screen.as_mut()?;
        let (_, _, moved) = ps.drag.take()?;
        if !moved {
            return None;
        }
        self.plugin_event(&PEvent::Drag {
            phase: PPhase::End,
            x,
            y,
        })
    }

    /// A mouse drag that happened between press and release (the pointer
    /// path reports it once): start at the press point, end at the release.
    pub(crate) fn plugin_mouse_drag(
        &mut self,
        from: (i32, i32),
        to: (i32, i32),
    ) -> Option<Refresh> {
        let a = self.plugin_event(&PEvent::Drag {
            phase: PPhase::Start,
            x: from.0,
            y: from.1,
        });
        let b = self.plugin_event(&PEvent::Drag {
            phase: PPhase::End,
            x: to.0,
            y: to.1,
        });
        merge(a, b)
    }

    /// Emulator control lookup: the plugin names its tappable regions.
    pub(crate) fn plugin_control(&self, label: &str) -> Option<(i32, i32)> {
        let p = self.plugin_screen.as_ref()?;
        let centre = |r: sigma_plugin_api::Rect| (r.x + r.w / 2, r.y + r.h / 2);
        if label == "Close" {
            let c = self.screen.plugin_close_rect(&self.keyboard);
            return Some((c.x + c.w / 2, c.y + c.h / 2));
        }
        p.frame
            .hits
            .iter()
            .find(|h| h.label == label)
            .map(|h| centre(h.rect))
    }

    /// Title and frame for drawing.
    pub(crate) fn plugin_drawable(&self) -> Option<(&Frame, &str)> {
        self.plugin_screen
            .as_ref()
            .map(|p| (&p.frame, p.title.as_str()))
    }
}
