//! Kobo and desktop constructors for [`sigma_core::paths::Paths`] (ADR 0001).
//!
//! Environment overrides (`SIGMA_NOTES`, `SIGMA_LOG_DIR`, `SIGMA_KOBO_CONF`,
//! `SIGMA_PLUGINS`)
//! are applied here, not in `sigma-core`.

pub use sigma_core::paths::Paths;
use std::path::PathBuf;

fn env_or(key: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(key).map(PathBuf::from).unwrap_or(default)
}

/// The Kobo Forma layout under `/mnt/onboard`, after environment overrides.
///
/// This function is available on every target so tests can inspect the
/// layout without enabling the `kobo` feature.
#[must_use]
pub fn kobo() -> Paths {
    Paths::new(
        env_or(
            "SIGMA_NOTES",
            PathBuf::from("/mnt/onboard/sigma-scribe/notes"),
        ),
        env_or(
            "SIGMA_LOG_DIR",
            PathBuf::from("/mnt/onboard/.adds/sigma-scribe/log"),
        ),
        PathBuf::from("/mnt/onboard/.adds/sigma-scribe"),
        env_or(
            "SIGMA_KOBO_CONF",
            PathBuf::from("/mnt/onboard/.kobo/Kobo/Kobo eReader.conf"),
        ),
    )
}

/// Desktop and emulator defaults (`target/emu-*`), after environment
/// overrides. There is no Nickel conf unless `SIGMA_KOBO_CONF` is set.
#[must_use]
pub fn desktop() -> Paths {
    Paths::new(
        env_or("SIGMA_NOTES", PathBuf::from("target/emu-notes")),
        env_or("SIGMA_LOG_DIR", PathBuf::from("target/emu-log")),
        PathBuf::from("target/emu-sync"),
        env_or("SIGMA_KOBO_CONF", PathBuf::new()),
    )
    .with_extra_fonts(vec![PathBuf::from(
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc#0",
    )])
    .with_extra_dicts(vec![PathBuf::from("assets/ja"), PathBuf::from("assets/zh")])
}

/// [`kobo`] when the `kobo` feature is on, otherwise [`desktop`].
#[must_use]
pub fn current() -> Paths {
    if cfg!(feature = "kobo") {
        kobo()
    } else {
        desktop()
    }
}

/// The plugins folder: `SIGMA_PLUGINS`, else [`Paths::plugins`].
#[must_use]
pub fn plugins_dir(paths: &Paths) -> PathBuf {
    std::env::var_os("SIGMA_PLUGINS")
        .map(PathBuf::from)
        .unwrap_or_else(|| paths.plugins())
}

/// Installs [`crate::paths::current`] as the process-wide [`sigma_core::SearchPaths`].
pub fn install_current() -> sigma_core::SearchPaths {
    let paths = current();
    let search = sigma_core::SearchPaths {
        plugins: plugins_dir(&paths),
        paths,
    };
    search.clone().install();
    search
}

#[cfg(test)]
mod tests {
    use super::{desktop, kobo};
    use std::path::Path;

    #[test]
    fn kobo_layout_uses_onboard() {
        // Do not assert the env-overridable notes/log/locale fields; only
        // the fixed install root is stable in any test process.
        let p = kobo();
        assert_eq!(p.adds, Path::new("/mnt/onboard/.adds/sigma-scribe"));
        assert_eq!(
            p.plugins(),
            Path::new("/mnt/onboard/.adds/sigma-scribe/plugins")
        );
        assert_eq!(
            p.skk_dict(),
            Path::new("/mnt/onboard/.adds/sigma-scribe/dict/skk-jisyo.txt")
        );
        assert!(p.extra_fonts.is_empty());
        assert!(p.extra_dicts.is_empty());
    }

    #[test]
    fn desktop_layout_uses_emu_dirs() {
        let p = desktop();
        assert_eq!(p.adds, Path::new("target/emu-sync"));
        assert_eq!(p.plugins(), Path::new("target/emu-sync/plugins"));
        assert_eq!(
            p.extra_fonts,
            [Path::new(
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc#0"
            )]
        );
        assert_eq!(
            p.extra_dicts,
            [Path::new("assets/ja"), Path::new("assets/zh")]
        );
    }
}
