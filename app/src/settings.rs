//! Settings: applying a change, saving, logging, language, and the Settings sheet.

use super::*;

impl App {
    /// Applies Settings → Logging: the log file on or off, and its level.
    pub(crate) fn apply_logging(&self) {
        sigma_core::log::configure(
            self.settings.log,
            self.settings.log_level,
            Some(self.log_dir.clone()),
        );
    }

    /// Where the log file goes (tests point this at a temp folder).
    pub fn set_log_dir(&mut self, dir: impl Into<PathBuf>) {
        self.log_dir = dir.into();
        self.apply_logging();
    }

    /// The Kobo style board while Enhanced keyboard is off (v1.13); back to
    /// the Compact / Full board (its saved choice) when it is on. Layer and
    /// shift start over when the style changes.
    pub fn sync_keyboard_style(&mut self) {
        use sigma_core::keyboard::{Layer, Style};
        let want = if self.settings.kobo_keyboard() {
            Style::Kobo
        } else {
            Style::Enhanced
        };
        if self.keyboard.style != want {
            self.keyboard.style = want;
            self.keyboard.layer = Layer::Letters;
            self.keyboard.shift = false;
            self.keyboard.shift_lock = false;
        }
    }

    /// Writes every setting to `notes/.sigma-settings` (atomic).
    pub(crate) fn save_settings(&mut self) {
        self.settings.keyboard = self.keyboard.layout;
        let _ = sigma_core::store::write_atomic(
            &self.library.root().join(SETTINGS),
            &self.settings.serialize(),
        );
    }

    /// The page theme (Options → Paper).
    #[must_use]
    pub fn theme(&self) -> Theme {
        Theme::for_paper(self.settings.paper)
    }

    /// The interface language (v1.4); normally from the Kobo's locale at
    /// start. Re-lays out so translated labels are measured.
    pub fn set_language(&mut self, l: sigma_render::i18n::Lang) {
        self.system_lang = l;
        sigma_render::i18n::set_lang(resolve_lang(&self.settings, l));
        self.orientation_label = self.orientation_status();
    }

    /// Applies one Options / keyboard-bar change: refits Line mode,
    /// re-lays out (and resizes on rotation), saves the settings file.
    /// Paper and orientation need a full (flashing) refresh.
    pub fn change(&mut self, c: Change) -> Refresh {
        if let Change::Plugin(i, on) = c {
            return self.set_plugin(i, on);
        }
        if let Change::PluginSetting(p, s, o) = c {
            return self.set_plugin_setting(usize::from(p), usize::from(s), usize::from(o));
        }
        if c == Change::TrainSamples {
            self.options = false;
            self.open_ink();
            if let Some(ink) = &mut self.ink {
                ink.mode = sigma_render::ink::InkMode::Train(0);
            }
            return Refresh::Full;
        }
        if c == Change::DeleteSamples {
            let p = samples_path(self.library.root());
            if let Err(e) = std::fs::remove_file(&p) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    sigma_core::warn!("samples", "delete: {e}");
                }
            }
            if let Some(d) = p.parent() {
                let _ = std::fs::remove_dir(d);
            }
            self.hwr = Hwr::default();
            return Refresh::Partial;
        }
        if let Change::Backlight(n) = c {
            // Not a saved setting: the front light keeps its level.
            self.light_step = n.min(sigma_core::settings::BACKLIGHT_MAX);
            self.light_want = Some(self.light_step);
            return Refresh::Partial;
        }
        self.settings.apply(c);
        if matches!(c, Change::Log(_) | Change::LogLevel(_)) {
            self.apply_logging();
            if self.settings.log {
                sigma_core::info!("app", "logging level {}", self.settings.log_level.label());
            }
        }
        sigma_core::debug!("settings", "{c:?}");
        self.screen.ink_words = self.settings.word_mode();
        if c == Change::LangOverride(true) {
            // Start from the language in use, so turning it on changes nothing.
            use sigma_core::settings::UiLang;
            self.settings.ui_lang = UiLang::from_code(self.system_lang.code()).unwrap_or_default();
        }
        if matches!(c, Change::LangOverride(_) | Change::UiLang(_)) {
            sigma_render::i18n::set_lang(resolve_lang(&self.settings, self.system_lang));
            self.orientation_label = self.orientation_status();
        }
        if let Change::Keyboard(l) = c {
            self.keyboard = Keyboard {
                layout: l,
                clipboard: std::mem::take(&mut self.keyboard.clipboard),
                ..Keyboard::default()
            };
        }
        self.keyboard.lang = self.settings.kb_layout;
        self.sync_keyboard_style();
        if !self.keyboard.lang.has_compact() {
            // Too many keys for the compact board (v1.9).
            self.keyboard.layout = sigma_core::keyboard::Layout::Full;
        }
        sigma_device::usbkbd::set_layout(self.settings.kb_layout);
        let anchor = match c.refit() {
            Some(Some(true)) => Some(Anchor::Size),
            Some(Some(false)) => Some(Anchor::Margin),
            _ => None,
        };
        let (w, h) = logical_size(self.phys, self.settings.rotation);
        if (self.screen.width, self.screen.height) != (w, h) {
            self.screen.resize(w, h);
            self.canvas = Canvas::new(w, h, 255);
            self.shown = Canvas::new(w, h, 0);
            self.sel_ui = None;
        }
        self.screen.configure(&mut self.settings, anchor);
        self.orientation_label = self.orientation_status();
        self.save_settings();
        if c.needs_full_refresh() {
            Refresh::Full
        } else {
            Refresh::Partial
        }
    }

    /// Opens the Options sheet (closes preview and selection chrome).
    pub fn open_options(&mut self) {
        sigma_core::debug!("ui", "open settings");
        self.help = None;
        self.close_find();
        self.close_graph();
        self.save();
        self.preview = None;
        self.sel_ui = None;
        self.drag = None;
        self.options = true;
        self.hwr.samples = std::fs::read_to_string(samples_path(self.library.root()))
            .map(|t| sigma_hwr::samples_from_text(&t).len())
            .unwrap_or(0);
        self.settings_page = sigma_render::SettingsPage::Root;
        self.settings_sub = 0;
        self.settings_focus = None;
        self.refresh_plugin_list();
    }

    pub fn options_model(&self) -> OptionsModel<'_> {
        OptionsModel {
            settings: &self.settings,
            theme: self.theme(),
            keyboard: &self.keyboard,
            orientation_status: &self.orientation_label,
            page: self.settings_page,
            sub: self.settings_sub,
            language: sigma_core::settings::UiLang::from_code(sigma_render::i18n::lang().code())
                .unwrap_or_default()
                .name(),
            version: env!("CARGO_PKG_VERSION"),
            focus: self.settings_focus,
            hw_samples: self.hwr.samples,
            light: self.light_have.then_some(self.light_step),
            ui_scale_pending: self.ui_scale_drag,
            plugins: &self.plugin_list,
        }
    }

    /// Settings: one level up, or closed from the top (Esc, back arrow).
    pub(crate) fn settings_back(&mut self) -> Refresh {
        match sigma_render::options::parent(self.settings_page) {
            Some(p) => {
                self.settings_page = p;
                self.settings_sub = 0;
            }
            None => self.options = false,
        }
        Refresh::Full
    }

    /// Next step towards a Settings control that is not on this page.
    #[must_use]
    pub fn settings_step(&self, name: &str) -> Option<String> {
        if !self.options {
            return None;
        }
        self.screen.options_step(&self.options_model(), name)
    }

    pub(crate) fn options_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let focus = self
            .screen
            .options_slider_at(&self.options_model(), x, y)
            .map(|t| t.0);
        let refocus = focus != self.settings_focus;
        self.settings_focus = focus;
        let r = self.options_hit_tap(x, y);
        if refocus && r.is_none() {
            return Some(Refresh::Partial);
        }
        r
    }

    pub(crate) fn options_touch(
        &mut self,
        phase: Phase,
        x: i32,
        y: i32,
    ) -> Option<Option<Refresh>> {
        match phase {
            Phase::Down => {
                let (lab, ch) = self.screen.options_slider_at(&self.options_model(), x, y)?;
                self.slider_touch = Some(lab);
                self.settings_focus = Some(lab);
                // UI scale rescales everything: only the thumb moves while
                // the finger is down, the scale changes on release.
                if let Some(Change::UiScaleTo(v)) = ch {
                    self.ui_scale_drag = Some(v);
                    return Some(Some(Refresh::Partial));
                }
                Some(Some(match ch {
                    Some(c) => self.change(c),
                    None => Refresh::Partial,
                }))
            }
            Phase::Move | Phase::Up => {
                let lab = self.slider_touch?;
                if phase == Phase::Up {
                    self.slider_touch = None;
                }
                let ch = self
                    .screen
                    .options_slider_drag(&self.options_model(), lab, x);
                if lab == "UI scale" {
                    let Some(Change::UiScaleTo(v)) = ch else {
                        return Some(None);
                    };
                    if phase == Phase::Up {
                        self.ui_scale_drag = None;
                        return Some(Some(if v == self.settings.ui_scale {
                            Refresh::Partial
                        } else {
                            self.change(Change::UiScaleTo(v))
                        }));
                    }
                    let moved = self.ui_scale_drag != Some(v);
                    self.ui_scale_drag = Some(v);
                    return Some(moved.then_some(Refresh::Partial));
                }
                Some(match ch {
                    Some(Change::KbSize(v)) if v == self.settings.kb_size => None,
                    Some(Change::SizeTo(v)) if (v - self.settings.size).abs() < 0.05 => None,
                    Some(Change::LeadingTo(v)) if (v - self.settings.leading).abs() < 0.05 => None,
                    Some(Change::MarginTo(v)) if (v - self.settings.margin).abs() < 0.05 => None,
                    Some(Change::UiScaleTo(v)) if v == self.settings.ui_scale => None,
                    // The same light step again (every drag event repeats it): no
                    // redraw, no sysfs write, no log line.
                    Some(Change::Backlight(n)) if n == self.light_step => None,
                    Some(c) => Some(self.change(c)),
                    None => None,
                })
            }
        }
    }

    fn options_hit_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        match self.screen.options_hit(&self.options_model(), x, y) {
            OptionsHit::Close => {
                self.options = false;
                Some(Refresh::Full)
            }
            OptionsHit::Change(c) => Some(self.change(c)),
            OptionsHit::Back => Some(self.settings_back()),
            OptionsHit::Page => {
                let n = self.screen.options_layout(&self.options_model()).pages;
                self.settings_sub = (self.settings_sub + 1) % n.max(1);
                Some(Refresh::Full)
            }
            OptionsHit::Open(p) => {
                self.settings_page = p;
                self.settings_sub = 0;
                Some(Refresh::Full)
            }
            OptionsHit::Pick(c) => {
                self.change(c);
                Some(self.settings_back())
            }
            OptionsHit::Help(i) => {
                self.options = false;
                self.open_help();
                if let Some(h) = self.help.as_mut() {
                    h.page = sigma_render::help::MENU[i].2;
                }
                Some(Refresh::Full)
            }
            OptionsHit::Nothing => None,
        }
    }

    /// Puts the USB port in host mode while Options → USB keyboard is On,
    /// and back when it is turned off (v0.8). A failure is logged and
    /// retried only when the setting changes again.
    pub fn sync_usb_host(&mut self, dev: &mut dyn Device) {
        if self.settings.usb != self.usb_host {
            self.usb_host = self.settings.usb;
            if let Err(e) = dev.usb_host(self.usb_host) {
                sigma_core::error!("usb", "{e}");
            }
        }
    }

    /// Reads the front light once, then sends the step Display → Backlight
    /// asked for (v1.12). A failure is logged; the slider keeps its place.
    pub fn sync_light(&mut self, dev: &mut dyn Device) {
        if !self.light_probed {
            self.light_probed = true;
            if let Some(n) = dev.light_step() {
                self.light_have = true;
                self.light_step = n;
            }
        }
        if let Some(n) = self.light_want.take() {
            if let Err(e) = dev.set_light(n) {
                sigma_core::error!("light", "{e}");
            }
        }
    }
}
