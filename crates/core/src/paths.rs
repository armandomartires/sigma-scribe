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
    /// Extra font files to try (for example a system Noto CJK on a desktop).
    pub extra_fonts: Vec<PathBuf>,
    /// Extra dictionary files or directories (for example `assets/ja` and
    /// `assets/zh` in a development checkout).
    pub extra_dicts: Vec<PathBuf>,
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
            extra_fonts: Vec::new(),
            extra_dicts: Vec::new(),
        }
    }

    /// Sets [`Self::extra_fonts`].
    #[must_use]
    pub fn with_extra_fonts(mut self, fonts: Vec<PathBuf>) -> Self {
        self.extra_fonts = fonts;
        self
    }

    /// Sets [`Self::extra_dicts`].
    #[must_use]
    pub fn with_extra_dicts(mut self, dicts: Vec<PathBuf>) -> Self {
        self.extra_dicts = dicts;
        self
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

/// The process-wide search roots, set by the app at start from the
/// platform constructors. Portable crates read this instead of hard-coding
/// device paths. A later call replaces the previous value, so tests can
/// change environment overrides between cases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchPaths {
    /// The platform roots.
    pub paths: Paths,
    /// Plugin folder, after `SIGMA_PLUGINS`.
    pub plugins: PathBuf,
}

static SEARCH: std::sync::RwLock<Option<SearchPaths>> = std::sync::RwLock::new(None);

impl SearchPaths {
    /// Stores these roots for this process.
    pub fn install(self) {
        *SEARCH.write().unwrap_or_else(|e| e.into_inner()) = Some(self);
    }

    /// The roots last passed to [`Self::install`], if any.
    #[must_use]
    pub fn current() -> Option<Self> {
        SEARCH.read().unwrap_or_else(|e| e.into_inner()).clone()
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
