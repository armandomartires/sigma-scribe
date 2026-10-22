//! The Kobo's interface language (v1.4, S14, decision 31).
//!
//! Nickel keeps it in `/mnt/onboard/.kobo/Kobo/Kobo eReader.conf`, section
//! `[ApplicationPreferences]`, key `CurrentLocale` (this Forma: `en`, seen
//! in the 2026-10-20 backup). The app only **reads** that file — Nickel
//! owns it and we never write it. Reading the real file on the device is
//! a device check; the parser is tested against a fixture.

use std::path::{Path, PathBuf};

/// Where Nickel keeps its settings on a Kobo.
pub const KOBO_CONF: &str = "/mnt/onboard/.kobo/Kobo/Kobo eReader.conf";

/// `CurrentLocale` from the text of `Kobo eReader.conf`, if present.
#[must_use]
pub fn current_locale(conf: &str) -> Option<String> {
    let mut in_prefs = false;
    for line in conf.lines() {
        let l = line.trim().trim_start_matches('\u{feff}');
        if l.starts_with('[') {
            in_prefs = l.eq_ignore_ascii_case("[ApplicationPreferences]");
            continue;
        }
        if !in_prefs {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            if k.trim() == "CurrentLocale" {
                let v = v.trim().trim_matches('"');
                return (!v.is_empty()).then(|| v.to_owned());
            }
        }
    }
    None
}

/// The config path: `SIGMA_KOBO_CONF` (emulator, tests) or [`KOBO_CONF`].
#[must_use]
pub fn conf_path() -> PathBuf {
    std::env::var_os("SIGMA_KOBO_CONF").map_or_else(|| PathBuf::from(KOBO_CONF), PathBuf::from)
}

/// Reads the locale (read-only); missing or unreadable → `None` (English).
#[must_use]
pub fn read_locale(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    current_locale(&String::from_utf8_lossy(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../tests/fixtures/kobo-ereader-fr.conf");

    #[test]
    fn reads_current_locale_from_the_fixture() {
        assert_eq!(current_locale(FIXTURE).as_deref(), Some("fr"));
    }

    #[test]
    fn only_the_application_preferences_section_counts() {
        let t =
            "[Other]\nCurrentLocale=de\n[ApplicationPreferences]\nFoo=1\nCurrentLocale = pt_BR\n";
        assert_eq!(current_locale(t).as_deref(), Some("pt_BR"));
        assert_eq!(current_locale("[Other]\nCurrentLocale=de\n"), None);
        assert_eq!(
            current_locale("[ApplicationPreferences]\nCurrentLocale=\n"),
            None
        );
        assert_eq!(current_locale(""), None);
    }

    #[test]
    fn a_missing_file_is_none_and_is_never_created() {
        let p = std::env::temp_dir().join(format!("no-kobo-{}.conf", std::process::id()));
        assert_eq!(read_locale(&p), None);
        assert!(!p.exists());
    }
}
