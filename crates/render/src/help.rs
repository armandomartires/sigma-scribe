//! The Help sheet (brief §6 "Help", S7): a menu of two pages with
//! breadcrumbs. **App info** (version, licence, author, credits) and the
//! **Markdown reference** (each syntax with a source sample and one
//! sentence). Sub-pages are laid out with the preview engine and paged with
//! the page-turn buttons; they say "Back to Help" and are titled
//! `Help / App info` or `Help / Markdown reference`.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::preview::{self, Doc, Type};
use crate::theme::Theme;
use sigma_core::markdown::{Block, BlockKind, Run, Style};
use sigma_core::Keyboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpPage {
    Menu,
    About,
    Markdown,
}

impl HelpPage {
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Menu => "Help",
            Self::About => "Help / App info",
            Self::Markdown => "Help / Markdown reference",
        }
    }
}

/// Menu rows: label and hint (the mock's wording).
pub const MENU: [(&str, &str, HelpPage); 2] = [
    ("App info", "Version, license, and author", HelpPage::About),
    (
        "Markdown reference",
        "The syntax this page can render",
        HelpPage::Markdown,
    ),
];

/// The Markdown reference (the mock's entries, brief §4 order).
pub const REFERENCE: [(&str, &str, &str); 11] = [
    ("Heading", "# Title\n## Section\n### Smaller", "These are the rows in Contents."),
    ("Bold and italic", "**bold** and *italic*", "Two stars for bold, one for italic."),
    (
        "List",
        "- first\n- [ ] not yet\n- [x] done\n1. numbered",
        "A hyphen, or a number and a dot. Brackets make a checkbox.",
    ),
    ("Quote", "> A line set apart.", "A greater-than sign at the start."),
    ("Rule", "---", "Three hyphens on their own line."),
    ("Code", "`word`\n\n```\na block\n```", "Backticks. Three of them fence a block."),
    (
        "Picture",
        "![A path](https://example.com/path.jpg)",
        "An exclamation mark, words for the picture, then the address. Shown as [image: …] on the Kobo.",
    ),
    (
        "Web link",
        "[Kobo](https://example.com)",
        "Shown underlined; the Kobo has no browser to open it. A note still uses double brackets.",
    ),
    (
        "Table",
        "| Ink | Paper |\n| --- | --- |\n| black | white |",
        "A header, a row of dashes, then the cells. Shown as plain text for now.",
    ),
    ("Strike", "~~no longer true~~", "Two tildes on each side."),
    (
        "Link to a note",
        "[[chapter-01]]\n[[field-notes|morning]]",
        "The note name, or a name and the words to show. A dashed link creates the note. Links are what the graph draws.",
    ),
];

pub struct HelpModel<'a> {
    pub page: HelpPage,
    /// Laid-out sub-page (None on the menu).
    pub doc: Option<&'a Doc>,
    /// Page of `doc` shown.
    pub at: usize,
    pub theme: Theme,
    pub keyboard: &'a Keyboard,
}

#[derive(Clone, Debug, Default)]
pub struct HelpLayout {
    pub base: Layout,
    pub close: Rect,
    /// "Back to Help" (sub-pages).
    pub crumb: Rect,
    /// Menu rows.
    pub rows: [Rect; 2],
    /// Text column of a sub-page.
    pub area: Rect,
    /// Page indicator band.
    pub foot: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpHit {
    Close,
    Back,
    Open(HelpPage),
    Nothing,
}

fn run(text: &str, style: Style) -> Run {
    Run {
        text: text.to_owned(),
        style,
        target: None,
    }
}

fn para(text: &str, style: Style) -> Block {
    Block {
        kind: BlockKind::Paragraph,
        runs: vec![run(text, style)],
        line: 0,
    }
}

const STRONG: Style = Style {
    strong: true,
    em: false,
    strike: false,
    code: false,
    link: None,
};

/// Blocks of a sub-page. `about` is (label, text) pairs for App info.
#[must_use]
pub fn help_blocks(page: HelpPage, about: &[(String, String)]) -> Vec<Block> {
    let mut out = Vec::new();
    match page {
        HelpPage::Menu => {}
        HelpPage::About => {
            for (label, text) in about {
                out.push(para(label, STRONG));
                out.push(para(text, Style::default()));
            }
        }
        HelpPage::Markdown => {
            for (name, source, note) in REFERENCE {
                out.push(para(name, STRONG));
                out.push(Block {
                    kind: BlockKind::Code,
                    runs: source.lines().map(|l| run(l, Style::default())).collect(),
                    line: 0,
                });
                out.push(para(note, Style::default()));
            }
        }
    }
    out
}

impl EditorScreen {
    #[must_use]
    pub fn help_layout(&self, keyboard: &Keyboard) -> HelpLayout {
        let base = self.layout(false, keyboard);
        let s = self.scale();
        let w = self.width as i32;
        let h = self.height as i32;
        let top = base.title.h;
        let row_h = s.px(80.0);
        let crumb = Rect::new(0, top, w, s.px(56.0));
        let foot = Rect::new(0, h - s.px(48.0), w, s.px(48.0));
        let m = s.px(28.0);
        // Same column rule as the page: locked portrait width in landscape.
        let cw = (w.min(h)).min(w) - 2 * m;
        let ax = (w - cw) / 2;
        let ay = crumb.y + crumb.h + s.px(16.0);
        HelpLayout {
            close: base.files_button,
            crumb,
            rows: [
                Rect::new(0, top + s.px(8.0), w, row_h),
                Rect::new(0, top + s.px(8.0) + row_h, w, row_h),
            ],
            area: Rect::new(ax, ay, cw, foot.y - ay - s.px(8.0)),
            foot,
            base,
        }
    }

    /// Lays a sub-page out in the help column (pages for the page-turn
    /// buttons).
    #[must_use]
    pub fn build_help(
        &self,
        keyboard: &Keyboard,
        page: HelpPage,
        about: &[(String, String)],
    ) -> Doc {
        let l = self.help_layout(keyboard);
        let s = self.scale();
        let ty = Type {
            body: 20.0 * s.0,
            leading: 1.5,
            scale: s,
        };
        let lines = preview::layout(
            &help_blocks(page, about),
            &self.fonts,
            l.area.w,
            ty,
            &|_| false,
        );
        Doc {
            pages: preview::paginate(&lines, l.area.h),
            lines,
            width: l.area.w,
        }
    }

    #[must_use]
    pub fn help_hit(&self, m: &HelpModel<'_>, x: i32, y: i32) -> HelpHit {
        let l = self.help_layout(m.keyboard);
        if l.close.inset(-self.scale().px(4.0)).contains(x, y) {
            return HelpHit::Close;
        }
        if m.page == HelpPage::Menu {
            for (r, (_, _, p)) in l.rows.iter().zip(MENU) {
                if r.contains(x, y) {
                    return HelpHit::Open(p);
                }
            }
        } else if l.crumb.contains(x, y) {
            return HelpHit::Back;
        }
        HelpHit::Nothing
    }

    /// Centre of a Help control (emulator scripts): `Close`, `Back`,
    /// `App info`, `Markdown reference`.
    #[must_use]
    pub fn help_control(&self, m: &HelpModel<'_>, name: &str) -> Option<(i32, i32)> {
        let l = self.help_layout(m.keyboard);
        let c = |r: Rect| (r.x + r.w / 2, r.y + r.h / 2);
        match name {
            "Close" => Some(c(l.close)),
            "Back" if m.page != HelpPage::Menu => {
                Some((l.crumb.x + l.crumb.w / 4, l.crumb.y + l.crumb.h / 2))
            }
            _ if m.page == HelpPage::Menu => MENU
                .iter()
                .position(|(n, _, _)| *n == name)
                .map(|i| (l.rows[i].x + l.rows[i].w / 3, l.rows[i].y + l.rows[i].h / 2)),
            _ => None,
        }
    }

    pub fn draw_help(&self, c: &mut Canvas, m: &HelpModel<'_>) -> HelpLayout {
        let t = m.theme;
        let l = self.help_layout(m.keyboard);
        let s = self.scale();
        let ui = self.ui_px();
        c.fill(c.bounds(), t.paper);
        self.draw_title(c, &l.base, m.page.title(), None, t, false);
        let r = l.close;
        let (cx, cy, d) = (r.x + r.w / 2, r.y + r.h / 2, s.px(8.0));
        for k in -d..=d {
            for th in 0..s.px(1.75).max(2) {
                c.blend(cx + k, cy + k + th, t.ink, 255);
                c.blend(cx + k, cy - k + th, t.ink, 255);
            }
        }
        let tx = s.px(28.0) as f32;
        if m.page == HelpPage::Menu {
            for (r, (label, hint, _)) in l.rows.iter().zip(MENU) {
                self.fonts.draw(
                    c,
                    Face::AtkinsonBold,
                    ui * 1.1,
                    tx,
                    r.y as f32 + r.h as f32 * 0.45,
                    label,
                    t.ink,
                );
                self.fonts.draw(
                    c,
                    Face::Atkinson,
                    ui * 0.85,
                    tx,
                    r.y as f32 + r.h as f32 * 0.8,
                    hint,
                    t.mute,
                );
                c.fill(Rect::new(0, r.y + r.h - 1, r.w, 1), t.rule);
            }
            return l;
        }
        let by =
            l.crumb.y as f32 + l.crumb.h as f32 / 2.0 + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            tx,
            by,
            "\u{2190} Back to Help",
            t.ink,
        );
        c.fill(
            Rect::new(0, l.crumb.y + l.crumb.h - 1, l.crumb.w, 1),
            t.rule,
        );
        if let Some(doc) = m.doc {
            let at = m.at.min(doc.pages.len().saturating_sub(1));
            if let Some(range) = doc.pages.get(at) {
                preview::draw_page(c, &self.fonts, l.area, &doc.lines, range.clone(), t, s);
            }
            let n = doc.pages.len();
            if n > 1 {
                let label = format!("page {} of {n} \u{b7} page buttons turn", at + 1);
                c.fill(Rect::new(0, l.foot.y, l.foot.w, 1), t.rule);
                let fy = l.foot.y as f32
                    + l.foot.h as f32 / 2.0
                    + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
                self.fonts
                    .draw(c, Face::Atkinson, ui, tx, fy, &label, t.mute);
            }
        }
        l
    }
}

/// "?" in a circle (brief §3 icon 9).
pub fn draw_help_icon(screen: &EditorScreen, c: &mut Canvas, r: Rect, ink: u8) {
    let s = screen.scale();
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    let rad = s.px(10.0) as f32;
    let st = s.px(1.75).max(2) as f32;
    let k = s.px(12.0);
    for dy in -k..=k {
        for dx in -k..=k {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if (d - rad).abs() <= st / 2.0 {
                c.blend(cx + dx, cy + dy, ink, 255);
            }
        }
    }
    let size = screen.ui_px();
    let w = screen.fonts.width(Face::AtkinsonBold, size, "?");
    screen.fonts.draw(
        c,
        Face::AtkinsonBold,
        size,
        cx as f32 - w / 2.0,
        cy as f32 + screen.fonts.ascent(Face::AtkinsonBold, size) / 2.6,
        "?",
        ink,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_opens_pages_and_reference_has_every_entry() {
        let e = EditorScreen::new(1440, 1920);
        let k = Keyboard::default();
        let mut m = HelpModel {
            page: HelpPage::Menu,
            doc: None,
            at: 0,
            theme: Theme::DAY,
            keyboard: &k,
        };
        for (name, _, p) in MENU {
            let (x, y) = e.help_control(&m, name).unwrap();
            assert_eq!(e.help_hit(&m, x, y), HelpHit::Open(p));
        }
        let doc = e.build_help(&k, HelpPage::Markdown, &[]);
        let text: String = doc
            .lines
            .iter()
            .flat_map(|l| l.pieces.iter().map(|p| p.text.clone() + " "))
            .collect();
        for (name, source, _) in REFERENCE {
            assert!(text.contains(name), "{name}");
            assert!(
                text.contains(source.lines().next().unwrap().trim()),
                "{source}"
            );
        }
        assert!(doc.pages.len() >= 2, "the reference needs page turns");
        m.page = HelpPage::Markdown;
        m.doc = Some(&doc);
        let (x, y) = e.help_control(&m, "Back").unwrap();
        assert_eq!(e.help_hit(&m, x, y), HelpHit::Back);
        let mut c = Canvas::new(1440, 1920, 255);
        e.draw_help(&mut c, &m);
    }
}
