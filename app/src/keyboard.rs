//! The on-screen keyboard: its bar buttons, chords and what follows a key.

use crate::App;
use sigma_core::keyboard::{Dir, Key};
use sigma_core::settings::{Pad, Side};
use sigma_core::{Change, KeyAction};
use sigma_device::{HwKey, Mods, Refresh};
use sigma_render::KbButton;

impl App {
    pub(crate) fn kb_button(&mut self, b: KbButton) -> Option<Refresh> {
        let c = match b {
            KbButton::Mode(m) if m == self.settings.land_keys => return None,
            KbButton::Mode(m) => Change::LandKeys(m),
            KbButton::PadToggle => Change::Pad(match self.settings.pad {
                Pad::Numbers => Pad::Arrows,
                Pad::Arrows => Pad::Numbers,
            }),
            KbButton::PadSide => Change::PadSide(match self.settings.pad_side {
                Side::Left => Side::Right,
                Side::Right => Side::Left,
            }),
        };
        Some(self.change(c))
    }

    /// A physical key with modifiers (USB keyboard, v0.8). Routed through
    /// the on-screen keyboard's own key and shortcut code (S2.5) with the
    /// chord's modifiers, so Ctrl+Z/Y/A/C/X/V/F and Shift/Ctrl+arrows behave
    /// exactly like their on-screen versions. The on-screen modifier state is
    /// restored afterwards. Sheets ignore chords; Ctrl+F in preview opens Find.
    pub(crate) fn chord(&mut self, key: HwKey, mods: Mods) -> Option<Refresh> {
        let ctrl_f = mods.ctrl && key == HwKey::Char('f');
        // Alt+Left = Back (v1.0.2), in write mode and preview.
        if mods.alt && !mods.ctrl && key == HwKey::Left && !self.history.is_empty() {
            if self.options
                || self.help.is_some()
                || self.graph.is_some()
                || self.plugin_screen.is_some()
                || self.files.is_some()
            {
                return None;
            }
            return self.go_back();
        }
        if self.options
            || self.help.is_some()
            || self.graph.is_some()
            || self.plugin_screen.is_some()
            || self.files.is_some()
        {
            return None;
        }
        if self.preview.is_some() {
            return if ctrl_f { Some(self.open_find()) } else { None };
        }
        let k = match key {
            HwKey::Char(c) => Key::Char(c.to_ascii_lowercase()),
            HwKey::Backspace => Key::Back,
            HwKey::Delete => Key::Del,
            HwKey::Return => Key::Return,
            HwKey::Tab => Key::Tab,
            HwKey::Left => Key::Arrow(Dir::Left),
            HwKey::Right => Key::Arrow(Dir::Right),
            HwKey::Up => Key::Arrow(Dir::Up),
            HwKey::Down => Key::Arrow(Dir::Down),
            HwKey::Escape => Key::Esc,
            HwKey::Home => Key::Home,
            HwKey::End => Key::End,
            HwKey::PageUp => Key::PgUp,
            HwKey::PageDown => Key::PgDn,
            HwKey::Insert => {
                if self.rename.is_some() || self.find_typing() || self.scratch_focused() {
                    return None;
                }
                self.buffer.toggle_overwrite();
                return Some(Refresh::Partial);
            }
        };
        let kb = &mut self.keyboard;
        let saved = (kb.shift, kb.ctrl, kb.alt);
        (kb.shift, kb.ctrl, kb.alt) = (mods.shift, mods.ctrl, mods.alt);
        let r = if self.rename.is_some() {
            self.type_into_field(k)
        } else if self.find_typing() && !ctrl_f {
            self.find_key(k)
        } else if self.scratch_focused() {
            self.scratch_key(k)
        } else {
            let tab = self.settings.tab_kind();
            self.keyboard.rtl = self.caret_rtl();
            let a = self.keyboard.press(k, &mut self.buffer, tab);
            self.after_key(a)
        };
        let kb = &mut self.keyboard;
        (kb.shift, kb.ctrl, kb.alt) = saved;
        r
    }

    pub(crate) fn after_key(&mut self, action: KeyAction) -> Option<Refresh> {
        let page = self.screen.layout(self.keys_visible(), &self.keyboard).page;
        match action {
            KeyAction::Edited => {
                self.save();
                Some(Refresh::Fast)
            }
            KeyAction::Moved | KeyAction::Relabelled => Some(Refresh::Fast),
            KeyAction::Vertical { down, extend } => {
                self.screen
                    .caret_row_ext(&mut self.buffer, page, down, extend);
                Some(Refresh::Fast)
            }
            KeyAction::Page { forward, extend } => {
                self.screen
                    .page_turn_ext(&mut self.buffer, page, forward, extend);
                Some(Refresh::Partial)
            }
            KeyAction::Relayout => {
                self.save_settings();
                Some(Refresh::Full)
            }
            KeyAction::Find => Some(self.open_find()),
            KeyAction::Escape if self.find.is_some() => {
                self.close_find();
                Some(Refresh::Full)
            }
            KeyAction::Escape | KeyAction::Ignored => None,
        }
    }

    pub(crate) fn keys_visible(&self) -> bool {
        self.rename.is_some()
            || (self.files.is_none()
                && self.preview.is_none()
                && !self.options
                && self.graph.is_none()
                && self.plugin_screen.is_none()
                && self.help.is_none()
                && self.keys_on)
    }

    /// The caret is in a right-to-left row of the note.
    pub(crate) fn caret_rtl(&self) -> bool {
        let page = self.screen.layout(self.keys_visible(), &self.keyboard).page;
        self.screen.caret_rtl(&self.buffer, page)
    }
}
