//! Japanese input (v1.10, S20.0): romaji → kana and kana → kanji.
//!
//! Typing is the usual flow: letters make hiragana (`romaji_feed`), Space
//! converts the whole reading ([`convert`]), Space again moves to the next
//! candidate, Enter commits. Conversion looks words up in an SKK-format
//! dictionary and picks the cheapest way to cut the reading into words
//! (fewer, longer words win; kana that no word covers stay kana). Verbs and
//! adjectives are found by their stem plus the first kana of the ending
//! (食べ + ます → 食べます), as SKK's okurigana entries do.
//!
//! A seed dictionary written for this project is built in; a full SKK
//! dictionary can be added with [`load_skk`].

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

const SEED: &str = include_str!("../../../assets/ja/seed.skk");
const SEED_OKURI: &str = include_str!("../../../assets/ja/seed-okuri.txt");

/// Romaji → kana. Longest keys are not needed first: a key is taken as soon
/// as the typed letters equal it and no longer key starts with them.
const TABLE: &[(&str, &str)] = &[
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("kya", "きゃ"),
    ("kyu", "きゅ"),
    ("kyo", "きょ"),
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("gya", "ぎゃ"),
    ("gyu", "ぎゅ"),
    ("gyo", "ぎょ"),
    ("sa", "さ"),
    ("si", "し"),
    ("shi", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("sha", "しゃ"),
    ("shu", "しゅ"),
    ("she", "しぇ"),
    ("sho", "しょ"),
    ("sya", "しゃ"),
    ("syu", "しゅ"),
    ("syo", "しょ"),
    ("za", "ざ"),
    ("zi", "じ"),
    ("ji", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("ja", "じゃ"),
    ("ju", "じゅ"),
    ("je", "じぇ"),
    ("jo", "じょ"),
    ("jya", "じゃ"),
    ("jyu", "じゅ"),
    ("jyo", "じょ"),
    ("zya", "じゃ"),
    ("zyu", "じゅ"),
    ("zyo", "じょ"),
    ("ta", "た"),
    ("ti", "ち"),
    ("chi", "ち"),
    ("tu", "つ"),
    ("tsu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("cha", "ちゃ"),
    ("chu", "ちゅ"),
    ("che", "ちぇ"),
    ("cho", "ちょ"),
    ("tya", "ちゃ"),
    ("tyu", "ちゅ"),
    ("tyo", "ちょ"),
    ("cya", "ちゃ"),
    ("cyu", "ちゅ"),
    ("cyo", "ちょ"),
    ("tsa", "つぁ"),
    ("thi", "てぃ"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("dya", "ぢゃ"),
    ("dyu", "ぢゅ"),
    ("dyo", "ぢょ"),
    ("dhi", "でぃ"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("nya", "にゃ"),
    ("nyu", "にゅ"),
    ("nyo", "にょ"),
    ("nn", "ん"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("hu", "ふ"),
    ("fu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("hya", "ひゃ"),
    ("hyu", "ひゅ"),
    ("hyo", "ひょ"),
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    ("bya", "びゃ"),
    ("byu", "びゅ"),
    ("byo", "びょ"),
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("pya", "ぴゃ"),
    ("pyu", "ぴゅ"),
    ("pyo", "ぴょ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("mya", "みゃ"),
    ("myu", "みゅ"),
    ("myo", "みょ"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("rya", "りゃ"),
    ("ryu", "りゅ"),
    ("ryo", "りょ"),
    ("wa", "わ"),
    ("wi", "うぃ"),
    ("wu", "う"),
    ("we", "うぇ"),
    ("wo", "を"),
    ("va", "ゔぁ"),
    ("vu", "ゔ"),
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("la", "ぁ"),
    ("li", "ぃ"),
    ("lu", "ぅ"),
    ("le", "ぇ"),
    ("lo", "ぉ"),
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("lya", "ゃ"),
    ("lyu", "ゅ"),
    ("lyo", "ょ"),
    ("xtu", "っ"),
    ("ltu", "っ"),
    ("xtsu", "っ"),
    ("xwa", "ゎ"),
];

/// Punctuation typed on a US keyboard and the Japanese mark it makes.
fn punct(c: char) -> Option<char> {
    Some(match c {
        '-' => 'ー',
        ',' => '、',
        '.' => '。',
        '/' => '・',
        '[' => '「',
        ']' => '」',
        '!' => '！',
        '?' => '？',
        '~' => '〜',
        _ => return None,
    })
}

fn lookup(pend: &str) -> Option<&'static str> {
    TABLE.iter().find(|(k, _)| *k == pend).map(|(_, v)| *v)
}

fn is_prefix(pend: &str) -> bool {
    TABLE
        .iter()
        .any(|(k, _)| k.len() > pend.len() && k.starts_with(pend))
}

/// Feeds one typed character. `kana` is the reading so far, `pend` the
/// romaji letters waiting for the rest of their syllable.
pub fn romaji_feed(kana: &mut String, pend: &mut String, c: char) {
    if c.is_ascii_alphabetic() {
        pend.push(c.to_ascii_lowercase());
        resolve(kana, pend);
        return;
    }
    if c == '\'' && pend == "n" {
        kana.push('ん');
        pend.clear();
        return;
    }
    finish(kana, pend);
    kana.push(punct(c).unwrap_or(c));
}

fn resolve(kana: &mut String, pend: &mut String) {
    while let Some(first) = pend.chars().next() {
        let b: Vec<char> = pend.chars().collect();
        // kk → っk, tt → っt …
        if b.len() >= 2 && b[0] == b[1] && !"aiueon".contains(b[0]) {
            kana.push('っ');
            pend.remove(0);
            continue;
        }
        // n before a consonant is ん.
        if b[0] == 'n' && b.len() >= 2 && !"aiueoyn'".contains(b[1]) {
            kana.push('ん');
            pend.remove(0);
            continue;
        }
        if let Some(k) = lookup(pend) {
            kana.push_str(k);
            pend.clear();
            return;
        }
        if is_prefix(pend) {
            return;
        }
        // Not a romaji sequence: the letter stays as typed.
        kana.push(first);
        pend.remove(0);
    }
}

/// Ends the syllable in progress: a lone n is ん.
pub fn finish(kana: &mut String, pend: &mut String) {
    if pend == "n" {
        kana.push('ん');
    } else {
        kana.push_str(pend);
    }
    pend.clear();
}

/// Hiragana → katakana.
#[must_use]
pub fn to_katakana(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{3041}'..='\u{3096}' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// The SKK okurigana letter of a kana (the consonant of its row).
fn okuri_letter(k: char) -> Option<char> {
    Some(match k {
        'あ' | 'い' | 'う' | 'え' | 'お' => match k {
            'あ' => 'a',
            'い' => 'i',
            'う' => 'u',
            'え' => 'e',
            _ => 'o',
        },
        'か' | 'き' | 'く' | 'け' | 'こ' => 'k',
        'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご' => 'g',
        'さ' | 'し' | 'す' | 'せ' | 'そ' => 's',
        'ざ' | 'じ' | 'ず' | 'ぜ' | 'ぞ' => 'z',
        'た' | 'ち' | 'つ' | 'て' | 'と' => 't',
        'だ' | 'ぢ' | 'づ' | 'で' | 'ど' => 'd',
        'な' | 'に' | 'ぬ' | 'ね' | 'の' => 'n',
        'は' | 'ひ' | 'ふ' | 'へ' | 'ほ' => 'h',
        'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ' => 'b',
        'ぱ' | 'ぴ' | 'ぷ' | 'ぺ' | 'ぽ' => 'p',
        'ま' | 'み' | 'む' | 'め' | 'も' => 'm',
        'や' | 'ゆ' | 'よ' => 'y',
        'ら' | 'り' | 'る' | 'れ' | 'ろ' => 'r',
        'わ' | 'を' => 'w',
        _ => return None,
    })
}

/// The dictionary: words by reading, and words with okurigana by stem and
/// the first kana of the ending (seed) or its SKK letter.
#[derive(Default)]
pub struct Dict {
    words: HashMap<String, Vec<String>>,
    by_kana: HashMap<(String, char), Vec<String>>,
    by_letter: HashMap<(String, char), Vec<String>>,
    max_word: usize,
    max_stem: usize,
    /// Readings that the project's own seed lists (preferred over the long
    /// tail of a full dictionary: names, rare words).
    seed: std::collections::HashSet<String>,
    loading_seed: bool,
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    if !s.is_empty() && !v.iter().any(|x| x == s) {
        v.push(s.to_string());
    }
}

impl Dict {
    fn add_word(&mut self, reading: &str, cand: &str) {
        if self.loading_seed {
            self.seed.insert(reading.to_string());
        }
        self.max_word = self.max_word.max(reading.chars().count());
        push_unique(self.words.entry(reading.to_string()).or_default(), cand);
    }

    fn add_okuri_kana(&mut self, stem: &str, kana: char, cand: &str) {
        self.max_stem = self.max_stem.max(stem.chars().count());
        push_unique(
            self.by_kana.entry((stem.to_string(), kana)).or_default(),
            cand,
        );
    }

    /// Reads SKK lines: `よみ /候補/候補;注釈/`; the okuri-ari section has
    /// the reading as stem + letter (`おくr /送/`).
    pub fn load_skk(&mut self, text: &str) {
        let mut ari = false;
        for line in text.lines() {
            if let Some(c) = line.strip_prefix(";;") {
                if c.contains("okuri-ari") {
                    ari = true;
                } else if c.contains("okuri-nasi") {
                    ari = false;
                }
                continue;
            }
            let Some((key, rest)) = line.split_once(' ') else {
                continue;
            };
            let cands = rest
                .split('/')
                .map(|c| c.split(';').next().unwrap_or(""))
                .filter(|c| !c.is_empty() && !c.starts_with('('));
            if ari {
                let mut chars: Vec<char> = key.chars().collect();
                let Some(letter) = chars.pop().filter(char::is_ascii_lowercase) else {
                    continue;
                };
                let stem: String = chars.into_iter().collect();
                self.max_stem = self.max_stem.max(stem.chars().count());
                let e = self.by_letter.entry((stem, letter)).or_default();
                for c in cands {
                    push_unique(e, c);
                }
            } else {
                for c in cands {
                    self.add_word(key, c);
                }
            }
        }
    }

    fn load_seed(&mut self) {
        self.loading_seed = true;
        self.load_skk(SEED);
        self.loading_seed = false;
        self.load_okuri(SEED_OKURI);
    }

    /// The project's own okurigana lines (see assets/ja/seed-okuri.txt).
    fn load_okuri(&mut self, text: &str) {
        let row = |l: &str| -> &'static str {
            match l {
                "k" => "かきくけこい",
                "k0" => "かきくけこ",
                "g" => "がぎぐげごい",
                "s" => "さしすせそ",
                "t" => "たちつてとっ",
                "n" => "なにぬねのん",
                "b" => "ばびぶべぼん",
                "m" => "まみむめもん",
                "r" => "らりるれろっ",
                "w" => "わいうえおっ",
                _ => "",
            }
        };
        for line in text.lines() {
            if line.starts_with('#') {
                continue;
            }
            let f: Vec<&str> = line.split('|').collect();
            match f.as_slice() {
                ["V", stem, r, kanji] => {
                    for k in row(r).chars() {
                        self.add_okuri_kana(stem, k, kanji);
                    }
                }
                ["I", stem, k, kanji] | ["X", stem, k, kanji] => {
                    if let Some(k) = k.chars().next() {
                        self.add_okuri_kana(stem, k, kanji);
                    }
                }
                ["I1", stem, kanji] => {
                    for k in "まなたてるれろよさせらず".chars() {
                        self.add_okuri_kana(stem, k, kanji);
                    }
                }
                ["A", stem, k, kanji] => {
                    let set = if *k == "い" { "いかくけ" } else { k };
                    for c in set.chars() {
                        self.add_okuri_kana(stem, c, kanji);
                    }
                }
                _ => {}
            }
        }
    }

    /// The word for a stem and the kana that follows, and whether it is
    /// from the seed (preferred).
    fn okuri(&self, stem: &str, k: char) -> Option<(&String, bool)> {
        self.by_kana
            .get(&(stem.to_string(), k))
            .and_then(|v| v.first())
            .map(|c| (c, true))
            .or_else(|| {
                let l = okuri_letter(k)?;
                self.by_letter
                    .get(&(stem.to_string(), l))
                    .and_then(|v| v.first())
                    .map(|c| (c, false))
            })
    }
}

fn dict() -> &'static RwLock<Dict> {
    static D: OnceLock<RwLock<Dict>> = OnceLock::new();
    D.get_or_init(|| {
        let mut d = Dict::default();
        d.load_seed();
        // A fuller SKK dictionary, when one is installed (docs/japanese.md).
        let extra = std::env::var("SIGMA_SKK_DICT").ok();
        for path in extra.iter().map(String::as_str).chain([
            "/mnt/onboard/.adds/sigma-scribe/dict/skk-jisyo.txt",
            // A development checkout (the emulator runs from its root).
            "assets/ja/skk-jisyo.txt",
        ]) {
            if let Ok(text) = std::fs::read_to_string(path) {
                d.load_skk(&text);
                break;
            }
        }
        RwLock::new(d)
    })
}

/// Adds an SKK dictionary (a full SKK-JISYO, say) after the seed.
pub fn load_skk(text: &str) {
    if let Ok(mut d) = dict().write() {
        d.load_skk(text);
    }
}

/// Kana that stay kana when alone (particles): a one-kana word never
/// replaces them.
const PARTICLES: &str = "はがをにでとものへやかねよわ";

/// Endings and function words that stay kana; covering them whole keeps a
/// dictionary word from cutting them up (ました ≠ ま + 下).
const GRAMMAR: &[&str] = &[
    "ます",
    "ました",
    "ません",
    "ませんでした",
    "ましょう",
    "ませ",
    "です",
    "でした",
    "でしょう",
    "ない",
    "なかった",
    "なくて",
    "なく",
    "たい",
    "たかった",
    "ている",
    "ていた",
    "ていない",
    "ています",
    "ていました",
    "てください",
    "ください",
    "から",
    "ので",
    "のに",
    "けど",
    "けれど",
    "でも",
    "しかし",
    "そして",
    "それから",
    "する",
    "した",
    "して",
    "します",
    "しました",
    "しない",
    "しよう",
    "できる",
    "できます",
    "できない",
    "いる",
    "います",
    "いた",
    "いません",
    "ある",
    "あります",
    "ありません",
    "あった",
    "あって",
    "だった",
    "だろう",
    "ですか",
    "ですね",
    "ますか",
    "ですよ",
    "ですが",
    "ですから",
    "たら",
    "れば",
    "ので",
    "こと",
    "よう",
    "ほど",
    "まで",
    "より",
    "など",
    "だけ",
    "しか",
    "ばかり",
    "ながら",
    "ために",
    "という",
    "といった",
];

/// How many alternative cuts are kept per position.
const NBEST: usize = 4;

/// The cheapest cuts of `reading` into words, best first (up to `NBEST`).
/// Costs: the project's own words, particles and endings are cheap (about
/// 100), a long-tail dictionary word dearer (about 300), a kana nothing
/// covers dearest (400); fewer words and longer words win.
fn cuts(d: &Dict, reading: &str) -> Vec<String> {
    let r: Vec<char> = reading.chars().collect();
    let n = r.len();
    // dp[j]: the cheapest ways (cost, text) to cover r[..j].
    let mut dp: Vec<Vec<(u32, String)>> = vec![Vec::new(); n + 1];
    dp[0].push((0, String::new()));
    for j in 0..n {
        let here = dp[j].clone();
        let mut relax = |to: usize, add: u32, s: &str| {
            for (cost, text) in &here {
                let cell = &mut dp[to];
                let t = format!("{text}{s}");
                let c = cost + add;
                if let Some(e) = cell.iter_mut().find(|(_, x)| *x == t) {
                    // The same text by a cheaper way.
                    e.0 = e.0.min(c);
                    cell.sort_by_key(|(c, _)| *c);
                    continue;
                }
                cell.push((c, t));
                cell.sort_by_key(|(c, _)| *c);
                cell.truncate(NBEST);
            }
        };
        relax(
            j + 1,
            if PARTICLES.contains(r[j]) { 100 } else { 400 },
            &r[j].to_string(),
        );
        for g in GRAMMAR {
            let len = g.chars().count();
            if j + len <= n && r[j..j + len].iter().copied().eq(g.chars()) {
                relax(j + len, 90 - len as u32, g);
            }
        }
        for len in 1..=d.max_word.min(n - j) {
            let key: String = r[j..j + len].iter().collect();
            if let Some(c) = d.words.get(&key).and_then(|v| v.first()) {
                if len == 1 && PARTICLES.contains(r[j]) {
                    continue;
                }
                let base = if d.seed.contains(&key) { 100 } else { 300 };
                relax(j + len, base - len as u32, c);
            }
        }
        for stem in 1..=d.max_stem.min(n - j - 1) {
            let key: String = r[j..j + stem].iter().collect();
            if let Some((c, seed)) = d.okuri(&key, r[j + stem]) {
                let base = if seed { 120 } else { 340 };
                relax(
                    j + stem + 1,
                    base - stem as u32,
                    &format!("{c}{}", r[j + stem]),
                );
            }
        }
    }
    dp.swap_remove(n).into_iter().map(|(_, t)| t).collect()
}

/// The cheapest cut of `reading` into words.
#[cfg(test)]
fn best_cut(d: &Dict, reading: &str) -> String {
    cuts(d, reading)
        .into_iter()
        .next()
        .unwrap_or_else(|| reading.to_string())
}

/// Conversion candidates for a reading, best first: the best cuts into
/// words, the readings that are one word, the hiragana, the katakana.
#[must_use]
pub fn convert(reading: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Ok(d) = dict().read() {
        for cut in cuts(&d, reading) {
            push_unique(&mut out, &cut);
        }
        if let Some(v) = d.words.get(reading) {
            for c in v {
                push_unique(&mut out, c);
            }
        }
    }
    push_unique(&mut out, reading);
    push_unique(&mut out, &to_katakana(reading));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kana(s: &str) -> String {
        let (mut k, mut p) = (String::new(), String::new());
        for c in s.chars() {
            romaji_feed(&mut k, &mut p, c);
        }
        finish(&mut k, &mut p);
        k
    }

    #[test]
    fn romaji_makes_hiragana() {
        assert_eq!(kana("konnnichiha"), "こんにちは");
        assert_eq!(kana("nihongo"), "にほんご");
        assert_eq!(kana("gakkou"), "がっこう");
        assert_eq!(kana("kyou"), "きょう");
        assert_eq!(kana("shinbun"), "しんぶん");
        assert_eq!(kana("onnna"), "おんな");
        assert_eq!(kana("kon'nichiha"), "こんにちは");
        assert_eq!(kana("tochuu"), "とちゅう");
        assert_eq!(kana("xtu"), "っ");
        assert_eq!(kana("fuji-san."), "ふじーさん。");
        assert_eq!(kana("n"), "ん");
        assert_eq!(kana("kanpai!"), "かんぱい！");
        assert_eq!(kana("sya"), "しゃ");
        assert_eq!(kana("kitte"), "きって");
    }

    #[test]
    fn pending_letters_wait_for_their_syllable() {
        let (mut k, mut p) = (String::new(), String::new());
        for c in "ka".chars() {
            romaji_feed(&mut k, &mut p, c);
        }
        romaji_feed(&mut k, &mut p, 'k');
        assert_eq!((k.as_str(), p.as_str()), ("か", "k"));
        romaji_feed(&mut k, &mut p, 'y');
        assert_eq!((k.as_str(), p.as_str()), ("か", "ky"));
    }

    #[test]
    fn words_convert_to_kanji() {
        assert_eq!(convert("にほんご")[0], "日本語");
        assert_eq!(convert("がくせい")[0], "学生");
        assert_eq!(convert("わたしはがくせいです")[0], "私は学生です");
        assert_eq!(
            convert("きょうはいいてんきですね")[0],
            "今日は良い天気ですね"
        );
    }

    #[test]
    fn verbs_and_adjectives_take_their_endings() {
        assert_eq!(convert("たべます")[0], "食べます");
        assert_eq!(convert("みました")[0], "見ました");
        assert_eq!(convert("かきました")[0], "書きました");
        assert_eq!(convert("おおきいいぬ")[0], "大きい犬");
        assert_eq!(convert("たかかった")[0], "高かった");
        assert_eq!(convert("いきます")[0], "行きます");
        assert_eq!(convert("のんだ")[0], "飲んだ");
    }

    #[test]
    fn the_candidates_end_with_kana_and_katakana() {
        let c = convert("にほん");
        assert_eq!(c[0], "日本");
        assert!(c.contains(&"にほん".to_string()) && c.contains(&"ニホン".to_string()));
        assert_eq!(convert("ぬぬぬ"), vec!["ぬぬぬ", "ヌヌヌ"]);
        assert_eq!(convert("ぱそこん")[0], "パソコン");
    }

    /// The seed plus the full SKK-JISYO.L (assets/ja/skk-jisyo.txt).
    fn full() -> Dict {
        let mut d = Dict::default();
        d.load_seed();
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/ja/skk-jisyo.txt");
        d.load_skk(&std::fs::read_to_string(path).expect("assets/ja/skk-jisyo.txt"));
        d
    }

    #[test]
    fn the_full_dictionary_loads_and_converts_sentences() {
        let t = std::time::Instant::now();
        let d = full();
        eprintln!("loaded in {:?}: {} words", t.elapsed(), d.words.len());
        assert!(d.words.len() > 100_000);
        let cases = [
            ("わたしはがくせいです", "私は学生です"),
            ("たべます", "食べます"),
            (
                "きのうともだちとえいがをみました",
                "昨日友達と映画を見ました",
            ),
            ("にほんごをべんきょうしています", "日本語を勉強しています"),
            ("あしたはあめがふるでしょう", "明日は雨が降るでしょう"),
            ("とうきょうにすんでいます", "東京に住んでいます"),
            ("ほんをよみたいです", "本を読みたいです"),
            ("おおきいいぬがはしっています", "大きい犬が走っています"),
            ("きょうはさむいですね", "今日は寒いですね"),
            ("かいぎはごごさんじからです", "会議は午後三時からです"),
        ];
        let bad: Vec<String> = cases
            .iter()
            .filter_map(|(r, want)| {
                let got = best_cut(&d, r);
                (got != *want).then(|| format!("{r}: {got} (want {want})"))
            })
            .collect();
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn an_added_skk_dictionary_extends_the_seed() {
        let mut d = Dict::default();
        d.load_skk(
            ";; okuri-ari entries.\nおくr /送/\n;; okuri-nasi entries.\nきしゃ /記者;press/汽車/\n",
        );
        assert_eq!(d.words["きしゃ"], vec!["記者", "汽車"]);
        assert_eq!(d.okuri("おく", 'る').map(|x| x.0.as_str()), Some("送"));
    }
}
