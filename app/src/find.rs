//! The Find and replace bar.

use super::*;

impl App {
    pub(crate) fn find_typing(&self) -> bool {
        self.in_editor() && self.find.as_ref().is_some_and(|f| f.focus.is_some())
    }

    pub(crate) fn in_find_bar(&self, x: i32, y: i32) -> bool {
        self.in_editor() && self.find.is_some() && self.page_layout().find_bar.contains(x, y)
    }

    /// Opens the Find bar (Ctrl+F or the magnifier). A one-line selection
    /// seeds the query. Full refresh: the page moves down.
    pub fn open_find(&mut self) -> Refresh {
        let seed = self
            .buffer
            .selected_text()
            .filter(|t| !t.contains('\n'))
            .map(str::to_owned);
        let f = self.find.get_or_insert_with(FindState::default);
        if let Some(t) = seed {
            f.query = Buffer::new(t);
        }
        f.focus = Some(FindField::Find);
        self.scratch_focus = false;
        self.sel_ui = None;
        self.screen.find_open = true;
        // Show the seeded / previous query's match at the caret.
        let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
        self.select_match(sigma_core::find::at_or_after(&self.find_matches(), from));
        Refresh::Full
    }

    pub fn close_find(&mut self) {
        self.find = None;
        self.screen.find_open = false;
    }

    /// Matches of the query in the open note.
    #[must_use]
    pub fn find_matches(&self) -> Vec<(usize, usize)> {
        self.find.as_ref().map_or_else(Vec::new, |f| {
            sigma_core::find::matches(self.buffer.text(), f.query.text(), f.case)
        })
    }

    /// `Find`, `None`, or `i of n`.
    #[must_use]
    pub fn find_label(&self) -> String {
        let ms = self.find_matches();
        let q = self.find.as_ref().map_or("", |f| f.query.text());
        let cur = sigma_core::find::selected(&ms, self.buffer.selection());
        sigma_core::find::count_label(q, &ms, cur)
    }

    fn select_match(&mut self, i: Option<usize>) {
        if let Some(&(a, b)) = i.and_then(|i| self.find_matches().get(i).copied()).as_ref() {
            self.buffer.select(a, b);
            self.screen.follow_caret(&self.buffer, self.page_rect());
        }
    }

    fn find_step(&mut self, forward: bool) -> Option<Refresh> {
        let ms = self.find_matches();
        let (a, b) = self
            .buffer
            .selection()
            .unwrap_or((self.buffer.caret(), self.buffer.caret()));
        let i = if forward {
            // After the selected match, or from the caret.
            let from = if sigma_core::find::selected(&ms, Some((a, b))).is_some() {
                b.max(a + 1)
            } else {
                a
            };
            sigma_core::find::next(&ms, from)
        } else {
            sigma_core::find::prev(&ms, a)
        };
        i?;
        self.select_match(i);
        Some(Refresh::Partial)
    }

    fn find_replace_one(&mut self) -> Option<Refresh> {
        let ms = self.find_matches();
        if sigma_core::find::selected(&ms, self.buffer.selection()).is_none() {
            return self.find_step(true);
        }
        let with = self.find.as_ref()?.replace.text().to_owned();
        // An empty replacement deletes the match (still one undo step).
        self.buffer.insert(&with);
        self.save();
        let at = self.buffer.caret();
        let next = sigma_core::find::at_or_after(&self.find_matches(), at);
        self.select_match(next);
        Some(Refresh::Partial)
    }

    fn find_replace_all(&mut self) -> Option<Refresh> {
        let ms = self.find_matches();
        if ms.is_empty() {
            return None;
        }
        let with = self.find.as_ref()?.replace.text().to_owned();
        let text = sigma_core::find::replace_all(self.buffer.text(), &ms, &with);
        // Keep the caret near where it was: count the bytes the replaced
        // matches before it gained or lost.
        let at = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
        let shift: isize = ms
            .iter()
            .filter(|m| m.1 <= at)
            .map(|m| with.len() as isize - (m.1 - m.0) as isize)
            .sum();
        let caret = (at as isize + shift).max(0) as usize;
        self.buffer.replace_text(text, caret);
        self.save();
        self.screen.follow_caret(&self.buffer, self.page_rect());
        Some(Refresh::Partial)
    }

    pub(crate) fn find_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.page_layout();
        match self.screen.find_hit(&l, x, y) {
            FindHit::Field(fld) => {
                self.scratch_focus = false;
                let f = self.find.as_mut()?;
                f.focus = Some(fld);
                let b = match fld {
                    FindField::Find => &mut f.query,
                    FindField::Replace => &mut f.replace,
                };
                let len = b.text().len();
                b.set_caret(len);
                Some(Refresh::Fast)
            }
            FindHit::Close => {
                self.close_find();
                Some(Refresh::Full)
            }
            FindHit::Prev => self.find_step(false),
            FindHit::Next => self.find_step(true),
            FindHit::Replace => self.find_replace_one(),
            FindHit::All => self.find_replace_all(),
            FindHit::Case => {
                let f = self.find.as_mut()?;
                f.case = !f.case;
                let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
                self.select_match(sigma_core::find::at_or_after(&self.find_matches(), from));
                Some(Refresh::Partial)
            }
            FindHit::Bar | FindHit::Outside => None,
        }
    }

    /// After the query changed: select the first match from the start of
    /// the current selection (incremental search).
    fn query_changed(&mut self) {
        let from = self.buffer.selection().map_or(self.buffer.caret(), |s| s.0);
        let ms = self.find_matches();
        match sigma_core::find::at_or_after(&ms, from) {
            Some(i) => self.select_match(Some(i)),
            None => {
                let c = self.buffer.caret().min(from);
                self.buffer.set_caret(c);
            }
        }
    }

    fn find_field(&mut self) -> Option<(&mut Buffer, FindField)> {
        let f = self.find.as_mut()?;
        let fld = f.focus?;
        Some((
            match fld {
                FindField::Find => &mut f.query,
                FindField::Replace => &mut f.replace,
            },
            fld,
        ))
    }

    /// On-screen key while a Find field has focus.
    pub(crate) fn find_key(&mut self, k: Key) -> Option<Refresh> {
        match k {
            Key::Return => return self.find_step(true),
            Key::Esc => {
                self.close_find();
                return Some(Refresh::Full);
            }
            Key::Tab => {
                let f = self.find.as_mut()?;
                f.focus = Some(match f.focus {
                    Some(FindField::Find) => FindField::Replace,
                    _ => FindField::Find,
                });
                return Some(Refresh::Fast);
            }
            Key::PgUp | Key::PgDn | Key::Arrow(Dir::Up | Dir::Down) => return None,
            _ => {}
        }
        let mut kb = std::mem::take(&mut self.keyboard);
        let (field, fld) = self.find_field()?;
        let before = field.revision();
        let action = kb.press(k, field, TabKind::Spaces(0));
        let edited = field.revision() != before;
        self.keyboard = kb;
        if action == KeyAction::Relayout {
            self.save_settings();
            return Some(Refresh::Full);
        }
        if edited && fld == FindField::Find {
            self.query_changed();
            return Some(Refresh::Partial);
        }
        Some(Refresh::Fast)
    }

    /// Hardware (USB) key while a Find field has focus.
    pub(crate) fn find_hw(&mut self, k: HwKey) -> Option<Refresh> {
        let key = match k {
            HwKey::Char(c) => Key::Char(c),
            HwKey::Backspace => Key::Back,
            HwKey::Delete => Key::Del,
            HwKey::Return => Key::Return,
            HwKey::Tab => Key::Tab,
            HwKey::Left => Key::Arrow(Dir::Left),
            HwKey::Right => Key::Arrow(Dir::Right),
            HwKey::Up
            | HwKey::Down
            | HwKey::Home
            | HwKey::End
            | HwKey::PageUp
            | HwKey::PageDown
            | HwKey::Insert => return None,
            HwKey::Escape => Key::Esc,
        };
        if let Key::Char(c) = key {
            let (field, fld) = self.find_field()?;
            field.insert(&c.to_string());
            if fld == FindField::Find {
                self.query_changed();
                return Some(Refresh::Partial);
            }
            return Some(Refresh::Fast);
        }
        self.find_key(key)
    }
}
