//! The Files sheet and renaming notes and folders in place.

use super::*;

/// Replaces a buffer's text with `new` (undoable), caret kept nearby.
fn take_text(b: &mut Buffer, new: &str) {
    let at = b.caret().min(new.len());
    let at = (0..=at)
        .rev()
        .find(|&i| new.is_char_boundary(i))
        .unwrap_or(0);
    b.select_all();
    b.insert(new);
    b.set_caret(at);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RenameTarget {
    Current,
    Entry(NotePath),
}

impl App {
    #[must_use]
    pub fn is_renaming(&self) -> bool {
        self.rename.is_some()
    }

    #[must_use]
    pub fn rename_text(&self) -> Option<&str> {
        self.rename.as_ref().map(|(_, b)| b.text())
    }

    pub(crate) fn rows(&self) -> Vec<Row> {
        let Some(f) = &self.files else {
            return Vec::new();
        };
        if f.moving.is_some() {
            return f
                .targets
                .iter()
                .map(|p| Row {
                    name: if p.is_root() {
                        "Files (top level)".into()
                    } else {
                        p.as_str().replace('/', " / ")
                    },
                    detail: "Folder".into(),
                    folder: true,
                    current: false,
                })
                .collect();
        }
        f.entries
            .iter()
            .map(|e| Row {
                name: e.path.name().to_owned(),
                detail: match e.kind {
                    Kind::Folder => "Folder".into(),
                    Kind::Note if e.words == 1 => "1 word".into(),
                    Kind::Note => format!("{} words", e.words),
                },
                folder: e.kind == Kind::Folder,
                current: e.kind == Kind::Note && (e.path == self.current),
            })
            .collect()
    }

    pub(crate) fn files_model<'a>(&'a self, rows: &'a [Row]) -> Option<FilesModel<'a>> {
        let f = self.files.as_ref()?;
        Some(FilesModel {
            title: if f.folder.is_root() {
                "Files".into()
            } else {
                format!("Files / {}", f.folder.as_str().replace('/', " / "))
            },
            crumb: (!f.folder.is_root()).then(|| {
                let up = f.folder.parent();
                if up.is_root() {
                    "Up to Files".into()
                } else {
                    format!("Up to {}", up.name())
                }
            }),
            rows,
            more: f.more,
            armed: f.armed,
            moving: f.moving.as_ref().map(NotePath::name),
            message: f.message.as_deref(),
            first: f.first,
            theme: self.theme(),
            rename: self.rename.as_ref().map(|(_, b)| b),
            keyboard: &self.keyboard,
        })
    }

    fn refresh_files(&mut self) {
        let lib = self.library.clone();
        if let Some(f) = &mut self.files {
            f.entries = lib.list(&f.folder).unwrap_or_default();
            f.more = None;
            f.armed = None;
            f.first = f.first.min(f.entries.len().saturating_sub(1));
        }
    }

    /// Opens the Files sheet at the current note's folder.
    pub fn open_files(&mut self) {
        sigma_core::debug!("ui", "open files");
        self.help = None;
        self.close_find();
        self.close_graph();
        self.save();
        self.files = Some(FilesState {
            folder: self.current.parent(),
            ..FilesState::default()
        });
        self.refresh_files();
    }

    /// After a rename / move rewrote the open note's ink links (v1.5.1,
    /// `sigma_core::ink::carry`) or wiki links (v1.5.2) on disk: take the new text into the
    /// buffer, keeping the caret as close as it can.
    fn reload_ink_links(&mut self) {
        let path = self.library.root().join(self.current.as_str());
        let Ok(disk) = std::fs::read_to_string(&path) else {
            return;
        };
        if disk != self.buffer.text() {
            take_text(&mut self.buffer, &disk);
            self.save();
        }
        // v1.5.2: `[[wiki]]` links in the scratch pad may have been
        // rewritten too.
        if let Ok(s) = std::fs::read_to_string(self.library.root().join(SCRATCH)) {
            if s != self.scratch.text() {
                take_text(&mut self.scratch, &s);
                self.save_scratch();
            }
        }
    }

    pub(crate) fn start_rename(&mut self, target: RenameTarget) {
        let name = match &target {
            RenameTarget::Current => self.current.name().to_owned(),
            RenameTarget::Entry(p) => p.name().to_owned(),
        };
        let k = &self.keyboard;
        self.keyboard = Keyboard {
            layout: k.layout,
            lang: k.lang,
            clipboard: k.clipboard.clone(),
            ..Keyboard::default()
        };
        self.rename = Some((target, Buffer::new(name)));
    }

    pub(crate) fn finish_rename(&mut self, ok: bool) {
        let Some((target, field)) = self.rename.take() else {
            return;
        };
        if !ok {
            return;
        }
        let path = match &target {
            RenameTarget::Current => self.current.clone(),
            RenameTarget::Entry(p) => p.clone(),
        };
        self.save();
        self.save_scratch();
        match self.library.rename(&path, field.text()) {
            Ok(new) => {
                if self.current == path {
                    self.current = new;
                } else if self.current.within(&path) && !path.is_root() {
                    if let Ok(p) = NotePath::parse(&format!(
                        "{}{}",
                        new.as_str(),
                        &self.current.as_str()[path.as_str().len()..]
                    )) {
                        self.current = p;
                    }
                }
                let _ = self.library.set_last_opened(&self.current);
                self.reload_ink_links();
                self.refresh_files();
            }
            Err(e) => {
                if let Some(f) = &mut self.files {
                    f.message = Some(e.to_string());
                }
            }
        }
    }

    pub(crate) fn type_into_field(&mut self, key: Key) -> Option<Refresh> {
        let (_, field) = self.rename.as_mut()?;
        match key {
            Key::Return => {
                self.finish_rename(true);
                return Some(Refresh::Full);
            }
            Key::Tab | Key::PgUp | Key::PgDn | Key::Arrow(Dir::Up | Dir::Down) => {}
            Key::Esc => {
                self.finish_rename(false);
                return Some(Refresh::Full);
            }
            k => {
                if self.keyboard.press(k, field, TabKind::Spaces(0)) == KeyAction::Relayout {
                    self.save_settings();
                    return Some(Refresh::Full);
                }
            }
        }
        Some(Refresh::Fast)
    }

    pub(crate) fn hw_into_field(&mut self, k: HwKey) -> Option<Refresh> {
        let (_, field) = self.rename.as_mut()?;
        match k {
            HwKey::Char(c) => {
                self.keyboard
                    .composer
                    .type_char(field, self.settings.kb_layout, c);
            }
            HwKey::Backspace => self
                .keyboard
                .composer
                .backspace(field, self.settings.kb_layout),
            HwKey::Delete => field.delete_forward(),
            HwKey::Left => field.left(),
            HwKey::Right => field.right(),
            HwKey::Return => {
                self.finish_rename(true);
                return Some(Refresh::Full);
            }
            HwKey::Escape => {
                self.finish_rename(false);
                return Some(Refresh::Full);
            }
            HwKey::Tab
            | HwKey::Up
            | HwKey::Down
            | HwKey::Home
            | HwKey::End
            | HwKey::PageUp
            | HwKey::PageDown
            | HwKey::Insert => {}
        }
        Some(Refresh::Fast)
    }

    pub(crate) fn files_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let rows = self.rows();
        let hit = self
            .files_model(&rows)
            .map_or(FilesHit::Nothing, |m| self.screen.files_hit(&m, x, y));
        let lib = self.library.clone();
        let f = self.files.as_mut()?;
        let was_armed = f.armed.take();
        f.message = None;
        match hit {
            FilesHit::Close => {
                self.files = None;
                return Some(Refresh::Full);
            }
            FilesHit::Up => {
                f.folder = f.folder.parent();
                f.first = 0;
                self.refresh_files();
            }
            FilesHit::Open(i) => {
                if let Some(target) = f.moving.clone() {
                    let dest = f.targets.get(i).cloned().unwrap_or_default();
                    f.moving = None;
                    self.save();
                    self.save_scratch();
                    match lib.move_to(&target, &dest) {
                        Ok(new) => {
                            if self.current == target {
                                self.current = new;
                            } else if self.current.within(&target) {
                                if let Ok(p) = NotePath::parse(&format!(
                                    "{}{}",
                                    new.as_str(),
                                    &self.current.as_str()[target.as_str().len()..]
                                )) {
                                    self.current = p;
                                }
                            }
                            let _ = lib.set_last_opened(&self.current);
                            self.reload_ink_links();
                            self.refresh_files();
                        }
                        Err(e) => {
                            self.refresh_files();
                            if let Some(f) = &mut self.files {
                                f.message = Some(e.to_string());
                            }
                        }
                    }
                    return Some(Refresh::Partial);
                }
                let e = f.entries.get(i)?.clone();
                if e.kind == Kind::Folder {
                    f.folder = e.path;
                    f.first = 0;
                    self.refresh_files();
                } else {
                    self.files = None;
                    if e.path != self.current {
                        self.push_history();
                    }
                    self.open(&e.path);
                    return Some(Refresh::Full);
                }
            }
            FilesHit::More(i) => f.more = if f.more == Some(i) { None } else { Some(i) },
            FilesHit::Rename(i) => {
                let p = f.entries.get(i)?.path.clone();
                self.start_rename(RenameTarget::Entry(p));
            }
            FilesHit::Move(i) => {
                let p = f.entries.get(i)?.path.clone();
                let is_dir = lib.fs_path(&p).is_dir();
                f.targets = lib
                    .folders()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| !(is_dir && t.within(&p)) && *t != p.parent())
                    .collect();
                f.moving = Some(p);
                f.more = None;
                f.first = 0;
            }
            FilesHit::Delete(i) => {
                if was_armed == Some(i) {
                    let p = f.entries.get(i)?.path.clone();
                    match lib.delete(&p) {
                        Ok(()) => {
                            let gone = self.current.within(&p);
                            self.refresh_files();
                            if gone {
                                let next = lib.all_notes().ok().and_then(|n| n.into_iter().next());
                                if let Some(n) = next {
                                    // The deleted note must not be re-saved.
                                    self.autosave = Autosave::new(self.buffer.revision());
                                    self.current = n.clone();
                                    self.open(&n);
                                }
                            }
                        }
                        Err(e) => f.message = Some(e.to_string()),
                    }
                } else {
                    f.armed = Some(i);
                }
            }
            FilesHit::NewNote => {
                let folder = f.folder.clone();
                match lib.create_note(&folder, "untitled") {
                    Ok(p) => {
                        self.files = None;
                        self.open(&p);
                        self.start_rename(RenameTarget::Current);
                        return Some(Refresh::Full);
                    }
                    Err(e) => f.message = Some(e.to_string()),
                }
            }
            FilesHit::NewFolder => {
                let folder = f.folder.clone();
                match lib.create_folder(&folder, "new folder") {
                    Ok(p) => {
                        self.refresh_files();
                        // Brief: a new folder starts in rename.
                        self.start_rename(RenameTarget::Entry(p));
                    }
                    Err(e) => f.message = Some(e.to_string()),
                }
            }
            FilesHit::CancelMove => {
                f.moving = None;
            }
            FilesHit::Key(k) => return self.type_into_field(k),
            FilesHit::RenameOk => self.finish_rename(true),
            FilesHit::RenameCancel => self.finish_rename(false),
            FilesHit::Nothing => {}
        }
        Some(Refresh::Partial)
    }
}
