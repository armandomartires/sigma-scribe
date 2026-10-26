//! Σscribe rendering: an 8-bit greyscale canvas, the brief's design
//! tokens, real fonts, and the v0.1 editor screen. Device-independent: the
//! FBInk backend blits the canvas to the Forma, the desktop backend writes PNG.

#![warn(missing_docs)]

pub mod canvas;
pub mod drawer;
pub mod editor;
pub mod files;
pub mod findbar;
pub mod fonts;
pub mod graph;
pub mod graphmap;
pub mod help;
pub mod i18n;
pub mod icons;
pub mod ink;
pub mod options;
pub mod plugin_screen;
pub mod pointer;
pub mod preview;
pub mod radio;
pub mod scrollbar;
pub(crate) mod shape;
pub mod slider;
pub mod switch;
pub mod theme;
pub mod typeface;

pub use canvas::{Canvas, Rect};
pub use drawer::{DrawerHit, DrawerModel, Pane};
pub use editor::{BarAction, EditorScreen, Hit, KbButton, PreviewModel, SelGeom, SelUi};
pub use files::{FilesHit, FilesModel, Row};
pub use findbar::{FindField, FindHit, FindModel};
pub use fonts::{Face, Fonts};
pub use graph::{GraphHit, GraphModel, GraphRow};
pub use graphmap::{MapView, ZOOMS};
pub use help::{HelpHit, HelpModel, HelpPage};
pub use options::{OptionsHit, OptionsModel, SettingsPage};
pub use preview::Doc;
pub use theme::Theme;
