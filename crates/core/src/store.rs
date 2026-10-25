//! Note persistence and autosave.
//!
//! Notes are plain `.md` / `.txt` files so they can leave the device over USB
//! (brief §8). [`FsNoteStore`] writes atomically: temp file, fsync, rename,
//! fsync of the directory, so a power cut costs at most the edit in flight.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Where notes are loaded from and saved to, by name.
pub trait NoteStore {
    /// The note called `name`, or `None` when there is no such note.
    ///
    /// # Errors
    /// The backend could not read the note.
    fn load(&self, name: &str) -> io::Result<Option<String>>;
    /// Writes `body` as the note called `name`, replacing any old content.
    ///
    /// # Errors
    /// The backend could not durably write the note.
    fn save(&mut self, name: &str, body: &str) -> io::Result<()>;
}

/// Files under one root directory. Names are relative paths like
/// `field-notes.md` or `drafts/chapter-01.md`.
#[derive(Clone, Debug)]
pub struct FsNoteStore {
    root: PathBuf,
}

impl FsNoteStore {
    /// A store rooted at `root`. Nothing is touched on disk until a save.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, name: &str) -> io::Result<PathBuf> {
        let rel = Path::new(name);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "note name escapes the library",
            ));
        }
        Ok(self.root.join(rel))
    }
}

/// Temp file, fsync, rename, fsync of the directory.
///
/// # Errors
/// Any filesystem error; the previous file is left intact.
pub fn write_atomic(path: &Path, body: &str) -> io::Result<()> {
    write_atomic_bytes(path, body.as_bytes())
}

/// [`write_atomic`] for binary data (plugin caches).
///
/// # Errors
/// Any filesystem error; the previous file is left intact.
pub fn write_atomic_bytes(path: &Path, body: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    fs::create_dir_all(&dir)?;
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("note");
    let tmp = dir.join(format!(".{file_name}.tmp"));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(body)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Ok(d) = File::open(&dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

impl NoteStore for FsNoteStore {
    fn load(&self, name: &str) -> io::Result<Option<String>> {
        match fs::read_to_string(self.path(name)?) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn save(&mut self, name: &str, body: &str) -> io::Result<()> {
        write_atomic(&self.path(name)?, body)
    }
}

/// In-memory store for tests and the simulator.
#[derive(Clone, Debug, Default)]
pub struct MemoryNoteStore {
    /// The notes, by name.
    pub notes: BTreeMap<String, String>,
    /// When set, every save fails and leaves `notes` unchanged.
    pub fail: bool,
}

impl NoteStore for MemoryNoteStore {
    fn load(&self, name: &str) -> io::Result<Option<String>> {
        Ok(self.notes.get(name).cloned())
    }

    fn save(&mut self, name: &str, body: &str) -> io::Result<()> {
        if self.fail {
            return Err(io::Error::other("simulated failure"));
        }
        self.notes.insert(name.to_owned(), body.to_owned());
        Ok(())
    }
}

/// Where the current note stands with respect to the store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveState {
    /// The last change is written.
    Saved,
    /// A write is in flight.
    Saving,
    /// The last write failed, with the reason.
    Failed(String),
}

impl SaveState {
    /// Status-bar word (brief §3: "· saved").
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Saved => "saved",
            Self::Saving => "saving",
            Self::Failed(_) => "not saved",
        }
    }
}

/// Autosave on every change; no save button.
#[derive(Clone, Debug)]
pub struct Autosave {
    saved_revision: u64,
    /// Where the note stands now.
    pub state: SaveState,
}

impl Autosave {
    /// Tracker for a note whose `revision` is already saved.
    #[must_use]
    pub fn new(revision: u64) -> Self {
        Self {
            saved_revision: revision,
            state: SaveState::Saved,
        }
    }

    /// Whether `revision` differs from the last saved revision.
    #[must_use]
    pub fn dirty(&self, revision: u64) -> bool {
        revision != self.saved_revision
    }

    /// Synchronous write (desktop / Plan B). Cobalt's async store uses
    /// [`Autosave::begin`] and [`Autosave::landed`] instead.
    pub fn sync(&mut self, store: &mut dyn NoteStore, name: &str, body: &str, revision: u64) {
        if !self.dirty(revision) && self.state == SaveState::Saved {
            return;
        }
        match store.save(name, body) {
            Ok(()) => self.landed(revision),
            Err(e) => self.state = SaveState::Failed(e.to_string()),
        }
    }

    /// A write has started; the state becomes [`SaveState::Saving`].
    pub fn begin(&mut self) {
        self.state = SaveState::Saving;
    }

    /// The write of `revision` is durable.
    pub fn landed(&mut self, revision: u64) {
        self.saved_revision = revision;
        self.state = SaveState::Saved;
    }

    /// The write failed because of `why`; the saved revision is unchanged.
    pub fn failed(&mut self, why: impl Into<String>) {
        self.state = SaveState::Failed(why.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Buffer;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sigma-core-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn fs_round_trip_and_nested() {
        let root = tmpdir("rt");
        let mut s = FsNoteStore::new(&root);
        assert_eq!(s.load("a.md").unwrap(), None);
        s.save("drafts/chapter-01.md", "# One\n").unwrap();
        assert_eq!(
            s.load("drafts/chapter-01.md").unwrap().as_deref(),
            Some("# One\n")
        );
        s.save("drafts/chapter-01.md", "# Two\n").unwrap();
        assert_eq!(
            fs::read_to_string(root.join("drafts/chapter-01.md")).unwrap(),
            "# Two\n"
        );
        // No temp file left behind.
        assert_eq!(fs::read_dir(root.join("drafts")).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_escaping_names() {
        let mut s = FsNoteStore::new(tmpdir("esc"));
        assert!(s.save("../x.md", "").is_err());
        assert!(s.save("/etc/x.md", "").is_err());
    }

    #[test]
    fn autosave_tracks_revisions_and_failures() {
        let mut store = MemoryNoteStore::default();
        let mut b = Buffer::new("");
        let mut a = Autosave::new(b.revision());
        b.insert("hello");
        assert!(a.dirty(b.revision()));
        a.sync(&mut store, "n.md", b.text(), b.revision());
        assert_eq!(a.state, SaveState::Saved);
        assert_eq!(store.notes["n.md"], "hello");
        store.fail = true;
        b.insert("!");
        a.sync(&mut store, "n.md", b.text(), b.revision());
        assert_eq!(a.state.label(), "not saved");
        store.fail = false;
        a.sync(&mut store, "n.md", b.text(), b.revision());
        assert_eq!(store.notes["n.md"], "hello!");
        assert!(!a.dirty(b.revision()));
    }
}
