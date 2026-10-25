//! The paged scroll bar shared by Settings, Help and Preview.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PagedBar {
    Settings,
    Help,
    Preview,
}

impl App {
    /// A finger on Settings (v1.4.4): a touch that starts on a slider is a
    /// drag that snaps to the nearest stop; anything else goes on to the
    /// gesture recogniser (taps).
    /// The paged scroll bar on the screen in front, if it has more than one
    /// page: Settings, a Help sub-page or Preview (v1.13.1).
    pub(crate) fn paged_bar(&self) -> Option<(PagedBar, sigma_render::scrollbar::Paged)> {
        if self.ink.is_some() || self.rename.is_some() {
            return None;
        }
        if self.options {
            let l = self.screen.options_layout(&self.options_model());
            return l.bar.map(|b| (PagedBar::Settings, b));
        }
        if let Some(h) = &self.help {
            let b = self.screen.help_paged_bar(&self.help_model(h))?;
            return Some((PagedBar::Help, b));
        }
        if self.files.is_some() || self.graph.is_some() || self.plugin_screen.is_some() {
            return None;
        }
        let p = self.preview.as_ref()?;
        let b = self
            .screen
            .preview_paged_bar(&self.keyboard, p.doc.pages.len(), p.page)?;
        Some((PagedBar::Preview, b))
    }

    /// Turns to `page` on the bar's screen; `None` when already there.
    fn set_bar_page(&mut self, kind: PagedBar, page: usize, last: bool) -> Option<Refresh> {
        match kind {
            PagedBar::Settings => {
                if page == self.settings_sub {
                    return None;
                }
                self.settings_sub = page;
                Some(if last {
                    Refresh::Full
                } else {
                    Refresh::Partial
                })
            }
            PagedBar::Help => {
                let h = self.help.as_mut()?;
                (page != h.at).then(|| {
                    h.at = page;
                    Refresh::Partial
                })
            }
            PagedBar::Preview => {
                let p = self.preview.as_mut()?;
                (page != p.page).then(|| {
                    p.page = page;
                    Refresh::Partial
                })
            }
        }
    }

    /// A tap on the bar: the page before or after, from the thumb's centre.
    pub(crate) fn bar_tap(&mut self, x: i32, y: i32) -> Option<Option<Refresh>> {
        let (kind, b) = self.paged_bar()?;
        b.zone
            .contains(x, y)
            .then(|| self.set_bar_page(kind, b.tap(y), true))
    }

    /// A finger on a paged scroll bar: drag the thumb, or tap above or below
    /// it. `None` when the touch is not on the bar.
    pub(crate) fn bar_touch(&mut self, phase: Phase, x: i32, y: i32) -> Option<Option<Refresh>> {
        match phase {
            Phase::Down => {
                let (_, b) = self.paged_bar()?;
                if !b.zone.contains(x, y) {
                    return None;
                }
                self.bar_drag = Some((y, false));
                Some(None)
            }
            Phase::Move | Phase::Up => {
                let (y0, moved) = self.bar_drag?;
                let (kind, b) = self.paged_bar()?;
                let moved = moved || (y - y0).abs() > self.screen.scale().px(12.0);
                if phase == Phase::Up {
                    self.bar_drag = None;
                    let page = if moved { b.drag_to(y) } else { b.tap(y) };
                    return Some(self.set_bar_page(kind, page, true));
                }
                self.bar_drag = Some((y0, moved));
                Some(if moved {
                    self.set_bar_page(kind, b.drag_to(y), false)
                } else {
                    None
                })
            }
        }
    }
}
