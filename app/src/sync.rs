//! Sync in the app (v1.16, `docs/dev/sync-design.md`): the Syncthing supervisor behind
//! Settings → Sync, and noticing that a note changed on disk underneath the
//! app, so that what Syncthing delivers is never written over.

use crate::files::take_text;
use crate::{merge, App};
use sigma_core::library::SCRATCH;
use sigma_core::settings::Change;
use sigma_core::sync::{Cmd, Config, Supervisor, View};
use sigma_core::Autosave;
use sigma_device::Refresh;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file's modification time and length: cheap to compare, and enough to
/// tell that someone else wrote it (Syncthing sets the modification time of
/// what it delivers to the sender's, so the length helps; the text is
/// compared before anything is decided).
pub(crate) type Stamp = (SystemTime, u64);

pub(crate) fn stamp_of(p: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// FNV-1a over the bytes: tells "the same text" without keeping a copy.
pub(crate) fn text_hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// How often the open note is compared with the disk while sync is on, ms.
const WATCH_MS: u64 = 1000;

fn adds_dir() -> PathBuf {
    if cfg!(target_arch = "arm") {
        "/mnt/onboard/.adds/sigma-scribe".into()
    } else {
        "target/emu-sync".into()
    }
}

/// The Syncthing executable: `SIGMA_SYNCTHING`, else the one in the Kobo
/// package (`.adds/sigma-scribe/syncthing/syncthing`), else
/// `target/emu-sync/syncthing/syncthing` on a desktop.
#[must_use]
pub fn syncthing_binary() -> PathBuf {
    std::env::var_os("SIGMA_SYNCTHING").map_or_else(
        || adds_dir().join("syncthing").join("syncthing"),
        PathBuf::from,
    )
}

/// Syncthing's `--home`: `SIGMA_SYNC_HOME`, else `syncthing-home` beside
/// the program (which an update of the package never replaces).
#[must_use]
pub fn syncthing_home() -> PathBuf {
    std::env::var_os("SIGMA_SYNC_HOME")
        .map_or_else(|| adds_dir().join("syncthing-home"), PathBuf::from)
}

/// How to run Syncthing for the notes in `notes`, its log in `log_dir`.
#[must_use]
pub fn sync_config(notes: &Path, log_dir: &Path) -> Config {
    let mut c = Config::new(
        syncthing_binary(),
        syncthing_home(),
        notes.to_path_buf(),
        log_dir.join("syncthing.log"),
    );
    if let Some(p) = std::env::var("SIGMA_SYNC_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        c.port = p;
    }
    c.kill_by_name = cfg!(target_arch = "arm");
    c.ca_file = ca_file_beside(&c.binary);
    c
}

/// `ca-certificates.crt` beside the Syncthing executable, as the Kobo
/// package ships it, when it is there.
#[must_use]
pub fn ca_file_beside(binary: &Path) -> Option<PathBuf> {
    let f = binary.with_file_name("ca-certificates.crt");
    f.is_file().then_some(f)
}

/// `YYYYMMDD-HHMMSS` (UTC) for a conflict file name.
fn compact_time() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let digits: String = sigma_core::log::timestamp(secs)
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    // An unset clock gives question marks: say so rather than guess.
    if digits.len() == 14 {
        format!("{}-{}", &digits[..8], &digits[8..])
    } else {
        "unknown-time".to_owned()
    }
}

/// `name` with `.conflict-<time>` before its extension: `a.md` becomes
/// `a.conflict-20261028-141500.md`. Not Syncthing's own `.sync-conflict-`
/// pattern, which Syncthing prunes after `maxConflicts`: this copy is the
/// reader's own text and is never removed by anything but the reader.
fn conflict_name(name: &str) -> String {
    let (stem, ext) = name.rsplit_once('.').map_or((name, ""), |(s, e)| (s, e));
    let stem = stem.trim_start_matches('.');
    let t = compact_time();
    if ext.is_empty() {
        format!("{stem}.conflict-{t}")
    } else {
        format!("{stem}.conflict-{t}.{ext}")
    }
}

impl App {
    /// What Syncthing last reported (the Sync page shows it).
    #[must_use]
    pub fn sync_view(&self) -> &View {
        &self.sync_view
    }

    /// Shows `v` on the Sync page as if Syncthing had reported it (the
    /// emulator's `sync-demo` step and the tests).
    pub fn set_sync_view(&mut self, v: View) {
        self.sync_view = v;
    }

    /// Replaces how Syncthing is run (tests point it at a fake).
    pub fn set_sync_config(&mut self, cfg: Config) {
        self.sync_cfg = cfg;
    }

    /// Starts Syncthing when Settings → Sync is on and it is not running,
    /// stops it when it is off.
    pub fn apply_sync(&mut self) {
        match (self.sync_wanted(), self.sync.is_some()) {
            (true, false) => {
                sigma_core::info!("sync", "on");
                self.sync = Some(Supervisor::start(self.sync_cfg.clone()));
                self.sync_gen = 0;
            }
            (false, true) => {
                self.stop_sync();
                sigma_core::info!("sync", "off");
            }
            _ => {}
        }
    }

    /// Stops Syncthing and waits for it (a moment: it is asked to shut down,
    /// and killed after eight seconds). Done when sync is turned off and
    /// when the app exits.
    pub fn stop_sync(&mut self) {
        if let Some(s) = self.sync.take() {
            s.stop();
        }
        self.sync_view = View::default();
        self.sync_gen = 0;
    }

    /// The reader's Accept / Ignore on the Sync page: sent to Syncthing, and
    /// taken off the list at once (the next poll puts it back if it failed).
    pub(crate) fn sync_command(&mut self, c: Change) -> Refresh {
        let v = &mut self.sync_view;
        let cmd = match c {
            Change::SyncAcceptDevice(i) | Change::SyncIgnoreDevice(i) => {
                let i = usize::from(i);
                (i < v.pending_devices.len()).then(|| {
                    let d = v.pending_devices.remove(i);
                    if matches!(c, Change::SyncAcceptDevice(_)) {
                        Cmd::AcceptDevice {
                            id: d.id,
                            name: d.name,
                        }
                    } else {
                        Cmd::IgnoreDevice { id: d.id }
                    }
                })
            }
            Change::SyncAcceptFolder(i) | Change::SyncIgnoreFolder(i) => {
                let i = usize::from(i);
                (i < v.pending_folders.len()).then(|| {
                    let f = v.pending_folders.remove(i);
                    if matches!(c, Change::SyncAcceptFolder(_)) {
                        Cmd::AcceptFolder {
                            id: f.id,
                            label: f.label,
                            device: f.device,
                        }
                    } else {
                        Cmd::IgnoreFolder {
                            id: f.id,
                            device: f.device,
                        }
                    }
                })
            }
            _ => None,
        };
        if let (Some(cmd), Some(s)) = (cmd, &self.sync) {
            s.command(cmd);
        }
        Refresh::Partial
    }

    /// On every tick while sync is on: takes in what the supervisor read,
    /// and (once a second) looks for notes changed on disk. Returns the
    /// refresh a visible change needs.
    pub(crate) fn sync_tick(&mut self, t: u64) -> Option<Refresh> {
        if !self.settings.sync {
            return None;
        }
        let mut r = None;
        let page = sigma_render::SettingsPage::Category(sigma_render::options::SYNC_CATEGORY);
        let visible = self.options && self.settings_page == page;
        if let Some(s) = &self.sync {
            s.set_fast(visible);
            let g = s.generation();
            if g != self.sync_gen {
                self.sync_gen = g;
                let new = s.view();
                if new.status() != self.sync_view.status()
                    && !matches!(
                        (new.status(), self.sync_view.status()),
                        (
                            sigma_core::sync::Status::Syncing(..),
                            sigma_core::sync::Status::Syncing(..)
                        )
                    )
                {
                    sigma_core::info!("sync", "status: {:?}", new.status());
                }
                self.sync_view = new;
                if visible {
                    r = Some(Refresh::Partial);
                }
            }
        }
        if t >= self.watch_at {
            self.watch_at = t + WATCH_MS;
            r = merge(r, self.watch_notes());
        }
        r
    }

    pub(crate) fn note_fs_path(&self) -> PathBuf {
        self.library.root().join(self.current.as_str())
    }

    /// Remembers the open note's file as it is now, holding the buffer's
    /// text (just read from it, or just written to it).
    pub(crate) fn remember_note_stamp(&mut self) {
        self.note_stamp = stamp_of(&self.note_fs_path());
        self.note_hash = text_hash(self.buffer.text());
    }

    /// The same for the scratch pad.
    pub(crate) fn remember_scratch_stamp(&mut self) {
        self.scratch_stamp = stamp_of(&self.library.root().join(SCRATCH));
        self.scratch_hash = text_hash(self.scratch.text());
    }

    /// Whether the file at `path` was changed by someone else since `seen`
    /// (when its text had the hash `known`): its new text. A file that is
    /// gone, or was only touched and holds the same text, is not a change.
    fn changed_elsewhere(path: &Path, seen: &mut Option<Stamp>, known: u64) -> Option<String> {
        let now = stamp_of(path)?;
        if seen.is_none_or(|s| s == now) {
            return None;
        }
        let disk = std::fs::read_to_string(path).ok()?;
        *seen = Some(now);
        (text_hash(&disk) != known).then_some(disk)
    }

    /// Before the open note is written: if its file changed on disk since
    /// the app read or wrote it (Syncthing delivered a newer version), the
    /// buffer is **not** written over it. The reader's text goes to
    /// `name.conflict-<time>.ext` beside it, the disk's text becomes the
    /// buffer, and this returns true (nothing to write).
    pub(crate) fn guard_note_write(&mut self) -> bool {
        let path = self.note_fs_path();
        let Some(disk) = Self::changed_elsewhere(&path, &mut self.note_stamp, self.note_hash)
        else {
            return false;
        };
        let name = self.current.name().to_owned();
        let (folder, mine) = (self.current.parent(), self.buffer.text().to_owned());
        let kept = self.keep_conflict(&folder, &name, &mine);
        sigma_core::warn!(
            "sync",
            "{} changed on disk while it was open: your text is kept as {kept}",
            self.current.as_str()
        );
        take_text(&mut self.buffer, &disk);
        self.autosave = Autosave::new(self.buffer.revision());
        self.remember_note_stamp();
        self.after_reload();
        true
    }

    /// The same for the scratch pad.
    pub(crate) fn guard_scratch_write(&mut self) -> bool {
        let path = self.library.root().join(SCRATCH);
        let Some(disk) = Self::changed_elsewhere(&path, &mut self.scratch_stamp, self.scratch_hash)
        else {
            return false;
        };
        let mine = self.scratch.text().to_owned();
        let kept = self.keep_conflict(&sigma_core::NotePath::root(), "scratchpad.md", &mine);
        sigma_core::warn!(
            "sync",
            "the scratch pad changed on disk while it was open: your text is kept as {kept}"
        );
        take_text(&mut self.scratch, &disk);
        self.scratch_save = Autosave::new(self.scratch.revision());
        self.remember_scratch_stamp();
        true
    }

    /// Writes `text` as a conflict copy of `name` in `folder`; the name it
    /// got, or "nothing" when it could not be written (the log says why).
    fn keep_conflict(&mut self, folder: &sigma_core::NotePath, name: &str, text: &str) -> String {
        use sigma_core::NoteStore;
        let free = self.library.unique(folder, &conflict_name(name));
        match self.store.save(free.as_str(), text) {
            Ok(()) => free.as_str().to_owned(),
            Err(e) => {
                sigma_core::error!("sync", "could not keep the conflict copy: {e}");
                "nothing (the write failed)".to_owned()
            }
        }
    }

    /// The text of the open note changed under the preview or the find bar.
    fn after_reload(&mut self) {
        if self.preview.is_some() {
            self.preview = None;
            self.enter_preview();
        }
    }

    /// Takes in a note or the scratch pad that changed on disk while the
    /// reader had no unsaved edit in it.
    fn watch_notes(&mut self) -> Option<Refresh> {
        let mut changed = false;
        let path = self.note_fs_path();
        if !self.read_only && !self.autosave.dirty(self.buffer.revision()) {
            if let Some(disk) = Self::changed_elsewhere(&path, &mut self.note_stamp, self.note_hash)
            {
                sigma_core::info!(
                    "sync",
                    "{} changed on disk: reloaded",
                    self.current.as_str()
                );
                take_text(&mut self.buffer, &disk);
                self.autosave = Autosave::new(self.buffer.revision());
                self.remember_note_stamp();
                self.after_reload();
                changed = true;
            }
        }
        if !self.scratch_save.dirty(self.scratch.revision()) {
            let sp = self.library.root().join(SCRATCH);
            if let Some(disk) =
                Self::changed_elsewhere(&sp, &mut self.scratch_stamp, self.scratch_hash)
            {
                sigma_core::info!("sync", "the scratch pad changed on disk: reloaded");
                take_text(&mut self.scratch, &disk);
                self.scratch_save = Autosave::new(self.scratch.revision());
                self.remember_scratch_stamp();
                changed = true;
            }
        }
        changed.then_some(Refresh::Partial)
    }
}
