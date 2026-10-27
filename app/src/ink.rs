//! The ink panel and handwriting recognition.

use super::*;

/// A stroke's points in panel pixels (v1.5.4 gestures).
fn stroke_pts(s: &sigma_core::ink::Stroke) -> Vec<(f32, f32)> {
    s.points.iter().map(|p| (p.x as f32, p.y as f32)).collect()
}

/// Bounding box `(x0, y0, x1, y1)` of points.
fn pts_box(p: &[(f32, f32)]) -> (f32, f32, f32, f32) {
    p.iter()
        .fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, &(x, y)| {
            (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y))
        })
}

impl App {
    /// Opens the writing panel (v1.5.0).
    pub fn open_ink(&mut self) {
        self.sel_ui = None;
        self.screen.ink_words = self.settings.word_mode();
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

    /// The live-ink refresh: DU, or A2 when Settings → Fast ink is on.
    #[must_use]
    pub fn ink_refresh(&self) -> Refresh {
        if self.settings.fast_ink() {
            Refresh::InkA2
        } else {
            Refresh::Ink
        }
    }

    /// Touches while the panel is open: the panel owns every touch
    /// (`Some`); outside it they do nothing.
    pub(crate) fn ink_touch(
        &mut self,
        phase: Phase,
        x: i32,
        y: i32,
        t: u64,
    ) -> Option<Option<Refresh>> {
        use sigma_core::ink::Point;
        use sigma_render::ink::{draw_segment, pen_px};
        let mode = self.ink.as_ref()?.mode;
        let l = self.panel_layout()?;
        // v1.5.4 caret tap: a tap on the note above the panel moves the
        // caret there (pending ink is read first); the panel stays open.
        if !l.panel.contains(x, y) && self.ink.as_ref()?.ink.live.is_none() {
            return Some(self.caret_tap(phase, x, y));
        }
        if mode != sigma_render::ink::InkMode::Draw {
            return Some(self.letters_touch(phase, x, y, t, &l));
        }
        let width = pen_px(self.settings.ink_pen);
        let refresh = self.ink_refresh();
        // Contact size when the panel reports it (v1.5.1); else None.
        let major = sigma_device::evdev::contact_major();
        let p = Point { x, y, t, major };
        let ink = self.ink.as_mut()?;
        let add = |r: Option<Rect>, n: Rect| Some(r.map_or(n, |o| o.union(&n)));
        match phase {
            Phase::Down => {
                if l.area.contains(x, y) {
                    ink.ink.down(p);
                    let r = draw_segment(&mut self.canvas, p, p, width, l.area);
                    ink.pending = add(ink.pending, r);
                    ink.dirty = add(ink.dirty, r);
                    ink.fast = true;
                    ink.last_up = None;
                    return Some(Some(refresh));
                }
                ink.press = self.screen.ink_hit(&l, x, y);
                Some(None)
            }
            Phase::Move => {
                if ink.ink.live.is_none() {
                    return Some(None);
                }
                // Clamp to the area: a stroke that leaves it keeps going
                // along the edge instead of ending.
                let q = Point {
                    x: x.clamp(l.area.x, l.area.x + l.area.w - 1),
                    y: y.clamp(l.area.y, l.area.y + l.area.h - 1),
                    t,
                    major,
                };
                let Some((a, b)) = ink.ink.moved(q) else {
                    return Some(None);
                };
                let r = draw_segment(&mut self.canvas, a, b, width, l.area);
                ink.pending = add(ink.pending, r);
                ink.dirty = add(ink.dirty, r);
                ink.fast = true;
                Some(Some(refresh))
            }
            Phase::Up => {
                if ink.ink.live.is_some() {
                    ink.ink.up();
                    ink.last_up = Some(t);
                    // v1.5.4: a scratch-out erases the strokes it crosses.
                    let size = l.area.h as f32;
                    let last = ink.ink.strokes.last().map(stroke_pts).unwrap_or_default();
                    if sigma_hwr::gesture::classify(&last, size)
                        == Some(sigma_hwr::gesture::Gesture::ScratchOut)
                    {
                        ink.ink.strokes.pop();
                        let bb = pts_box(&last);
                        ink.ink
                            .strokes
                            .retain(|s| !sigma_hwr::gesture::overlaps(pts_box(&stroke_pts(s)), bb));
                        ink.last_up = None;
                        return Some(Some(Refresh::Partial));
                    }
                    return Some(None);
                }
                let pressed = ink.press.take();
                let b = self.screen.ink_hit(&l, x, y);
                if pressed.is_none() || pressed != b {
                    return Some(None);
                }
                Some(self.ink_button(b?))
            }
        }
    }

    /// Letters / Train touches (v1.5.2): ink goes in one box at a time;
    /// starting in another box reads the pending one first.
    fn letters_touch(
        &mut self,
        phase: Phase,
        x: i32,
        y: i32,
        t: u64,
        l: &sigma_render::ink::InkLayout,
    ) -> Option<Refresh> {
        use sigma_core::ink::Point;
        use sigma_render::ink::{draw_segment, pen_px};
        let width = pen_px(self.settings.ink_pen);
        let refresh = self.ink_refresh();
        let major = sigma_device::evdev::contact_major();
        let add = |r: Option<Rect>, n: Rect| Some(r.map_or(n, |o| o.union(&n)));
        let mut out = None;
        match phase {
            Phase::Down => {
                if let Some(i) = l.boxes.iter().position(|b| b.contains(x, y)) {
                    let other = self
                        .ink
                        .as_ref()
                        .and_then(|k| k.boxi)
                        .is_some_and(|b| b != i);
                    if other {
                        out = self.read_letter();
                    }
                    let ink = self.ink.as_mut()?;
                    let p = Point { x, y, t, major };
                    ink.ink.down(p);
                    ink.boxi = Some(i);
                    ink.last_up = None;
                    if out.is_none() {
                        let r = draw_segment(&mut self.canvas, p, p, width, l.boxes[i]);
                        ink.pending = add(ink.pending, r);
                        ink.dirty = add(ink.dirty, r);
                        ink.fast = true;
                        out = Some(refresh);
                    }
                    return out;
                }
                self.ink.as_mut()?.press = self.screen.ink_hit(l, x, y);
                None
            }
            Phase::Move => {
                let ink = self.ink.as_mut()?;
                let (Some(i), Some(_)) = (ink.boxi, ink.ink.live.as_ref()) else {
                    return None;
                };
                let b = l.boxes[i];
                // A report outside the box is not this stroke (clamping it
                // drew lines to the box corners); leave it out.
                if !b.contains(x, y) {
                    sigma_device::evdev::log_anomaly(format_args!(
                        "dropped move outside box ({x},{y}) box {},{} {}x{}",
                        b.x, b.y, b.w, b.h
                    ));
                    return None;
                }
                let q = Point { x, y, t, major };
                let (a, bb) = ink.ink.moved(q)?;
                let r = draw_segment(&mut self.canvas, a, bb, width, b);
                ink.pending = add(ink.pending, r);
                ink.dirty = add(ink.dirty, r);
                ink.fast = true;
                Some(refresh)
            }
            Phase::Up => {
                let ink = self.ink.as_mut()?;
                if ink.ink.live.is_some() {
                    ink.ink.up();
                    ink.last_up = Some(t);
                    if ink.mode == sigma_render::ink::InkMode::Free {
                        return self.free_gesture(l);
                    }
                    return None;
                }
                let pressed = ink.press.take();
                let b = self.screen.ink_hit(l, x, y);
                if pressed.is_none() || pressed != b {
                    return None;
                }
                self.ink_button(b?)
            }
        }
    }

    /// v1.5.4: a tap above the panel places the caret (Down remembers the
    /// spot; Up within 24 px of it acts).
    fn caret_tap(&mut self, phase: Phase, x: i32, y: i32) -> Option<Refresh> {
        match phase {
            Phase::Down => {
                self.ink.as_mut()?.tap_at = Some((x, y));
                None
            }
            Phase::Move => None,
            Phase::Up => {
                let (dx, dy) = self.ink.as_mut()?.tap_at.take()?;
                if (dx - x).abs() > 24 || (dy - y).abs() > 24 {
                    return None;
                }
                self.place_caret(x, y)
            }
        }
    }

    /// Caret tap: pending ink is read, then the caret goes to the tapped
    /// spot of the note.
    pub(crate) fn place_caret(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let at = match self.editor_hit(x, y) {
            Hit::Caret(at) => at,
            // The title bar stays live while the panel is open: reading
            // the pending ink, the panel closes and the tap goes on to it.
            Hit::Back
            | Hit::Files
            | Hit::TogglePreview
            | Hit::Options
            | Hit::Find
            | Hit::Graph
            | Hit::Drawer(_) => {
                let _ = self.read_letter();
                self.ink = None;
                return self.handle_inner(Event::Tap { x, y });
            }
            _ => return None,
        };
        let _ = self.read_letter();
        self.buffer.set_caret(at);
        let ink = self.ink.as_mut()?;
        ink.replace_ok = false;
        ink.cands.clear();
        Some(Refresh::Partial)
    }

    /// v1.5.4 gestures in the free area, on the stroke that just lifted:
    /// scratch-out over pending ink erases what it crosses, over nothing
    /// deletes the last reading (or the word before the caret); a long
    /// flat flick left is Backspace (joins lines at a line start), right
    /// is Space.
    fn free_gesture(&mut self, l: &sigma_render::ink::InkLayout) -> Option<Refresh> {
        use sigma_hwr::gesture::{classify, overlaps, Gesture};
        let ink = self.ink.as_mut()?;
        let last = stroke_pts(ink.ink.strokes.last()?);
        let g = classify(&last, l.area.h as f32)?;
        ink.ink.strokes.pop();
        let others = !ink.ink.strokes.is_empty();
        match g {
            Gesture::ScratchOut if others => {
                // The free area holds one line: everything in the scratch's
                // x-range goes (dots and descenders too).
                let bb = pts_box(&last);
                ink.ink.strokes.retain(|s| {
                    let b = pts_box(&stroke_pts(s));
                    !overlaps((b.0, 0.0, b.2, 0.0), (bb.0, 0.0, bb.2, 0.0))
                });
                if ink.ink.strokes.is_empty() {
                    ink.last_up = None;
                    ink.boxi = None;
                }
                Some(Refresh::Partial)
            }
            Gesture::ScratchOut => {
                ink.last_up = None;
                let n = if ink.replace_ok { ink.last_len } else { 0 };
                ink.replace_ok = false;
                ink.cands.clear();
                if n > 0 {
                    for _ in 0..n {
                        self.buffer.backspace();
                    }
                } else {
                    let c = self.buffer.caret();
                    let text = self.buffer.text();
                    let trimmed = text[..c].trim_end_matches([' ', '\t']).len();
                    let start = self.buffer.word_left(trimmed);
                    if start == c {
                        return Some(Refresh::Partial);
                    }
                    self.buffer.select(start, c);
                    self.buffer.backspace();
                }
                self.save();
                Some(Refresh::Partial)
            }
            Gesture::FlickLeft => {
                if !others {
                    ink.last_up = None;
                }
                self.letters_type("", true)
            }
            Gesture::FlickRight => {
                if !others {
                    ink.last_up = None;
                }
                self.letters_type(" ", false)
            }
        }
    }

    /// The pending box's strokes in box units.
    fn letter_strokes(&self) -> Option<Vec<Vec<(f32, f32)>>> {
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
    fn read_letter(&mut self) -> Option<Refresh> {
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
    fn read_word(&mut self) -> Option<Refresh> {
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
    fn ink_undo(&mut self, text: bool) -> Option<Refresh> {
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
    fn letters_type(&mut self, s: &str, back: bool) -> Option<Refresh> {
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

    /// Train → Save: appends the box's ink as a sample of the prompted
    /// character to `.handwriting/samples.txt`, then the next character.
    fn save_sample(&mut self) -> Option<Refresh> {
        use sigma_render::ink::InkMode;
        let strokes = self.letter_strokes();
        let ink = self.ink.as_mut()?;
        let InkMode::Train(i) = ink.mode else {
            return None;
        };
        let strokes = strokes?;
        let ch = sigma_hwr::CHARSET.chars().nth(i)?;
        let path = samples_path(self.library.root());
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        let line = sigma_hwr::samples_to_text(&[sigma_hwr::Sample { ch, strokes }]);
        if text.is_empty() {
            text = line;
        } else {
            text.push_str(line.lines().nth(1).unwrap_or(""));
            text.push('\n');
        }
        let res = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, text));
        let ink = self.ink.as_mut()?;
        match res {
            Ok(()) => {
                ink.saved += 1;
                ink.error = None;
                self.hwr.samples += 1;
            }
            Err(e) => ink.error = Some(e.to_string()),
        }
        ink.ink = sigma_core::ink::Ink::default();
        ink.boxi = None;
        ink.mode = InkMode::Train((i + 1) % sigma_hwr::CHARSET.chars().count());
        Some(Refresh::Partial)
    }

    /// A candidate `picked` replaced `typed`, the last reading: learn the
    /// corrected letter's ink, if the correction is clear (one letter;
    /// see `sigma_hwr::free::learnable`) and learning is on. Picking again
    /// for the same reading replaces what it taught.
    fn learn_pick(&mut self, typed: &str, picked: &str) {
        if !self.settings.ink_learn {
            return;
        }
        let Some(ink) = self.ink.as_ref() else {
            return;
        };
        let sample = match &ink.read_ink {
            Some(crate::ReadInk::Letter(strokes)) => {
                let mut cs = picked.chars();
                match (cs.next(), cs.next()) {
                    (Some(ch), None) if !ch.is_whitespace() => Some((ch, strokes.clone())),
                    _ => None,
                }
            }
            Some(crate::ReadInk::Free(r)) => sigma_hwr::free::learnable(r, typed, picked)
                .map(|(ch, segs)| (ch, segs.iter().map(|&(a, b)| vec![a, b]).collect())),
            None => None,
        };
        let Some((ch, strokes)) = sample else {
            return;
        };
        if !sigma_hwr::CHARSET.contains(ch) {
            return;
        }
        let line = sigma_hwr::samples_to_text(&[sigma_hwr::Sample { ch, strokes }])
            .lines()
            .nth(1)
            .unwrap_or_default()
            .to_string();
        let root = self.library.root().to_path_buf();
        let path = crate::learned_path(&root);
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        let mut lines: Vec<String> = old
            .lines()
            .filter(|l| !l.starts_with("# ") && !l.is_empty())
            .map(str::to_owned)
            .collect();
        let prev = self.ink.as_mut().and_then(|i| i.learned.take());
        if let Some(p) = prev {
            if let Some(i) = lines.iter().rposition(|l| *l == p) {
                lines.remove(i);
            }
        }
        lines.push(line.clone());
        // Keep the newest LEARN_PER_CHAR of this character.
        let of_ch = |l: &String| l.split('\t').next() == Some(ch.to_string().as_str());
        let extra = lines
            .iter()
            .filter(|l| of_ch(l))
            .count()
            .saturating_sub(crate::LEARN_PER_CHAR);
        for _ in 0..extra {
            if let Some(i) = lines.iter().position(of_ch) {
                lines.remove(i);
            }
        }
        let header = sigma_hwr::samples_to_text(&[]);
        let text = header + &lines.join("\n") + "\n";
        let res = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, text));
        match res {
            Ok(()) => {
                if let Some(i) = self.ink.as_mut() {
                    i.learned = Some(line);
                }
                sigma_core::debug!("ink", "learned {ch:?} from a correction");
                self.hwr.refit(&root);
            }
            Err(e) => sigma_core::warn!("ink", "learned samples: {e}"),
        }
    }

    /// A panel button (touch Up over it, or a Tap).
    pub(crate) fn ink_button(&mut self, b: sigma_render::ink::InkButton) -> Option<Refresh> {
        use sigma_render::ink::{InkButton, InkMode};
        let mode = self.ink.as_ref()?.mode;
        match (mode, b) {
            (InkMode::Draw, InkButton::Guide) => {
                self.screen.sketch_guide = !self.screen.sketch_guide;
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
            (InkMode::Letters, InkButton::Train) => {
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

/// The writing panel while open (v1.5.0, plan §12.3).
#[derive(Clone, Debug, Default)]
pub struct InkState {
    pub ink: sigma_core::ink::Ink,
    /// Segments drawn into the canvas since the last ink frame.
    pub(crate) pending: Option<Rect>,
    /// Everything inked since the last GL16 cleanup.
    pub(crate) dirty: Option<Rect>,
    /// When the last stroke lifted (ms).
    last_up: Option<u64>,
    /// The next render is an ink-only frame.
    pub(crate) fast: bool,
    /// The next `PointerCleanup` render is the ink cleanup pass.
    pub(crate) cleanup: bool,
    /// A button pressed on Down, acted on at Up over the same button.
    press: Option<sigma_render::ink::InkButton>,
    /// Write errors are shown in the status line.
    pub error: Option<String>,
    /// Draw, Letters or Train (v1.5.2).
    pub mode: sigma_render::ink::InkMode,
    /// Letters: the box the pending strokes are in.
    pub boxi: Option<usize>,
    /// Letters: candidates for the last letter, and that it was typed by
    /// recognition (so a candidate tap may replace it).
    pub cands: Vec<String>,
    replace_ok: bool,
    /// Words: characters typed for the last word (with its space), so a
    /// candidate can replace it; and the lexicon for this panel.
    last_len: usize,
    lex: Option<sigma_hwr::word::Lexicon>,
    /// Free (v1.5.3): first candidate shown, 123, language filter.
    pub first: usize,
    pub digits: bool,
    pub lang: Option<&'static str>,
    /// Train: samples saved this session.
    pub saved: usize,
    /// The ink of the last reading, to learn from if a candidate corrects
    /// it.
    read_ink: Option<ReadInk>,
    /// The learned-samples line the last reading taught, replaced if
    /// another candidate is picked for the same reading.
    learned: Option<String>,
    /// v1.5.4: where a touch above the panel went down (caret tap).
    tap_at: Option<(i32, i32)>,
}

/// What the last reading was made from (learning from corrections).
#[derive(Clone, Debug)]
pub(crate) enum ReadInk {
    /// A Letters box: its strokes in box units.
    Letter(Vec<Vec<(f32, f32)>>),
    /// The free area: the reading, with each letter's ink.
    Free(sigma_hwr::free::Reading),
}

/// Letters: idle time after a stroke before the box is read (ms).
pub const LETTER_MS: u64 = 800;
/// Words (v1.5.3): idle time before the line is read (ms).
pub const WORD_MS: u64 = 1200;

/// Idle time after a stroke before the GL16 cleanup (plan §12.4).
pub const INK_CLEANUP_MS: u64 = 600;
