//! Saving enrolment samples and learning from corrections.

use crate::handwriting::samples_path;
use crate::App;
use sigma_device::Refresh;

impl App {
    /// Train → Save: appends the box's ink as a sample of the prompted
    /// character to `.handwriting/samples.txt`, then the next character.
    pub(super) fn save_sample(&mut self) -> Option<Refresh> {
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
    pub(super) fn learn_pick(&mut self, typed: &str, picked: &str) {
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
}
