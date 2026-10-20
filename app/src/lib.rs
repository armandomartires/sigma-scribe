//! Sigma Scribe v0.2: a library of plain `.md` / `.txt` notes in folders,
//! the Files sheet, rename in place, and the v0.1 editor with on-screen
//! keyboard and autosave. Generic over the device so the Forma, the desktop
//! window and the headless emulator run the same code.

use sigma_core::buffer::TabKind;
use sigma_core::keyboard::{Dir, Key, Layout};
use sigma_core::library::SCRATCH;
use sigma_core::{
    Autosave, Buffer, Entry, FsNoteStore, KeyAction, Keyboard, Kind, Library, NotePath, NoteStore,
};
use sigma_device::{Device, Event, HwKey, Refresh};
use sigma_render::editor::Model;
use sigma_render::{Canvas, EditorScreen, FilesHit, FilesModel, Hit, Rect, Row, Theme};
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum RenameTarget {
    Current,
    Entry(NotePath),
}

#[derive(Clone, Debug, Default)]
pub struct FilesState {
    pub folder: NotePath,
    pub entries: Vec<Entry>,
    pub more: Option<usize>,
    pub armed: Option<usize>,
    pub moving: Option<NotePath>,
    pub targets: Vec<NotePath>,
    pub message: Option<String>,
    pub first: usize,
}

pub struct App {
    pub library: Library,
    pub current: NotePath,
    pub buffer: Buffer,
    pub keyboard: Keyboard,
    pub autosave: Autosave,
    pub keys_on: bool,
    pub screen: EditorScreen,
    store: FsNoteStore,
    pub files: Option<FilesState>,
    rename: Option<(RenameTarget, Buffer)>,
    pub canvas: Canvas,
    shown: Canvas,
    pub running: bool,
}

impl App {
    /// Opens the library at `root`, seeding it on first run, and restores
    /// the last opened note.
    #[must_use]
    pub fn new(width: u32, height: u32, root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let library = Library::new(&root);
        let _ = library.seed(&SEED_FILES);
        let current = library
            .last_opened()
            .or_else(|| NotePath::parse(NOTE).ok().filter(|p| library.exists(p)))
            .or_else(|| library.all_notes().ok().and_then(|n| n.into_iter().next()))
            .unwrap_or_else(|| NotePath::parse(NOTE).unwrap_or_default());
        let store = FsNoteStore::new(&root);
        let body = store
            .load(current.as_str())
            .ok()
            .flatten()
            .unwrap_or_default();
        let buffer = Buffer::new(body);
        let _ = library.set_last_opened(&current);
        let layout = load_layout(&root);
        Self {
            autosave: Autosave::new(buffer.revision()),
            library,
            current,
            buffer,
            keyboard: Keyboard::with_layout(layout),
            keys_on: true,
            screen: EditorScreen::new(width, height),
            store,
            files: None,
            rename: None,
            canvas: Canvas::new(width, height, 255),
            shown: Canvas::new(width, height, 0),
            running: true,
        }
    }

    /// Remembers Full / Compact in `notes/.sigma-settings`.
    fn save_settings(&self) {
        let _ = sigma_core::store::write_atomic(
            &self.library.root().join(SETTINGS),
            &format!("keyboard={}\n", self.keyboard.layout.name()),
        );
    }

    fn after_key(&mut self, action: KeyAction) -> Option<Refresh> {
        let page = self.screen.layout(self.keys_visible(), &self.keyboard).page;
        match action {
            KeyAction::Edited => {
                self.save();
                Some(Refresh::Fast)
            }
            KeyAction::Moved | KeyAction::Relabelled => Some(Refresh::Fast),
            KeyAction::Vertical { down, extend } => {
                self.screen
                    .caret_row_ext(&mut self.buffer, page, down, extend);
                Some(Refresh::Fast)
            }
            KeyAction::Page { forward, extend } => {
                self.screen
                    .page_turn_ext(&mut self.buffer, page, forward, extend);
                Some(Refresh::Partial)
            }
            KeyAction::Relayout => {
                self.save_settings();
                Some(Refresh::Full)
            }
            KeyAction::Escape | KeyAction::Ignored => None,
        }
    }

    fn save(&mut self) {
        let rev = self.buffer.revision();
        self.autosave.sync(
            &mut self.store,
            self.current.as_str(),
            self.buffer.text(),
            rev,
        );
    }

    /// Saves the open note and opens another.
    pub fn open(&mut self, p: &NotePath) {
        self.save();
        let body = self
            .store
            .load(p.as_str())
            .ok()
            .flatten()
            .unwrap_or_default();
        self.current = p.clone();
        self.buffer = Buffer::new(body);
        self.autosave = Autosave::new(self.buffer.revision());
        self.screen.scroll = 0;
        let _ = self.library.set_last_opened(p);
    }

    #[must_use]
    pub fn is_renaming(&self) -> bool {
        self.rename.is_some()
    }

    #[must_use]
    pub fn rename_text(&self) -> Option<&str> {
        self.rename.as_ref().map(|(_, b)| b.text())
    }

    fn keys_visible(&self) -> bool {
        self.rename.is_some() || (self.files.is_none() && self.keys_on)
    }

    fn rows(&self) -> Vec<Row> {
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

    fn files_model<'a>(&'a self, rows: &'a [Row]) -> Option<FilesModel<'a>> {
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
            theme: Theme::DAY,
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
        self.save();
        self.files = Some(FilesState {
            folder: self.current.parent(),
            ..FilesState::default()
        });
        self.refresh_files();
    }

    /// Redraws and sends only what changed, with the given refresh class.
    ///
    /// # Errors
    /// The device could not present the frame.
    pub fn render(&mut self, dev: &mut dyn Device, refresh: Refresh) -> std::io::Result<()> {
        let mut c = std::mem::replace(&mut self.canvas, Canvas::new(1, 1, 0));
        if self.files.is_some() {
            let rows = self.rows();
            if let Some(m) = self.files_model(&rows) {
                self.screen.draw_files(&mut c, &m);
            }
        } else {
            let renaming_current = matches!(&self.rename, Some((RenameTarget::Current, _)));
            let model = Model {
                file_name: self.current.name(),
                buffer: &self.buffer,
                keyboard: &self.keyboard,
                keys_on: self.keys_visible(),
                save: &self.autosave.state,
                theme: Theme::DAY,
                rename: if renaming_current {
                    self.rename.as_ref().map(|(_, b)| b)
                } else {
                    None
                },
            };
            self.screen.draw(&mut c, &model);
        }
        self.canvas = c;
        // Only what changed goes to the panel: a pressed modifier redraws its
        // cap, a typed letter its text row and the word count.
        let regions = if refresh == Refresh::Full {
            vec![self.canvas.bounds()]
        } else {
            self.canvas.diff_regions(&self.shown, 24, 8)
        };
        for r in &regions {
            dev.present(&self.canvas, *r, refresh)?;
        }
        if !regions.is_empty() {
            self.shown.pixels.copy_from_slice(&self.canvas.pixels);
        }
        Ok(())
    }

    fn start_rename(&mut self, target: RenameTarget) {
        let name = match &target {
            RenameTarget::Current => self.current.name().to_owned(),
            RenameTarget::Entry(p) => p.name().to_owned(),
        };
        let k = &self.keyboard;
        self.keyboard = Keyboard {
            layout: k.layout,
            clipboard: k.clipboard.clone(),
            ..Keyboard::default()
        };
        self.rename = Some((target, Buffer::new(name)));
    }

    fn finish_rename(&mut self, ok: bool) {
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
                self.refresh_files();
            }
            Err(e) => {
                if let Some(f) = &mut self.files {
                    f.message = Some(e.to_string());
                }
            }
        }
    }

    fn type_into_field(&mut self, key: Key) -> Option<Refresh> {
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

    fn hw_into_field(&mut self, k: HwKey) -> Option<Refresh> {
        let (_, field) = self.rename.as_mut()?;
        match k {
            HwKey::Char(c) => field.insert(&c.to_string()),
            HwKey::Backspace => field.backspace(),
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
            HwKey::Tab | HwKey::Up | HwKey::Down => {}
        }
        Some(Refresh::Fast)
    }

    fn files_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
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

    /// Applies one event; returns the refresh it needs (or `None`).
    pub fn handle(&mut self, e: Event) -> Option<Refresh> {
        let page = self.screen.layout(self.keys_visible(), &self.keyboard).page;
        match e {
            Event::Quit | Event::Power => {
                self.save();
                self.running = false;
                None
            }
            Event::Key(k) if self.rename.is_some() => self.hw_into_field(k),
            Event::Tap { x, y } if self.files.is_some() => self.files_tap(x, y),
            Event::PageTurn { forward } if self.files.is_some() => {
                let rows = self.rows();
                let visible = self
                    .files_model(&rows)
                    .map_or(1, |m| self.screen.files_layout(&m).visible.max(1));
                let f = self.files.as_mut()?;
                let n = if f.moving.is_some() {
                    f.targets.len()
                } else {
                    f.entries.len()
                };
                f.first = if forward {
                    (f.first + visible).min(n.saturating_sub(1))
                } else {
                    f.first.saturating_sub(visible)
                };
                Some(Refresh::Partial)
            }
            Event::Key(HwKey::Escape) if self.files.is_some() => {
                self.files = None;
                Some(Refresh::Full)
            }
            Event::Key(_) if self.files.is_some() => None,
            Event::PageTurn { forward } => {
                self.screen.page_turn(&mut self.buffer, page, forward);
                Some(Refresh::Partial)
            }
            Event::Key(k) => {
                match k {
                    HwKey::Char(c) => self.buffer.insert(&c.to_string()),
                    HwKey::Backspace => self.buffer.backspace(),
                    HwKey::Delete => self.buffer.delete_forward(),
                    HwKey::Return => self.buffer.newline(),
                    HwKey::Tab => self.buffer.insert_tab(TabKind::default()),
                    HwKey::Left => self.buffer.left(),
                    HwKey::Right => self.buffer.right(),
                    HwKey::Up => self.screen.caret_row(&mut self.buffer, page, false),
                    HwKey::Down => self.screen.caret_row(&mut self.buffer, page, true),
                    HwKey::Escape => return None,
                }
                self.save();
                Some(Refresh::Fast)
            }
            Event::Tap { x, y } => {
                let renaming = matches!(&self.rename, Some((RenameTarget::Current, _)));
                let model = Model {
                    file_name: self.current.name(),
                    buffer: &self.buffer,
                    keyboard: &self.keyboard,
                    keys_on: self.keys_visible(),
                    save: &self.autosave.state,
                    theme: Theme::DAY,
                    rename: if renaming {
                        self.rename.as_ref().map(|(_, b)| b)
                    } else {
                        None
                    },
                };
                match self.screen.hit(&model, x, y) {
                    Hit::Key(k) if self.rename.is_some() => self.type_into_field(k),
                    Hit::Key(k) => {
                        let a = self.keyboard.press(k, &mut self.buffer, TabKind::default());
                        self.after_key(a)
                    }
                    Hit::ToggleKeys => {
                        self.keys_on = !self.keys_on;
                        Some(Refresh::Partial)
                    }
                    Hit::Caret(at) => {
                        self.buffer.set_caret(at);
                        Some(Refresh::Fast)
                    }
                    Hit::Files => {
                        self.open_files();
                        Some(Refresh::Full)
                    }
                    Hit::StartRename => {
                        self.start_rename(RenameTarget::Current);
                        Some(Refresh::Partial)
                    }
                    Hit::RenameOk => {
                        self.finish_rename(true);
                        Some(Refresh::Partial)
                    }
                    Hit::RenameCancel => {
                        self.finish_rename(false);
                        Some(Refresh::Partial)
                    }
                    Hit::Nothing => None,
                }
            }
        }
    }

    /// Main loop until power/quit or end of input.
    ///
    /// # Errors
    /// The device failed to present a frame.
    pub fn run(&mut self, dev: &mut dyn Device) -> std::io::Result<()> {
        self.render(dev, Refresh::Full)?;
        while self.running {
            let Some(e) = dev.next_event() else { break };
            if let Some(r) = self.handle(e) {
                self.render(dev, r)?;
            }
        }
        self.save();
        Ok(())
    }

    /// Centre of the on-screen key with this label (emulator scripts).
    #[must_use]
    pub fn key_centre(&self, label: &str) -> Option<(i32, i32)> {
        let rows = self.rows();
        let keys = match self.files_model(&rows) {
            Some(m) if self.rename.is_some() => self.screen.files_layout(&m).base.keys,
            _ => self.screen.layout(self.keys_visible(), &self.keyboard).keys,
        };
        keys.iter()
            .find(|(_, k)| self.keyboard.label(*k) == label)
            .map(|(r, _)| centre(*r))
    }

    /// Centre of a named control (emulator scripts): `Files`, `Title`,
    /// `OK`, `Cancel`, `Close`, `Up`, `New note`, `New folder`, `Rename`,
    /// `Move`, `Delete`, `More:<row name>`, or a row name.
    #[must_use]
    pub fn control_centre(&self, label: &str) -> Option<(i32, i32)> {
        let rows = self.rows();
        if let Some(m) = self.files_model(&rows) {
            let l = self.screen.files_layout(&m);
            let find_row = |name: &str| l.rows.iter().find(|(i, _, _)| rows[*i].name == name);
            return match label {
                "OK" => Some(centre(l.base.ok_rect)),
                "Cancel" if self.rename.is_some() => Some(centre(l.base.cancel_rect)),
                "Cancel" => Some(centre(l.bottom[0].union(&l.bottom[1]))),
                "Close" => Some(centre(l.close)),
                "Up" => Some(centre(l.crumb)),
                "New note" => Some(centre(l.bottom[0])),
                "New folder" => Some(centre(l.bottom[1])),
                "Rename" => l.actions.map(|a| centre(a[0])),
                "Move" => l.actions.map(|a| centre(a[1])),
                "Delete" => l.actions.map(|a| centre(a[2])),
                s if s.starts_with("More:") => find_row(&s[5..]).map(|(_, _, m)| centre(*m)),
                s => find_row(s).map(|(_, r, _)| (r.x + r.w / 3, r.y + r.h / 2)),
            };
        }
        let l = self.screen.layout(self.keys_visible(), &self.keyboard);
        match label {
            "Files" => Some(centre(l.files_button)),
            "Title" => Some(centre(l.name_rect)),
            "OK" => Some(centre(l.ok_rect)),
            "Cancel" => Some(centre(l.cancel_rect)),
            "Keys" => Some(centre(l.keys_button)),
            _ => None,
        }
    }

    #[must_use]
    pub fn page_rect(&self) -> Rect {
        self.screen.layout(self.keys_visible(), &self.keyboard).page
    }
}

const SETTINGS: &str = ".sigma-settings";

fn load_layout(root: &std::path::Path) -> Layout {
    std::fs::read_to_string(root.join(SETTINGS))
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("keyboard=").and_then(Layout::parse))
        })
        .unwrap_or_default()
}

fn centre(r: Rect) -> (i32, i32) {
    (r.x + r.w / 2, r.y + r.h / 2)
}
