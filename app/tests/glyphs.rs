//! v1.4.6: no UI character may draw as an empty box ("tofu").
//! Fails when any interface string, in any language, has a character that
//! no bundled face can draw (after the per-character fallback).

use sigma_render::fonts::{Face, Fonts};
use sigma_render::i18n::{self, tr, Lang};

const UI_FACES: [Face; 5] = [
    Face::Atkinson,
    Face::AtkinsonBold,
    Face::Literata,
    Face::LiterataItalic,
    Face::LiterataSemi,
];

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// English keys and translations from the catalogs.
fn catalog() -> (Vec<String>, Vec<String>) {
    let (mut keys, mut vals) = (Vec::new(), Vec::new());
    for code in ["fr", "pt"] {
        let t = std::fs::read_to_string(root().join(format!("assets/i18n/{code}.txt"))).unwrap();
        for line in t.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            match line.strip_prefix("= ") {
                Some(v) => vals.push(v.to_owned()),
                None => keys.push(line.to_owned()),
            }
        }
    }
    (keys, vals)
}

/// Hangul, kana, Han and full-width punctuation come from the optional script
/// font pack (docs/font-packs.md), which CI does not have; the bundled faces
/// are not expected to draw them.
fn from_font_pack(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x11FF | 0x3000..=0x30FF | 0x3130..=0x318F | 0x3400..=0x9FFF
        | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
}

fn check(f: &Fonts, what: &str, text: &str, bad: &mut Vec<String>) {
    for face in UI_FACES {
        let m = f.missing(face, text);
        if !m.is_empty() {
            bad.push(format!("{what}: {face:?} cannot draw {m:?} in {text:?}"));
        }
    }
}

#[test]
fn every_catalog_string_in_every_language_has_glyphs() {
    let f = Fonts::new();
    let (keys, vals) = catalog();
    assert!(keys.len() > 200 && vals.len() > 200);
    let mut bad = Vec::new();
    for k in &keys {
        check(&f, "en", k, &mut bad);
    }
    for v in &vals {
        check(&f, "fr/pt", v, &mut bad);
    }
    for l in [Lang::PseudoLong] {
        i18n::set_lang(l);
        for k in &keys {
            check(&f, &format!("{l:?}"), &tr(k), &mut bad);
        }
    }
    i18n::set_lang(Lang::En);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Non-ASCII characters in string literals of the UI code (comments
/// skipped), including `\u{…}` escapes.
#[test]
fn every_literal_ui_character_has_glyphs() {
    let f = Fonts::new();
    let mut bad = Vec::new();
    let mut seen = 0;
    for dir in [
        "crates/render/src",
        "crates/core/src",
        "app/src",
        "app/src/bin",
    ] {
        for e in std::fs::read_dir(root().join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            for (n, line) in src.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                // Only what is inside "…" on this line.
                let mut chars = Vec::new();
                for (i, part) in code.split('"').enumerate() {
                    if i % 2 == 0 {
                        continue;
                    }
                    chars.extend(part.chars().filter(|c| !c.is_ascii()));
                    let mut rest = part;
                    while let Some(at) = rest.find("\\u{") {
                        let tail = &rest[at + 3..];
                        if let Some(end) = tail.find('}') {
                            if let Some(c) = u32::from_str_radix(&tail[..end], 16)
                                .ok()
                                .and_then(char::from_u32)
                            {
                                chars.push(c);
                            }
                            rest = &tail[end..];
                        } else {
                            break;
                        }
                    }
                }
                for c in chars {
                    if c.is_control()
                        || from_font_pack(c)
                        || c == '\u{fe0f}'
                        || ('\u{300}'..='\u{36f}').contains(&c)
                    {
                        continue; // not drawn on its own (combining marks compose)
                    }
                    seen += 1;
                    check(
                        &f,
                        &format!("{}:{}", p.display(), n + 1),
                        &c.to_string(),
                        &mut bad,
                    );
                }
            }
        }
    }
    assert!(seen > 10);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn atkinson_arrows_fall_back_to_literata() {
    let f = Fonts::new();
    // The 1.4.5 bug: Atkinson has no U+2190, so "← Up to Files" showed a box.
    assert!(!f.has_glyph(Face::Atkinson, '\u{2190}'));
    assert_eq!(f.face_for(Face::Atkinson, '\u{2190}'), Face::Literata);
    assert!(f
        .missing(Face::Atkinson, "\u{2190} \u{2192} \u{2191} \u{2193}")
        .is_empty());
    // A character no face has is reported (the test fails on it).
    assert_eq!(f.missing(Face::Atkinson, "a\u{e000}"), vec!['\u{e000}']);
    // Characters a face has stay in that face.
    assert_eq!(f.face_for(Face::Atkinson, 'a'), Face::Atkinson);
}
