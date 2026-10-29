//! Sync (v1.16): Syncthing on the Kobo, and what the app needs to run it.
//!
//! The design is in `docs/dev/sync-design.md`. In short: the app starts a bundled
//! Syncthing while Settings → Sync is on, talks to its REST API on the
//! loopback ([`rest`], [`json`]), and shows what it reads ([`model`]). The
//! [`supervisor`] owns the process in a thread, so the input loop never
//! waits for it. Log lines use the target `sync`.
//!
//! Nothing here touches a note: the notes folder is only read (to count
//! conflict files) and given a `.stignore` once.

pub mod json;
pub mod model;
pub mod rest;
pub mod supervisor;

pub use model::{FolderView, Peer, PendingDevice, PendingFolder, Run, Status, View, MAX_PENDING};
pub use supervisor::{last_error, loopback_up, Cmd, Config, Supervisor};

use std::io;
use std::path::Path;

/// The `.stignore` written into the notes folder when sync sets it up and
/// there is none: the files that belong to this device, and the app's
/// temporary files. See `docs/dev/sync-design.md` §5.
pub const STIGNORE: &str = "\
// Σscribe: files that belong to this device, or that the app makes and removes.
.trash
.sigma-settings
.sigma-last-opened
.handwriting
.*.tmp
";

/// Writes [`STIGNORE`] into `notes` unless a `.stignore` is already there
/// (an edited one is never touched).
///
/// # Errors
/// The file could not be written.
pub fn ensure_ignore(notes: &Path) -> io::Result<()> {
    let p = notes.join(".stignore");
    if p.exists() {
        return Ok(());
    }
    crate::store::write_atomic(&p, STIGNORE)
}

/// Most folder entries looked at when counting conflicts.
const SCAN_LIMIT: usize = 20_000;

/// How many conflict files are in `root` and below: Syncthing's
/// (`*.sync-conflict-*`) and the app's own copies of a note that changed
/// underneath it (`*.conflict-*`), ignoring the dotted folders (`.trash`, `.ink`, …). A folder
/// that cannot be read counts as empty; the walk stops at 20,000
/// entries.
#[must_use]
pub fn count_conflicts(root: &Path) -> usize {
    let mut seen = 0;
    let mut n = 0;
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            seen += 1;
            if seen > SCAN_LIMIT {
                return n;
            }
            let name = e.file_name();
            let Some(name) = name.to_str() else { continue };
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            if is_dir {
                if !name.starts_with('.') {
                    todo.push(e.path());
                }
            } else if name.contains(".sync-conflict-") || name.contains(".conflict-") {
                n += 1;
            }
        }
    }
    n
}

/// `s` with everything but letters, digits and `-._~` as `%XX`, for a URL
/// query value (a folder ID is whatever its creator typed).
#[must_use]
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// 32 random bytes as hex: the API key of one run. Read from the system's
/// random source; if that cannot be read, from the clock and the process
/// id (a loopback-only key, but not a fixed one).
#[must_use]
pub fn random_key() -> String {
    use std::io::Read;
    let mut b = [0u8; 32];
    let ok = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .is_ok();
    if !ok {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let mut x = t as u64 ^ (u64::from(std::process::id()) << 32);
        for byte in &mut b {
            // xorshift64: enough to avoid a fixed key.
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *byte = (x >> 24) as u8;
        }
    }
    b.iter().map(|v| format!("{v:02x}")).collect()
}

/// A folder ID the way Syncthing makes them: ten lower-case base32 letters
/// and digits in two groups, `xxxxx-xxxxx`.
#[must_use]
pub fn random_folder_id() -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let key = random_key();
    let mut id = String::with_capacity(11);
    for (i, pair) in key.as_bytes().chunks(2).take(10).enumerate() {
        if i == 5 {
            id.push('-');
        }
        let byte = std::str::from_utf8(pair)
            .ok()
            .and_then(|h| u8::from_str_radix(h, 16).ok())
            .unwrap_or(0);
        id.push(char::from(ALPHABET[usize::from(byte % 32)]));
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("sigma-sync-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn stignore_is_written_once_and_never_overwritten() {
        let d = tmp("ignore");
        ensure_ignore(&d).unwrap();
        let text = std::fs::read_to_string(d.join(".stignore")).unwrap();
        for line in [
            ".trash",
            ".sigma-settings",
            ".sigma-last-opened",
            ".handwriting",
            ".*.tmp",
        ] {
            assert!(text.lines().any(|l| l == line), "{line}");
        }
        // The notes, their ink and the scratch pad are not ignored.
        assert!(!text
            .lines()
            .any(|l| l.contains("ink") || l.contains("scratch")));
        std::fs::write(d.join(".stignore"), "mine\n").unwrap();
        ensure_ignore(&d).unwrap();
        assert_eq!(
            std::fs::read_to_string(d.join(".stignore")).unwrap(),
            "mine\n"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn conflicts_are_counted_outside_dotted_folders() {
        let d = tmp("conflicts");
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::create_dir_all(d.join(".trash/1")).unwrap();
        for f in [
            "a.md",
            "a.sync-conflict-20261028-141500-ABCDEFG.md",
            "sub/b.sync-conflict-20261028-141501-ABCDEFG.txt",
            "c.conflict-20261028-141501.md",
            ".trash/1/c.sync-conflict-20261028-141502-ABCDEFG.md",
        ] {
            std::fs::write(d.join(f), "x").unwrap();
        }
        assert_eq!(count_conflicts(&d), 3);
        assert_eq!(count_conflicts(&d.join("missing")), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn percent_encoding_and_keys() {
        assert_eq!(percent_encode("abc-1.x_~"), "abc-1.x_~");
        assert_eq!(percent_encode("a b/é&"), "a%20b%2F%C3%A9%26");
        let (a, b) = (random_key(), random_key());
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn folder_ids_look_like_syncthings() {
        let id = random_folder_id();
        let ok = |g: &str| {
            g.len() == 5
                && g.bytes()
                    .all(|c| c.is_ascii_lowercase() || (b'2'..=b'7').contains(&c))
        };
        let (a, b) = id.split_once('-').unwrap();
        assert!(ok(a) && ok(b), "{id}");
        assert_ne!(id, random_folder_id());
    }
}
