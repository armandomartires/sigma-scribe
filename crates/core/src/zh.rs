//! Chinese pinyin input (v1.11, S21.0): letters in, characters out.
//!
//! The typed pinyin is segmented into syllables in every possible way
//! (`xian` = xian or xi an) and looked up in a dictionary of words by their
//! syllables (`ni hao` → 你好). The candidates are: the best sentence (the
//! cheapest cover of the input by words, from word frequencies), then the
//! words that start the input, longest first. Choosing a candidate that
//! covers only the start of the input leaves the rest to be typed on.
//!
//! The dictionaries (Simplified and Traditional) are built by
//! `tools/zh-dict.py` from Rime's luna-pinyin and essay data (LGPL-3.0,
//! `assets/zh/PINYIN-NOTICE.txt`) and read from `dict/` beside the app; a
//! small seed written for this project is built in.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

/// Simplified or Traditional characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    /// Simplified Chinese.
    Hans,
    /// Traditional Chinese.
    Hant,
}

/// One candidate: the text and how many bytes of the input it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cand {
    /// The characters to insert.
    pub text: String,
    /// Bytes of the input (from its start) this candidate replaces.
    pub used: usize,
}

/// A pinyin dictionary: words by their syllables, and the known syllables.
/// Its contents are private to this module.
#[derive(Default)]
pub struct Dict {
    /// Words by pinyin key (syllables joined by one space), best first.
    words: HashMap<String, Vec<(String, u32)>>,
    syls: HashSet<String>,
    max_syl: usize,
}

const LONGEST_SYLLABLE: usize = 6;

impl Dict {
    fn add(&mut self, key: &str, word: &str, w: u32) {
        let v = self.words.entry(key.to_string()).or_default();
        if !v.iter().any(|x| x.0 == word) {
            v.push((word.to_string(), w));
        }
        let n = key.split(' ').count();
        self.max_syl = self.max_syl.max(n);
        if n == 1 {
            self.syls.insert(key.to_string());
        }
    }

    /// The built-in seed: characters by syllable, words by pinyin.
    fn seed(script: Script) -> Self {
        let (chars, words) = match script {
            Script::Hans => (
                include_str!("../../../assets/zh/seed-chars.txt"),
                include_str!("../../../assets/zh/seed-words.txt"),
            ),
            Script::Hant => (
                include_str!("../../../assets/zh/seed-chars-hant.txt"),
                include_str!("../../../assets/zh/seed-words-hant.txt"),
            ),
        };
        let mut d = Self::default();
        for line in chars.lines().filter(|l| !l.starts_with('#')) {
            if let Some((syl, cs)) = line.split_once(' ') {
                for (i, c) in cs.chars().enumerate() {
                    d.add(syl, &c.to_string(), 1_000_000 / (i as u32 + 2));
                }
            }
        }
        for line in words.lines().filter(|l| !l.starts_with('#')) {
            if let Some((key, word)) = line.rsplit_once(' ') {
                let n = d.words.get(key).map_or(0, Vec::len) as u32;
                d.add(key, word, 4_000_000 / (n + 1));
            }
        }
        d
    }

    /// A built dictionary file: `key<TAB>word weight<TAB>word weight…`.
    fn parse(text: &str) -> Self {
        let mut d = Self::default();
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let mut it = line.split('\t');
            let Some(key) = it.next() else { continue };
            for item in it {
                if let Some((w, n)) = item.rsplit_once(' ') {
                    d.add(key, w, n.parse().unwrap_or(1));
                }
            }
        }
        d
    }
}

fn dict(script: Script) -> &'static RwLock<Dict> {
    static HANS: OnceLock<RwLock<Dict>> = OnceLock::new();
    static HANT: OnceLock<RwLock<Dict>> = OnceLock::new();
    let (cell, name) = match script {
        Script::Hans => (&HANS, "pinyin-hans.txt"),
        Script::Hant => (&HANT, "pinyin-hant.txt"),
    };
    cell.get_or_init(|| {
        let env = std::env::var("SIGMA_ZH_DICT").ok();
        let dir_paths = [
            "/mnt/onboard/.adds/sigma-scribe/dict/".to_string(),
            // A development checkout (the emulator runs from its root).
            "assets/zh/".to_string(),
        ];
        let plugin_paths = crate::plugins::enabled_files(&format!("dict/{name}"));
        let loose = env
            .iter()
            .chain(dir_paths.iter())
            .map(|dir| std::path::PathBuf::from(format!("{}/{name}", dir.trim_end_matches('/'))));
        // The env override first, then enabled plugins, then the loose files.
        let mut candidates: Vec<_> = loose.collect();
        let at = usize::from(env.is_some());
        candidates.splice(at..at, plugin_paths);
        for path in candidates {
            if let Ok(text) = std::fs::read_to_string(path) {
                return RwLock::new(Dict::parse(&text));
            }
        }
        RwLock::new(Dict::seed(script))
    })
}

/// A word from the dictionary for tests and tools: replaces the dictionary
/// of `script` is not possible (it is read once), so tests use `Dict`.
#[cfg(test)]
fn test_dict(script: Script) -> Dict {
    let name = match script {
        Script::Hans => "pinyin-hans.txt",
        Script::Hant => "pinyin-hant.txt",
    };
    let path = format!("{}/../../assets/zh/{name}", env!("CARGO_MANIFEST_DIR"));
    Dict::parse(&std::fs::read_to_string(path).expect("assets/zh pinyin dictionary"))
}

/// Cost of a word of weight `w`: the log of its improbability, so that the
/// cheapest cover is the likeliest one.
fn cost(w: u32) -> f64 {
    (5.0e8_f64 / f64::from(w.max(1))).ln() + 1.0
}

/// Every way the start of `input[pos..]` is a run of syllables that is a
/// dictionary key: (end, key). `end` includes an apostrophe after the run.
fn runs(d: &Dict, input: &str, pos: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut budget = 3000;
    fn go(
        d: &Dict,
        b: &[u8],
        pos: usize,
        key: &mut String,
        n: usize,
        budget: &mut i32,
        out: &mut Vec<(usize, String)>,
    ) {
        if *budget <= 0 || n >= d.max_syl.max(1) {
            return;
        }
        for len in 1..=LONGEST_SYLLABLE.min(b.len() - pos) {
            *budget -= 1;
            let Ok(s) = std::str::from_utf8(&b[pos..pos + len]) else {
                continue;
            };
            if s.contains('\'') {
                break;
            }
            if !d.syls.contains(s) {
                continue;
            }
            let was = key.len();
            if n > 0 {
                key.push(' ');
            }
            key.push_str(s);
            let mut end = pos + len;
            if b.get(end) == Some(&b'\'') {
                end += 1;
            }
            if d.words.contains_key(key.as_str()) {
                out.push((end, key.clone()));
            }
            go(d, b, end, key, n + 1, budget, out);
            key.truncate(was);
        }
    }
    let mut key = String::new();
    go(d, input.as_bytes(), pos, &mut key, 0, &mut budget, &mut out);
    out
}

/// The cheapest cover of the longest reachable start of `input` by words:
/// (end, text, number of words).
fn sentence(d: &Dict, input: &str) -> Option<(usize, String, usize)> {
    let n = input.len();
    let mut best: Vec<Option<(f64, String, usize, usize)>> = vec![None; n + 1];
    best[0] = Some((0.0, String::new(), 0, 0));
    for pos in 0..n {
        let Some((c0, t0, k0, _)) = best[pos].clone() else {
            continue;
        };
        for (end, key) in runs(d, input, pos) {
            let (w, wt) = &d.words[&key][0];
            let c = c0 + cost(*wt);
            if best[end].as_ref().is_none_or(|b| c < b.0) {
                best[end] = Some((c, format!("{t0}{w}"), k0 + 1, pos));
            }
        }
    }
    (1..=n)
        .rev()
        .find_map(|j| best[j].clone().map(|(_, t, k, _)| (j, t, k)))
}

fn candidates_in(d: &Dict, input: &str) -> Vec<Cand> {
    let input = input.trim_start_matches('\'');
    let mut out: Vec<Cand> = Vec::new();
    let push = |text: &str, used: usize, out: &mut Vec<Cand>| {
        if !text.is_empty() && !out.iter().any(|c| c.text == text) {
            out.push(Cand {
                text: text.to_string(),
                used,
            });
        }
    };
    if let Some((end, text, k)) = sentence(d, input) {
        if k > 1 {
            push(&text, end, &mut out);
        }
    }
    let mut runs = runs(d, input, 0);
    runs.sort_by_key(|a| std::cmp::Reverse(a.0));
    let mut ends: Vec<usize> = runs.iter().map(|r| r.0).collect();
    ends.dedup();
    for end in ends {
        let mut words: Vec<&(String, u32)> = runs
            .iter()
            .filter(|r| r.0 == end)
            .flat_map(|r| d.words[&r.1].iter())
            .collect();
        words.sort_by_key(|a| std::cmp::Reverse(a.1));
        for (w, _) in words {
            push(w, end, &mut out);
        }
    }
    out.truncate(40);
    out
}

/// The candidates for the pinyin typed so far, best first. Empty when the
/// input does not start with a syllable.
#[must_use]
pub fn candidates(script: Script, input: &str) -> Vec<Cand> {
    match dict(script).read() {
        Ok(d) => candidates_in(&d, input),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(d: &Dict, input: &str) -> String {
        candidates_in(d, input)
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default()
    }

    #[test]
    fn the_seed_converts_common_words() {
        let d = Dict::seed(Script::Hans);
        assert_eq!(first(&d, "nihao"), "你好");
        assert_eq!(first(&d, "zhongguo"), "中国");
        let t = Dict::seed(Script::Hant);
        assert_eq!(first(&t, "zhongguo"), "中國");
    }

    #[test]
    fn candidates_say_how_much_input_they_cover() {
        let d = Dict::seed(Script::Hans);
        let c = candidates_in(&d, "nihao");
        assert_eq!((c[0].text.as_str(), c[0].used), ("你好", 5));
        // Single characters of the first syllable cover only it.
        assert!(c.iter().any(|x| x.text == "你" && x.used == 2));
        // Letters that are no syllable stay out.
        assert!(candidates_in(&d, "xyz").is_empty());
        let c = candidates_in(&d, "nihaoz");
        assert_eq!(c[0].used, 5);
    }

    #[test]
    fn the_full_dictionaries_convert_sentences() {
        let d = test_dict(Script::Hans);
        assert!(d.words.len() > 50_000, "{}", d.words.len());
        for (pinyin, want) in [
            ("nihao", "你好"),
            ("woaizhongguo", "我爱中国"),
            ("jintiantianqihenhao", "今天天气很好"),
            ("woxiangchifan", "我想吃饭"),
            ("xiexie", "谢谢"),
            ("beijingdaxue", "北京大学"),
        ] {
            assert_eq!(first(&d, pinyin), want, "{pinyin}");
        }
        let t = test_dict(Script::Hant);
        assert_eq!(first(&t, "woaizhongguo"), "我愛中國");
        assert_eq!(first(&t, "tushuguan"), "圖書館");
        // An apostrophe separates syllables: xi'an is not xian.
        assert_eq!(first(&d, "xi'an"), "西安");
        assert_eq!(first(&d, "xian"), "先");
        // ü is typed v.
        assert_eq!(first(&d, "nv"), "女");
        assert_eq!(first(&d, "lvse"), "绿色");
    }
}
