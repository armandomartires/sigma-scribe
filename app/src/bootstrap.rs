//! First-run seed notes and the folders the app reads and writes.

use sigma_core::library::SCRATCH;
use std::path::PathBuf;

pub const NOTE: &str = "field-notes.md";
pub const SEED: &str = include_str!("../seed/field-notes.md");
pub const SEED_FILES: [(&str, &str); 4] = [
    (NOTE, SEED),
    (
        "drafts/chapter-01.md",
        include_str!("../seed/drafts/chapter-01.md"),
    ),
    ("scratch.txt", include_str!("../seed/scratch.txt")),
    (SCRATCH, ""),
];

/// Where notes live: [`sigma_device::paths::current`] (so `SIGMA_NOTES`,
/// else the Kobo onboard folder or `./target/emu-notes` on a desktop).
#[must_use]
pub fn notes_dir() -> PathBuf {
    sigma_device::paths::current().notes
}

/// Where the log file goes (Settings → Logging):
/// [`sigma_device::paths::current`] (so `SIGMA_LOG_DIR`, else the Kobo
/// log folder or `./target/emu-log` on a desktop).
#[must_use]
pub fn log_dir() -> PathBuf {
    sigma_device::paths::current().log
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_tests_use_the_desktop_feature_not_kobo() {
        // CI and `cargo test` run with default features.
        const { assert!(cfg!(feature = "desktop")) };
        const { assert!(!cfg!(feature = "kobo")) };
    }
}
