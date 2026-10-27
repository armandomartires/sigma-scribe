//! The ink panel's state and timing constants.

use sigma_render::Rect;

/// The writing panel while open (v1.5.0, plan §12.3).
#[derive(Clone, Debug, Default)]
pub struct InkState {
    pub ink: sigma_core::ink::Ink,
    /// Segments drawn into the canvas since the last ink frame.
    pub(crate) pending: Option<Rect>,
    /// Everything inked since the last GL16 cleanup.
    pub(crate) dirty: Option<Rect>,
    /// When the last stroke lifted (ms).
    pub(super) last_up: Option<u64>,
    /// The next render is an ink-only frame.
    pub(crate) fast: bool,
    /// The next `PointerCleanup` render is the ink cleanup pass.
    pub(crate) cleanup: bool,
    /// A button pressed on Down, acted on at Up over the same button.
    pub(super) press: Option<sigma_render::ink::InkButton>,
    /// Write errors are shown in the status line.
    pub error: Option<String>,
    /// Draw, Letters or Train (v1.5.2).
    pub mode: sigma_render::ink::InkMode,
    /// Letters: the box the pending strokes are in.
    pub boxi: Option<usize>,
    /// Letters: candidates for the last letter, and that it was typed by
    /// recognition (so a candidate tap may replace it).
    pub cands: Vec<String>,
    pub(super) replace_ok: bool,
    /// Words: characters typed for the last word (with its space), so a
    /// candidate can replace it; and the lexicon for this panel.
    pub(super) last_len: usize,
    pub(super) lex: Option<sigma_hwr::word::Lexicon>,
    /// Free (v1.5.3): first candidate shown, 123, language filter.
    pub first: usize,
    pub digits: bool,
    pub lang: Option<&'static str>,
    /// Train: samples saved this session.
    pub saved: usize,
    /// The ink of the last reading, to learn from if a candidate corrects
    /// it.
    pub(super) read_ink: Option<ReadInk>,
    /// The learned-samples line the last reading taught, replaced if
    /// another candidate is picked for the same reading.
    pub(super) learned: Option<String>,
    /// v1.5.4: where a touch above the panel went down (caret tap).
    pub(super) tap_at: Option<(i32, i32)>,
}

/// What the last reading was made from (learning from corrections).
#[derive(Clone, Debug)]
pub(crate) enum ReadInk {
    /// A Letters box: its strokes in box units.
    Letter(Vec<Vec<(f32, f32)>>),
    /// The free area: the reading, with each letter's ink.
    Free(sigma_hwr::free::Reading),
}

/// Letters: idle time after a stroke before the box is read (ms).
pub const LETTER_MS: u64 = 800;
/// Words (v1.5.3): idle time before the line is read (ms).
pub const WORD_MS: u64 = 1200;

/// Idle time after a stroke before the GL16 cleanup (plan §12.4).
pub const INK_CLEANUP_MS: u64 = 600;
