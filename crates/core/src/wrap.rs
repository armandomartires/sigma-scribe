//! Soft wrapping by measured width.
//!
//! Re-exported from [`sigma_plugin_api::wrap`] so plugins can use the same
//! helpers through `sigma-plugin-api` without depending on `sigma-core`
//! internals. Behaviour is unchanged.

pub use sigma_plugin_api::wrap::{
    caret_cell, offset_at, wrap, MonoMeasure, TextMeasure, VisualLine,
};
