//! The built-in Graph sheet and its map.

use super::*;

impl App {
    /// Forgets the cached map layout (perf measurements).
    pub fn clear_map_cache(&mut self) {
        self.map_cache = None;
    }

    /// Opens the Graph sheet with fresh rows.
    pub fn open_graph(&mut self) {
        sigma_core::debug!("ui", "open graph");
        self.help = None;
        self.close_find();
        self.save();
        self.sel_ui = None;
        self.drag = None;
        let g = self.link_graph();
        let name = |i: usize| stem(g.notes[i].name()).to_owned();
        let rows: Vec<GraphRow> = (0..g.notes.len())
            .map(|i| {
                let to: Vec<String> = g.out[i]
                    .iter()
                    .map(|l| match l.to {
                        Some(j) => name(j),
                        None => format!("{} (new)", l.target),
                    })
                    .collect();
                let from: Vec<String> = g
                    .backlinks(&g.notes[i])
                    .iter()
                    .map(|p| stem(p.name()).to_owned())
                    .collect();
                let mut parts = Vec::new();
                if !to.is_empty() {
                    parts.push(format!("\u{2192} {}", to.join(", ")));
                }
                if !from.is_empty() {
                    parts.push(format!("\u{2190} {}", from.join(", ")));
                }
                GraphRow {
                    name: g.notes[i].as_str().to_owned(),
                    detail: if parts.is_empty() {
                        "No links".into()
                    } else {
                        parts.join("  \u{b7}  ")
                    },
                    current: g.notes[i] == self.current,
                }
            })
            .collect();
        let mut links: Vec<(usize, usize)> = g
            .out
            .iter()
            .enumerate()
            .flat_map(|(i, out)| out.iter().filter_map(move |l| l.to.map(|j| (i, j))))
            .filter(|(a, b)| a != b)
            .collect();
        links.sort_unstable();
        links.dedup();
        if self.open_plugin_graph(&g.notes, &rows, &links, g.edges()) {
            self.graph = None;
            return;
        }
        let names: Vec<String> = g.notes.iter().map(|n| n.as_str().to_owned()).collect();
        let key = graphmap::layout_key(&names, &links);
        let (layout, layout_us) = match &self.map_cache {
            Some((k, l)) if *k == key => (l.clone(), 0),
            _ => {
                let t0 = std::time::Instant::now();
                let l = std::sync::Arc::new(graphmap::layout(names.len(), &links, MAP_SEED));
                let us = t0.elapsed().as_micros() as u64;
                self.map_cache = Some((key, l.clone()));
                (l, us.max(1))
            }
        };
        let map = self.graph.as_ref().is_none_or(|g| g.map);
        self.graph = Some(GraphState {
            edges: g.edges(),
            notes: g.notes,
            rows,
            first: 0,
            map,
            links,
            layout,
            zoom: 0,
            pan: (0.0, 0.0),
            selected: None,
            layout_us,
        });
    }

    pub(crate) fn graph_model<'a>(&'a self, g: &'a GraphState) -> GraphModel<'a> {
        GraphModel {
            rows: &g.rows,
            first: g.first,
            edges: g.edges,
            theme: self.theme(),
            keyboard: &self.keyboard,
            map: g.map.then_some(MapView {
                pos: &g.layout.pos,
                orphan: &g.layout.orphan,
                edges: &g.links,
                zoom: g.zoom,
                pan: g.pan,
                selected: g.selected,
            }),
        }
    }

    pub(crate) fn graph_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let g = self.graph.as_ref()?;
        let hit = self.screen.graph_hit(&self.graph_model(g), x, y);
        match hit {
            GraphHit::Close => {
                self.graph = None;
                Some(Refresh::Full)
            }
            GraphHit::Row(i) => {
                let p = g.notes.get(i)?.clone();
                self.graph = None;
                if p != self.current {
                    self.push_history();
                }
                self.open(&p);
                Some(Refresh::Full)
            }
            GraphHit::Map | GraphHit::List => {
                let map = matches!(hit, GraphHit::Map);
                let g = self.graph.as_mut()?;
                if g.map == map {
                    return None;
                }
                g.map = map;
                g.selected = None;
                Some(Refresh::Full)
            }
            GraphHit::ZoomIn | GraphHit::ZoomOut => {
                self.graph_zoom(matches!(hit, GraphHit::ZoomIn))
            }
            GraphHit::Open => {
                let s = g.selected?;
                self.graph_open_preview(s)
            }
            GraphHit::Node(i) => {
                if g.selected == Some(i) {
                    return self.graph_open_preview(i);
                }
                self.graph.as_mut()?.selected = Some(i);
                // v1.0.2: the pill grows in one non-flashing GL16 region
                // (v1.0.4: the GL16 class again; 1.0.3 sent it as DU).
                Some(Refresh::PointerCleanup)
            }
            GraphHit::Nothing => {
                let g = self.graph.as_mut()?;
                if g.map && g.selected.take().is_some() {
                    return Some(Refresh::PointerCleanup);
                }
                None
            }
        }
    }

    /// Opens a map node in preview (the sheet closes).
    fn graph_open_preview(&mut self, i: usize) -> Option<Refresh> {
        let p = self.graph.as_ref()?.notes.get(i)?.clone();
        self.graph = None;
        self.map_drag = None;
        if p != self.current {
            self.push_history();
            self.open(&p);
        }
        self.enter_preview();
        Some(Refresh::Full)
    }

    /// One zoom step; the view stays centred where it is. Full refresh.
    pub(crate) fn graph_zoom(&mut self, zoom_in: bool) -> Option<Refresh> {
        let g = self.graph.as_mut()?;
        if !g.map {
            return None;
        }
        let z = if zoom_in {
            (g.zoom + 1).min(ZOOMS.len() - 1)
        } else {
            g.zoom.saturating_sub(1)
        };
        if z == g.zoom {
            return None;
        }
        g.zoom = z;
        if z == 0 {
            g.pan = (0.0, 0.0);
        }
        Some(Refresh::Full)
    }

    /// Pans the map by a screen distance (the picture follows the finger).
    pub(crate) fn graph_pan(&mut self, dx: i32, dy: i32) -> Option<Refresh> {
        let g = self.graph.as_ref()?;
        if !g.map || (dx == 0 && dy == 0) {
            return None;
        }
        let scale = self
            .screen
            .graph_layout(&self.graph_model(g))
            .scale
            .max(1e-6);
        let g = self.graph.as_mut()?;
        let lim = 2.0;
        g.pan.0 = (g.pan.0 - f64::from(dx) / scale).clamp(-lim, lim);
        g.pan.1 = (g.pan.1 - f64::from(dy) / scale).clamp(-lim, lim);
        Some(Refresh::Partial)
    }

    /// Whether a point is on the map drawing area.
    pub(crate) fn on_map(&self, x: i32, y: i32) -> bool {
        self.graph.as_ref().is_some_and(|g| {
            g.map
                && self
                    .screen
                    .graph_layout(&self.graph_model(g))
                    .area
                    .contains(x, y)
        })
    }

    pub(crate) fn graph_page(&mut self, forward: bool) -> Option<Refresh> {
        let g = self.graph.as_ref()?;
        if g.map {
            // Page buttons pan half a view down (forward) or up.
            let h = self.screen.graph_layout(&self.graph_model(g)).area.h / 2;
            return self.graph_pan(0, if forward { -h } else { h });
        }
        let visible = self
            .screen
            .graph_layout(&self.graph_model(g))
            .visible
            .max(1);
        let n = g.rows.len();
        let g = self.graph.as_mut()?;
        let next = if forward {
            if g.first + visible < n {
                g.first + visible
            } else {
                g.first
            }
        } else {
            g.first.saturating_sub(visible)
        };
        (next != g.first).then(|| {
            g.first = next;
            Refresh::Partial
        })
    }
}
