//! The ink panel and handwriting recognition.

use crate::App;
use sigma_core::Change;
use sigma_device::{Mods, Refresh};
use sigma_render::Rect;

mod gestures;
mod recognition;
mod state;
mod training;

pub(crate) use state::ReadInk;
pub use state::{InkState, INK_CLEANUP_MS, LETTER_MS, WORD_MS};

impl App {
    /// Opens the writing panel (v1.5.0).
    pub fn open_ink(&mut self) {
        self.sel_ui = None;
        self.screen.ink_words = self.settings.word_mode();
        self.panel_prefs();
        // v1.5.3 (decision 55): the free writing area is the main input;
        // boxes only with Settings → Letter boxes; Draw is one ⇄ away.
        self.ink = Some(InkState {
            mode: if self.settings.ink_boxes {
                sigma_render::ink::InkMode::Letters
            } else {
                sigma_render::ink::InkMode::Free
            },
            ..InkState::default()
        });
    }

    /// The panel's saved choices: the dotted guide and the key block's side.
    pub(crate) fn panel_prefs(&mut self) {
        self.screen.sketch_guide = self.settings.ink_guide;
        self.screen.ink_keys_right = self.settings.ink_keys == sigma_core::settings::Side::Right;
    }

    /// The live-ink refresh: DU, or A2 when Settings → Fast ink is on.
    #[must_use]
    pub fn ink_refresh(&self) -> Refresh {
        if self.settings.fast_ink() {
            Refresh::InkA2
        } else {
            Refresh::Ink
        }
    }

    /// A panel button (touch Up over it, or a Tap).
    pub(crate) fn ink_button(&mut self, b: sigma_render::ink::InkButton) -> Option<Refresh> {
        use sigma_render::ink::{InkButton, InkMode};
        let mode = self.ink.as_ref()?.mode;
        match (mode, b) {
            // Every mode (v1.16): Line shows or hides the dotted guide, ⇄
            // moves the key block to the other side; both are saved.
            (_, InkButton::Guide) => {
                self.change(sigma_core::settings::Change::InkGuide(
                    !self.settings.ink_guide,
                ));
                return Some(Refresh::Partial);
            }
            (_, InkButton::Side) => {
                use sigma_core::settings::Side;
                let other = match self.settings.ink_keys {
                    Side::Left => Side::Right,
                    Side::Right => Side::Left,
                };
                self.change(sigma_core::settings::Change::InkKeys(other));
                return Some(Refresh::Partial);
            }
            (InkMode::Draw, InkButton::Mode) => {
                let boxes = self.settings.ink_boxes;
                let ink = self.ink.as_mut()?;
                ink.ink = sigma_core::ink::Ink::default();
                ink.mode = if boxes {
                    InkMode::Letters
                } else {
                    InkMode::Free
                };
                ink.boxi = None;
                return Some(Refresh::Partial);
            }
            (InkMode::Letters | InkMode::Free, InkButton::Mode) => {
                let _ = self.read_letter();
                let ink = self.ink.as_mut()?;
                ink.mode = InkMode::Draw;
                ink.cands.clear();
                ink.replace_ok = false;
                return Some(Refresh::Partial);
            }
            (InkMode::Free, InkButton::Cand(k)) => {
                let ink = self.ink.as_mut()?;
                let w = ink.cands.get(k)?.clone();
                if !ink.replace_ok || k == 0 {
                    return None;
                }
                let typed = ink.cands.first()?.clone();
                self.learn_pick(&typed, &w);
                let ink = self.ink.as_mut()?;
                let n = ink.last_len;
                ink.cands.swap(0, k);
                let typed = format!("{w} ");
                ink.last_len = typed.chars().count();
                for _ in 0..n {
                    self.buffer.backspace();
                }
                self.buffer.insert(&typed);
                self.save();
                return Some(Refresh::Partial);
            }
            (InkMode::Free, InkButton::Space) => return self.letters_type(" ", false),
            (InkMode::Free, InkButton::Type(ch)) => {
                return self.letters_type(&ch.to_string(), false);
            }
            (InkMode::Free, InkButton::Digits) => {
                let ink = self.ink.as_mut()?;
                ink.digits = !ink.digits;
                return Some(Refresh::Partial);
            }
            (InkMode::Free, InkButton::Lang) => {
                let ink = self.ink.as_mut()?;
                ink.lang = match ink.lang {
                    None => Some("en"),
                    Some("en") => Some("fr"),
                    Some("fr") => Some("pt"),
                    _ => None,
                };
                return Some(Refresh::Partial);
            }
            (InkMode::Free, InkButton::Prev | InkButton::Next) => {
                let l = self.panel_layout()?;
                let shown = l
                    .buttons
                    .iter()
                    .filter(|(_, b)| matches!(b, InkButton::Cand(_)))
                    .count()
                    .max(1);
                let ink = self.ink.as_mut()?;
                let old = ink.first;
                ink.first = if b == InkButton::Next {
                    if ink.first + shown < ink.cands.len() {
                        ink.first + shown
                    } else {
                        ink.first
                    }
                } else {
                    ink.first.saturating_sub(shown)
                };
                return (ink.first != old).then_some(Refresh::Partial);
            }
            (InkMode::Free, InkButton::Enter) => return self.letters_type("\n", false),
            (InkMode::Free, InkButton::Back) => return self.letters_type("", true),
            (InkMode::Free, InkButton::Cancel) => {
                let _ = self.read_letter();
                self.ink = None;
                return Some(Refresh::Partial);
            }
            (InkMode::Free, InkButton::Undo) => return self.ink_undo(true),
            (InkMode::Letters | InkMode::Free, InkButton::Redo) => {
                let _ = self.read_letter();
                if !self.buffer.redo() {
                    return None;
                }
                if let Some(ink) = &mut self.ink {
                    ink.cands.clear();
                    ink.replace_ok = false;
                }
                self.save();
                return Some(Refresh::Partial);
            }
            (
                InkMode::Letters | InkMode::Free,
                InkButton::Home | InkButton::End | InkButton::PgUp | InkButton::PgDn,
            ) => {
                use sigma_device::HwKey;
                let _ = self.read_letter();
                if let Some(ink) = &mut self.ink {
                    // The caret moves on: the last reading is no longer at it.
                    ink.cands.clear();
                    ink.replace_ok = false;
                }
                let key = match b {
                    InkButton::Home => HwKey::Home,
                    InkButton::End => HwKey::End,
                    InkButton::PgUp => HwKey::PageUp,
                    _ => HwKey::PageDown,
                };
                let _ = self.chord(key, Mods::default());
                return Some(Refresh::Partial);
            }
            (InkMode::Letters | InkMode::Free, InkButton::Ins) => {
                let _ = self.read_letter();
                self.buffer.toggle_overwrite();
                return Some(Refresh::Partial);
            }
            (InkMode::Letters, InkButton::Cancel) => {
                let _ = self.read_letter();
                self.ink = None;
                return Some(Refresh::Partial);
            }
            (InkMode::Letters, InkButton::Space) => return self.letters_type(" ", false),
            (InkMode::Letters, InkButton::Enter) => return self.letters_type("\n", false),
            (InkMode::Letters, InkButton::Back) => return self.letters_type("", true),
            (InkMode::Letters, InkButton::Cand(k)) => {
                let ink = self.ink.as_mut()?;
                let c = ink.cands.get(k)?.clone();
                if !ink.replace_ok || k == 0 {
                    return None;
                }
                let typed = ink.cands.first()?.clone();
                self.learn_pick(&typed, &c);
                let ink = self.ink.as_mut()?;
                ink.cands.swap(0, k);
                self.buffer.backspace();
                self.buffer.insert(&c);
                self.save();
                return Some(Refresh::Partial);
            }
            (InkMode::Letters | InkMode::Free, InkButton::Train) => {
                let _ = self.read_letter();
                let ink = self.ink.as_mut()?;
                ink.mode = InkMode::Train(0);
                ink.saved = 0;
                ink.cands.clear();
                return Some(Refresh::Partial);
            }
            (InkMode::Train(_), InkButton::Save) => return self.save_sample(),
            (InkMode::Train(i), InkButton::Skip) => {
                let ink = self.ink.as_mut()?;
                ink.ink = sigma_core::ink::Ink::default();
                ink.boxi = None;
                ink.mode = InkMode::Train((i + 1) % sigma_hwr::CHARSET.chars().count());
                return Some(Refresh::Partial);
            }
            (InkMode::Train(_), InkButton::Cancel) => {
                let ink = self.ink.as_mut()?;
                let saved = ink.saved;
                ink.ink = sigma_core::ink::Ink::default();
                ink.boxi = None;
                ink.mode = InkMode::Letters;
                if !self.settings.ink_boxes {
                    ink.mode = InkMode::Free;
                }
                if saved > 0 {
                    // Fine-tune on the new samples (background).
                    self.hwr.load(self.library.root());
                }
                return Some(Refresh::Partial);
            }
            (InkMode::Letters, InkButton::Undo) => return self.ink_undo(true),
            (InkMode::Train(_), InkButton::Undo) => return self.ink_undo(false),
            (InkMode::Draw, _) => {}
            _ => return None,
        }
        let ink = self.ink.as_mut()?;
        match b {
            InkButton::Undo => ink.ink.undo().then_some(Refresh::Partial),
            InkButton::Clear => ink.ink.clear().then_some(Refresh::Partial),
            InkButton::Pen => {
                let next = match self.settings.ink_pen {
                    sigma_core::ink::Pen::Finger => sigma_core::ink::Pen::Stylus,
                    sigma_core::ink::Pen::Stylus => sigma_core::ink::Pen::Finger,
                };
                Some(self.change(Change::InkPen(next)))
            }
            InkButton::Cancel => {
                self.ink = None;
                Some(Refresh::Partial)
            }
            InkButton::Insert => Some(self.insert_ink()),
            _ => None,
        }
    }

    /// After a stroke and `INK_CLEANUP_MS` idle: one GL16 pass over what
    /// was inked (plan §12.4).
    pub(crate) fn ink_tick(&mut self, t: u64) -> Option<Refresh> {
        let ink = self.ink.as_mut()?;
        let wait = match ink.mode {
            sigma_render::ink::InkMode::Letters => Some(LETTER_MS),
            sigma_render::ink::InkMode::Free => Some(WORD_MS),
            _ => None,
        };
        if let Some(ms) = wait {
            if ink.boxi.is_some()
                && ink.ink.live.is_none()
                && ink.last_up.is_some_and(|u| t >= u + ms)
            {
                return self.read_letter();
            }
        }
        if wait.is_some() && ink.boxi.is_some() {
            // The read redraws the panel; no separate cleanup.
            return None;
        }
        let up = ink.last_up?;
        if ink.ink.live.is_some() || ink.dirty.is_none() || t < up + INK_CLEANUP_MS {
            return None;
        }
        ink.last_up = None;
        ink.cleanup = true;
        Some(Refresh::PointerCleanup)
    }

    /// Insert: writes the SVG and strokes file next to the note and puts
    /// the image line at the caret, on its own line.
    fn insert_ink(&mut self) -> Refresh {
        let Some(st) = self.ink.take() else {
            return Refresh::Partial;
        };
        if st.ink.strokes.is_empty() {
            return Refresh::Partial;
        }
        let note = self.library.root().join(self.current.as_str());
        match sigma_core::ink::save(&st.ink, self.settings.ink_pen, &note) {
            Ok(Some(md)) => {
                let text = self.buffer.text();
                let at = self.buffer.caret();
                // A block of its own (blank lines around it), so preview
                // draws it (v1.5.1).
                let (head, tail) = (&text[..at], &text[at..]);
                let before = if head.is_empty() || head.ends_with("\n\n") {
                    ""
                } else if head.ends_with('\n') {
                    "\n"
                } else {
                    "\n\n"
                };
                let after = if tail.starts_with("\n\n") {
                    ""
                } else if tail.starts_with('\n') || tail.is_empty() {
                    "\n"
                } else {
                    "\n\n"
                };
                self.buffer.insert(&format!("{before}{md}{after}"));
                self.save();
                Refresh::Partial
            }
            Ok(None) => Refresh::Partial,
            Err(e) => {
                sigma_core::error!("ink", "save failed: {e}");
                self.ink = Some(InkState {
                    error: Some(e.to_string()),
                    ..st
                });
                Refresh::Partial
            }
        }
    }

    /// What the open panel shows (v1.5.3).
    pub(crate) fn ink_view(&self) -> Option<sigma_render::ink::LettersView> {
        let ink = self.ink.as_ref()?;
        Some(sigma_render::ink::LettersView {
            cands: ink.cands.clone(),
            saved: ink.saved,
            first: ink.first,
            digits: ink.digits,
            lang: ink.lang,
        })
    }

    /// The open panel's layout.
    #[must_use]
    pub fn panel_layout(&self) -> Option<sigma_render::ink::InkLayout> {
        let v = self.ink_view()?;
        Some(self.screen.ink_layout_for(self.ink.as_ref()?.mode, &v))
    }

    /// Letter box `i` of the open panel (emulator scripts, v1.5.2).
    #[must_use]
    pub fn letter_box(&self, i: usize) -> Option<Rect> {
        self.panel_layout()?.boxes.get(i).copied()
    }
}
