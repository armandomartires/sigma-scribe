//! The Graph map view (v1.0, S9.1; look v1.0.2, S10.2): notes as small
//! circles (the selection a pill with its full name), links as plain
//! straight lines, on positions from
//! `sigma_core::graphmap`. The view opens fitted to the whole graph and
//! centred on the current note; four zoom levels; panning moves the centre.
//! Everything is solid ink / mute / rule on paper, so it stays crisp at 16
//! grey levels and inverts cleanly in Night.

use crate::canvas::{Canvas, Rect};
use crate::editor::EditorScreen;
use crate::fonts::Face;
use crate::graph::{GraphHit, GraphLayout, GraphModel};
use crate::i18n::tr;

/// Zoom factors over the fitted scale.
pub const ZOOMS: [f64; 4] = [1.0, 1.6, 2.6, 4.2];

/// The state of the Graph map: positions, links, zoom, pan and selection.
#[derive(Clone, Copy, Debug)]
pub struct MapView<'a> {
    /// Layout position of every note (index = row index).
    pub pos: &'a [(f64, f64)],
    /// Whether each note has no links (index = row index).
    pub orphan: &'a [bool],
    /// Directed links between notes (from, to).
    pub edges: &'a [(usize, usize)],
    /// Index into [`ZOOMS`].
    pub zoom: usize,
    /// Offset of the view centre from the current note, layout units.
    pub pan: (f64, f64),
    /// The selected note, if any (index = row index).
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
        let bh = s.px(46.0);
        let mut out = Vec::new();
        for i in 0..n {
            let (x, y) = self.map_point(area, scale, centre, v.pos[i]);
            let r = if v.selected == Some(i) {
                // The selection: a pill with the full name (v1.0.2).
                let w = (self.fonts.width(
                    Face::AtkinsonBold,
                    self.label_size(),
                    label(&m.rows[i].name),
                ) as i32
                    + bh)
                    .max(bh);
                Rect::new(x - w / 2, y - bh / 2, w, bh)
            } else {
                let d = self.map_dot(v, i, cur);
                Rect::new(x - d / 2, y - d / 2, d, d)
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

    /// Dot diameter: neighbours and the current note a little larger.
    fn map_dot(&self, v: &MapView<'_>, i: usize, cur: Option<usize>) -> i32 {
        let s = self.scale();
        if Some(i) == cur || self.map_emph(v, i) {
            s.px(30.0)
        } else {
            s.px(22.0)
        }
    }

    /// Whether node `i` gets a label beside its dot (decision S10.2-D1):
    /// always for the current note and the selection's neighbours; for
    /// every note only when the zoom leaves room.
    fn map_label_beside(&self, v: &MapView<'_>, i: usize, cur: Option<usize>, all: bool) -> bool {
        v.selected != Some(i) && (all || Some(i) == cur || self.map_emph(v, i))
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

    /// Touch target of a node: its shape, grown to at least 44 × 44 brief
    /// px around the centre.
    #[must_use]
    pub fn map_target(&self, r: Rect) -> Rect {
        let min = self.scale().px(44.0);
        let (w, h) = (r.w.max(min), r.h.max(min));
        Rect::new(r.x + r.w / 2 - w / 2, r.y + r.h / 2 - h / 2, w, h)
    }

    pub(crate) fn map_hit(&self, l: &GraphLayout, x: i32, y: i32) -> GraphHit {
        l.nodes
            .iter()
            .rev()
            .find(|(_, r)| self.map_target(*r).contains(x, y))
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
        let mut f = tr(&format!(
            "{n} note{} \u{b7} {} link{}",
            if n == 1 { "" } else { "s" },
            m.edges,
            if m.edges == 1 { "" } else { "s" }
        ))
        .into_owned();
        if let Some(s) = v.selected.and_then(|s| m.rows.get(s)) {
            f.push_str(&format!(
                " \u{b7} {} \u{2014} {}",
                label(&s.name),
                tr("tap again to open")
            ));
        } else if m.edges == 0 {
            f.push_str(" \u{b7} ");
            f.push_str(&tr("write [[a note name]] to join two notes"));
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
            // Plain lines (v1.0.2), clipped to the node shapes.
            let start = boxes.get(&a).map_or(pa, |r| exit(*r, pb, pa));
            let end = boxes.get(&b).map_or(pb, |r| exit(*r, pa, pb));
            thick_line(c, start, end, w, col);
        }
        // Nodes.
        let cur = (0..n).find(|&i| m.rows[i].current);
        let border = s.px(2.0).max(2);
        let size = self.label_size();
        let all = self.map_labels_all(n, l.scale);
        let maxw = s.px(200.0) as f32;
        let gap = s.px(8.0);
        for &(i, r) in &l.nodes {
            let emph = self.map_emph(v, i);
            let is_cur = Some(i) == cur;
            let sel = v.selected == Some(i);
            let quiet = v.selected.is_some() && !emph && !is_cur;
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
            let rad = r.h / 2;
            fill_round(c, r, rad, edge);
            let bw = if sel || emph { border * 2 } else { border };
            fill_round(c, r.inset(bw), rad - bw, bg);
            if sel && is_cur {
                fill_round(c, r.inset(bw * 2), rad - bw * 2, t.ink);
            }
            let (text, face, tx, ink) = if sel {
                let face = Face::AtkinsonBold;
                let text = label(&m.rows[i].name).to_owned();
                let w = self.fonts.width(face, size, &text);
                (text, face, r.x as f32 + (r.w as f32 - w) / 2.0, fg)
            } else if self.map_label_beside(v, i, cur, all) {
                let face = if is_cur {
                    Face::AtkinsonBold
                } else {
                    Face::Atkinson
                };
                let ink = if quiet { t.mute } else { t.ink };
                (
                    self.node_label(&m.rows[i].name, maxw),
                    face,
                    (r.x + r.w + gap) as f32,
                    ink,
                )
            } else {
                continue;
            };
            self.fonts.draw(
                c,
                face,
                size,
                tx,
                r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6,
                &text,
                ink,
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
pub(crate) fn thick_line(c: &mut Canvas, a: (i32, i32), b: (i32, i32), w: i32, v: u8) {
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

/// Filled rounded rectangle with corner radius `rad` (radius = h/2 gives
/// a circle or a pill). Hard edges, so it is crisp at 16 grey levels.
pub(crate) fn fill_round(c: &mut Canvas, r: Rect, rad: i32, v: u8) {
    if r.w <= 0 || r.h <= 0 {
        return;
    }
    let rad = rad.clamp(0, r.w.min(r.h) / 2);
    let rf = f64::from(rad);
    for row in 0..r.h {
        let fy = f64::from(row) + 0.5;
        let dy = if fy < rf {
            rf - fy
        } else if fy > f64::from(r.h) - rf {
            fy - (f64::from(r.h) - rf)
        } else {
            0.0
        };
        let inset = if dy > 0.0 {
            (rf - (rf * rf - dy * dy).max(0.0).sqrt()).round() as i32
        } else {
            0
        };
        c.fill(Rect::new(r.x + inset, r.y + row, r.w - 2 * inset, 1), v);
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
    fn round_dots_pill_for_selection_and_44px_targets() {
        let e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let mut r = rows(5);
        r[3].name = "dir/a-rather-long-note-name-for-the-pill.md".into();
        let edges = [(0, 1), (1, 2), (3, 4)];
        let lay = sigma_core::graphmap::layout(5, &edges, 1);
        let mut v = MapView {
            pos: &lay.pos,
            orphan: &lay.orphan,
            edges: &edges,
            zoom: 0,
            pan: (0.0, 0.0),
            selected: None,
        };
        let model = |v: MapView<'_>| -> Vec<(usize, Rect)> {
            let m = GraphModel {
                rows: &r,
                first: 0,
                edges: 3,
                theme: Theme::DAY,
                keyboard: &k,
                map: Some(v),
            };
            e.graph_layout(&m).nodes
        };
        let min = e.scale().px(44.0);
        for (_, b) in model(v) {
            assert_eq!(b.w, b.h, "round dot {b:?}");
            let t = e.map_target(b);
            assert!(t.w >= min && t.h >= min);
        }
        v.selected = Some(3);
        let nodes = model(v);
        let pill = nodes.iter().find(|(i, _)| *i == 3).unwrap().1;
        let full = e.fonts.width(
            Face::AtkinsonBold,
            e.label_size(),
            "a-rather-long-note-name-for-the-pill",
        );
        assert!(pill.w as f32 > full && pill.w > pill.h, "{pill:?}");
        // Neighbour (4) keeps a larger dot than an unrelated note (1).
        let d = |i: usize| nodes.iter().find(|(j, _)| *j == i).unwrap().1.w;
        assert!(d(4) > d(1));
        // Rounded fill: corners stay paper, centre is ink.
        let mut c = Canvas::new(40, 20, 255);
        fill_round(&mut c, Rect::new(0, 0, 40, 20), 10, 0);
        assert_eq!(c.pixels[0], 255);
        assert_eq!(c.pixels[10 * 40 + 20], 0);
    }

    #[test]
    fn exit_point_on_box_border() {
        let r = Rect::new(100, 100, 100, 40);
        let p = exit(r, (400, 120), (150, 120));
        assert_eq!(p, (202, 120));
    }
}
