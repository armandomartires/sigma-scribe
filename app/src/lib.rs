//! Sigma Scribe v0.1: one note, on-screen keyboard, autosave to a plain
//! `.md` file. Generic over the device so the Forma, the desktop window and
//! the headless emulator run the same code.

use sigma_core::buffer::TabKind;
use sigma_core::{Autosave, Buffer, FsNoteStore, KeyAction, Keyboard, NoteStore};
use sigma_device::{Device, Event, HwKey, Refresh};
use sigma_render::editor::Model;
use sigma_render::{Canvas, EditorScreen, Hit, Rect, Theme};
use std::path::PathBuf;

pub const NOTE: &str = "field-notes.md";
pub const SEED: &str = include_str!("../seed/field-notes.md");

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

pub struct App {
    pub buffer: Buffer,
    pub keyboard: Keyboard,
    pub autosave: Autosave,
    pub keys_on: bool,
    pub screen: EditorScreen,
    pub store: FsNoteStore,
    pub canvas: Canvas,
    shown: Canvas,
    pub running: bool,
}

impl App {
    #[must_use]
    pub fn new(width: u32, height: u32, store: FsNoteStore) -> Self {
        let body = store
            .load(NOTE)
            .ok()
            .flatten()
            .unwrap_or_else(|| SEED.to_owned());
        let buffer = Buffer::new(body);
        let mut app = Self {
            autosave: Autosave::new(buffer.revision()),
            buffer,
            keyboard: Keyboard::default(),
            keys_on: true,
            screen: EditorScreen::new(width, height),
            store,
            canvas: Canvas::new(width, height, 255),
            shown: Canvas::new(width, height, 0),
            running: true,
        };
        // First run writes the seed so the file exists on USB storage.
        app.autosave = Autosave::new(u64::MAX);
        app.save();
        app
    }

    fn model(&self) -> Model<'_> {
        Model {
            file_name: NOTE,
            buffer: &self.buffer,
            keyboard: &self.keyboard,
            keys_on: self.keys_on,
            save: &self.autosave.state,
            theme: Theme::DAY,
        }
    }

    fn save(&mut self) {
        let rev = self.buffer.revision();
        self.autosave
            .sync(&mut self.store, NOTE, self.buffer.text(), rev);
    }

    /// Redraws and sends only what changed, with the given refresh class.
    ///
    /// # Errors
    /// The device could not present the frame.
    pub fn render(&mut self, dev: &mut dyn Device, refresh: Refresh) -> std::io::Result<()> {
        let mut c = std::mem::replace(&mut self.canvas, Canvas::new(1, 1, 0));
        let model = Model {
            file_name: NOTE,
            buffer: &self.buffer,
            keyboard: &self.keyboard,
            keys_on: self.keys_on,
            save: &self.autosave.state,
            theme: Theme::DAY,
        };
        self.screen.draw(&mut c, &model);
        self.canvas = c;
        let region = if refresh == Refresh::Full {
            Some(self.canvas.bounds())
        } else {
            self.canvas.diff(&self.shown)
        };
        if let Some(r) = region {
            dev.present(&self.canvas, r, refresh)?;
            self.shown.pixels.copy_from_slice(&self.canvas.pixels);
        }
        Ok(())
    }

    /// Applies one event; returns the refresh it needs (or `None`).
    pub fn handle(&mut self, e: Event) -> Option<Refresh> {
        let page = self.screen.layout(self.keys_on, &self.keyboard).page;
        match e {
            Event::Quit | Event::Power => {
                self.save();
                self.running = false;
                None
            }
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
            Event::Tap { x, y } => match self.screen.hit(&self.model(), x, y) {
                Hit::Key(k) => {
                    if self.keyboard.press(k, &mut self.buffer, TabKind::default())
                        == KeyAction::Edited
                    {
                        self.save();
                    }
                    Some(Refresh::Fast)
                }
                Hit::ToggleKeys => {
                    self.keys_on = !self.keys_on;
                    Some(Refresh::Partial)
                }
                Hit::Caret(at) => {
                    self.buffer.set_caret(at);
                    Some(Refresh::Fast)
                }
                Hit::Nothing => None,
            },
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
        self.screen
            .layout(self.keys_on, &self.keyboard)
            .keys
            .iter()
            .find(|(_, k)| self.keyboard.label(*k) == label)
            .map(|(r, _)| (r.x + r.w / 2, r.y + r.h / 2))
    }

    #[must_use]
    pub fn page_rect(&self) -> Rect {
        self.screen.layout(self.keys_on, &self.keyboard).page
    }
}
