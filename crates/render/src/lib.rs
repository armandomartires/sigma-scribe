//! Sigma Scribe rendering: an 8-bit greyscale canvas, the brief's design
//! tokens, real fonts, and the v0.1 editor screen. Device-independent: the
//! FBInk backend blits the canvas to the Forma, the desktop backend writes PNG.

pub mod canvas;
pub mod editor;
pub mod files;
pub mod fonts;
pub mod theme;

pub use canvas::{Canvas, Rect};
pub use editor::{BarAction, EditorScreen, Hit, SelGeom, SelUi};
pub use files::{FilesHit, FilesModel, Row};
pub use fonts::{Face, Fonts};
pub use theme::Theme;
