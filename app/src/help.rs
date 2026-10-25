//! The Help sheet and the About page.

use super::*;

impl App {
    /// App info rows: version, licence, author, where notes live, credits.
    #[must_use]
    pub fn about(&self) -> Vec<(String, String)> {
        use sigma_render::i18n::{tr, tr_name, Lang};
        let lang = sigma_render::i18n::lang();
        let language = match lang {
            Lang::Fr => "Français. Traduction rédigée par machine, au mieux ; une relecture par un locuteur natif est nécessaire. Suit la langue du Kobo.".to_owned(),
            Lang::Pt => "Português. Tradução redigida por máquina, da melhor forma possível; precisa de revisão por um falante nativo. Segue o idioma do Kobo.".to_owned(),
            Lang::En | Lang::PseudoLong | Lang::PseudoRtl => {
                "English. Follows the Kobo's language, or Settings → Language.".to_owned()
            }
            l => format!(
                "{}. {}",
                sigma_core::settings::UiLang::from_code(l.code())
                    .unwrap_or_default()
                    .name(),
                tr("Machine-drafted translation, best effort; a native review is needed. Follows the Kobo's language.")
            ),
        };
        let rows: Vec<(String, String)> = vec![
            (
                "Σscribe".into(),
                format!(
                    "Version {}. A notebook for the Kobo Forma: plain Markdown files on an e-ink page, typed on the glass.",
                    env!("CARGO_PKG_VERSION")
                ),
            ),
            (
                "License".into(),
                "AGPL-3.0-only. The source is on GitHub: armandomartires/sigma-scribe.".into(),
            ),
            ("Author".into(), "SIGMArecon.".into()),
            (
                "Your notes".into(),
                tr_name(
                    "Plain .md and .txt files in {name}. Connect the Kobo over USB to copy them. The scratch pad is the hidden file .scratchpad.md.",
                    &self.library.root().display().to_string(),
                ),
            ),
            (
                "Buttons".into(),
                "The page buttons scroll the page. The power button saves and goes back to the reader. Every change is saved as you type.".into(),
            ),
            (
                "USB keyboard & mouse".into(),
                "Needs a powered hub or a Y-cable: the reader does not power the port. See Help \u{2192} USB keyboard, mouse & export.".into(),
            ),
            (
                "Credits".into(),
                "Literata and Atkinson Hyperlegible fonts (SIL Open Font License 1.1). Letter recognition: own model, trained only on synthetic ink from OFL fonts. FBInk by NiLuJe (GPL-3.0-or-later). NickelMenu by pgaskin. Icons: Lucide (ISC licence), lucide.dev.".into(),
            ),
        ];
        let mut out: Vec<(String, String)> = rows
            .into_iter()
            .map(|(k, v)| (tr(&k).into_owned(), tr(&v).into_owned()))
            .collect();
        out.push((tr("Language").into_owned(), language));
        out
    }

    pub fn open_help(&mut self) {
        sigma_core::debug!("ui", "open help");
        self.close_find();
        self.close_graph();
        self.files = None;
        self.options = false;
        self.save();
        self.sel_ui = None;
        self.drag = None;
        self.help = Some(HelpState {
            page: HelpPage::Menu,
            doc: None,
            at: 0,
        });
    }

    pub fn help_model<'a>(&'a self, h: &'a HelpState) -> HelpModel<'a> {
        HelpModel {
            page: h.page,
            doc: h.doc.as_ref(),
            at: h.at,
            theme: self.theme(),
            keyboard: &self.keyboard,
        }
    }

    pub(crate) fn help_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let h = self.help.as_ref()?;
        match self.screen.help_hit(&self.help_model(h), x, y) {
            HelpHit::Close => {
                self.help = None;
                Some(Refresh::Full)
            }
            HelpHit::Back => self.help_back(),
            HelpHit::Open(page) => {
                let started = std::time::Instant::now();
                let doc = self.screen.build_help(&self.keyboard, page, &self.about());
                sigma_core::debug!(
                    "ui",
                    "open help page {page:?} at {x},{y}: {} pages, {} lines, built in {} ms",
                    doc.pages.len(),
                    doc.lines.len(),
                    started.elapsed().as_millis()
                );
                self.help = Some(HelpState {
                    page,
                    doc: Some(doc),
                    at: 0,
                });
                Some(Refresh::Full)
            }
            HelpHit::Nothing => None,
        }
    }

    /// Escape / Back: a sub-page goes back to the menu, the menu closes.
    pub(crate) fn help_back(&mut self) -> Option<Refresh> {
        let h = self.help.as_mut()?;
        if h.page == HelpPage::Menu {
            self.help = None;
        } else {
            sigma_core::debug!("ui", "help back from {:?}", h.page);
            h.page = HelpPage::Menu;
            h.doc = None;
            h.at = 0;
        }
        Some(Refresh::Full)
    }

    pub(crate) fn help_page(&mut self, forward: bool) -> Option<Refresh> {
        let h = self.help.as_mut()?;
        let n = h.doc.as_ref()?.pages.len();
        let next = if forward {
            (h.at + 1).min(n.saturating_sub(1))
        } else {
            h.at.saturating_sub(1)
        };
        (next != h.at).then(|| {
            h.at = next;
            Refresh::Partial
        })
    }
}
