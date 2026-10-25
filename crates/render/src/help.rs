//! The Help sheet (brief §6 "Help", S7): a menu of two pages with
//! breadcrumbs. **App info** (version, licence, author, credits) and the
//! **Markdown reference** (each syntax with a source sample and one
//! sentence). Sub-pages are laid out with the preview engine and paged with
//! the page-turn buttons; they say "Back to Help" and are titled
//! `Help / App info` or `Help / Markdown reference`.

use crate::canvas::{Canvas, Rect};
use crate::editor::{EditorScreen, Layout};
use crate::fonts::Face;
use crate::i18n::tr;
use crate::preview::{self, Doc, Type};
use crate::theme::Theme;
use sigma_core::markdown::{Block, BlockKind, Run, Style};
use sigma_core::Keyboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpPage {
    Menu,
    About,
    Markdown,
    /// USB keyboard, shortcuts and export (v0.8).
    Keyboard,
}

impl HelpPage {
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Menu => "Help",
            Self::About => "Help / App info",
            Self::Markdown => "Help / Markdown reference",
            Self::Keyboard => "Help / USB keyboard, mouse & export",
        }
    }
}

/// Menu rows: label and hint (the mock's wording).
pub const MENU: [(&str, &str, HelpPage); 3] = [
    ("App info", "Version, license, and author", HelpPage::About),
    (
        "Markdown reference",
        "The syntax this page can render",
        HelpPage::Markdown,
    ),
    (
        "USB keyboard, mouse & export",
        "Hub, shortcuts, getting files off",
        HelpPage::Keyboard,
    ),
];

/// The USB keyboard and export page (v0.8): heading, text, optional code.
pub const KEYBOARD_HELP: [(&str, &str, &str); 7] = [
    (
        "Powered hub",
        "The Forma does not power its USB port. Plug the keyboard into a powered USB hub, or a Y-cable with its own power supply, and the hub into the reader with a micro-USB OTG adapter.",
        "",
    ),
    (
        "Connecting",
        "Turn on Options \u{2192} USB keyboard & mouse first: it switches the port to host mode, so a computer cannot see the Kobo until you turn it off or leave the app. Then plug the keyboard in; the status bar shows USB within a second (no USB while none is found). The on-screen keyboard keeps working, and unplugging is safe.",
        "",
    ),
    (
        "Shortcuts",
        "The same as the on-screen Ctrl key.",
        "Ctrl+Z  undo      Ctrl+Y  redo\nCtrl+A  select all\nCtrl+C  copy      Ctrl+X  cut\nCtrl+V  paste     Ctrl+F  find\nShift+arrows      select\nCtrl+Left/Right   word by word\nHome / End        line start / end\nCtrl+Home / End   top / end of note\nPgUp / PgDn       one screen",
    ),
    (
        "Mouse and touchpad",
        "A USB mouse or touchpad (also the touchpad of a keyboard such as the Logitech K400 Plus) shows a small arrow. Click = tap; drag on the note selects; double-click selects a word; the wheel or two-finger scroll moves the note and turns pages in preview and lists. The arrow hides after 3 seconds still and when you type. Right click does nothing.",
        "",
    ),
    (
        "Layout",
        "Options \u{2192} Layout sets the keyboard: US, UK, French, German, Spanish, Portuguese or Italian. Esc closes Find, sheets and selections.",
        "",
    ),
    (
        "Export",
        "Your notes are plain .md files. Connect the Kobo to a computer over USB (leave Σscribe first), choose Connect, and copy the sigma-scribe/notes folder. Nothing else is needed; sync (e.g. Syncthing) may come later.",
        "",
    ),
    (
        "If nothing happens",
        "Check the hub's power, try another keyboard (simple wired ones work best), then leave and reopen the app. The device log .adds/sigma-scribe/sigma-scribe.log notes each keyboard found.",
        "",
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
    /// Back arrow in the top bar (sub-pages).
    pub back: Option<Rect>,
    /// Baseline of the page title.
    pub heading_y: i32,
    /// Where "Page i of n" goes, right of the title.
    pub pager: Rect,
    /// Menu rows.
    pub rows: [Rect; 3],
    /// Text column of a sub-page.
    pub area: Rect,
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
                out.push(para(&tr(label), STRONG));
                out.push(para(text, Style::default()));
            }
        }
        HelpPage::Keyboard => {
            for (name, text, code) in KEYBOARD_HELP {
                out.push(para(&tr(name), STRONG));
                out.push(para(&tr(text), Style::default()));
                if !code.is_empty() {
                    out.push(Block {
                        kind: BlockKind::Code,
                        runs: code.lines().map(|l| run(l, Style::default())).collect(),
                        line: 0,
                    });
                }
            }
        }
        HelpPage::Markdown => {
            for (name, source, note) in REFERENCE {
                out.push(para(&tr(name), STRONG));
                out.push(Block {
                    kind: BlockKind::Code,
                    runs: source.lines().map(|l| run(l, Style::default())).collect(),
                    line: 0,
                });
                out.push(para(&tr(note), Style::default()));
            }
        }
    }
    out
}

impl EditorScreen {
    /// The Help screen's geometry: the same top bar and title as Settings
    /// (`page_header`), then either the menu rows or the text column of a
    /// sub-page (`sub`).
    #[must_use]
    pub fn help_layout(&self, keyboard: &Keyboard, sub: bool) -> HelpLayout {
        let base = self.layout(false, keyboard);
        let s = self.scale();
        let w = self.width as i32;
        let h = self.height as i32;
        let hd = self.page_header(&base, sub);
        let row_h = s.px(64.0);
        let rows = [0, 1, 2].map(|i| Rect::new(hd.pad, hd.top + i * row_h, w - 2 * hd.pad, row_h));
        // The page's own column: as wide as the title's margins in portrait,
        // the locked portrait width in landscape.
        let cw = w.min(h) - 2 * hd.pad;
        let ax = (w - cw) / 2;
        let ay = hd.top + s.px(8.0);
        HelpLayout {
            close: base.close_button,
            back: hd.back,
            heading_y: hd.heading_y,
            pager: hd.pager,
            rows,
            area: Rect::new(ax, ay, cw, h - s.px(12.0) - ay),
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
        let l = self.help_layout(keyboard, true);
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

    /// The scroll bar of a sub-page, as a paged bar (v1.13.1).
    #[must_use]
    pub fn help_paged_bar(&self, m: &HelpModel<'_>) -> Option<crate::scrollbar::Paged> {
        let doc = m.doc.filter(|_| m.page != HelpPage::Menu)?;
        let l = self.help_layout(m.keyboard, true);
        let page = Rect::new(0, l.area.y, self.width as i32, l.area.h);
        let at = m.at.min(doc.pages.len().saturating_sub(1));
        crate::scrollbar::Paged::new(self.page_scale(), page, doc.pages.len(), at)
    }

    #[must_use]
    pub fn help_hit(&self, m: &HelpModel<'_>, x: i32, y: i32) -> HelpHit {
        let l = self.help_layout(m.keyboard, m.page != HelpPage::Menu);
        if l.close.inset(-self.scale().px(4.0)).contains(x, y) {
            return HelpHit::Close;
        }
        if m.page == HelpPage::Menu {
            for (r, (_, _, p)) in l.rows.iter().zip(MENU) {
                if r.contains(x, y) {
                    return HelpHit::Open(p);
                }
            }
        } else if l
            .back
            .is_some_and(|b| b.inset(-self.scale().px(4.0)).contains(x, y))
        {
            return HelpHit::Back;
        }
        HelpHit::Nothing
    }

    /// Centre of a Help control (emulator scripts): `Close`, `Back`,
    /// `App info`, `Markdown reference`.
    #[must_use]
    pub fn help_control(&self, m: &HelpModel<'_>, name: &str) -> Option<(i32, i32)> {
        let l = self.help_layout(m.keyboard, m.page != HelpPage::Menu);
        let c = |r: Rect| (r.x + r.w / 2, r.y + r.h / 2);
        match name {
            "Close" => Some(c(l.close)),
            "Back" => l.back.map(c),
            _ if m.page == HelpPage::Menu => MENU
                .iter()
                .position(|(n, _, _)| *n == name)
                .map(|i| (l.rows[i].x + l.rows[i].w / 3, l.rows[i].y + l.rows[i].h / 2)),
            _ => None,
        }
    }

    pub fn draw_help(&self, c: &mut Canvas, m: &HelpModel<'_>) -> HelpLayout {
        let t = m.theme;
        let sub = m.page != HelpPage::Menu;
        let l = self.help_layout(m.keyboard, sub);
        let s = self.scale();
        let ui = self.ui_px();
        c.fill(c.bounds(), t.paper);
        let big = MENU
            .iter()
            .find(|(_, _, p)| *p == m.page)
            .map_or("Help", |(n, _, _)| *n);
        let pages = m
            .doc
            .map(|d| (m.at.min(d.pages.len().saturating_sub(1)), d.pages.len()));
        let hd = self.page_header(&l.base, sub);
        self.draw_page_header(c, &l.base, &hd, "Help", big, pages, t);
        if !sub {
            // The topics, as Settings lists its categories: the name in the
            // serif, "Open" in italics, what is inside in small print.
            for (r, (label, hint, _)) in l.rows.iter().zip(MENU) {
                let mid = r.y as f32 + r.h as f32 / 2.0;
                let base = mid + self.fonts.ascent(Face::Literata, ui) / 2.6;
                self.fonts.draw(
                    c,
                    Face::Literata,
                    ui,
                    r.x as f32,
                    base - ui * 0.45,
                    &tr(label),
                    t.ink,
                );
                self.fonts.draw(
                    c,
                    Face::Atkinson,
                    ui * 0.8,
                    r.x as f32,
                    mid + ui * 1.0,
                    &tr(hint),
                    t.mute,
                );
                let open = tr("Open");
                let ow = self.fonts.width(Face::LiterataItalic, ui, &open);
                self.fonts.draw(
                    c,
                    Face::LiterataItalic,
                    ui,
                    (r.x + r.w) as f32 - ow,
                    base,
                    &open,
                    t.ink,
                );
                c.fill(Rect::new(r.x, r.y + r.h - 1, r.w, 1), t.rule);
            }
            return l;
        }
        if let Some(doc) = m.doc {
            let at = m.at.min(doc.pages.len().saturating_sub(1));
            if let Some(range) = doc.pages.get(at) {
                preview::draw_page(c, &self.fonts, l.area, &doc.lines, range.clone(), t, s);
            }
            if let Some(b) = self.help_paged_bar(m) {
                crate::scrollbar::draw_bar(c, self.page_scale(), (b.strip, b.thumb), t, t.paper);
            }
        }
        l
    }
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
