//! Vietnamese typing (v1.7, S17.0): Telex and VNI.
//!
//! The composer keeps the keystrokes of the word being typed (`raw`) and
//! this module turns them into the word to show, so a later key can add a
//! mark or a tone to an earlier letter and a repeated key takes it back
//! (`aa` → â, `aaa` → aa). Nothing here knows about buffers: `convert` is a
//! pure function of the keystrokes.
//!
//! Tone marks sit by the modern rule (hòa, thủy, quả): on the marked vowel
//! (the second of ươ), else on the middle of three vowels, else on the
//! second of a pair followed by a final consonant (toán), else on the
//! first. The `u` of `qu` and the `i` of `gi`
//! are consonants.

const VOWELS: &str = "aeiouy";

/// (base letter, mark) → the six forms: no tone, sắc, huyền, hỏi, ngã, nặng.
const FORMS: [(char, Mark, &str); 12] = [
    ('a', Mark::None, "aáàảãạ"),
    ('a', Mark::Breve, "ăắằẳẵặ"),
    ('a', Mark::Circ, "âấầẩẫậ"),
    ('e', Mark::None, "eéèẻẽẹ"),
    ('e', Mark::Circ, "êếềểễệ"),
    ('i', Mark::None, "iíìỉĩị"),
    ('o', Mark::None, "oóòỏõọ"),
    ('o', Mark::Circ, "ôốồổỗộ"),
    ('o', Mark::Horn, "ơớờởỡợ"),
    ('u', Mark::None, "uúùủũụ"),
    ('u', Mark::Horn, "ưứừửữự"),
    ('y', Mark::None, "yýỳỷỹỵ"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    None,
    Circ,
    Breve,
    Horn,
    Stroke,
}

#[derive(Clone, Copy, Debug)]
struct Letter {
    base: char,
    mark: Mark,
    upper: bool,
}

impl Letter {
    fn plain(c: char) -> Self {
        Self {
            base: c.to_ascii_lowercase(),
            mark: Mark::None,
            upper: c.is_uppercase(),
        }
    }

    fn is_vowel(self) -> bool {
        VOWELS.contains(self.base)
    }
}

#[derive(Default)]
struct Syllable {
    letters: Vec<Letter>,
    /// 0 none, 1 sắc, 2 huyền, 3 hỏi, 4 ngã, 5 nặng.
    tone: usize,
    /// After a key was taken back, the rest of the word is typed as is.
    literal: bool,
}

impl Syllable {
    /// Index of the last vowel in the trailing vowel-then-consonants shape
    /// that satisfies `want`: only consonants may follow it.
    fn last_vowel(&self, want: impl Fn(Letter) -> bool) -> Option<usize> {
        let i = self.letters.iter().rposition(|l| l.is_vowel())?;
        want(self.letters[i]).then_some(i).or_else(|| {
            // The wanted vowel may sit before a vowel pair (ươ, ie…):
            // look back through the trailing vowel run.
            (0..i)
                .rev()
                .take_while(|&j| self.letters[j].is_vowel())
                .find(|&j| want(self.letters[j]))
        })
    }

    fn has_vowel(&self) -> bool {
        self.letters.iter().any(|l| l.is_vowel())
    }

    fn push(&mut self, c: char) {
        self.letters.push(Letter::plain(c));
    }

    /// Puts `mark` on a vowel that takes it, or takes it back when it is
    /// already there. False when there is no such vowel.
    fn mark_vowel(&mut self, bases: &str, mark: Mark) -> bool {
        let Some(i) = self.last_vowel(|l| bases.contains(l.base)) else {
            return false;
        };
        // The pair uo takes the horn on both letters (thương, được).
        if mark == Mark::Horn && self.letters[i].base == 'o' && i > 0 {
            let u = self.letters[i - 1];
            let after_q = i > 1 && self.letters[i - 2].base == 'q';
            if u.base == 'u' && !after_q {
                let both = u.mark == Mark::Horn && self.letters[i].mark == Mark::Horn;
                let m = if both { Mark::None } else { Mark::Horn };
                self.letters[i - 1].mark = m;
                self.letters[i].mark = m;
                self.literal = both;
                return !both;
            }
        }
        let l = &mut self.letters[i];
        if l.mark == mark {
            l.mark = Mark::None;
            self.literal = true;
            // The caller writes the key itself.
            return false;
        }
        l.mark = mark;
        true
    }
}

/// Which keys do what in a method.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Act {
    Tone(usize),
    ClearTone,
    Circ,
    Hook,
    Stroke,
    Plain,
}

fn action(vni: bool, c: char) -> Act {
    let k = c.to_ascii_lowercase();
    if vni {
        return match k {
            '1'..='5' => Act::Tone(k as usize - '0' as usize),
            '0' => Act::ClearTone,
            '6' => Act::Circ,
            '7' | '8' => Act::Hook,
            '9' => Act::Stroke,
            _ => Act::Plain,
        };
    }
    match k {
        's' => Act::Tone(1),
        'f' => Act::Tone(2),
        'r' => Act::Tone(3),
        'x' => Act::Tone(4),
        'j' => Act::Tone(5),
        'z' => Act::ClearTone,
        'a' | 'e' | 'o' => Act::Circ,
        'w' => Act::Hook,
        'd' => Act::Stroke,
        _ => Act::Plain,
    }
}

/// The word shown for the keystrokes `raw` (one word, no spaces).
#[must_use]
pub fn convert(raw: &str, vni: bool) -> String {
    let mut s = Syllable::default();
    for c in raw.chars() {
        if s.literal {
            s.push(c);
            continue;
        }
        let k = c.to_ascii_lowercase();
        let done = match action(vni, c) {
            Act::Plain => false,
            Act::Tone(t) => {
                if !s.has_vowel() {
                    false
                } else if s.tone == t {
                    s.tone = 0;
                    s.literal = true;
                    false
                } else {
                    s.tone = t;
                    true
                }
            }
            Act::ClearTone => {
                if s.tone == 0 {
                    false
                } else {
                    s.tone = 0;
                    true
                }
            }
            Act::Circ => {
                // Telex: the key names the vowel; VNI: the last a / e / o.
                let bases = if vni {
                    "aeo".to_string()
                } else {
                    k.to_string()
                };
                s.mark_vowel(&bases, Mark::Circ)
            }
            Act::Hook => {
                let (bases, mark) = match (vni, k) {
                    (true, '8') => ("a", Mark::Breve),
                    (true, _) => ("ou", Mark::Horn),
                    (false, _) => ("aou", Mark::Horn),
                };
                // Telex w: a → ă, o and u → ơ, ư; a bare w is ư.
                let hit = if mark == Mark::Breve || vni {
                    s.mark_vowel(bases, mark)
                } else {
                    s.telex_w()
                };
                if !hit && !s.literal && !vni && k == 'w' {
                    s.letters.push(Letter {
                        base: 'u',
                        mark: Mark::Horn,
                        upper: c.is_uppercase(),
                    });
                    true
                } else {
                    hit
                }
            }
            Act::Stroke => match s.letters.first().copied() {
                Some(l) if l.base == 'd' && l.mark == Mark::None => {
                    s.letters[0].mark = Mark::Stroke;
                    true
                }
                Some(l) if l.base == 'd' && l.mark == Mark::Stroke => {
                    s.letters[0].mark = Mark::None;
                    s.literal = true;
                    false
                }
                _ => false,
            },
        };
        if !done {
            s.push(c);
        }
    }
    s.render()
}

impl Syllable {
    /// Telex w on the last of a, o, u: ă, ơ, ư (the pair uo gets both).
    fn telex_w(&mut self) -> bool {
        let Some(i) = self.last_vowel(|l| "aou".contains(l.base)) else {
            return false;
        };
        let mark = if self.letters[i].base == 'a' {
            Mark::Breve
        } else {
            Mark::Horn
        };
        self.mark_vowel(&self.letters[i].base.to_string(), mark)
    }

    fn render(&self) -> String {
        let at = self.tone_at();
        let mut out = String::new();
        for (i, l) in self.letters.iter().enumerate() {
            let tone = if Some(i) == at { self.tone } else { 0 };
            let ch = match (l.base, l.mark) {
                ('d', Mark::Stroke) => 'đ',
                (b, m) => FORMS
                    .iter()
                    .find(|f| f.0 == b && f.1 == m)
                    .and_then(|f| f.2.chars().nth(tone))
                    .unwrap_or(b),
            };
            if l.upper {
                out.extend(ch.to_uppercase());
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// The vowel that carries the tone.
    fn tone_at(&self) -> Option<usize> {
        if self.tone == 0 {
            return None;
        }
        let ls = &self.letters;
        let is_vowel = |i: usize| {
            let l = ls[i];
            if !l.is_vowel() {
                return false;
            }
            let prev = i.checked_sub(1).map(|j| ls[j].base);
            let next_vowel = ls.get(i + 1).is_some_and(|n| n.is_vowel());
            !(l.base == 'u' && prev == Some('q')
                || l.base == 'i' && prev == Some('g') && next_vowel)
        };
        let first = (0..ls.len()).find(|&i| is_vowel(i))?;
        let run: Vec<usize> = (first..ls.len()).take_while(|&i| is_vowel(i)).collect();
        if let Some(&m) = run.iter().rev().find(|&&i| ls[i].mark != Mark::None) {
            return Some(m);
        }
        let last = *run.last()?;
        let closed = last + 1 < ls.len();
        Some(match run.len() {
            1 => run[0],
            2 if closed => run[1],
            2 => run[0],
            _ => run[1],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::convert;

    fn t(s: &str) -> String {
        convert(s, false)
    }
    fn v(s: &str) -> String {
        convert(s, true)
    }

    #[test]
    fn telex_words() {
        assert_eq!(t("vieetj"), "việt");
        assert_eq!(t("Vieetj"), "Việt");
        assert_eq!(t("nguwowif"), "người");
        assert_eq!(t("thuowng"), "thương");
        assert_eq!(t("dduwowcj"), "được");
        assert_eq!(t("ddi"), "đi");
        assert_eq!(t("chaof"), "chào");
        assert_eq!(t("hoaf"), "hòa");
        assert_eq!(t("thuyr"), "thủy");
        assert_eq!(t("quas"), "quá");
        assert_eq!(t("gif"), "gì");
        assert_eq!(t("giuwx"), "giữ");
        assert_eq!(t("toans"), "toán");
        assert_eq!(t("khuyeens"), "khuyến");
        assert_eq!(t("hoaif"), "hoài");
        assert_eq!(t("muwa"), "mưa");
        assert_eq!(t("tieengs"), "tiếng");
        assert_eq!(t("Nam"), "Nam");
        assert_eq!(t("DDaf"), "Đà");
    }

    #[test]
    fn telex_keys_are_taken_back_by_repeating() {
        assert_eq!(t("aa"), "â");
        assert_eq!(t("aaa"), "aa");
        assert_eq!(t("ass"), "as");
        assert_eq!(t("dd"), "đ");
        assert_eq!(t("ddd"), "dd");
        assert_eq!(t("aw"), "ă");
        assert_eq!(t("aww"), "aw");
        assert_eq!(t("asz"), "a");
        // Without a vowel a tone key is a letter.
        assert_eq!(t("six"), "sĩ");
        assert_eq!(t("xin"), "xin");
        assert_eq!(t("w"), "ư");
    }

    #[test]
    fn vni_words() {
        assert_eq!(v("Vie6t5"), "Việt");
        assert_eq!(v("ngu7o7i2"), "người");
        assert_eq!(v("d9i"), "đi");
        assert_eq!(v("a8"), "ă");
        assert_eq!(v("a66"), "a6");
        assert_eq!(v("to1"), "tó");
        assert_eq!(v("hoa2"), "hòa");
        assert_eq!(v("5"), "5");
        assert_eq!(v("a1"), "á");
        assert_eq!(v("a10"), "a");
    }
}
