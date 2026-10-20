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

pub const SCRATCH: &str = ".scratchpad.md";
const STATE: &str = ".sigma-last-opened";

/// A validated path relative to the library root ("" = root folder).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NotePath(String);

impl NotePath {
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

    #[must_use]
    pub fn root() -> Self {
        Self(String::new())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or("")
    }

    #[must_use]
    pub fn parent(&self) -> NotePath {
        match self.0.rfind('/') {
            Some(i) => Self(self.0[..i].to_owned()),
            None => Self::root(),
        }
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Folder,
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: NotePath,
    pub kind: Kind,
    /// Words in a note (0 for folders).
    pub words: usize,
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

#[derive(Clone, Debug)]
pub struct Library {
    root: PathBuf,
}

impl Library {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn fs_path(&self, p: &NotePath) -> PathBuf {
        if p.is_root() {
            self.root.clone()
        } else {
            self.root.join(p.as_str())
        }
    }

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
        fs::rename(self.fs_path(p), self.fs_path(&target))?;
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

    #[must_use]
    pub fn last_opened(&self) -> Option<NotePath> {
        let s = fs::read_to_string(self.root.join(STATE)).ok()?;
        let p = NotePath::parse(s.trim()).ok()?;
        (self.fs_path(&p).is_file()).then_some(p)
    }

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
