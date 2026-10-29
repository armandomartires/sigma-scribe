//! The note library (brief §6 Files, §8 Data): plain `.md` / `.txt` files in
//! nested folders under one root, e.g. `/mnt/onboard/sigma-scribe/notes`.
//!
//! Every path the app handles is a [`NotePath`]: relative, `/`-separated,
//! no `..`, no absolute paths, no hidden components except the scratch file.
//! Names are cleaned; collisions get a ` 2`, ` 3`, … suffix instead of
//! overwriting. The last real note cannot be deleted.

use crate::text::clean_name;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// File name of the scratch note at the library root (the one hidden file
/// [`NotePath::parse`] accepts).
pub const SCRATCH: &str = ".scratchpad.md";
const STATE: &str = ".sigma-last-opened";
/// Folder under the library root that deleted notes are moved into.
pub const TRASH: &str = ".trash";
/// File in each trash folder (`.trash/<id>/`) that names the path the item had.
/// A path cannot start with a dot, so it never clashes with the item.
const ORIGIN: &str = ".origin";

/// A validated path relative to the library root ("" = root folder).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NotePath(String);

impl NotePath {
    /// Parses and checks a library path; leading and trailing `/` are
    /// ignored, and an empty string is the root.
    ///
    /// # Errors
    /// The path is absolute, escapes the root, or has hidden/empty parts.
    pub fn parse(s: &str) -> io::Result<Self> {
        let s = s.trim_matches('/');
        if s.is_empty() {
            return Ok(Self::root());
        }
        if s == SCRATCH {
            return Ok(Self(s.to_owned()));
        }
        for part in s.split('/') {
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.starts_with('.')
                || part.contains('\\')
                || part.chars().any(char::is_control)
            {
                return Err(invalid(format!("unsafe path {s:?}")));
            }
        }
        Ok(Self(s.to_owned()))
    }

    /// The library root folder (the empty path).
    #[must_use]
    pub fn root() -> Self {
        Self(String::new())
    }

    /// The path as a `/`-separated string (`""` for the root).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is the root folder.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// The last component (`""` for the root).
    #[must_use]
    pub fn name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or("")
    }

    /// The containing folder; the root for top-level paths and for the root
    /// itself.
    #[must_use]
    pub fn parent(&self) -> NotePath {
        match self.0.rfind('/') {
            Some(i) => Self(self.0[..i].to_owned()),
            None => Self::root(),
        }
    }

    /// The path of `name` inside this folder. `name` is not checked.
    #[must_use]
    pub fn join(&self, name: &str) -> NotePath {
        if self.is_root() {
            Self(name.to_owned())
        } else {
            Self(format!("{}/{name}", self.0))
        }
    }

    /// `self` is `other` or inside it.
    #[must_use]
    pub fn within(&self, other: &NotePath) -> bool {
        other.is_root() || self.0 == other.0 || self.0.starts_with(&format!("{}/", other.0))
    }
}

impl fmt::Display for NotePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_root() { "/" } else { &self.0 })
    }
}

fn invalid(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg)
}

/// What a listed [`Entry`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A folder.
    Folder,
    /// A `.md` or `.txt` note file.
    Note,
}

/// One item of a folder listing, as returned by [`Library::list`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Path of the item in the library.
    pub path: NotePath,
    /// Whether it is a folder or a note.
    pub kind: Kind,
    /// Words in a note (0 for folders).
    pub words: usize,
}

/// One deleted note or folder in the trash, as returned by
/// [`Library::list_trash`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trashed {
    /// The trash folder it sits in (`.trash/<id>/`): the deletion time in
    /// unix seconds, with `-2`, `-3`… when several went in the same second.
    pub id: String,
    /// The path it had in the library.
    pub path: NotePath,
    /// When it was deleted, in unix seconds.
    pub deleted: u64,
    /// Bytes in it, a note's ink included.
    pub size: u64,
    /// Whether it was a folder (with everything in it).
    pub folder: bool,
}

/// `.md` or `.txt` (case-insensitive).
#[must_use]
pub fn is_note_name(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    (l.ends_with(".md") || l.ends_with(".txt")) && l.len() > 3
}

fn split_ext(name: &str) -> (&str, &str) {
    let l = name.to_ascii_lowercase();
    for ext in [".md", ".txt"] {
        if l.ends_with(ext) && name.len() > ext.len() {
            return (
                &name[..name.len() - ext.len()],
                &name[name.len() - ext.len()..],
            );
        }
    }
    (name, "")
}

/// Cleans a user-typed note name, keeping or adding an allowed extension.
#[must_use]
pub fn clean_note_name(raw: &str, default_ext: &str) -> String {
    let cleaned = clean_name(raw);
    let (stem, ext) = split_ext(&cleaned);
    let stem = clean_name(stem);
    let ext = if ext.is_empty() {
        default_ext.to_owned()
    } else {
        ext.to_ascii_lowercase()
    };
    format!("{stem}{ext}")
}

/// The deletion time in a trash folder's name (`1730000000`, `1730000000-2`).
fn bin_time(id: &str) -> Option<u64> {
    let (secs, n) = id.split_once('-').unwrap_or((id, "2"));
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    secs.parse().ok()
}

/// The path the item in the trash folder `bin` had: its record, or, for a
/// folder from before there were records, the single chain of folders down
/// to a note (or to a folder with several things in it).
fn bin_origin(bin: &Path) -> Option<NotePath> {
    if let Ok(s) = fs::read_to_string(bin.join(ORIGIN)) {
        return NotePath::parse(&s).ok().filter(|p| !p.is_root());
    }
    let mut parts: Vec<String> = Vec::new();
    let mut dir = bin.to_path_buf();
    loop {
        let mut kids: Vec<(String, bool)> = fs::read_dir(&dir)
            .ok()?
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_str()?.to_owned();
                (!n.starts_with('.')).then(|| (n, e.path().is_dir()))
            })
            .collect();
        match (kids.len(), kids.pop()) {
            (1, Some((n, true))) => {
                parts.push(n.clone());
                dir.push(n);
            }
            (1, Some((n, false))) => {
                parts.push(n);
                break;
            }
            _ => break,
        }
    }
    NotePath::parse(&parts.join("/"))
        .ok()
        .filter(|p| !p.is_root())
}

/// Bytes in a file, or in every file under a folder.
fn tree_size(p: &Path) -> u64 {
    let Ok(md) = fs::symlink_metadata(p) else {
        return 0;
    };
    if !md.is_dir() {
        return md.len();
    }
    fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| tree_size(&e.path())).sum())
        .unwrap_or(0)
}

/// Removes the trash folder `bin` once nothing is left in it but its record
/// and empty folders. Anything else stays where it is.
fn prune(bin: &Path) {
    fn empty_dirs(d: &Path) -> bool {
        let Ok(rd) = fs::read_dir(d) else {
            return false;
        };
        let mut all = true;
        for e in rd.flatten() {
            let p = e.path();
            if !(p.is_dir() && !e.file_type().is_ok_and(|t| t.is_symlink()) && empty_dirs(&p)) {
                all = false;
            }
        }
        all && fs::remove_dir(d).is_ok()
    }
    let origin = bin.join(ORIGIN);
    let record = fs::read_to_string(&origin).ok();
    let _ = fs::remove_file(&origin);
    if !empty_dirs(bin) {
        // Something is left: keep the record so it can still be found.
        if let Some(r) = record {
            let _ = fs::write(origin, r);
        }
    }
}

/// The note library: a root folder on disk and the operations on the notes
/// and folders under it.
#[derive(Clone, Debug)]
pub struct Library {
    root: PathBuf,
}

impl Library {
    /// A library rooted at `root`. Nothing is created or checked on disk.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The root folder on disk.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The on-disk path of `p`.
    #[must_use]
    pub fn fs_path(&self, p: &NotePath) -> PathBuf {
        if p.is_root() {
            self.root.clone()
        } else {
            self.root.join(p.as_str())
        }
    }

    /// Whether `p` exists on disk.
    #[must_use]
    pub fn exists(&self, p: &NotePath) -> bool {
        self.fs_path(p).exists()
    }

    /// Folders first, then notes; both alphabetical (case-insensitive).
    /// Hidden files, temp files and non-note files are not listed.
    ///
    /// # Errors
    /// The folder cannot be read.
    pub fn list(&self, folder: &NotePath) -> io::Result<Vec<Entry>> {
        let mut out = Vec::new();
        let dir = self.fs_path(folder);
        if !dir.exists() {
            return Ok(out);
        }
        for e in fs::read_dir(dir)? {
            let e = e?;
            let Some(name) = e.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let path = folder.join(&name);
            let ft = e.file_type()?;
            if ft.is_dir() {
                out.push(Entry {
                    path,
                    kind: Kind::Folder,
                    words: 0,
                });
            } else if ft.is_file() && is_note_name(&name) {
                let words = fs::read_to_string(e.path())
                    .map(|t| crate::text::word_count(&t))
                    .unwrap_or(0);
                out.push(Entry {
                    path,
                    kind: Kind::Note,
                    words,
                });
            }
        }
        out.sort_by(|a, b| {
            (a.kind == Kind::Note, a.path.name().to_lowercase())
                .cmp(&(b.kind == Kind::Note, b.path.name().to_lowercase()))
        });
        Ok(out)
    }

    /// Every folder (root first), for the Move picker.
    ///
    /// # Errors
    /// A folder cannot be read.
    pub fn folders(&self) -> io::Result<Vec<NotePath>> {
        let mut out = vec![NotePath::root()];
        let mut i = 0;
        while i < out.len() {
            let here = out[i].clone();
            let mut subs: Vec<NotePath> = self
                .list(&here)?
                .into_iter()
                .filter(|e| e.kind == Kind::Folder)
                .map(|e| e.path)
                .collect();
            let at = i + 1;
            for (n, s) in subs.drain(..).enumerate() {
                out.insert(at + n, s);
            }
            i += 1;
        }
        Ok(out)
    }

    /// All real notes (not the scratch file), depth-first.
    ///
    /// # Errors
    /// A folder cannot be read.
    pub fn all_notes(&self) -> io::Result<Vec<NotePath>> {
        let mut out = Vec::new();
        for f in self.folders()? {
            out.extend(
                self.list(&f)?
                    .into_iter()
                    .filter(|e| e.kind == Kind::Note)
                    .map(|e| e.path),
            );
        }
        Ok(out)
    }

    /// A free name in `folder` based on `name` (`a.md`, `a 2.md`, `a 3.md`…).
    #[must_use]
    pub fn unique(&self, folder: &NotePath, name: &str) -> NotePath {
        let (stem, ext) = split_ext(name);
        let taken = |n: &str| {
            let dir = self.fs_path(folder);
            fs::read_dir(&dir).ok().is_some_and(|rd| {
                rd.flatten().any(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|x| x.eq_ignore_ascii_case(n))
                })
            })
        };
        let mut candidate = name.to_owned();
        let mut n = 2;
        while taken(&candidate) {
            candidate = format!("{stem} {n}{ext}");
            n += 1;
        }
        folder.join(&candidate)
    }

    /// Creates an empty note in `folder` from a user-typed name (`untitled`
    /// if blank, `.md` if no extension), with a numeric suffix if the name is
    /// taken. Returns its path.
    ///
    /// # Errors
    /// The note cannot be created.
    pub fn create_note(&self, folder: &NotePath, raw_name: &str) -> io::Result<NotePath> {
        let name = clean_note_name(
            if raw_name.trim().is_empty() {
                "untitled"
            } else {
                raw_name
            },
            ".md",
        );
        let p = self.unique(folder, &name);
        fs::create_dir_all(self.fs_path(folder))?;
        crate::store::write_atomic(&self.fs_path(&p), "")?;
        Ok(p)
    }

    /// Creates a folder in `parent` from a user-typed name (`new folder` if
    /// blank), with a numeric suffix if the name is taken. Returns its path.
    ///
    /// # Errors
    /// The folder cannot be created.
    pub fn create_folder(&self, parent: &NotePath, raw_name: &str) -> io::Result<NotePath> {
        let name = clean_name(if raw_name.trim().is_empty() {
            "new folder"
        } else {
            raw_name
        });
        let p = self.unique(parent, &name);
        fs::create_dir_all(self.fs_path(&p))?;
        Ok(p)
    }

    /// Renames in place. Notes keep (or get) `.md` / `.txt`; a taken name
    /// gets a numeric suffix. Returns the new path.
    ///
    /// # Errors
    /// Missing source, the scratch file, or a filesystem error.
    pub fn rename(&self, p: &NotePath, raw_name: &str) -> io::Result<NotePath> {
        self.guard(p)?;
        let before = self.all_notes().unwrap_or_default();
        let src = self.fs_path(p);
        let name = if src.is_dir() {
            clean_name(raw_name)
        } else {
            let (_, ext) = split_ext(p.name());
            clean_note_name(raw_name, if ext.is_empty() { ".md" } else { ext })
        };
        if name == p.name() {
            return Ok(p.clone());
        }
        // Case-only rename on vfat: go through a temp name.
        let target = if name.eq_ignore_ascii_case(p.name()) {
            p.parent().join(&name)
        } else {
            self.unique(&p.parent(), &name)
        };
        if name.eq_ignore_ascii_case(p.name()) {
            let tmp = self.unique(&p.parent(), &format!(".rename-{name}"));
            fs::rename(&src, self.fs_path(&tmp))?;
            fs::rename(self.fs_path(&tmp), self.fs_path(&target))?;
        } else {
            fs::rename(&src, self.fs_path(&target))?;
        }
        if !self.fs_path(&target).is_dir() {
            crate::ink::carry(&src, &self.fs_path(&target))?;
        }
        self.relink(&before, p, &target)?;
        Ok(target)
    }

    /// Moves a note or folder into `dest`. Returns the new path.
    ///
    /// # Errors
    /// Moving a folder into itself, the scratch file, or a filesystem error.
    pub fn move_to(&self, p: &NotePath, dest: &NotePath) -> io::Result<NotePath> {
        self.guard(p)?;
        if self.fs_path(p).is_dir() && dest.within(p) {
            return Err(invalid(format!("cannot move {p} into itself")));
        }
        if !self.fs_path(dest).is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no folder {dest}"),
            ));
        }
        if p.parent() == *dest {
            return Ok(p.clone());
        }
        let target = self.unique(dest, p.name());
        let before = self.all_notes().unwrap_or_default();
        let was_dir = self.fs_path(p).is_dir();
        fs::rename(self.fs_path(p), self.fs_path(&target))?;
        if !was_dir {
            crate::ink::carry(&self.fs_path(p), &self.fs_path(&target))?;
        }
        self.relink(&before, p, &target)?;
        Ok(target)
    }

    /// Deletes a note or a folder with everything in it. Refuses to delete
    /// the last real note.
    ///
    /// # Errors
    /// The scratch file, the last note, or a filesystem error.
    pub fn delete(&self, p: &NotePath) -> io::Result<()> {
        self.guard(p)?;
        let path = self.fs_path(p);
        let remaining = self
            .all_notes()?
            .into_iter()
            .filter(|n| !n.within(p))
            .count();
        if remaining == 0 {
            return Err(invalid("the last note cannot be deleted".into()));
        }
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }

    /// Moves a note or a folder into the trash instead of deleting it:
    /// `.trash/<unix seconds>/<its path>` under the library root, with the
    /// note's ink beside it. Nothing is listed or indexed there, and the
    /// files can be copied back over USB. Refuses what [`Library::delete`]
    /// refuses.
    ///
    /// # Errors
    /// The scratch file, the last note, or a filesystem error.
    pub fn trash(&self, p: &NotePath) -> io::Result<PathBuf> {
        self.guard(p)?;
        let remaining = self
            .all_notes()?
            .into_iter()
            .filter(|n| !n.within(p))
            .count();
        if remaining == 0 {
            return Err(invalid("the last note cannot be deleted".into()));
        }
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let base = self.root.join(TRASH);
        let mut bin = base.join(secs.to_string());
        let mut n = 2;
        while bin.exists() {
            bin = base.join(format!("{secs}-{n}"));
            n += 1;
        }
        let from = self.fs_path(p);
        let to = bin.join(p.as_str());
        if let Some(d) = to.parent() {
            fs::create_dir_all(d)?;
        }
        // Written first, so that a folder holding one subfolder is not taken
        // for a note in a subfolder when it is listed.
        fs::write(bin.join(ORIGIN), p.as_str())?;
        let was_dir = from.is_dir();
        if let Err(e) = fs::rename(&from, &to) {
            prune(&bin);
            return Err(e);
        }
        if !was_dir {
            crate::ink::carry(&from, &to)?;
        }
        Ok(to)
    }

    /// What is in the trash, newest first: the path each item had, when it
    /// was deleted and how big it is. Items whose folder was made by an
    /// older version (no record of the path) are told apart as well as the
    /// layout allows.
    ///
    /// # Errors
    /// The trash folder exists and cannot be read.
    pub fn list_trash(&self) -> io::Result<Vec<Trashed>> {
        let base = self.root.join(TRASH);
        if !base.is_dir() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for e in fs::read_dir(&base)? {
            let e = e?;
            let Some(id) = e.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(deleted) = bin_time(&id) else {
                continue;
            };
            let bin = e.path();
            let Some(path) = bin_origin(&bin) else {
                continue;
            };
            let item = bin.join(path.as_str());
            let folder = item.is_dir();
            if !folder && !item.is_file() {
                continue;
            }
            let mut size = tree_size(&item);
            if !folder {
                if let Some(ink) = item.parent().zip(item.file_stem()) {
                    size += tree_size(&ink.0.join(".ink").join(ink.1));
                }
            }
            out.push(Trashed {
                id,
                path,
                deleted,
                size,
                folder,
            });
        }
        out.sort_by(|a, b| (b.deleted, b.id.len(), &b.id).cmp(&(a.deleted, a.id.len(), &a.id)));
        Ok(out)
    }

    /// How many items are in the trash. Cheaper than [`Library::list_trash`]:
    /// it does not measure them.
    #[must_use]
    pub fn trash_len(&self) -> usize {
        let Ok(rd) = fs::read_dir(self.root.join(TRASH)) else {
            return 0;
        };
        rd.flatten()
            .filter(|e| {
                let bin = e.path();
                e.file_name().to_str().and_then(bin_time).is_some()
                    && bin_origin(&bin).is_some_and(|p| bin.join(p.as_str()).exists())
            })
            .count()
    }

    /// Puts the trash item `id` (from [`Trashed::id`]) back at the path it
    /// had, making its folders again. If that path is taken, it comes back
    /// as `name (restored).md` (`(restored 2)`… and so on) and nothing is
    /// overwritten. A note's ink comes with it. Returns the path it got.
    ///
    /// # Errors
    /// No such item, or a filesystem error.
    pub fn restore(&self, id: &str) -> io::Result<NotePath> {
        if bin_time(id).is_none() {
            return Err(invalid(format!("{id:?} is not in the trash")));
        }
        let bin = self.root.join(TRASH).join(id);
        let path = bin_origin(&bin)
            .filter(|p| bin.join(p.as_str()).exists())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not in the trash"))?;
        let from = bin.join(path.as_str());
        let was_dir = from.is_dir();
        let folder = path.parent();
        let name = path.name();
        let mut target = path.clone();
        if self.taken(&target) {
            let (stem, ext) = if was_dir { (name, "") } else { split_ext(name) };
            let mut n = 1;
            loop {
                let tag = if n == 1 {
                    "restored".to_owned()
                } else {
                    format!("restored {n}")
                };
                target = folder.join(&format!("{stem} ({tag}){ext}"));
                if !self.taken(&target) {
                    break;
                }
                n += 1;
            }
        }
        let to = self.fs_path(&target);
        if let Some(d) = to.parent() {
            fs::create_dir_all(d)?;
        }
        fs::rename(&from, &to)?;
        if !was_dir {
            crate::ink::carry(&from, &to)?;
        }
        prune(&bin);
        Ok(target)
    }

    /// Deletes everything in the trash for good, and returns how many items
    /// there were. The one permanent delete: it only follows the user asking
    /// for it.
    ///
    /// # Errors
    /// A filesystem error.
    pub fn empty_trash(&self) -> io::Result<usize> {
        let n = self.list_trash()?.len();
        match fs::remove_dir_all(self.root.join(TRASH)) {
            Ok(()) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(e),
        }
    }

    /// Whether anything is at `p`, ignoring case like [`Library::unique`].
    fn taken(&self, p: &NotePath) -> bool {
        fs::read_dir(self.fs_path(&p.parent()))
            .ok()
            .is_some_and(|rd| {
                rd.flatten().any(|e| {
                    e.file_name()
                        .to_str()
                        .is_some_and(|x| x.eq_ignore_ascii_case(p.name()))
                })
            })
    }

    /// After `old` (a note or folder) became `new` (v1.5.2): rewrites
    /// `[[wiki]]` links in every note and the scratch pad that pointed at a
    /// moved note (`links::rewrite`), instead of leaving them broken.
    fn relink(&self, before: &[NotePath], old: &NotePath, new: &NotePath) -> io::Result<()> {
        let moved: Vec<(NotePath, NotePath)> = before
            .iter()
            .filter(|n| *n == old || n.within(old))
            .filter_map(|n| {
                let tail = &n.as_str()[old.as_str().len()..];
                NotePath::parse(&format!("{}{tail}", new.as_str()))
                    .ok()
                    .map(|m| (n.clone(), m))
            })
            .collect();
        if moved.is_empty() {
            return Ok(());
        }
        let mut files = self.all_notes().unwrap_or_default();
        if let Ok(s) = NotePath::parse(SCRATCH) {
            files.push(s);
        }
        for f in files {
            let path = self.fs_path(&f);
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            if let Some(fixed) = crate::links::rewrite(&text, before, &moved) {
                crate::store::write_atomic(&path, &fixed)?;
            }
        }
        Ok(())
    }

    fn guard(&self, p: &NotePath) -> io::Result<()> {
        if p.is_root() || p.as_str() == SCRATCH {
            return Err(invalid(format!("{p} cannot be changed")));
        }
        if !self.exists(p) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{p} does not exist"),
            ));
        }
        Ok(())
    }

    /// Writes the seed library on first run (brief §8). Idempotent.
    ///
    /// # Errors
    /// The library root cannot be written.
    pub fn seed(&self, files: &[(&str, &str)]) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        if !self.all_notes()?.is_empty() {
            return Ok(());
        }
        for (name, body) in files {
            let p = NotePath::parse(name)?;
            let fsp = self.fs_path(&p);
            if let Some(dir) = fsp.parent() {
                fs::create_dir_all(dir)?;
            }
            if !fsp.exists() {
                crate::store::write_atomic(&fsp, body)?;
            }
        }
        Ok(())
    }

    /// The note recorded by [`Library::set_last_opened`], if the record can
    /// be read, parses, and still names an existing file.
    #[must_use]
    pub fn last_opened(&self) -> Option<NotePath> {
        let s = fs::read_to_string(self.root.join(STATE)).ok()?;
        let p = NotePath::parse(s.trim()).ok()?;
        (self.fs_path(&p).is_file()).then_some(p)
    }

    /// Records `p` as the last opened note, in a hidden state file at the
    /// library root.
    ///
    /// # Errors
    /// The state file cannot be written.
    pub fn set_last_opened(&self, p: &NotePath) -> io::Result<()> {
        crate::store::write_atomic(&self.root.join(STATE), p.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(tag: &str) -> Library {
        let d = std::env::temp_dir().join(format!("sigma-lib-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        Library::new(d)
    }

    fn names(l: &Library, f: &NotePath) -> Vec<String> {
        l.list(f)
            .unwrap()
            .into_iter()
            .map(|e| e.path.name().to_owned())
            .collect()
    }

    #[test]
    fn note_paths_refuse_unsafe_input() {
        for bad in [
            "../x.md",
            "a/../../b.md",
            "a//b.md",
            ".hidden.md",
            "a/.git/x.md",
            "a\\b.md",
            "x\u{0}.md",
            "./x.md",
        ] {
            assert!(NotePath::parse(bad).is_err(), "{bad:?} accepted");
        }
        // A leading slash is treated as library-relative, never absolute.
        assert_eq!(
            NotePath::parse("/etc/passwd").unwrap().as_str(),
            "etc/passwd"
        );
        assert!(NotePath::parse("../x.md").is_err());
        assert!(NotePath::parse("a/../b.md").is_err());
        assert!(NotePath::parse(".hidden.md").is_err());
        assert!(NotePath::parse("drafts/chapter-01.md").is_ok());
        assert_eq!(
            NotePath::parse("/drafts/x.md/").unwrap().as_str(),
            "drafts/x.md"
        );
        assert!(NotePath::parse(SCRATCH).is_ok());
    }

    #[test]
    fn names_are_cleaned_and_extensions_kept() {
        assert_eq!(clean_note_name("  My  ../Note ", ".md"), "My Note.md");
        assert_eq!(clean_note_name("plan.TXT", ".md"), "plan.txt");
        assert_eq!(clean_note_name("a/b", ".md"), "a b.md");
        assert_eq!(clean_note_name("", ".md"), "untitled.md");
        assert_eq!(clean_note_name("...", ".txt"), "untitled.txt");
        assert!(
            is_note_name("x.md")
                && is_note_name("Y.TXT")
                && !is_note_name("x.pdf")
                && !is_note_name(".md")
        );
    }

    #[test]
    fn create_list_and_collisions() {
        let l = lib("create");
        let r = NotePath::root();
        assert_eq!(l.create_note(&r, "").unwrap().as_str(), "untitled.md");
        assert_eq!(
            l.create_note(&r, "untitled").unwrap().as_str(),
            "untitled 2.md"
        );
        assert_eq!(
            l.create_note(&r, "UNTITLED.md").unwrap().as_str(),
            "UNTITLED 3.md"
        );
        let f = l.create_folder(&r, "drafts").unwrap();
        assert_eq!(l.create_folder(&r, "drafts").unwrap().as_str(), "drafts 2");
        l.create_note(&f, "chapter").unwrap();
        fs::write(l.fs_path(&r).join("photo.jpg"), b"x").unwrap();
        fs::write(l.fs_path(&r).join(".hidden.md"), b"x").unwrap();
        assert_eq!(
            names(&l, &r),
            [
                "drafts",
                "drafts 2",
                "untitled 2.md",
                "UNTITLED 3.md",
                "untitled.md"
            ]
        );
        assert_eq!(l.all_notes().unwrap().len(), 4);
        assert_eq!(
            l.folders()
                .unwrap()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["/", "drafts", "drafts 2"]
        );
    }

    #[test]
    fn rename_cleans_avoids_collisions_and_keeps_extension() {
        let l = lib("rename");
        let r = NotePath::root();
        let a = l.create_note(&r, "a").unwrap();
        let b = l.create_note(&r, "b.txt").unwrap();
        assert_eq!(l.rename(&b, "a").unwrap().as_str(), "a.txt"); // .txt kept, no clash with a.md
        let c = l.create_note(&r, "c").unwrap();
        assert_eq!(l.rename(&c, "a.md").unwrap().as_str(), "a 2.md"); // collision → suffix
        assert_eq!(l.rename(&a, "A").unwrap().as_str(), "A.md"); // case-only rename
        assert!(l.exists(&NotePath::parse("A.md").unwrap()));
        assert_eq!(
            l.rename(&NotePath::parse("A.md").unwrap(), "../../evil")
                .unwrap()
                .as_str(),
            "evil.md"
        );
        let f = l.create_folder(&r, "f").unwrap();
        assert_eq!(l.rename(&f, "Folder / One").unwrap().as_str(), "Folder One");
        assert!(l
            .rename(&NotePath::parse("missing.md").unwrap(), "x")
            .is_err());
    }

    #[test]
    fn rename_and_move_rewrite_wiki_links_elsewhere() {
        let d = std::env::temp_dir().join(format!("sigma-relink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("drafts")).unwrap();
        fs::create_dir_all(d.join("archive")).unwrap();
        fs::write(d.join("drafts/chapter.md"), "# Chapter\nsee [[hub]]\n").unwrap();
        fs::write(
            d.join("hub.md"),
            "[[chapter]] [[Chapter|ch. 1]] [[drafts/chapter#Intro]] `[[chapter]]`\n",
        )
        .unwrap();
        fs::write(d.join(SCRATCH), "todo [[chapter]]\n").unwrap();
        let lib = Library::new(&d);
        let ch = NotePath::parse("drafts/chapter.md").unwrap();
        let new = lib.rename(&ch, "opening").unwrap();
        assert_eq!(
            fs::read_to_string(d.join("hub.md")).unwrap(),
            "[[opening]] [[opening|ch. 1]] [[drafts/opening#Intro]] `[[chapter]]`\n"
        );
        assert_eq!(
            fs::read_to_string(d.join(SCRATCH)).unwrap(),
            "todo [[opening]]\n"
        );
        // Moving the folder: path links follow it.
        let moved = lib
            .move_to(
                &NotePath::parse("drafts").unwrap(),
                &NotePath::parse("archive").unwrap(),
            )
            .unwrap();
        assert_eq!(moved.as_str(), "archive/drafts");
        assert!(fs::read_to_string(d.join("hub.md"))
            .unwrap()
            .contains("[[archive/drafts/opening#Intro]]"));
        // Self-links in the renamed note follow too.
        let _ = new;
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn move_rules() {
        let l = lib("move");
        let r = NotePath::root();
        let f = l.create_folder(&r, "f").unwrap();
        let g = l.create_folder(&f, "g").unwrap();
        let n = l.create_note(&r, "n").unwrap();
        l.create_note(&f, "n").unwrap();
        assert_eq!(l.move_to(&n, &f).unwrap().as_str(), "f/n 2.md");
        assert!(l.move_to(&f, &g).is_err(), "folder into its own child");
        assert!(l.move_to(&f, &f).is_err(), "folder into itself");
        assert!(l
            .move_to(
                &NotePath::parse("f/n.md").unwrap(),
                &NotePath::parse("nope").unwrap()
            )
            .is_err());
        assert_eq!(l.move_to(&g, &r).unwrap().as_str(), "g");
    }

    #[test]
    fn delete_rules() {
        let l = lib("delete");
        let r = NotePath::root();
        let a = l.create_note(&r, "a").unwrap();
        assert!(l.delete(&a).is_err(), "last note");
        let f = l.create_folder(&r, "f").unwrap();
        let b = l.create_note(&f, "b").unwrap();
        assert!(l.delete(&b).is_ok());
        l.create_note(&f, "c").unwrap();
        assert!(l.delete(&f).is_ok(), "folder with notes, others remain");
        assert!(!l.exists(&f));
        let f2 = l.create_folder(&r, "f2").unwrap();
        l.create_note(&f2, "d").unwrap();
        l.delete(&a).unwrap();
        assert!(l.delete(&f2).is_err(), "folder holding the last note");
        fs::write(l.fs_path(&r).join(SCRATCH), "").unwrap();
        assert!(
            l.delete(&NotePath::parse(SCRATCH).unwrap()).is_err(),
            "scratch is undeletable"
        );
        assert!(l.delete(&r).is_err());
    }

    #[test]
    fn trash_moves_instead_of_deleting() {
        let l = lib("trash");
        let r = NotePath::root();
        let a = l.create_note(&r, "a").unwrap();
        let f = l.create_folder(&r, "f").unwrap();
        let b = l.create_note(&f, "b").unwrap();
        fs::write(l.fs_path(&b), "keep me").unwrap();
        let to = l.trash(&b).unwrap();
        assert!(!l.exists(&b));
        assert_eq!(fs::read_to_string(&to).unwrap(), "keep me");
        assert!(to.starts_with(l.fs_path(&r).join(TRASH)));
        assert!(to.ends_with("f/b.md"));
        assert_eq!(
            l.all_notes().unwrap(),
            vec![a.clone()],
            "trash is not listed"
        );
        // The same path trashed twice does not collide.
        let b2 = l.create_note(&f, "b").unwrap();
        let to2 = l.trash(&b2).unwrap();
        assert_ne!(to, to2);
        // A folder goes with everything in it.
        let g = l.create_folder(&r, "g").unwrap();
        l.create_note(&g, "c").unwrap();
        let tg = l.trash(&g).unwrap();
        assert!(tg.join("c.md").is_file());
        // The same refusals as delete.
        assert!(l.trash(&a).is_err(), "last note");
        assert!(l.trash(&r).is_err());
        fs::write(l.fs_path(&r).join(SCRATCH), "").unwrap();
        assert!(l.trash(&NotePath::parse(SCRATCH).unwrap()).is_err());
    }

    /// The trash items as (path, id), newest first.
    fn bin(l: &Library) -> Vec<(String, String)> {
        l.list_trash()
            .unwrap()
            .into_iter()
            .map(|t| (t.path.as_str().to_owned(), t.id))
            .collect()
    }

    /// Moves a trash folder's time back, so that the order is not a matter
    /// of the clock's second.
    fn age(l: &Library, id: &str, secs: u64) -> String {
        let new = secs.to_string();
        let base = l.root().join(TRASH);
        fs::rename(base.join(id), base.join(&new)).unwrap();
        new
    }

    #[test]
    fn the_trash_lists_newest_first_with_path_time_and_size() {
        let l = lib("trash-list");
        let r = NotePath::root();
        assert_eq!(l.list_trash().unwrap(), vec![], "no trash folder yet");
        assert_eq!(l.trash_len(), 0);
        l.create_note(&r, "keep").unwrap();
        let f = l.create_folder(&r, "f").unwrap();
        let old = l.create_note(&f, "old").unwrap();
        fs::write(l.fs_path(&old), "12345").unwrap();
        let new = l.create_note(&r, "new").unwrap();
        fs::write(l.fs_path(&new), "abc").unwrap();
        l.trash(&old).unwrap();
        l.trash(&new).unwrap();
        let ids: Vec<String> = bin(&l).into_iter().map(|(_, id)| id).collect();
        assert_eq!(ids.len(), 2);
        age(&l, &ids[0], 1_000);
        age(&l, &ids[1], 2_000);
        let t = l.list_trash().unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(l.trash_len(), 2);
        let (a, b) = (&t[0], &t[1]);
        assert_eq!((a.deleted, b.deleted), (2_000, 1_000), "newest first");
        assert_eq!(a.path.as_str(), "f/old.md");
        assert_eq!(a.size, 5);
        assert!(!a.folder);
        assert_eq!(b.path.as_str(), "new.md");
        assert_eq!(b.size, 3);
    }

    #[test]
    fn a_note_deleted_twice_in_a_second_lists_twice_newest_first() {
        let l = lib("trash-same-second");
        let r = NotePath::root();
        l.create_note(&r, "keep").unwrap();
        for body in ["one", "two", "three"] {
            let a = l.create_note(&r, "a").unwrap();
            fs::write(l.fs_path(&a), body).unwrap();
            l.trash(&a).unwrap();
        }
        // Make all three the same second, which is what a quick run does.
        let base = l.root().join(TRASH);
        let ids: Vec<String> = bin(&l).into_iter().map(|(_, id)| id).collect();
        assert_eq!(ids.len(), 3);
        let t = l.list_trash().unwrap();
        assert!(t
            .windows(2)
            .all(|w| (w[0].deleted, w[0].id.len()) >= (w[1].deleted, w[1].id.len())));
        // The newest of the same second is the one with the highest suffix.
        let newest = &t[0];
        assert_eq!(
            fs::read_to_string(base.join(&newest.id).join("a.md")).unwrap(),
            "three"
        );
    }

    #[test]
    fn restore_puts_a_note_back_and_makes_its_folders() {
        let l = lib("restore");
        let r = NotePath::root();
        l.create_note(&r, "keep").unwrap();
        let deep = l.create_folder(&r, "a").unwrap();
        let deep = l.create_folder(&deep, "b").unwrap();
        let n = l.create_note(&deep, "x").unwrap();
        fs::write(l.fs_path(&n), "words").unwrap();
        l.trash(&n).unwrap();
        // The folders go too, as when the user deletes them.
        let a = NotePath::parse("a").unwrap();
        l.trash(&a).unwrap();
        assert!(!l.exists(&a));
        let t = l.list_trash().unwrap();
        assert_eq!(
            t.iter().map(|t| t.path.as_str()).collect::<Vec<_>>(),
            ["a", "a/b/x.md"],
            "the folder, then the note in it",
        );
        // The note first: its folders are made again.
        let note = t.iter().find(|t| t.path == n).unwrap();
        // (The folder was trashed last, so the note's own bin is older.)
        assert!(!note.folder);
        let back = l.restore(&note.id).unwrap();
        assert_eq!(back, n);
        assert_eq!(fs::read_to_string(l.fs_path(&n)).unwrap(), "words");
        assert_eq!(l.all_notes().unwrap().len(), 2);
        assert_eq!(l.trash_len(), 1);
        // Its trash folder is gone, not left empty.
        assert!(!l.root().join(TRASH).join(&note.id).exists());
    }

    #[test]
    fn restore_never_overwrites_it_takes_a_free_name() {
        let l = lib("restore-clash");
        let r = NotePath::root();
        let f = l.create_folder(&r, "f").unwrap();
        l.create_note(&r, "keep").unwrap();
        let a = l.create_note(&f, "a").unwrap();
        fs::write(l.fs_path(&a), "first").unwrap();
        l.trash(&a).unwrap();
        // A new note took the name.
        let a2 = l.create_note(&f, "a").unwrap();
        assert_eq!(a2, a);
        fs::write(l.fs_path(&a2), "second").unwrap();
        let id = l.list_trash().unwrap()[0].id.clone();
        let back = l.restore(&id).unwrap();
        assert_eq!(back.as_str(), "f/a (restored).md");
        assert_eq!(fs::read_to_string(l.fs_path(&a)).unwrap(), "second");
        assert_eq!(fs::read_to_string(l.fs_path(&back)).unwrap(), "first");
        // And again: the next free name.
        l.trash(&a).unwrap();
        let a3 = l.create_note(&f, "a").unwrap();
        fs::write(l.fs_path(&a3), "third").unwrap();
        let id = l.list_trash().unwrap()[0].id.clone();
        assert_eq!(l.restore(&id).unwrap().as_str(), "f/a (restored 2).md");
        assert_eq!(fs::read_to_string(l.fs_path(&a3)).unwrap(), "third");
        assert_eq!(l.trash_len(), 0);
    }

    #[test]
    fn restore_brings_back_a_folder_and_a_notes_ink() {
        let l = lib("restore-folder");
        let r = NotePath::root();
        l.create_note(&r, "keep").unwrap();
        // A folder holding one folder holding one note: not a note in a
        // folder in a folder.
        let g = l.create_folder(&r, "g").unwrap();
        let h = l.create_folder(&g, "h").unwrap();
        let n = l.create_note(&h, "n").unwrap();
        l.trash(&g).unwrap();
        let t = l.list_trash().unwrap();
        assert_eq!(t.len(), 1);
        assert!(t[0].folder);
        assert_eq!(t[0].path, g);
        // The folder name is taken: the folder comes back under a free name.
        l.create_folder(&r, "g").unwrap();
        let back = l.restore(&t[0].id).unwrap();
        assert_eq!(back.as_str(), "g (restored)");
        assert!(l.fs_path(&back).join("h/n.md").is_file());
        assert!(!l.exists(&n), "the other g is still empty");
        // A note with ink: the ink goes to the trash and comes back.
        let p = l.create_note(&r, "pic").unwrap();
        let ink = l.root().join(".ink/pic");
        fs::create_dir_all(&ink).unwrap();
        fs::write(ink.join("1.svg"), "<svg/>").unwrap();
        fs::write(l.fs_path(&p), "![](.ink/pic/1.svg)").unwrap();
        l.trash(&p).unwrap();
        assert!(!ink.exists());
        let t = l.list_trash().unwrap();
        assert_eq!(
            t[0].size,
            "![](.ink/pic/1.svg)".len() as u64 + 6,
            "ink counted"
        );
        assert_eq!(l.restore(&t[0].id).unwrap(), p);
        assert!(ink.join("1.svg").is_file());
        assert_eq!(l.trash_len(), 0);
        assert!(!l.root().join(TRASH).join(&t[0].id).exists());
    }

    #[test]
    fn restore_refuses_what_is_not_in_the_trash() {
        let l = lib("restore-bad");
        let r = NotePath::root();
        l.create_note(&r, "keep").unwrap();
        for id in ["", "..", "../x", "1/2", "abc", "12-", "nope"] {
            assert!(l.restore(id).is_err(), "{id:?}");
        }
        assert!(l.restore("12345").is_err(), "no such folder");
        assert!(l.exists(&NotePath::parse("keep.md").unwrap()));
    }

    #[test]
    fn a_trash_folder_from_before_the_records_is_still_listed_and_restored() {
        let l = lib("trash-legacy");
        let r = NotePath::root();
        l.create_note(&r, "keep").unwrap();
        let b = l.root().join(TRASH);
        fs::create_dir_all(b.join("100/f/.ink/n")).unwrap();
        fs::write(b.join("100/f/n.md"), "old").unwrap();
        fs::write(b.join("100/f/.ink/n/1.svg"), "s").unwrap();
        fs::create_dir_all(b.join("200/g")).unwrap();
        fs::write(b.join("200/g/x.md"), "x").unwrap();
        fs::write(b.join("200/g/y.md"), "y").unwrap();
        let t = l.list_trash().unwrap();
        assert_eq!(
            t.iter()
                .map(|t| (t.path.as_str(), t.folder))
                .collect::<Vec<_>>(),
            [("g", true), ("f/n.md", false)]
        );
        assert_eq!(l.restore("100").unwrap().as_str(), "f/n.md");
        assert!(
            l.root().join("f/.ink/n/1.svg").is_file(),
            "ink beside the note"
        );
        assert_eq!(l.restore("200").unwrap().as_str(), "g");
        assert_eq!(l.trash_len(), 0);
    }

    #[test]
    fn empty_trash_removes_everything_under_it_and_nothing_else() {
        let l = lib("trash-empty");
        let r = NotePath::root();
        assert_eq!(l.empty_trash().unwrap(), 0, "nothing to empty");
        l.create_note(&r, "keep").unwrap();
        for n in ["a", "b"] {
            let p = l.create_note(&r, n).unwrap();
            l.trash(&p).unwrap();
        }
        let g = l.create_folder(&r, "g").unwrap();
        l.create_note(&g, "c").unwrap();
        l.trash(&g).unwrap();
        assert_eq!(l.trash_len(), 3);
        assert_eq!(l.empty_trash().unwrap(), 3);
        assert!(!l.root().join(TRASH).exists());
        assert_eq!(l.trash_len(), 0);
        assert!(l.list_trash().unwrap().is_empty());
        assert_eq!(names(&l, &r), ["keep.md"], "the library is untouched");
        // It can be used again afterwards.
        let p = l.create_note(&r, "again").unwrap();
        l.trash(&p).unwrap();
        assert_eq!(l.trash_len(), 1);
    }

    #[test]
    fn seed_and_last_opened() {
        let l = lib("seed");
        l.seed(&[
            ("field-notes.md", "# F"),
            ("drafts/chapter-01.md", "# C"),
            ("scratch.txt", "- a"),
            (SCRATCH, ""),
        ])
        .unwrap();
        assert_eq!(l.all_notes().unwrap().len(), 3);
        assert!(l.fs_path(&NotePath::parse(SCRATCH).unwrap()).exists());
        fs::write(
            l.fs_path(&NotePath::parse("field-notes.md").unwrap()),
            "kept",
        )
        .unwrap();
        l.seed(&[("field-notes.md", "# F")]).unwrap();
        assert_eq!(
            fs::read_to_string(l.fs_path(&NotePath::parse("field-notes.md").unwrap())).unwrap(),
            "kept"
        );
        assert_eq!(l.last_opened(), None);
        let c = NotePath::parse("drafts/chapter-01.md").unwrap();
        l.set_last_opened(&c).unwrap();
        assert_eq!(l.last_opened(), Some(c.clone()));
        fs::remove_file(l.fs_path(&c)).unwrap();
        assert_eq!(l.last_opened(), None, "deleted note is not restored");
        assert_eq!(
            names(&l, &NotePath::root()),
            ["drafts", "field-notes.md", "scratch.txt"]
        );
    }
}
