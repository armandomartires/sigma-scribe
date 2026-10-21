//! The Graph map view (v1.0, S9.1): notes as labelled boxes, links as
//! straight lines with arrowheads, on positions from
//! `sigma_core::graphmap`. The view opens fitted to the whole graph and
//! centred on the current note; four zoom levels; panning moves the centre.
//! Everything is solid ink / mute / rule on paper, so it stays crisp at 16
//! grey levels and inverts cleanly in Night.

use crate::canvas::{Canvas, Rect};
use crate::editor::EditorScreen;
use crate::fonts::Face;
use crate::graph::{GraphHit, GraphLayout, GraphModel};

/// Zoom factors over the fitted scale.
pub const ZOOMS: [f64; 4] = [1.0, 1.6, 2.6, 4.2];

#[derive(Clone, Copy, Debug)]
pub struct MapView<'a> {
    /// Layout position of every note (index = row index).
    pub pos: &'a [(f64, f64)],
    pub orphan: &'a [bool],
    /// Directed links between notes (from, to).
    pub edges: &'a [(usize, usize)],
    pub zoom: usize,
    /// Offset of the view centre from the current note, layout units.
    pub pan: (f64, f64),
    pub selected: Option<usize>,
}

/// Short label for a note path: file stem, truncated with "…".
fn label(name: &str) -> &str {
    let base = name.rsplit('/').next().unwrap_or(name);
    base.strip_suffix(".md").unwrap_or(base)
}

impl MapView<'_> {
    fn neighbour(&self, a: usize, b: usize) -> bool {
        self.edges
            .iter()
            .any(|&(x, y)| (x == a && y == b) || (x == b && y == a))
    }
}

impl EditorScreen {
    pub(crate) fn graph_toolbar(
        &self,
        m: &GraphModel<'_>,
        top: i32,
        h: i32,
    ) -> Vec<(GraphHit, Rect)> {
        let s = self.scale();
        let (bw, bh, gap) = (s.px(120.0), s.px(56.0), s.px(12.0));
        let y = top + (h - bh) / 2;
        let x0 = s.px(28.0);
        let mut out = vec![
            (GraphHit::Map, Rect::new(x0, y, bw, bh)),
            (GraphHit::List, Rect::new(x0 + bw, y, bw, bh)),
        ];
        if let Some(v) = &m.map {
            let zw = s.px(72.0);
            let right = self.width as i32 - x0;
            out.push((GraphHit::ZoomIn, Rect::new(right - zw, y, zw, bh)));
            out.push((
                GraphHit::ZoomOut,
                Rect::new(right - 2 * zw - gap, y, zw, bh),
            ));
            if v.selected.is_some() {
                out.push((GraphHit::Open, Rect::new(x0 + 2 * bw + 2 * gap, y, bw, bh)));
            }
        }
        out
    }

    pub(crate) fn draw_graph_toolbar(&self, c: &mut Canvas, m: &GraphModel<'_>, l: &GraphLayout) {
        let t = m.theme;
        let ui = self.ui_px();
        let th = self.scale().px(2.0).max(2);
        for &(h, r) in &l.toolbar {
            let on = matches!(
                (h, m.map.is_some()),
                (GraphHit::Map, true) | (GraphHit::List, false)
            );
            let zoom = m.map.map_or(0, |v| v.zoom);
            let dim = matches!((h, zoom), (GraphHit::ZoomOut, 0) | (GraphHit::ZoomIn, 3));
            c.fill(r, if on { t.ink } else { t.paper });
            c.stroke(r, th, if dim { t.rule } else { t.ink });
            let text = match h {
                GraphHit::Map => "Map",
                GraphHit::List => "List",
                GraphHit::ZoomOut => "\u{2212}",
                GraphHit::ZoomIn => "+",
                GraphHit::Open => "Open",
                _ => "",
            };
            let size = if matches!(h, GraphHit::ZoomIn | GraphHit::ZoomOut) {
                ui * 1.4
            } else {
                ui
            };
            let face = if on {
                Face::AtkinsonBold
            } else {
                Face::Atkinson
            };
            let w = self.fonts.width(face, size, text);
            let ink = if on {
                t.paper
            } else if dim {
                t.rule
            } else {
                t.ink
            };
            self.fonts.draw(
                c,
                face,
                size,
                r.x as f32 + (r.w as f32 - w) / 2.0,
                r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6,
                text,
                ink,
            );
        }
    }

    fn label_size(&self) -> f32 {
        self.ui_px() * 0.9
    }

    /// Truncated label and whether labels are drawn for every node.
    fn node_label(&self, name: &str, max: f32) -> String {
        let size = self.label_size();
        let full = label(name);
        if self.fonts.width(Face::Atkinson, size, full) <= max {
            return full.to_owned();
        }
        let mut s: String = full.to_owned();
        while !s.is_empty()
            && self
                .fonts
                .width(Face::Atkinson, size, &format!("{s}\u{2026}"))
                > max
        {
            s.pop();
        }
        format!("{s}\u{2026}")
    }

    /// Scale, centre and node boxes for the map area.
    pub(crate) fn map_nodes(
        &self,
        m: &GraphModel<'_>,
        v: &MapView<'_>,
        area: Rect,
    ) -> (f64, (f64, f64), Vec<(usize, Rect)>) {
        let s = self.scale();
        let n = v.pos.len().min(m.rows.len());
        let cur = (0..n).find(|&i| m.rows[i].current);
        let anchor = cur.map_or((0.0, 0.0), |i| v.pos[i]);
        let (mut dx, mut dy) = (1e-3f64, 1e-3f64);
        for p in &v.pos[..n] {
            dx = dx.max((p.0 - anchor.0).abs());
            dy = dy.max((p.1 - anchor.1).abs());
        }
        let (mx, my) = (f64::from(s.px(110.0)), f64::from(s.px(40.0)));
        let fit = ((f64::from(area.w) / 2.0 - mx) / dx)
            .min((f64::from(area.h) / 2.0 - my) / dy)
            .max(1.0);
        let scale = fit * ZOOMS[v.zoom.min(ZOOMS.len() - 1)];
        let centre = (anchor.0 + v.pan.0, anchor.1 + v.pan.1);
        let labels = self.map_labels_all(n, scale);
        let bh = s.px(46.0);
        let dot = s.px(18.0);
        let maxw = s.px(200.0) as f32;
        let mut out = Vec::new();
        for i in 0..n {
            let (x, y) = self.map_point(area, scale, centre, v.pos[i]);
            let r = if labels || Some(i) == cur || self.map_emph(v, i) {
                let text = self.node_label(&m.rows[i].name, maxw);
                let w =
                    self.fonts.width(Face::Atkinson, self.label_size(), &text) as i32 + s.px(24.0);
                Rect::new(x - w / 2, y - bh / 2, w, bh)
            } else {
                Rect::new(x - dot / 2, y - dot / 2, dot, dot)
            };
            if r.x + r.w >= area.x
                && r.x <= area.x + area.w
                && r.y + r.h >= area.y
                && r.y <= area.y + area.h
            {
                out.push((i, r));
            }
        }
        // Emphasised nodes last, so they draw on top and win taps.
        out.sort_by_key(|&(i, _)| {
            u8::from(self.map_emph(v, i))
                + 2 * u8::from(Some(i) == cur)
                + 4 * u8::from(v.selected == Some(i))
        });
        (scale, centre, out)
    }

    fn map_emph(&self, v: &MapView<'_>, i: usize) -> bool {
        v.selected.is_some_and(|s| s == i || v.neighbour(s, i))
    }

    /// Labels on every node when the average spacing leaves room for them;
    /// otherwise only the current, selected and neighbouring notes are
    /// labelled and the rest are dots (zoom in to read them).
    fn map_labels_all(&self, n: usize, scale: f64) -> bool {
        let spacing = scale * 1.8 / (n.max(1) as f64).sqrt();
        spacing >= f64::from(self.scale().px(150.0))
    }

    fn map_point(&self, area: Rect, scale: f64, centre: (f64, f64), p: (f64, f64)) -> (i32, i32) {
        (
            area.x + area.w / 2 + ((p.0 - centre.0) * scale).round() as i32,
            area.y + area.h / 2 + ((p.1 - centre.1) * scale).round() as i32,
        )
    }

    pub(crate) fn map_hit(&self, l: &GraphLayout, x: i32, y: i32) -> GraphHit {
        let slop = self.scale().px(14.0);
        l.nodes
            .iter()
            .rev()
            .find(|(_, r)| r.inset(-slop).contains(x, y))
            .map_or(GraphHit::Nothing, |&(i, _)| GraphHit::Node(i))
    }

    /// Centre of node `i` on screen if it is visible (emulator scripts).
    #[must_use]
    pub fn map_node_point(&self, m: &GraphModel<'_>, i: usize) -> Option<(i32, i32)> {
        let l = self.graph_layout(m);
        l.nodes
            .iter()
            .find(|(j, _)| *j == i)
            .map(|(_, r)| (r.x + r.w / 2, r.y + r.h / 2))
            .filter(|&(x, y)| l.area.contains(x, y))
    }

    pub(crate) fn map_footer(&self, m: &GraphModel<'_>, v: &MapView<'_>) -> String {
        let n = m.rows.len();
        let mut f = format!(
            "{n} note{} \u{b7} {} link{}",
            if n == 1 { "" } else { "s" },
            m.edges,
            if m.edges == 1 { "" } else { "s" }
        );
        if let Some(s) = v.selected.and_then(|s| m.rows.get(s)) {
            f.push_str(&format!(
                " \u{b7} {} \u{2014} tap again to open",
                label(&s.name)
            ));
        } else if m.edges == 0 {
            f.push_str(" \u{b7} write [[a note name]] to join two notes");
        }
        f
    }

    pub(crate) fn draw_map(
        &self,
        c: &mut Canvas,
        m: &GraphModel<'_>,
        v: &MapView<'_>,
        l: &GraphLayout,
    ) {
        let t = m.theme;
        let s = self.scale();
        let area = l.area;
        let n = v.pos.len().min(m.rows.len());
        let boxes: std::collections::HashMap<usize, Rect> = l.nodes.iter().copied().collect();
        let line_w = s.px(2.0).max(2);
        let arrow = f64::from(s.px(16.0));
        // Links: incident to the selection in ink, others mute (rule when a
        // selection exists, so the neighbourhood stands out).
        let mut edges: Vec<(usize, usize)> = v
            .edges
            .iter()
            .copied()
            .filter(|&(a, b)| a < n && b < n && a != b)
            .collect();
        edges.sort_by_key(|&(a, b)| v.selected.is_some_and(|s| s == a || s == b));
        for (a, b) in edges {
            let pa = self.map_point(area, l.scale, l.centre, v.pos[a]);
            let pb = self.map_point(area, l.scale, l.centre, v.pos[b]);
            let hot = v.selected.is_some_and(|s| s == a || s == b);
            let col = if hot {
                t.ink
            } else if v.selected.is_some() {
                t.rule
            } else {
                t.mute
            };
            let w = if hot { line_w + line_w / 2 } else { line_w };
            // Clip both ends to the boxes so arrowheads touch the border.
            let start = boxes.get(&a).map_or(pa, |r| exit(*r, pb, pa));
            let end = boxes.get(&b).map_or(pb, |r| exit(*r, pa, pb));
            thick_line(c, start, end, w, col);
            let len = f64::from(end.0 - start.0).hypot(f64::from(end.1 - start.1));
            if len > arrow * 2.5 {
                arrowhead(c, start, end, arrow, col);
            }
        }
        // Nodes.
        let cur = (0..n).find(|&i| m.rows[i].current);
        let border = s.px(2.0).max(2);
        let size = self.label_size();
        for &(i, r) in &l.nodes {
            let emph = self.map_emph(v, i);
            let is_cur = Some(i) == cur;
            let sel = v.selected == Some(i);
            let quiet = v.selected.is_some() && !emph && !is_cur;
            let small = r.h < s.px(30.0);
            let (bg, fg, edge) = if is_cur {
                (t.ink, t.paper, t.ink)
            } else if quiet {
                (t.paper, t.mute, t.rule)
            } else {
                (
                    t.paper,
                    t.ink,
                    if v.orphan.get(i) == Some(&true) {
                        t.mute
                    } else {
                        t.ink
                    },
                )
            };
            c.fill(r, bg);
            let bw = if sel {
                border * 3
            } else if emph {
                border * 2
            } else {
                border
            };
            c.stroke(r, bw, edge);
            if sel && is_cur {
                c.stroke(r.inset(-bw - border), border, t.ink);
            }
            if small {
                continue;
            }
            let text = self.node_label(&m.rows[i].name, r.w as f32);
            let face = if is_cur || sel {
                Face::AtkinsonBold
            } else {
                Face::Atkinson
            };
            let w = self.fonts.width(face, size, &text);
            self.fonts.draw(
                c,
                face,
                size,
                r.x as f32 + (r.w as f32 - w) / 2.0,
                r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6,
                &text,
                fg,
            );
        }
    }
}

/// Where the segment from `from` to the centre `to` of box `r` crosses its
/// border (the point on the border facing `from`).
fn exit(r: Rect, from: (i32, i32), to: (i32, i32)) -> (i32, i32) {
    let (dx, dy) = (f64::from(from.0 - to.0), f64::from(from.1 - to.1));
    if dx == 0.0 && dy == 0.0 {
        return to;
    }
    let (hw, hh) = (f64::from(r.w) / 2.0 + 2.0, f64::from(r.h) / 2.0 + 2.0);
    let k = (hw / dx.abs().max(1e-9))
        .min(hh / dy.abs().max(1e-9))
        .min(1.0);
    (
        to.0 + (dx * k).round() as i32,
        to.1 + (dy * k).round() as i32,
    )
}

/// A line `w` px thick: square stamps along a DDA (no anti-aliasing, so
/// it is crisp at 16 grey levels).
fn thick_line(c: &mut Canvas, a: (i32, i32), b: (i32, i32), w: i32, v: u8) {
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
    if steps > 20_000 {
        return;
    }
    let half = w / 2;
    for k in 0..=steps {
        let x = a.0 + (b.0 - a.0) * k / steps;
        let y = a.1 + (b.1 - a.1) * k / steps;
        c.fill(Rect::new(x - half, y - half, w, w), v);
    }
}

/// Filled triangle at `tip`, pointing along `from → tip`.
fn arrowhead(c: &mut Canvas, from: (i32, i32), tip: (i32, i32), size: f64, v: u8) {
    let (dx, dy) = (f64::from(tip.0 - from.0), f64::from(tip.1 - from.1));
    let len = dx.hypot(dy).max(1e-9);
    let (ux, uy) = (dx / len, dy / len);
    let (bx, by) = (f64::from(tip.0) - ux * size, f64::from(tip.1) - uy * size);
    let half = size * 0.5;
    let p = [
        (f64::from(tip.0), f64::from(tip.1)),
        (bx - uy * half, by + ux * half),
        (bx + uy * half, by - ux * half),
    ];
    let x0 = p.iter().map(|q| q.0).fold(f64::MAX, f64::min).floor() as i32;
    let x1 = p.iter().map(|q| q.0).fold(f64::MIN, f64::max).ceil() as i32;
    let y0 = p.iter().map(|q| q.1).fold(f64::MAX, f64::min).floor() as i32;
    let y1 = p.iter().map(|q| q.1).fold(f64::MIN, f64::max).ceil() as i32;
    let side = |a: (f64, f64), b: (f64, f64), x: f64, y: f64| {
        (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0)
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let s0 = side(p[0], p[1], fx, fy);
            let s1 = side(p[1], p[2], fx, fy);
            let s2 = side(p[2], p[0], fx, fy);
            if (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0) {
                c.blend(x, y, v, 255);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphRow;
    use crate::theme::Theme;
    use sigma_core::Keyboard;

    fn rows(n: usize) -> Vec<GraphRow> {
        (0..n)
            .map(|i| GraphRow {
                name: format!("dir/note-{i}.md"),
                detail: String::new(),
                current: i == 0,
            })
            .collect()
    }

    #[test]
    fn fit_hit_zoom_and_pan() {
        let e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let r = rows(6);
        let edges = [(0, 1), (1, 2), (2, 0), (3, 4)];
        let lay = sigma_core::graphmap::layout(6, &edges, 1);
        let mut v = MapView {
            pos: &lay.pos,
            orphan: &lay.orphan,
            edges: &edges,
            zoom: 0,
            pan: (0.0, 0.0),
            selected: None,
        };
        fn mk<'a>(r: &'a [GraphRow], k: &'a Keyboard, v: MapView<'a>) -> GraphModel<'a> {
            GraphModel {
                rows: r,
                first: 0,
                edges: 4,
                theme: Theme::DAY,
                keyboard: k,
                map: Some(v),
            }
        }
        let l = e.graph_layout(&mk(&r, &k, v));
        // Fitted: every node visible, current note at the centre.
        assert_eq!(l.nodes.len(), 6);
        for (_, b) in &l.nodes {
            assert!(
                l.area.contains(b.x, b.y) && l.area.contains(b.x + b.w, b.y + b.h),
                "{b:?}"
            );
        }
        let (cx, cy) = e.map_node_point(&mk(&r, &k, v), 0).unwrap();
        assert!((cx - (l.area.x + l.area.w / 2)).abs() <= 1);
        assert!((cy - (l.area.y + l.area.h / 2)).abs() <= 1);
        let (x, y) = e.map_node_point(&mk(&r, &k, v), 3).unwrap();
        assert_eq!(e.graph_hit(&mk(&r, &k, v), x, y), GraphHit::Node(3));
        // Buttons.
        for h in [
            GraphHit::Map,
            GraphHit::List,
            GraphHit::ZoomIn,
            GraphHit::ZoomOut,
        ] {
            let b = l.toolbar.iter().find(|(g, _)| *g == h).unwrap().1;
            assert_eq!(e.graph_hit(&mk(&r, &k, v), b.x + 5, b.y + 5), h);
        }
        assert!(!l.toolbar.iter().any(|(g, _)| *g == GraphHit::Open));
        v.selected = Some(3);
        assert!(e
            .graph_layout(&mk(&r, &k, v))
            .toolbar
            .iter()
            .any(|(g, _)| *g == GraphHit::Open));
        // Zoom grows the scale; pan moves the picture.
        v.zoom = 2;
        let lz = e.graph_layout(&mk(&r, &k, v));
        assert!((lz.scale / l.scale - ZOOMS[2]).abs() < 1e-9);
        v.pan = (0.1, 0.0);
        let (px, _) = e.map_node_point(&mk(&r, &k, v), 0).unwrap();
        assert!(px < cx);
        // Draws in both themes without panicking; the map is clipped to its area.
        for theme in [Theme::DAY, Theme::NIGHT] {
            let mut c = Canvas::new(1440, 1920, 0);
            let mut mm = mk(&r, &k, v);
            mm.theme = theme;
            e.draw_graph(&mut c, &mm);
        }
    }

    #[test]
    fn many_notes_use_dots_and_labels_truncate() {
        let e = EditorScreen::new(1440, 1920);
        assert!(!e.map_labels_all(300, 500.0));
        assert!(e.map_labels_all(4, 500.0));
        let t = e.node_label("a/very-long-note-name-that-goes-on-and-on.md", 150.0);
        assert!(t.ends_with('\u{2026}'));
        assert_eq!(e.node_label("x/short.md", 400.0), "short");
    }

    #[test]
    fn exit_point_on_box_border() {
        let r = Rect::new(100, 100, 100, 40);
        let p = exit(r, (400, 120), (150, 120));
        assert_eq!(p, (202, 120));
    }
}
