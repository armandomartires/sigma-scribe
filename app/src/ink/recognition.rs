//! Reading the ink: letter boxes, words, candidates and undo.

use crate::App;
use sigma_device::Refresh;

impl App {
    /// The pending box's strokes in box units.
    pub(super) fn letter_strokes(&self) -> Option<Vec<Vec<(f32, f32)>>> {
        let ink = self.ink.as_ref()?;
        let l = self.panel_layout()?;
        let b = *l.boxes.get(ink.boxi?)?;
        let v: Vec<Vec<(f32, f32)>> = ink
            .ink
            .strokes
            .iter()
            .map(|s| {
                s.points
                    .iter()
                    .map(|p| {
                        if ink.mode == sigma_render::ink::InkMode::Free {
                            // Free: panel pixels; the reader sizes the ink.
                            (p.x as f32, p.y as f32)
                        } else {
                            sigma_render::ink::box_units(b, p.x, p.y)
                        }
                    })
                    .collect()
            })
            .filter(|s: &Vec<(f32, f32)>| !s.is_empty())
            .collect();
        (!v.is_empty()).then_some(v)
    }

    /// Letters: reads the pending box, types the best character at the
    /// caret and offers the rest as candidates; the box empties.
    pub(super) fn read_letter(&mut self) -> Option<Refresh> {
        if self.ink.as_ref()?.mode == sigma_render::ink::InkMode::Free {
            return self.read_word();
        }
        let strokes = self.letter_strokes();
        let ink = self.ink.as_mut()?;
        ink.boxi = None;
        ink.ink = sigma_core::ink::Ink::default();
        ink.last_up = None;
        let strokes = strokes?;
        if self.hwr.rec.is_none() {
            self.hwr.load(self.library.root());
        }
        self.hwr.poll(false);
        let top = self.hwr.rec.as_ref()?.recognise(&strokes, 5);
        let best = top.first()?.0;
        self.buffer.insert(&best.to_string());
        self.save();
        let ink = self.ink.as_mut()?;
        ink.cands = top.iter().map(|(c, _)| c.to_string()).collect();
        ink.replace_ok = true;
        ink.read_ink = Some(crate::ReadInk::Letter(strokes));
        ink.learned = None;
        Some(Refresh::Partial)
    }

    /// Free (v1.5.3): reads all ink in the area (characters split at the
    /// gaps, decision 55), types it and a space at the caret; the reading
    /// and its alternatives go to the candidate bar.
    pub(super) fn read_word(&mut self) -> Option<Refresh> {
        let strokes = self.letter_strokes();
        let ink = self.ink.as_mut()?;
        ink.boxi = None;
        ink.ink = sigma_core::ink::Ink::default();
        ink.last_up = None;
        let strokes = strokes?;
        if self.hwr.rec.is_none() {
            self.hwr.load(self.library.root());
        }
        self.hwr.poll(false);
        let rec = self.hwr.rec.as_ref()?;
        if self.ink.as_ref()?.lex.is_none() {
            let mut words = sigma_hwr::word::builtin_words();
            let mut text = self.buffer.text().to_owned();
            text.push('\n');
            text.push_str(self.scratch.text());
            words.extend(sigma_hwr::word::Lexicon::words_in(&text, 3000));
            let lex = sigma_hwr::word::Lexicon::new(rec, &words);
            self.ink.as_mut()?.lex = Some(lex);
        }
        let ink = self.ink.as_ref()?;
        let per: Vec<Vec<sigma_hwr::Seg>> = strokes
            .iter()
            .map(|s| sigma_hwr::segments(std::slice::from_ref(s)))
            .collect();
        let allow = sigma_hwr::free::allow_lang(rec, ink.lang, ink.digits);
        // Word mode (experimental) reads every word with the lexicon;
        // otherwise only touching letters do.
        let wide = if self.settings.word_mode() {
            0.0
        } else {
            sigma_hwr::free::WIDE
        };
        let lex = (!ink.digits).then_some(ink.lex.as_ref()).flatten();
        let r = sigma_hwr::free::read_lex(rec, &per, allow.as_deref(), lex, wide);
        if r.text.is_empty() {
            return None;
        }
        let mut top: Vec<(String, f32)> = vec![(r.text.clone(), 0.0)];
        top.extend(r.alts.iter().map(|a| (a.clone(), 0.0)));
        let read = crate::ReadInk::Free(r);
        let best = top.first()?.0.clone();
        let typed = if best.ends_with(|c: char| c.is_alphanumeric()) {
            format!("{best} ")
        } else {
            best.clone()
        };
        self.buffer.insert(&typed);
        self.save();
        let ink = self.ink.as_mut()?;
        ink.last_len = typed.chars().count();
        ink.cands = top.into_iter().map(|(w, _)| w).collect();
        ink.first = 0;
        ink.replace_ok = true;
        ink.read_ink = Some(read);
        ink.learned = None;
        Some(Refresh::Partial)
    }

    /// Undo in the Letters and Free panels (v1.11.2): the last stroke that
    /// is still unread; with none pending (the reading is already in the
    /// note) it takes back what was last typed, as Ctrl+Z does. `text` off
    /// (Train) leaves the note alone.
    pub(super) fn ink_undo(&mut self, text: bool) -> Option<Refresh> {
        let ink = self.ink.as_mut()?;
        if ink.ink.undo() {
            if ink.ink.strokes.is_empty() {
                ink.boxi = None;
            }
            return Some(Refresh::Partial);
        }
        if !text || !self.buffer.undo() {
            return None;
        }
        ink.cands.clear();
        ink.replace_ok = false;
        self.save();
        Some(Refresh::Partial)
    }

    /// Types `s` from the letters panel (reads a pending box first).
    pub(super) fn letters_type(&mut self, s: &str, back: bool) -> Option<Refresh> {
        let words = self.ink.as_ref()?.mode == sigma_render::ink::InkMode::Free;
        let read = self.read_letter().is_some();
        if words && read && s == " " {
            // The word already brought its space.
            if let Some(ink) = &mut self.ink {
                ink.replace_ok = false;
            }
            return Some(Refresh::Partial);
        }
        if back {
            self.buffer.backspace();
        } else {
            self.buffer.insert(s);
        }
        self.save();
        if let Some(ink) = &mut self.ink {
            ink.cands.clear();
            ink.replace_ok = false;
        }
        Some(Refresh::Partial)
    }
}
