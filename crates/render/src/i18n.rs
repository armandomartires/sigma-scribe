//! Interface languages (v1.4, S14): English (source), Français, Português;
//! v1.6.2 (S16.2) adds the Kobo's other European UI languages.
//!
//! Catalog: English-source (gettext-style) — the English text is the key,
//! so control names in the emulator and tests stay English and a missing
//! translation falls back to English (decision S14-D1, no dependency).
//! Files `assets/i18n/<lang>.txt`, embedded: a line with the English text,
//! the next line `= ` and the translation; `\n` escapes a newline; `#`
//! starts a comment. Non-English files are **machine-drafted, best
//! effort** and need a native review.
//!
//! Dynamic text is matched by shape: numbers become `{n}` / `{m}` (in
//! order) and ` · `-separated parts are translated one by one, so
//! "92 words · line 17" uses the entries `{n} words` and `line {n}`.
//!
//! The language is per thread (the UI thread; each test its own).

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

/// An interface language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang {
    /// English, the source language.
    #[default]
    En,
    /// French.
    Fr,
    /// Portuguese.
    Pt,
    /// German.
    De,
    /// Spanish.
    Es,
    /// Italian.
    It,
    /// Dutch.
    Nl,
    /// Catalan.
    Ca,
    /// Danish.
    Da,
    /// Swedish.
    Sv,
    /// Test only: +40 % length and accents, to catch overflow.
    PseudoLong,
    /// Test only: English words, right-to-left chrome (v1.6 groundwork).
    PseudoRtl,
}

impl Lang {
    /// From a Kobo `CurrentLocale` value (`en`, `fr`, `pt_BR`, …), by its
    /// language prefix; anything else is English.
    #[must_use]
    pub fn from_locale(v: &str) -> Self {
        let p = v.trim().to_ascii_lowercase();
        let prefix = p.split(['_', '-', '.']).next().unwrap_or("");
        REAL.iter()
            .find(|(_, c, _)| *c == prefix)
            .map_or(Self::En, |r| r.0)
    }

    /// The language code: ISO 639-1 for the real languages, `en-XA` and
    /// `en-XB` for the two test languages.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
            Self::Pt => "pt",
            Self::De => "de",
            Self::Es => "es",
            Self::It => "it",
            Self::Nl => "nl",
            Self::Ca => "ca",
            Self::Da => "da",
            Self::Sv => "sv",
            Self::PseudoLong => "en-XA",
            Self::PseudoRtl => "en-XB",
        }
    }

    /// Right-to-left chrome (mirrored title bar). Real RTL languages
    /// (fa, ar) arrive in v1.6.
    #[must_use]
    pub fn rtl(self) -> bool {
        self == Self::PseudoRtl
    }

    /// Translations are machine-drafted (About says so).
    #[must_use]
    pub fn machine_drafted(self) -> bool {
        !matches!(self, Self::En | Self::PseudoLong | Self::PseudoRtl)
    }
}

thread_local! {
    static LANG: Cell<Lang> = const { Cell::new(Lang::En) };
}

/// Sets this thread's interface language.
pub fn set_lang(l: Lang) {
    LANG.with(|c| c.set(l));
}

/// This thread's interface language (English until set).
#[must_use]
pub fn lang() -> Lang {
    LANG.with(Cell::get)
}

/// Every translated language: variant, code, embedded catalog.
pub static REAL: &[(Lang, &str, &str)] = &[
    (Lang::Fr, "fr", include_str!("../../../assets/i18n/fr.txt")),
    (Lang::Pt, "pt", include_str!("../../../assets/i18n/pt.txt")),
    (Lang::De, "de", include_str!("../../../assets/i18n/de.txt")),
    (Lang::Es, "es", include_str!("../../../assets/i18n/es.txt")),
    (Lang::It, "it", include_str!("../../../assets/i18n/it.txt")),
    (Lang::Nl, "nl", include_str!("../../../assets/i18n/nl.txt")),
    (Lang::Ca, "ca", include_str!("../../../assets/i18n/ca.txt")),
    (Lang::Da, "da", include_str!("../../../assets/i18n/da.txt")),
    (Lang::Sv, "sv", include_str!("../../../assets/i18n/sv.txt")),
];

type Catalog = HashMap<String, String>;

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
}

/// Parses a catalog file into English → translation.
#[must_use]
pub fn parse(text: &str) -> Catalog {
    let mut out = HashMap::new();
    let mut key: Option<String> = None;
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if let Some(v) = line.strip_prefix("= ") {
            if let Some(k) = key.take() {
                out.insert(k, unescape(v));
            }
        } else {
            key = Some(unescape(line));
        }
    }
    out
}

fn catalog(l: Lang) -> Option<&'static Catalog> {
    static C: OnceLock<Vec<Catalog>> = OnceLock::new();
    let c = C.get_or_init(|| REAL.iter().map(|r| parse(r.2)).collect());
    REAL.iter().position(|r| r.0 == l).map(|i| &c[i])
}

/// Numbers in `s` (digit runs, with thin/normal-space thousands) replaced
/// by `{n}`, `{m}`; returns the template and the numbers.
fn template(s: &str) -> (String, Vec<String>) {
    let mut t = String::new();
    let mut nums = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_ascii_digit() {
            let mut j = i;
            let mut n = String::new();
            while j < cs.len() && (cs[j].is_ascii_digit() || cs[j] == '.') {
                n.push(cs[j]);
                j += 1;
            }
            t.push_str(if nums.is_empty() { "{n}" } else { "{m}" });
            nums.push(n);
            i = j;
        } else {
            t.push(cs[i]);
            i += 1;
        }
    }
    (t, nums)
}

fn lookup(c: &Catalog, s: &str) -> Option<String> {
    if let Some(v) = c.get(s) {
        return Some(v.clone());
    }
    let (t, nums) = template(s);
    if nums.is_empty() || nums.len() > 2 {
        return None;
    }
    let v = c.get(&t)?;
    let mut out = v.replace("{n}", &nums[0]);
    if let Some(m) = nums.get(1) {
        out = out.replace("{m}", m);
    }
    Some(out)
}

fn pseudo(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| match c {
            'a' => 'á',
            'e' => 'ë',
            'i' => 'ï',
            'o' => 'ô',
            'u' => 'ü',
            'A' => 'Å',
            'E' => 'É',
            'O' => 'Ö',
            c => c,
        })
        .collect();
    let extra = s.chars().filter(|c| c.is_alphabetic()).count() * 2 / 5;
    if extra > 0 {
        out.push(' ');
        out.push_str(&"~".repeat(extra.saturating_sub(1)));
    }
    out
}

/// Development aid: `SIGMA_I18N_MISSING=<file>` appends every text with
/// no translation (to build the catalogs).
/// Logs an untranslated text (development aid; `SIGMA_I18N_MISSING=file`).
/// Numbers alone and already-translated texts are not logged.
fn note_missing(c: &Catalog, s: &str) {
    let bare = s.chars().all(|ch| !ch.is_alphabetic());
    if bare || c.values().any(|v| v == s) {
        return;
    }
    if let Ok(p) = std::env::var("SIGMA_I18N_MISSING") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let _ = writeln!(f, "{}", s.replace('\n', "\\n"));
        }
    }
}

/// The interface text `s` in the current language (English if missing).
#[must_use]
pub fn tr(s: &str) -> Cow<'_, str> {
    let l = lang();
    if l == Lang::PseudoLong {
        return Cow::Owned(pseudo(s));
    }
    let Some(c) = catalog(l) else {
        return Cow::Borrowed(s);
    };
    if let Some(v) = lookup(c, s) {
        return Cow::Owned(v);
    }
    if s.contains(" · ") {
        let parts: Vec<String> = s
            .split(" · ")
            .map(|p| {
                lookup(c, p).unwrap_or_else(|| {
                    note_missing(c, p);
                    p.to_owned()
                })
            })
            .collect();
        return Cow::Owned(parts.join(" · "));
    }
    note_missing(c, s);
    Cow::Borrowed(s)
}

/// A text with a `{name}` placeholder (a note or folder name, never
/// translated): `tr_name("Move {name} to", "ideas.md")`.
#[must_use]
pub fn tr_name(template: &str, name: &str) -> String {
    tr(template).replace("{name}", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_prefixes_pick_the_language() {
        assert_eq!(Lang::from_locale("en"), Lang::En);
        assert_eq!(Lang::from_locale("fr"), Lang::Fr);
        assert_eq!(Lang::from_locale("fr_CA"), Lang::Fr);
        assert_eq!(Lang::from_locale("pt_BR"), Lang::Pt);
        assert_eq!(Lang::from_locale("PT-pt"), Lang::Pt);
        assert_eq!(Lang::from_locale("de_DE"), Lang::De);
        assert_eq!(Lang::from_locale("es_MX"), Lang::Es);
        assert_eq!(Lang::from_locale("ja"), Lang::En, "unknown → English");
        assert_eq!(Lang::from_locale(""), Lang::En);
    }

    #[test]
    fn exact_dynamic_and_fallback() {
        set_lang(Lang::Fr);
        assert_eq!(tr("Paper"), "Papier");
        assert_eq!(tr("92 words · line 17"), "92 mots · ligne 17");
        assert_eq!(tr("no such text"), "no such text", "falls back to English");
        set_lang(Lang::En);
        assert_eq!(tr("Paper"), "Paper");
        set_lang(Lang::PseudoLong);
        assert!(tr("Paper").chars().count() > "Paper".len());
        set_lang(Lang::En);
    }

    #[test]
    fn catalogs_parse_and_keep_placeholders() {
        for &(_, name, text) in REAL {
            assert!(
                text.contains("machine-drafted, best effort"),
                "{name} header"
            );
            let c = parse(text);
            assert!(c.len() > 100, "{name}: {} entries", c.len());
            for (k, v) in &c {
                for p in ["{n}", "{m}"] {
                    assert_eq!(k.contains(p), v.contains(p), "{name}: {k:?} → {v:?}");
                }
                assert!(!v.trim().is_empty(), "{name}: empty {k:?}");
            }
        }
        // Every language translates the same English texts as French.
        let f = parse(REAL[0].2);
        for &(_, name, text) in REAL {
            let p = parse(text);
            let mut missing: Vec<_> = f.keys().filter(|k| !p.contains_key(*k)).collect();
            missing.extend(p.keys().filter(|k| !f.contains_key(*k)));
            assert!(missing.is_empty(), "{name} differs from fr: {missing:?}");
        }
    }
}

#[cfg(test)]
mod fit_tests {
    use super::*;

    /// Button and key labels must stay short so they fit their cells.
    #[test]
    fn short_labels_stay_short() {
        let short = [
            "Cut", "Copy", "Paste", "Keys", "Cancel", "OK", "Shift", "Back", "Del", "Space",
            "Enter", "Return", "Caps", "Home", "End", "Prev", "Next", "Find", "Replace", "Folder",
            "More", "Rename", "Move", "Delete", "Paper", "Night", "On", "Off",
        ];
        for &(l, _, _) in REAL {
            let c = catalog(l).unwrap();
            for k in short {
                let v = c.get(k).unwrap_or_else(|| panic!("{l:?} lacks {k}"));
                let (e, t) = (k.chars().count(), v.chars().count());
                assert!(t <= (e * 2).max(e + 5), "{l:?} {k} -> {v}");
            }
        }
    }
}
