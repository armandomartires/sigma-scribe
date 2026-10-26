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

/// Where notes live: `SIGMA_NOTES`, else `/mnt/onboard/sigma-scribe/notes`
/// on the Kobo (visible over USB), else `./target/emu-notes` on a desktop.
#[must_use]
pub fn notes_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SIGMA_NOTES") {
        return p.into();
    }
    if cfg!(target_arch = "arm") {
        "/mnt/onboard/sigma-scribe/notes".into()
    } else {
        "target/emu-notes".into()
    }
}

/// Where the log file goes (Settings → Logging): `SIGMA_LOG_DIR`, else
/// `.adds/sigma-scribe/log` on the Kobo, else `./target/emu-log` on a
/// desktop.
#[must_use]
pub fn log_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SIGMA_LOG_DIR") {
        return p.into();
    }
    if cfg!(target_arch = "arm") {
        "/mnt/onboard/.adds/sigma-scribe/log".into()
    } else {
        "target/emu-log".into()
    }
}
