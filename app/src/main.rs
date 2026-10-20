//! Sigma Scribe v0.1: one note, on-screen keyboard, autosave.

use kobo_sdk::{Context, KoboApp, StoreResult};
use sigma_core::buffer::TabKind;
use sigma_core::{Autosave, Buffer, KeyAction, Keyboard};
use sigma_ui::{move_vertical, resolve, EditorView, Intent};

const NOTE: &str = "field-notes.md";
const SEED: &str = include_str!("../seed/field-notes.md");

/// Cobalt store keys allow only `[a-z0-9._-]`, at most 256 keys and
/// 256 KiB per value (kobo-protocol `is_valid_key`). v0.1 has one note; S2
/// replaces this with id-based keys plus an export path for USB.
fn key(name: &str) -> String {
    format!("note.{name}")
}

struct Scribe {
    buffer: Buffer,
    keyboard: Keyboard,
    autosave: Autosave,
    keys_on: bool,
    scroll: usize,
    loaded: bool,
    in_flight: Option<u64>,
}

impl Default for Scribe {
    fn default() -> Self {
        Self {
            buffer: Buffer::new(""),
            keyboard: Keyboard::default(),
            autosave: Autosave::new(0),
            keys_on: true,
            scroll: 0,
            loaded: false,
            in_flight: None,
        }
    }
}

impl Scribe {
    fn view(&self) -> EditorView<'_> {
        EditorView {
            file_name: NOTE,
            buffer: &self.buffer,
            keyboard: &self.keyboard,
            keys_on: self.keys_on,
            save: &self.autosave.state,
            scroll: self.scroll,
        }
    }

    fn show(&mut self, context: &mut Context) {
        if !self.loaded {
            context.set_screen(
                kobo_sdk::ScreenBuilder::new("loading")
                    .top_bar(NOTE)
                    .activity("Opening", None)
                    .build(),
            );
            return;
        }
        let metrics = context.metrics();
        let mut view = self.view();
        let screen = view.build(&metrics);
        self.scroll = view.scroll;
        context.set_screen(screen);
    }

    fn save(&mut self, context: &mut Context) {
        if self.in_flight.is_none() && self.autosave.dirty(self.buffer.revision()) {
            self.autosave.begin();
            self.in_flight = Some(self.buffer.revision());
            context
                .store()
                .save(key(NOTE), self.buffer.text().as_bytes().to_vec());
        }
    }

    fn columns(&self, context: &Context) -> u16 {
        self.view().grid(&context.metrics()).0
    }
}

impl KoboApp for Scribe {
    fn on_start(&mut self, context: &mut Context) {
        context.store().load(key(NOTE));
        self.show(context);
    }

    fn on_action(&mut self, context: &mut Context, action: kobo_sdk::ActionId) {
        if !self.loaded {
            return;
        }
        match resolve(action, &self.keyboard) {
            Some(Intent::Key(k)) => {
                if self.keyboard.press(k, &mut self.buffer, TabKind::default()) == KeyAction::Edited
                {
                    self.save(context);
                }
            }
            Some(Intent::ToggleKeys) => self.keys_on = !self.keys_on,
            Some(Intent::Left) => self.buffer.left(),
            Some(Intent::Right) => self.buffer.right(),
            Some(Intent::Up | Intent::Down) => {
                let cols = self.columns(context);
                move_vertical(
                    &mut self.buffer,
                    cols,
                    matches!(resolve(action, &self.keyboard), Some(Intent::Down)),
                );
            }
            None => return,
        }
        self.show(context);
    }

    /// Page-turn buttons scroll the page (brief §3); the caret follows.
    fn on_page_turn(&mut self, context: &mut Context, forward: bool) {
        let cols = self.columns(context);
        let (_, rows) = self.view().grid(&context.metrics());
        for _ in 0..rows.saturating_sub(1) {
            move_vertical(&mut self.buffer, cols, forward);
        }
        self.show(context);
    }

    fn on_background(&mut self, context: &mut Context) {
        self.save(context);
    }

    fn can_suspend(&self) -> bool {
        !self.autosave.dirty(self.buffer.revision())
    }

    fn on_store(&mut self, context: &mut Context, result: StoreResult) {
        match result {
            StoreResult::Loaded { value, .. } => {
                let body = value
                    .and_then(|v| String::from_utf8(v).ok())
                    .unwrap_or_else(|| SEED.to_owned());
                self.buffer = Buffer::new(body);
                self.autosave = Autosave::new(self.buffer.revision());
                self.loaded = true;
            }
            StoreResult::Saved { .. } => {
                if let Some(rev) = self.in_flight.take() {
                    self.autosave.landed(rev);
                }
                // Keys typed while the write was in flight.
                self.save(context);
            }
            StoreResult::Denied(error) => {
                self.in_flight = None;
                self.autosave.failed(error.to_string());
                if !self.loaded {
                    self.buffer = Buffer::new(SEED);
                    self.loaded = true;
                }
            }
            _ => {}
        }
        self.show(context);
    }
}

fn main() {
    let _ = kobo_sdk::run("sigma-scribe", Scribe::default());
}
