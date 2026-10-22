//! The Files sheet (brief §6): a full-page overlay listing folders and notes.
//! Tap a folder to enter, a note to open; **More** reveals Rename / Move /
//! Delete (armed: first tap arms, second confirms); New note / New folder at
//! the bottom. In move mode the list shows destination folders instead.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::theme::Theme;
use sigma_core::{Buffer, Keyboard};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    /// Second line: word count, or "Folder".
    pub detail: String,
    pub folder: bool,
    pub current: bool,
}

pub struct FilesModel<'a> {
    /// "Files", or "Files / drafts".
    pub title: String,
    /// "Up to Files" when inside a folder.
    pub crumb: Option<String>,
    pub rows: &'a [Row],
    pub more: Option<usize>,
    pub armed: Option<usize>,
    /// Some(name) while choosing where to move `name`; rows are folders.
    pub moving: Option<&'a str>,
    pub message: Option<&'a str>,
    pub first: usize,
    pub theme: Theme,
    /// Rename field in the header (rename or new folder).
    pub rename: Option<&'a Buffer>,
    pub keyboard: &'a Keyboard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesHit {
    Close,
    Up,
    Open(usize),
    More(usize),
    Rename(usize),
    Move(usize),
    Delete(usize),
    NewNote,
    NewFolder,
    CancelMove,
    Key(sigma_core::keyboard::Key),
    RenameOk,
    RenameCancel,
    Nothing,
}

#[derive(Clone, Debug, Default)]
pub struct FilesLayout {
    pub base: Layout,
    pub close: Rect,
    pub crumb: Rect,
    pub rows: Vec<(usize, Rect, Rect)>,
    pub actions: Option<[Rect; 3]>,
    pub bottom: [Rect; 2],
    pub visible: usize,
}

impl EditorScreen {
    fn files_px(&self, v: f32) -> i32 {
        self.scale().px(v)
    }

    #[must_use]
    pub fn files_layout(&self, m: &FilesModel<'_>) -> FilesLayout {
        let base = self.layout(m.rename.is_some(), m.keyboard);
        let w = self.width as i32;
        let btn = self.files_px(44.0);
        let pad = self.files_px(20.0);
        let close = base.close_button;
        let mut y = base.title.h;
        let crumb = if m.crumb.is_some() && m.moving.is_none() {
            let r = Rect::new(0, y, w, btn + self.files_px(8.0));
            y += r.h;
            r
        } else {
            Rect::default()
        };
        let bottom_h = btn + self.files_px(24.0);
        let bottom_y = if m.rename.is_some() {
            base.keyboard.y
        } else {
            self.height as i32
        } - bottom_h;
        let bw = (w - 3 * pad) / 2;
        let bottom = [
            Rect::new(pad, bottom_y + self.files_px(12.0), bw, btn),
            Rect::new(2 * pad + bw, bottom_y + self.files_px(12.0), bw, btn),
        ];
        let row_h = self.files_px(76.0);
        let act_h = btn + self.files_px(16.0);
        let mut rows = Vec::new();
        let mut actions = None;
        let mut i = m.first;
        while i < m.rows.len() && y + row_h <= bottom_y {
            let r = Rect::new(0, y, w, row_h);
            let more = Rect::new(
                w - pad - self.files_px(80.0),
                y + (row_h - btn) / 2,
                self.files_px(80.0),
                btn,
            );
            rows.push((
                i,
                r,
                if m.moving.is_some() {
                    Rect::default()
                } else {
                    more
                },
            ));
            y += row_h;
            if m.more == Some(i) && m.moving.is_none() && y + act_h <= bottom_y {
                let aw = (w - 4 * pad) / 3;
                let ay = y + self.files_px(8.0);
                actions = Some([
                    Rect::new(pad, ay, aw, btn),
                    Rect::new(2 * pad + aw, ay, aw, btn),
                    Rect::new(3 * pad + 2 * aw, ay, aw, btn),
                ]);
                y += act_h;
            }
            i += 1;
        }
        let visible = rows.len();
        FilesLayout {
            base,
            close,
            crumb,
            rows,
            actions,
            bottom,
            visible,
        }
    }

    #[must_use]
    pub fn files_hit(&self, m: &FilesModel<'_>, x: i32, y: i32) -> FilesHit {
        let l = self.files_layout(m);
        if m.rename.is_some() {
            if l.base.ok_rect.contains(x, y) {
                return FilesHit::RenameOk;
            }
            if l.base.cancel_rect.contains(x, y) {
                return FilesHit::RenameCancel;
            }
            if let Some(&(_, k)) = l.base.keys.iter().find(|(r, _)| r.contains(x, y)) {
                return FilesHit::Key(k);
            }
            return if l.base.title.contains(x, y) {
                FilesHit::Nothing
            } else {
                FilesHit::RenameOk
            };
        }
        if l.close.inset(-self.files_px(4.0)).contains(x, y) {
            return if m.moving.is_some() {
                FilesHit::CancelMove
            } else {
                FilesHit::Close
            };
        }
        if l.crumb.contains(x, y) {
            return FilesHit::Up;
        }
        if let Some(a) = l.actions {
            let i = m.more.unwrap_or(0);
            for (n, r) in a.iter().enumerate() {
                if r.contains(x, y) {
                    return [FilesHit::Rename(i), FilesHit::Move(i), FilesHit::Delete(i)][n];
                }
            }
        }
        for &(i, r, more) in &l.rows {
            if more.inset(-self.files_px(4.0)).contains(x, y) {
                return FilesHit::More(i);
            }
            if r.contains(x, y) {
                return FilesHit::Open(i);
            }
        }
        if m.moving.is_some() {
            if l.bottom[0].union(&l.bottom[1]).contains(x, y) {
                return FilesHit::CancelMove;
            }
        } else if l.bottom[0].contains(x, y) {
            return FilesHit::NewNote;
        } else if l.bottom[1].contains(x, y) {
            return FilesHit::NewFolder;
        }
        FilesHit::Nothing
    }

    pub fn draw_files(&self, c: &mut Canvas, m: &FilesModel<'_>) -> FilesLayout {
        let t = m.theme;
        let l = self.files_layout(m);
        let ui = self.ui_px();
        let s = self.scale();
        c.fill(c.bounds(), t.paper);
        let title = match m.moving {
            Some(name) => format!("{}\u{2026}", crate::i18n::tr_name("Move {name} to", name)),
            None => m.title.replacen("Files", &tr("Files"), 1),
        };
        self.draw_title(c, &l.base, &title, m.rename, t, false);
        if m.rename.is_none() {
            // Close mark: an X in a 44 px target.
            crate::icons::draw_icon(c, crate::icons::Icon::Close, l.close, t.ink);
        }
        let base =
            |r: Rect, face| r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, ui) / 2.6;
        if let Some(crumb) = &m.crumb {
            if m.moving.is_none() {
                self.fonts.draw(
                    c,
                    Face::Atkinson,
                    ui,
                    s.px(20.0) as f32,
                    base(l.crumb, Face::Atkinson),
                    &format!("\u{2190} {}", tr(crumb)),
                    t.ink,
                );
                c.fill(
                    Rect::new(0, l.crumb.y + l.crumb.h - 1, l.crumb.w, 1),
                    t.rule,
                );
            }
        }
        if m.rows.is_empty() {
            let y = l.crumb.y.max(l.base.title.h) + s.px(80.0);
            let msg = if m.moving.is_some() {
                "No folders."
            } else {
                "No notes here yet. Tap New note."
            };
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui,
                s.px(20.0) as f32,
                y as f32,
                &tr(msg),
                t.mute,
            );
        }
        for &(i, r, more) in &l.rows {
            let row = &m.rows[i];
            if row.current {
                c.fill(Rect::new(0, r.y, s.px(6.0), r.h), t.ink);
            }
            let ix = s.px(20.0);
            if row.folder {
                crate::icons::draw_icon(
                    c,
                    crate::icons::Icon::Folder,
                    Rect::new(
                        ix - s.px(10.0),
                        r.y + (r.h - s.px(44.0)) / 2,
                        s.px(44.0),
                        s.px(44.0),
                    ),
                    t.ink,
                );
            }
            let tx = (ix + s.px(48.0)) as f32;
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
            self.fonts.draw(
                c,
                Face::Atkinson,
                ui * 0.85,
                tx,
                r.y as f32 + r.h as f32 * 0.8,
                &tr(&row.detail),
                t.mute,
            );
            if more.w > 0 {
                let open = m.more == Some(i);
                if open {
                    c.fill(more, t.wash);
                }
                c.stroke(more, 1, if open { t.ink } else { t.rule });
                let mw = self.fonts.width(Face::Atkinson, ui, &tr("More"));
                self.fonts.draw(
                    c,
                    Face::Atkinson,
                    ui,
                    more.x as f32 + (more.w as f32 - mw) / 2.0,
                    base(more, Face::Atkinson),
                    &tr("More"),
                    t.ink,
                );
            }
            c.fill(
                Rect::new(s.px(20.0), r.y + r.h - 1, r.w - s.px(40.0), 1),
                t.rule,
            );
        }
        if let Some(a) = l.actions {
            let armed = m.armed.is_some() && m.armed == m.more;
            for (n, r) in a.iter().enumerate() {
                let label = match n {
                    0 => "Rename",
                    1 => "Move",
                    _ if armed => "Tap again to delete",
                    _ => "Delete",
                };
                let label: &str = &tr(label);
                let inverted = n == 2 && armed;
                c.fill(*r, if inverted { t.ink } else { t.paper });
                c.stroke(*r, 1, t.ink);
                let lw = self.fonts.width(Face::AtkinsonBold, ui * 0.9, label);
                self.fonts.draw(
                    c,
                    Face::AtkinsonBold,
                    ui * 0.9,
                    r.x as f32 + (r.w as f32 - lw) / 2.0,
                    base(*r, Face::AtkinsonBold),
                    label,
                    if inverted { t.paper } else { t.ink },
                );
            }
        }
        if let Some(msg) = m.message {
            let y = l.bottom[0].y - s.px(16.0);
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                s.px(20.0) as f32,
                y as f32,
                &tr(msg),
                t.ink,
            );
        }
        c.fill(
            Rect::new(0, l.bottom[0].y - s.px(12.0), self.width as i32, 1),
            t.rule,
        );
        let labels: [&str; 2] = if m.moving.is_some() {
            ["Cancel", ""]
        } else {
            ["New note", "New folder"]
        };
        let rects = if m.moving.is_some() {
            [l.bottom[0].union(&l.bottom[1]), Rect::default()]
        } else {
            l.bottom
        };
        for (r, label) in rects.iter().zip(labels) {
            if r.w == 0 {
                continue;
            }
            let label: &str = &tr(label);
            c.stroke(*r, 1, t.ink);
            let lw = self.fonts.width(Face::AtkinsonBold, ui, label);
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                r.x as f32 + (r.w as f32 - lw) / 2.0,
                base(*r, Face::AtkinsonBold),
                label,
                t.ink,
            );
        }
        if m.rename.is_some() {
            self.draw_keys(c, &l.base, m.keyboard, t);
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row> {
        let mut v = vec![Row {
            name: "drafts".into(),
            detail: "Folder".into(),
            folder: true,
            current: false,
        }];
        for i in 0..30 {
            v.push(Row {
                name: format!("note {i}.md"),
                detail: format!("{i} words"),
                folder: false,
                current: i == 2,
            });
        }
        v
    }

    fn model<'a>(rows: &'a [Row], kb: &'a Keyboard) -> FilesModel<'a> {
        FilesModel {
            title: "Files".into(),
            crumb: None,
            rows,
            more: None,
            armed: None,
            moving: None,
            message: None,
            first: 0,
            theme: Theme::DAY,
            rename: None,
            keyboard: kb,
        }
    }

    #[test]
    fn rows_fit_and_are_hittable() {
        let e = EditorScreen::new(1440, 1920);
        let (r, kb) = (rows(), Keyboard::default());
        let m = model(&r, &kb);
        let l = e.files_layout(&m);
        // v1.1: rows follow the 200 % UI scale.
        assert!(l.visible >= 8 && l.visible < r.len(), "{} rows", l.visible);
        for &(i, row, more) in &l.rows {
            assert!(row.h >= 50 && more.w >= 50 && more.h >= 50);
            assert_eq!(
                e.files_hit(&m, row.x + 40, row.y + row.h / 2),
                FilesHit::Open(i)
            );
            assert_eq!(e.files_hit(&m, more.x + 5, more.y + 5), FilesHit::More(i));
        }
        assert_eq!(
            e.files_hit(&m, l.bottom[0].x + 5, l.bottom[0].y + 5),
            FilesHit::NewNote
        );
        assert_eq!(
            e.files_hit(&m, l.bottom[1].x + 5, l.bottom[1].y + 5),
            FilesHit::NewFolder
        );
        assert_eq!(
            e.files_hit(&m, l.close.x + 5, l.close.y + 5),
            FilesHit::Close
        );
    }

    #[test]
    fn more_reveals_actions_and_crumb_goes_up() {
        let e = EditorScreen::new(1440, 1920);
        let (r, kb) = (rows(), Keyboard::default());
        let mut m = model(&r, &kb);
        m.more = Some(1);
        m.crumb = Some("Up to Files".into());
        let l = e.files_layout(&m);
        let a = l.actions.expect("actions shown");
        assert_eq!(e.files_hit(&m, a[0].x + 5, a[0].y + 5), FilesHit::Rename(1));
        assert_eq!(e.files_hit(&m, a[1].x + 5, a[1].y + 5), FilesHit::Move(1));
        assert_eq!(e.files_hit(&m, a[2].x + 5, a[2].y + 5), FilesHit::Delete(1));
        assert_eq!(e.files_hit(&m, 30, l.crumb.y + 5), FilesHit::Up);
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_files(&mut c, &m);
    }

    #[test]
    fn move_mode_has_no_more_buttons_and_cancel() {
        let e = EditorScreen::new(1440, 1920);
        let (r, kb) = (rows(), Keyboard::default());
        let mut m = model(&r, &kb);
        m.moving = Some("note 1.md");
        let l = e.files_layout(&m);
        assert!(l.rows.iter().all(|(_, _, more)| more.w == 0));
        assert_eq!(
            e.files_hit(&m, l.bottom[1].x + 5, l.bottom[1].y + 5),
            FilesHit::CancelMove
        );
    }
}
