//! The Graph sheet (brief §6, S6): the brief allows "a calm list of notes
//! with their links" on the device instead of a drawn map (decision S6-D1).
//! One row per note: its name, then where it links (→) and what links to
//! it (←). Tap a row to open that note. Page-turn buttons scroll.
//! v1.0 (S9.1) adds the map view (`graphmap.rs`) and a Map / List toggle;
//! the list stays.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::theme::Theme;
use sigma_core::Keyboard;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRow {
    /// Path from the library root.
    pub name: String,
    /// `→ a, b · ← c`, or `No links`.
    pub detail: String,
    pub current: bool,
}

pub struct GraphModel<'a> {
    pub rows: &'a [GraphRow],
    pub first: usize,
    /// Links that reach a note, over the whole library.
    pub edges: usize,
    pub theme: Theme,
    pub keyboard: &'a Keyboard,
    /// `Some` when the map view is shown (v1.0); `None` shows the list.
    pub map: Option<crate::graphmap::MapView<'a>>,
}

#[derive(Clone, Debug, Default)]
pub struct GraphLayout {
    pub base: Layout,
    pub close: Rect,
    /// Row index and band of each visible row.
    pub rows: Vec<(usize, Rect)>,
    pub visible: usize,
    /// Where the "N links · page i of n" or empty-state note goes.
    pub foot: Rect,
    /// Map / List toggle, zoom and Open buttons (v1.0).
    pub toolbar: Vec<(GraphHit, Rect)>,
    /// The map drawing area (map view).
    pub area: Rect,
    /// Visible nodes and their boxes (map view), in drawing order.
    pub nodes: Vec<(usize, Rect)>,
    /// Pixels per layout unit and the layout point at the area's centre.
    pub scale: f64,
    pub centre: (f64, f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphHit {
    Close,
    Row(usize),
    /// v1.0 map view.
    Map,
    List,
    ZoomOut,
    ZoomIn,
    Open,
    Node(usize),
    Nothing,
}

impl EditorScreen {
    #[must_use]
    pub fn graph_layout(&self, m: &GraphModel<'_>) -> GraphLayout {
        let base = self.layout(false, m.keyboard);
        let s = self.scale();
        let w = self.width as i32;
        let row_h = s.px(80.0);
        let bar_h = s.px(72.0);
        let toolbar = self.graph_toolbar(m, base.title.h, bar_h);
        let top = base.title.h + bar_h + s.px(8.0);
        let foot_h = s.px(56.0);
        let avail = self.height as i32 - top - foot_h;
        let visible = (avail / row_h).max(1) as usize;
        let foot = Rect::new(0, self.height as i32 - foot_h, w, foot_h);
        if let Some(v) = &m.map {
            let area = Rect::new(0, base.title.h + bar_h, w, foot.y - base.title.h - bar_h);
            let (scale, centre, nodes) = self.map_nodes(m, v, area);
            return GraphLayout {
                close: base.close_button,
                foot,
                base,
                rows: Vec::new(),
                visible,
                toolbar,
                area,
                nodes,
                scale,
                centre,
            };
        }
        let rows = (m.first..m.rows.len().min(m.first + visible))
            .enumerate()
            .map(|(k, i)| (i, Rect::new(0, top + k as i32 * row_h, w, row_h)))
            .collect();
        GraphLayout {
            close: base.close_button,
            foot,
            base,
            rows,
            visible,
            toolbar,
            area: Rect::default(),
            nodes: Vec::new(),
            scale: 0.0,
            centre: (0.0, 0.0),
        }
    }

    #[must_use]
    pub fn graph_hit(&self, m: &GraphModel<'_>, x: i32, y: i32) -> GraphHit {
        let l = self.graph_layout(m);
        if l.close.inset(-self.scale().px(4.0)).contains(x, y) {
            return GraphHit::Close;
        }
        if let Some((h, _)) = l.toolbar.iter().find(|(_, r)| r.contains(x, y)) {
            return *h;
        }
        if m.map.is_some() {
            return if l.area.contains(x, y) {
                self.map_hit(&l, x, y)
            } else {
                GraphHit::Nothing
            };
        }
        l.rows
            .iter()
            .find(|(_, r)| r.contains(x, y))
            .map_or(GraphHit::Nothing, |(i, _)| GraphHit::Row(*i))
    }

    pub fn draw_graph(&self, c: &mut Canvas, m: &GraphModel<'_>) -> GraphLayout {
        let t = m.theme;
        let l = self.graph_layout(m);
        let s = self.scale();
        let ui = self.ui_px();
        c.fill(c.bounds(), t.paper);
        if let Some(v) = &m.map {
            self.draw_map(c, m, v, &l);
            // Bars are painted over the map, which clips it.
            c.fill(Rect::new(0, 0, l.area.w, l.area.y), t.paper);
            c.fill(Rect::new(0, l.foot.y, l.foot.w, l.foot.h), t.paper);
        }
        self.draw_title(c, &l.base, &tr("Graph"), None, t, false);
        self.draw_graph_toolbar(c, m, &l);
        crate::icons::draw_icon(c, crate::icons::Icon::Close, l.close, t.ink);
        let tx = s.px(28.0) as f32;
        if let Some(v) = &m.map {
            let foot = self.map_footer(m, v);
            c.fill(Rect::new(0, l.foot.y, l.foot.w, 1), t.rule);
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui,
                tx,
                l.foot.y as f32
                    + l.foot.h as f32 / 2.0
                    + self.fonts.ascent(Face::Atkinson, ui) / 2.6,
                &foot,
                t.mute,
            );
            return l;
        }
        let max = (self.width as i32 - s.px(56.0)) as f32;
        for &(i, r) in &l.rows {
            let row = &m.rows[i];
            if row.current {
                c.fill(Rect::new(0, r.y, s.px(6.0), r.h), t.ink);
            }
            let face = if row.current {
                Face::AtkinsonBold
            } else {
                Face::Atkinson
            };
            self.fonts.draw(
                c,
                face,
                ui * 1.1,
                tx,
                r.y as f32 + r.h as f32 * 0.45,
                &row.name,
                t.ink,
            );
            let mut detail = tr(&row.detail).into_owned();
            let full_len = detail.len();
            while !detail.is_empty() && self.fonts.width(Face::Atkinson, ui * 0.85, &detail) > max {
                detail.pop();
            }
            if detail.len() < full_len {
                detail.pop();
                detail.push('\u{2026}');
            }
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui * 0.85,
                tx,
                r.y as f32 + r.h as f32 * 0.8,
                &detail,
                t.mute,
            );
            c.fill(Rect::new(0, r.y + r.h - 1, r.w, 1), t.rule);
        }
        let pages = m.rows.len().div_ceil(l.visible.max(1)).max(1);
        let mut foot = if m.edges == 0 {
            "No links yet. Write [[a note name]] to join two notes.".to_owned()
        } else if m.edges == 1 {
            "1 link".to_owned()
        } else {
            format!("{} links", m.edges)
        };
        if pages > 1 {
            foot.push_str(&format!(
                " \u{b7} page {} of {pages}",
                m.first / l.visible.max(1) + 1
            ));
        }
        c.fill(Rect::new(0, l.foot.y, l.foot.w, 1), t.rule);
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            tx,
            l.foot.y as f32 + l.foot.h as f32 / 2.0 + self.fonts.ascent(Face::Atkinson, ui) / 2.6,
            &foot,
            t.mute,
        );
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_hit_and_scroll() {
        let e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let rows: Vec<GraphRow> = (0..40)
            .map(|i| GraphRow {
                name: format!("n{i}.md"),
                detail: "No links".into(),
                current: i == 0,
            })
            .collect();
        let mut m = GraphModel {
            rows: &rows,
            first: 0,
            edges: 0,
            theme: Theme::DAY,
            keyboard: &k,
            map: None,
        };
        let l = e.graph_layout(&m);
        assert!(l.visible > 5 && l.visible < 40);
        let (i, r) = l.rows[2];
        assert_eq!(e.graph_hit(&m, r.x + 50, r.y + r.h / 2), GraphHit::Row(i));
        assert_eq!(
            e.graph_hit(&m, l.close.x + 5, l.close.y + 5),
            GraphHit::Close
        );
        m.first = l.visible;
        let l2 = e.graph_layout(&m);
        assert_eq!(l2.rows[0].0, l.visible);
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_graph(&mut c, &m);
    }

    #[test]
    fn preview_links_and_backlinks_are_tappable() {
        let e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let known = |n: &str| n == "a";
        let back = vec![("b".to_owned(), "dir/b.md".to_owned())];
        let doc =
            e.build_preview_with("Go to [[a|the A note]] or [[zz]].", &k, &known, Some(&back));
        let all: String = doc
            .lines
            .iter()
            .flat_map(|l| l.pieces.iter().map(|p| p.text.clone()))
            .collect();
        assert!(all.contains("Backlinks"), "{all}");
        for (label, target) in [("the A note", "a"), ("zz", "zz"), ("b", "dir/b.md")] {
            let (x, y) = e.preview_link_point(&k, &doc, 0, label).unwrap();
            assert_eq!(
                e.preview_link_at(&k, &doc, 0, x, y).as_deref(),
                Some(target)
            );
        }
        let (x, y) = e.preview_link_point(&k, &doc, 0, "zz").unwrap();
        assert_eq!(e.preview_link_at(&k, &doc, 0, x, y + 300), None);
        let none = e.build_preview_with("x", &k, &known, Some(&[]));
        assert!(none
            .lines
            .iter()
            .any(|l| l.pieces.iter().any(|p| p.text.contains("No other notes"))));
    }
}
