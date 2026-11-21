//! Filesystem roots the portable crates take from the caller (ADR 0001).
//!
//! This type holds paths only. It contains no device literals and does not
//! spawn processes. The Kobo and desktop constructors live in
//! `sigma_device::paths`.

use std::path::PathBuf;

/// Filesystem roots for notes, logs, the install tree, and the locale file.
///
/// [`Self::plugins`], [`Self::fonts`] and [`Self::dict`] are derived from
/// [`Self::adds`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// The notes library root.
    pub notes: PathBuf,
    /// Directory for the app log (Settings → Logging).
    pub log: PathBuf,
    /// Install root (`.adds/sigma-scribe` on a Kobo; `target/emu-sync` on a
    /// desktop).
    pub adds: PathBuf,
    /// Nickel's `Kobo eReader.conf`, or a dummy path when there isn't one.
    pub locale_conf: PathBuf,
}

impl Paths {
    /// Assembles roots. Prefer the constructors in `sigma_device::paths`.
    #[must_use]
    pub fn new(
        notes: impl Into<PathBuf>,
        log: impl Into<PathBuf>,
        adds: impl Into<PathBuf>,
        locale_conf: impl Into<PathBuf>,
    ) -> Self {
        Self {
            notes: notes.into(),
            log: log.into(),
            adds: adds.into(),
            locale_conf: locale_conf.into(),
        }
    }

    /// Where plugins are installed: `<adds>/plugins`.
    #[must_use]
    pub fn plugins(&self) -> PathBuf {
        self.adds.join("plugins")
    }

    /// Where loose font packs live: `<adds>/fonts`.
    #[must_use]
    pub fn fonts(&self) -> PathBuf {
        self.adds.join("fonts")
    }

    /// Where dictionaries live: `<adds>/dict`.
    #[must_use]
    pub fn dict(&self) -> PathBuf {
        self.adds.join("dict")
    }

    /// The SKK dictionary file under [`Self::dict`].
    #[must_use]
    pub fn skk_dict(&self) -> PathBuf {
        self.dict().join("skk-jisyo.txt")
    }
}

#[cfg(test)]
mod tests {
    use super::Paths;
    use std::path::Path;

    #[test]
    fn derived_dirs_hang_off_adds() {
        let p = Paths::new("notes", "log", "adds", "locale.conf");
        assert_eq!(p.plugins(), Path::new("adds/plugins"));
        assert_eq!(p.fonts(), Path::new("adds/fonts"));
        assert_eq!(p.dict(), Path::new("adds/dict"));
        assert_eq!(p.skk_dict(), Path::new("adds/dict/skk-jisyo.txt"));
    }
}
