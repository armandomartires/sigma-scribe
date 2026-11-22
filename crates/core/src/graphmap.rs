//! Graph map layout (v1.0, S9.1): positions for the notes of the link
//! graph, computed deterministically.
//!
//! Re-exported from [`sigma_plugin_api::graphmap`] so plugins can use the
//! same helpers through `sigma-plugin-api` without depending on
//! `sigma-core` internals. Behaviour is unchanged.

pub use sigma_plugin_api::graphmap::{layout, layout_key, MapLayout, INNER, ITERATIONS, RING};
