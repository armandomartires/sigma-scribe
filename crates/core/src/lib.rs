//! Sigma Scribe editor core.
//!
//! Pure Rust with no platform dependency, so every rule in `design-brief.md`
//! that is not about pixels can be tested with `cargo test` on a desktop.
//! The Cobalt (Plan A) and FBInk (Plan B) front ends both sit on top of this.

pub mod buffer;
pub mod find;
pub mod gesture;
pub mod graphmap;
pub mod keyboard;
pub mod layouts;
pub mod library;
pub mod linefit;
pub mod links;
pub mod locale;
pub mod markdown;
pub mod outline;
pub mod settings;
pub mod store;
pub mod text;
pub mod wrap;

pub use buffer::Buffer;
pub use keyboard::{Key, KeyAction, Keyboard, Layer};
pub use library::{Entry, Kind, Library, NotePath};
pub use settings::{Change, Settings};
pub use store::{Autosave, FsNoteStore, MemoryNoteStore, NoteStore, SaveState};
pub use wrap::{wrap, MonoMeasure, TextMeasure, VisualLine};
