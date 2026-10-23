//! Korean typing (v1.7, S17.1): Dubeolsik jamo → syllables.
//!
//! Like `viet.rs`, a pure function of the keystrokes of the current run of
//! jamo: [`compose`] returns the text to show. Backspace removes the last
//! jamo and composes again, so a syllable decomposes jamo by jamo
//! (한 → 하 → ㅎ). The output is precomposed syllables (U+AC00 + (L×21 +
//! V)×28 + T); a jamo that cannot join a syllable stays a compatibility
//! jamo.

const LEADS: &str = "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ";
const VOWELS: &str = "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ";
/// Index 0 is "no final".
const TAILS: &str = " ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ";

/// Two vowels that make one.
const VOWEL_PAIRS: [(char, char, char); 7] = [
    ('ㅗ', 'ㅏ', 'ㅘ'),
    ('ㅗ', 'ㅐ', 'ㅙ'),
    ('ㅗ', 'ㅣ', 'ㅚ'),
    ('ㅜ', 'ㅓ', 'ㅝ'),
    ('ㅜ', 'ㅔ', 'ㅞ'),
    ('ㅜ', 'ㅣ', 'ㅟ'),
    ('ㅡ', 'ㅣ', 'ㅢ'),
];

/// Two final consonants that make one.
const TAIL_PAIRS: [(char, char, char); 11] = [
    ('ㄱ', 'ㅅ', 'ㄳ'),
    ('ㄴ', 'ㅈ', 'ㄵ'),
    ('ㄴ', 'ㅎ', 'ㄶ'),
    ('ㄹ', 'ㄱ', 'ㄺ'),
    ('ㄹ', 'ㅁ', 'ㄻ'),
    ('ㄹ', 'ㅂ', 'ㄼ'),
    ('ㄹ', 'ㅅ', 'ㄽ'),
    ('ㄹ', 'ㅌ', 'ㄾ'),
    ('ㄹ', 'ㅍ', 'ㄿ'),
    ('ㄹ', 'ㅎ', 'ㅀ'),
    ('ㅂ', 'ㅅ', 'ㅄ'),
];

/// A compatibility jamo (U+3131..U+3163), the characters Dubeolsik types.
#[must_use]
pub fn is_jamo(c: char) -> bool {
    ('\u{3131}'..='\u{3163}').contains(&c)
}

fn is_vowel(c: char) -> bool {
    VOWELS.contains(c)
}

#[derive(Clone, Copy, Default)]
struct Syl {
    lead: Option<char>,
    vowel: Option<char>,
    tail: Option<char>,
}

impl Syl {
    fn is_empty(self) -> bool {
        self.lead.is_none()
    }

    fn push_to(self, out: &mut String) {
        let Some(l) = self.lead else { return };
        let Some(v) = self.vowel else {
            out.push(l);
            return;
        };
        let li = LEADS.chars().position(|x| x == l);
        let vi = VOWELS.chars().position(|x| x == v);
        let ti = self
            .tail
            .map_or(Some(0), |t| TAILS.chars().position(|x| x == t));
        match (li, vi, ti) {
            (Some(li), Some(vi), Some(ti)) => {
                let code = 0xAC00 + (li * 21 + vi) * 28 + ti;
                out.push(char::from_u32(code as u32).unwrap_or(l));
            }
            _ => {
                out.push(l);
                out.push(v);
                out.extend(self.tail);
            }
        }
    }
}

fn pair(table: &[(char, char, char)], a: char, b: char) -> Option<char> {
    table.iter().find(|p| p.0 == a && p.1 == b).map(|p| p.2)
}

fn split_tail(t: char) -> Option<(char, char)> {
    TAIL_PAIRS.iter().find(|p| p.2 == t).map(|p| (p.0, p.1))
}

/// The text shown for the jamo `keys` (one run, no other characters).
#[must_use]
pub fn compose(keys: &str) -> String {
    let mut out = String::new();
    let mut s = Syl::default();
    for j in keys.chars() {
        if !is_jamo(j) {
            s.push_to(&mut out);
            s = Syl::default();
            out.push(j);
            continue;
        }
        if is_vowel(j) {
            match (s.lead, s.vowel, s.tail) {
                (None, ..) => out.push(j),
                (Some(_), None, _) => s.vowel = Some(j),
                (Some(_), Some(v), None) => match pair(&VOWEL_PAIRS, v, j) {
                    Some(w) => s.vowel = Some(w),
                    None => {
                        s.push_to(&mut out);
                        s = Syl::default();
                        out.push(j);
                    }
                },
                (Some(_), Some(_), Some(t)) => {
                    // The final moves to the next syllable (the second
                    // half of a double final; a single one whole).
                    let (keep, moved) = match split_tail(t) {
                        Some((a, b)) => (Some(a), b),
                        None => (None, t),
                    };
                    s.tail = keep;
                    s.push_to(&mut out);
                    s = Syl {
                        lead: Some(moved),
                        vowel: Some(j),
                        tail: None,
                    };
                }
            }
            continue;
        }
        // A consonant.
        match (s.lead, s.vowel, s.tail) {
            (None, ..) => s.lead = Some(j),
            (Some(_), None, _) => {
                s.push_to(&mut out);
                s = Syl {
                    lead: Some(j),
                    ..Syl::default()
                };
            }
            (Some(_), Some(_), None) if TAILS.contains(j) => s.tail = Some(j),
            (Some(_), Some(_), Some(t)) => match pair(&TAIL_PAIRS, t, j) {
                Some(d) => s.tail = Some(d),
                None => {
                    s.push_to(&mut out);
                    s = Syl {
                        lead: Some(j),
                        ..Syl::default()
                    };
                }
            },
            _ => {
                s.push_to(&mut out);
                s = Syl {
                    lead: Some(j),
                    ..Syl::default()
                };
            }
        }
    }
    if !s.is_empty() {
        s.push_to(&mut out);
    }
    out
}

/// The keys that type `text` on Dubeolsik: every syllable spelled out as
/// jamo (compound vowels and finals as their two parts), other characters
/// unchanged. For the emulator's `usb-type` and for tests.
#[must_use]
pub fn spell(text: &str) -> String {
    let mut out = String::new();
    let parts = |j: char, table: &[(char, char, char)], out: &mut String| match table
        .iter()
        .find(|p| p.2 == j)
    {
        Some(p) => {
            out.push(p.0);
            out.push(p.1);
        }
        None => out.push(j),
    };
    for c in text.chars() {
        let code = c as u32;
        if !(0xAC00..=0xD7A3).contains(&code) {
            out.push(c);
            continue;
        }
        let n = (code - 0xAC00) as usize;
        let (l, v, t) = (n / 588, n % 588 / 28, n % 28);
        out.extend(LEADS.chars().nth(l));
        parts(VOWELS.chars().nth(v).unwrap_or(' '), &VOWEL_PAIRS, &mut out);
        if t > 0 {
            parts(TAILS.chars().nth(t).unwrap_or(' '), &TAIL_PAIRS, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::compose;

    #[test]
    fn syllables_form_from_jamo() {
        assert_eq!(compose("ㅎㅏㄴ"), "한");
        assert_eq!(compose("ㅎㅏㄴㄱㅡㄹ"), "한글");
        assert_eq!(compose("ㅇㅏㄴㄴㅕㅇ"), "안녕");
        assert_eq!(compose("ㅎㅏㅅㅔㅇㅛ"), "하세요");
        assert_eq!(compose("ㅅㅏㄹㅏㅇ"), "사랑");
    }

    #[test]
    fn a_final_moves_to_the_next_syllable() {
        assert_eq!(compose("ㅇㅣㄹㅏ"), "이라");
        assert_eq!(compose("ㅂㅏㅂㅣ"), "바비");
        // A double final splits; a plain ㅇ keeps it whole.
        assert_eq!(compose("ㄷㅏㄹㄱㅣ"), "달기");
        assert_eq!(compose("ㄷㅏㄹㄱ"), "닭");
        assert_eq!(compose("ㄷㅏㄹㄱㅇㅣ"), "닭이");
    }

    #[test]
    fn compound_vowels_and_finals() {
        assert_eq!(compose("ㄱㅗㅏㄴ"), "관");
        assert_eq!(compose("ㅇㅜㅓㄴ"), "원");
        assert_eq!(compose("ㅇㅢ"), "의");
        assert_eq!(compose("ㅇㅏㄴㅎ"), "않");
        assert_eq!(compose("ㅂㅏㅂㅅㅇㅓ"), "밦어");
    }

    #[test]
    fn spelling_and_composing_are_inverse() {
        for w in [
            "한글",
            "안녕하세요",
            "닭이",
            "값어치",
            "괜찮아요",
            "의원",
            "밖",
        ] {
            assert_eq!(compose(&super::spell(w)), w, "{w}");
        }
    }

    #[test]
    fn stray_jamo_stay_alone() {
        assert_eq!(compose("ㅏ"), "ㅏ");
        assert_eq!(compose("ㄱㄱ"), "ㄱㄱ");
        assert_eq!(compose("ㅃㅏㅃ"), "빠ㅃ");
        assert_eq!(compose("ㄱㅏ1"), "가1");
    }

    #[test]
    fn removing_the_last_jamo_undoes_one_step() {
        let keys = "ㅎㅏㄴ";
        assert_eq!(compose(keys), "한");
        assert_eq!(compose(&keys[..keys.len() - 3]), "하");
        assert_eq!(compose("ㅎ"), "ㅎ");
    }
}
