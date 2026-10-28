//! The v0.1 screen (brief §3, §4, §7): title bar, page, status bar,
//! on-screen keyboard. Layout, drawing and hit-testing live together so a tap
//! always lands on what was drawn.

use crate::canvas::{Canvas, Rect};
use crate::fonts::{Face, Fonts, Measure};
use crate::i18n::tr;
use crate::preview::{self, Doc, Type};
use crate::theme::{Scale, Theme};
use sigma_core::keyboard::{Dir, Key};
use sigma_core::linefit::{self, Anchor, Fit, Limits, SizedMeasure};
use sigma_core::settings::{KeyTone, LandKeys, Pad, Side, Typeface, MARGIN_RANGE, SIZE_RANGE};
use sigma_core::wrap::{caret_cell, offset_at, wrap, VisualLine};
use sigma_core::Settings;
use sigma_core::{Buffer, Keyboard, SaveState};

/// What the screen shows; owned by the app.
pub struct Model<'a> {
    /// The note's file name, shown in the title bar.
    pub file_name: &'a str,
    /// The note's text, caret and selection.
    pub buffer: &'a Buffer,
    /// The on-screen keyboard's state.
    pub keyboard: &'a Keyboard,
    /// The on-screen keyboard is shown.
    pub keys_on: bool,
    /// Save state, shown in the status bar.
    pub save: &'a SaveState,
    /// Page colours.
    pub theme: Theme,
    /// Name field being edited in the title bar (rename in place).
    pub rename: Option<&'a Buffer>,
    /// Touch selection chrome (handles and action bar), v0.2.2.
    pub sel_ui: Option<SelUi>,
}

/// How the touch selection chrome is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SelUi {
    /// Show the action bar (hidden while a handle is dragged).
    pub bar: bool,
}

/// A button on the selection action bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarAction {
    /// Cut the selection.
    Cut,
    /// Copy the selection.
    Copy,
    /// Paste the clipboard.
    Paste,
    /// Select the whole note.
    SelectAll,
}

impl BarAction {
    /// The button's English label, which is also its translation key.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::SelectAll => "Select all",
        }
    }
}

/// Where the selection chrome sits on screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelGeom {
    /// Touch targets of the start / end handles (None when off-screen or
    /// when there is no selection).
    pub handles: [Option<Rect>; 2],
    /// The action bar, when it is shown.
    pub bar: Option<Rect>,
    /// The action bar's buttons with their touch targets.
    pub buttons: Vec<(Rect, BarAction)>,
}

/// What a tap at a screen point lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// A key of the on-screen keyboard.
    Key(Key),
    /// The Keys button: show or hide the on-screen keyboard.
    ToggleKeys,
    /// The Write button: open the ink panel (v1.5.0).
    Ink,
    /// Byte offset in the buffer where the caret should go.
    Caret(usize),
    /// A Japanese conversion candidate on the bar (v1.10).
    Candidate(usize),
    /// Selection handle 0 = start, 1 = end.
    Handle(usize),
    /// A button on the selection action bar.
    Bar(BarAction),
    /// Back arrow (v1.0.2): the previous note, mode and position.
    Back,
    /// Folder icon: open the Files sheet.
    Files,
    /// Eye (write mode) or pencil (preview): toggle preview, v0.3.
    TogglePreview,
    /// Tap on the file name: rename in place.
    StartRename,
    /// "Aa" in the title bar: the Options sheet (v0.4).
    Options,
    /// Magnifying glass: the Find bar (v0.6).
    Find,
    /// Connected nodes: the Graph sheet (v0.6).
    Graph,
    /// The drawer button: open or close the drawer menu, where each pane
    /// is shown or hidden (v1.16; one icon per pane before).
    DrawerMenu,
    /// The front-light button: open or close the light menu.
    LightMenu,
    /// The Wi-Fi button: open or close the Wi-Fi menu (v1.18).
    Wifi,
    /// Landscape keyboard bar: Split / Full / Centre, pad toggles (v0.4).
    KbButton(KbButton),
    /// OK of the rename field.
    RenameOk,
    /// Cancel of the rename field.
    RenameCancel,
    /// Nothing that reacts to a tap.
    Nothing,
}

/// Keys and landscape bar buttons with their rectangles.
type Board = (Vec<(Rect, Key)>, Vec<(Rect, KbButton)>);

/// Buttons in the landscape keyboard bar (brief §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KbButton {
    /// A landscape keyboard mode: Split, Full or Centre.
    Mode(LandKeys),
    /// Arrows ⇄ 123 pad (Full only).
    PadToggle,
    /// Pad left ⇄ Pad right (Full only).
    PadSide,
}

/// Where each part of the editor screen sits, in device pixels.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    /// The title bar.
    pub title: Rect,
    /// The page, between the title bar (or the Find bar) and the status bar.
    pub page: Rect,
    /// The status bar, above the keyboard.
    pub status: Rect,
    /// The Keys button, flush right in the status bar.
    pub keys_button: Rect,
    /// "Write" (v1.5.0): opens the ink panel; left of Keys.
    pub ink_button: Rect,
    /// The on-screen keyboard (zero height when hidden).
    pub keyboard: Rect,
    /// Each key with its rectangle (empty when the keyboard is hidden).
    pub keys: Vec<(Rect, Key)>,
    /// Title-bar icons, v1.0 (S9.2): twice the brief's size, in the
    /// mockup's order left to right after the title: Files, Preview, Find,
    /// Scratch, To-dos, Contents, Graph, Settings.
    pub files_button: Rect,
    /// Back arrow (v1.0.2), the slot left of Files; drawn and hit only
    /// while `EditorScreen::back_on`.
    pub back_button: Rect,
    /// Eye / pencil, right of the folder.
    pub preview_button: Rect,
    /// Close X of the sheets (Files, Graph, Help, Settings): the rightmost
    /// slot, where the Settings button sits in write mode.
    pub close_button: Rect,
    /// The file name in the title bar, left of the first icon.
    pub name_rect: Rect,
    /// OK of the rename field.
    pub ok_rect: Rect,
    /// Cancel of the rename field, left of OK.
    pub cancel_rect: Rect,
    /// The Settings (gear) button, the rightmost icon.
    pub options_button: Rect,
    /// The drawer button after Find (v1.16): opens the drawer menu.
    /// Zero-sized when no pane is available.
    pub drawer_button: Rect,
    /// The front-light button after Find: opens the light menu. Zero-sized
    /// when the device has no front light.
    pub light_button: Rect,
    /// The Wi-Fi button between Graph and Options (v1.18). Zero-sized when
    /// the device has no Wi-Fi control.
    pub wifi_button: Rect,
    /// Find (between the eye and the drawer button) and Graph (between the
    /// drawer button and Options).
    pub find_button: Rect,
    /// The Graph icon (zero-sized when it is off).
    pub graph_button: Rect,
    /// The Find bar under the title bar when open (zero height otherwise).
    pub find_bar: Rect,
    /// The text column inside the page: the page itself, or the locked
    /// portrait-width column (Line on, landscape) with gutters beside it.
    pub column: Rect,
    /// The landscape keyboard bar's buttons with their rectangles.
    pub kb_buttons: Vec<(Rect, KbButton)>,
}

/// Icon slots in the title bar (S9.2; v1.0.2 adds Back before Files, so
/// sizing always reserves room for it; v1.16 one drawer button for three;
/// then the front-light button and the Wi-Fi button).
pub const TITLE_ICONS: i32 = 9;

/// The editor screen: layout, drawing and hit-testing for write and
/// preview mode, with the view state they share (scroll, type settings).
pub struct EditorScreen {
    /// Panel width, device pixels.
    pub width: u32,
    /// Panel height, device pixels.
    pub height: u32,
    /// Shared with the plugins the app runs, so the fonts are parsed once.
    pub fonts: std::rc::Rc<Fonts>,
    /// Index of the visual row at the top of the page.
    pub scroll: usize,
    /// A USB keyboard is connected (v0.8).
    pub usb_live: bool,
    /// Typing overwrites (the Insert key): the status bar says `OVR`.
    pub overwrite: bool,
    /// Navigation history is not empty: show the Back arrow (v1.0.2).
    pub back_on: bool,
    /// The Graph icon is shown: the Graph plugin is on (or the built-in
    /// Graph is wanted). Off closes the icon's place up.
    pub graph_on: bool,
    /// The drawer panes on offer: Contents, To-dos and Scratch each come
    /// from a pane plugin (or the built-in pane is wanted). A pane that is
    /// off is not in the drawer menu; with none, the drawer button's place
    /// closes up.
    pub contents_on: bool,
    /// The To-dos pane is on offer.
    pub todos_on: bool,
    /// The Scratch pane is on offer.
    pub scratch_on: bool,
    /// The drawer menu is open under the drawer button (v1.16).
    pub drawer_menu: bool,
    /// The device has a front light: the light button is shown.
    pub light_on: bool,
    /// The light menu is open under the light button.
    pub light_menu: bool,
    /// The brightness step, 0 (off) to `BACKLIGHT_MAX`.
    pub light_level: u8,
    /// The warmth step, 0 (cool) to `WARMTH_MAX`; `None` without a mixer.
    pub warmth_level: Option<u8>,
    /// The Wi-Fi button is shown (v1.18): the app has a Wi-Fi backend.
    /// Off closes the button's place up.
    pub wifi_on: bool,
    /// The Wi-Fi menu is open under the Wi-Fi button (v1.18).
    pub wifi_menu: bool,
    /// What the Wi-Fi button shows: off, on, or connected.
    pub wifi_state: crate::wifi_menu::WifiState,
    /// Settings → Word mode is on (v1.5.3): Letters offers Words.
    pub ink_words: bool,
    /// Sketch: the dotted guide line is shown (v1.13.1).
    pub sketch_guide: bool,
    s: Scale,
    /// Keyboard scale (v1.4.3): page scale × `Settings::keyboard_scale`.
    k: Scale,
    /// The page scale (note text, margins): `s` without the UI scale (v1.1).
    page_s: Scale,
    // Typography defaults from the brief: 20 px, leading 1.5, margin 28 px.
    /// Body text size in brief px (times the page scale for device pixels).
    pub body_px: f32,
    /// Line height as a multiple of the body text size.
    pub leading: f32,
    /// Page margin in brief px.
    pub margin_px: f32,
    /// Page face (Options → Type).
    pub face: Face,
    /// Keyboard colours when they differ from the page (Options → Keyboard tone).
    pub key_theme: Option<Theme>,
    /// The settings in force (landscape keyboard, Line, USB flag).
    pub cfg: Settings,
    /// Open drawer panes (Contents, To-dos, Scratch): ticked in the drawer
    /// menu; the drawer button draws active while any is open.
    pub drawer_open: [bool; 3],
    /// The Find bar is open: it sits under the title bar and the page
    /// starts below it; the magnifier draws active.
    pub find_open: bool,
    /// The candidate bar is up: the text area stops above it (v1.10).
    cand_reserve: bool,
    /// The drawer rectangle while it is open in preview (v1.0.3): the
    /// preview page is laid out in the space beside it.
    pub preview_drawer: Option<Rect>,
}

/// Real-face measurement for Line mode.
struct FaceMeasure<'a> {
    fonts: &'a Fonts,
    face: Face,
}

impl SizedMeasure for FaceMeasure<'_> {
    fn width_at(&self, size: f32, text: &str) -> f32 {
        self.fonts.width(self.face, size, text)
    }
}

impl EditorScreen {
    /// A screen for a `width` × `height` panel with the default type settings.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            fonts: std::rc::Rc::new(Fonts::new()),
            scroll: 0,
            s: Scale(Scale::for_width(width.min(height)).0 * 2.0),
            k: Scale(Scale::for_width(width.min(height)).0 * 1.5),
            page_s: Scale::for_width(width.min(height)),
            body_px: 20.0,
            leading: 1.5,
            margin_px: 28.0,
            face: Face::Literata,
            key_theme: None,
            cfg: Settings::default(),
            usb_live: false,
            overwrite: false,
            back_on: false,
            graph_on: true,
            contents_on: true,
            todos_on: true,
            scratch_on: true,
            drawer_menu: false,
            light_on: false,
            light_menu: false,
            light_level: 0,
            warmth_level: None,
            wifi_on: false,
            wifi_menu: false,
            wifi_state: crate::wifi_menu::WifiState::Off,
            ink_words: false,
            sketch_guide: true,
            drawer_open: [false; 3],
            find_open: false,
            cand_reserve: false,
            preview_drawer: None,
        }
    }

    /// Scrolls so logical line `line` is the top row and puts the caret at
    /// its start (Contents jump, to-do open).
    pub fn jump_to_line(&mut self, buffer: &mut Buffer, page: Rect, line: usize) {
        let at = sigma_core::outline::line_range(buffer.text(), line).map_or(0, |r| r.0);
        buffer.set_caret(at);
        let rows = self.rows(buffer, page);
        self.scroll = rows.iter().position(|r| r.start == at).unwrap_or(0);
    }

    /// New panel size (orientation change). The scale follows the short
    /// side, so type and keys keep their size when the device turns.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.page_s = Scale::for_width(width.min(height));
        self.s = Scale(self.page_s.0 * f32::from(self.cfg.ui_scale) / 100.0);
        self.k = Scale(self.page_s.0 * f32::from(self.cfg.keyboard_scale()) / 100.0);
    }

    /// Whether the panel is wider than it is tall.
    #[must_use]
    pub fn landscape(&self) -> bool {
        self.width > self.height
    }

    /// Applies settings. With Line on, fits size and margin to the
    /// portrait-width column with the real face and writes the result back
    /// into `st` (brief §4: Size moves Margin, Margin moves Size).
    pub fn configure(&mut self, st: &mut Settings, anchor: Option<Anchor>) {
        self.face = match st.face {
            Typeface::Literata => Face::Literata,
            Typeface::Atkinson => Face::Atkinson,
        };
        self.key_theme = match st.key_tone {
            KeyTone::Theme => None,
            KeyTone::Light => Some(Theme::DAY),
            KeyTone::Dark => Some(Theme::NIGHT),
        };
        self.leading = st.leading;
        self.s = Scale(self.page_s.0 * f32::from(st.ui_scale) / 100.0);
        self.k = Scale(self.page_s.0 * f32::from(st.keyboard_scale()) / 100.0);
        if st.line_on {
            let k = self.page_s.0;
            let column = self.width.min(self.height) as f32;
            let f = linefit::fit(
                &FaceMeasure {
                    fonts: &self.fonts,
                    face: self.face,
                },
                column,
                st.line_words,
                Fit {
                    size: st.size * k,
                    margin: st.margin * k,
                },
                anchor.unwrap_or(Anchor::Size),
                Limits {
                    size: (SIZE_RANGE.0 * k, SIZE_RANGE.1 * k),
                    margin: (MARGIN_RANGE.0 * k, MARGIN_RANGE.1 * k),
                },
            );
            self.body_px = f.size / k;
            self.margin_px = f.margin / k;
            st.size = (self.body_px * 10.0).round() / 10.0;
            st.margin = (self.margin_px * 10.0).round() / 10.0;
        } else {
            self.body_px = st.size;
            self.margin_px = st.margin;
        }
        self.cfg = *st;
    }

    /// Words on the first row of the Line sample at the current settings
    /// (Options shows it; tests check it).
    #[must_use]
    pub fn sample_words_per_row(&self, page: Rect) -> usize {
        let text = format!("{} {}", linefit::SAMPLE, linefit::SAMPLE);
        let rows = wrap(&text, self.text_area(page).w as u32, &self.measure());
        text[rows[0].start..rows[0].end].split_whitespace().count()
    }

    pub(crate) fn px(&self, v: f32) -> i32 {
        self.s.px(v)
    }

    /// Keyboard units (v1.4.3): keys, gaps and the landscape bar.
    fn kpx(&self, v: f32) -> i32 {
        self.k.px(v)
    }

    /// The keyboard scale (v1.4.3).
    #[must_use]
    pub fn key_scale(&self) -> Scale {
        self.k
    }

    fn body(&self) -> f32 {
        self.body_px * self.page_s.0
    }

    fn line_h(&self) -> i32 {
        (self.body() * self.leading).round() as i32
    }

    /// How far a row is pushed right: a right-to-left row is flush right
    /// (v1.8), every other row starts at the left edge.
    fn row_shift(&self, text: &str, row: VisualLine, area_w: i32) -> i32 {
        let t = &text[row.start..row.end];
        let m = self.measure();
        if sigma_core::TextMeasure::rtl(&m, t) {
            (area_w - sigma_core::TextMeasure::width(&m, t.trim_end_matches(' ')) as i32).max(0)
        } else {
            0
        }
    }

    fn measure(&self) -> Measure<'_> {
        Measure {
            fonts: &self.fonts,
            face: self.face,
            size: self.body(),
        }
    }

    /// Lays out the screen, with or without the on-screen keyboard.
    #[must_use]
    pub fn layout(&self, keys_on: bool, keyboard: &Keyboard) -> Layout {
        let (w, h) = (self.width as i32, self.height as i32);
        let (btn, icon_gap) = self.title_icons(w);
        let title = Rect::new(0, 0, w, btn + 2 * self.px(6.0));
        let gap = self.kpx(6.0);
        let pad = self.kpx(10.0);
        let rows = keyboard.rows();
        // v1.1: the keyboard takes at most 42 % (portrait) / 45 % (landscape)
        // of the height, so the page keeps room at 200 %.
        // The Kobo style board has no landscape bar (v1.13): nothing on it
        // but keys.
        let kobo = keyboard.style == sigma_core::keyboard::Style::Kobo;
        let key_h = {
            let n = rows.len().max(1) as i32;
            let bar = if self.landscape() && !kobo {
                self.kpx(44.0) + gap
            } else {
                0
            };
            let cap = h * if self.landscape() { 45 } else { 42 } / 100;
            let fit = (cap - pad * 2 - bar - gap * (n - 1)) / n;
            self.kpx(58.0).min(fit).max(self.page_s.px(44.0))
        };
        // Landscape adds the Split · Full · Centre bar above the keys.
        let bar_h = if self.landscape() && !kobo {
            self.kpx(44.0) + gap
        } else {
            0
        };
        let kb_h = if keys_on {
            pad * 2 + bar_h + key_h * rows.len() as i32 + gap * (rows.len() as i32 - 1)
        } else {
            0
        };
        let keyboard_rect = Rect::new(0, h - kb_h, w, kb_h);
        let status_h = self.px(44.0) + 2 * self.px(4.0);
        let status = Rect::new(0, keyboard_rect.y - status_h, w, status_h);
        let fb_h = if self.find_open { self.find_bar_h() } else { 0 };
        let find_bar = Rect::new(0, title.h, w, fb_h);
        let page = Rect::new(0, title.h + fb_h, w, status.y - title.h - fb_h);
        let kb_w = self.px(64.0);
        let keys_button = Rect::new(
            w - self.px(12.0) - kb_w,
            status.y + (status_h - self.px(44.0)) / 2,
            kb_w,
            self.px(44.0),
        );
        let ink_w = self.px(72.0);
        let ink_button = Rect::new(
            keys_button.x - self.px(8.0) - ink_w,
            keys_button.y,
            ink_w,
            keys_button.h,
        );
        let (keys, kb_buttons) = if keys_on {
            self.board(keyboard, keyboard_rect, key_h, gap, pad, bar_h)
        } else {
            (Vec::new(), Vec::new())
        };
        let by = (title.h - btn) / 2;
        // Right-aligned group, mockup order left to right.
        let optional = [self.light_on, self.panes_on(), self.graph_on, self.wifi_on];
        let shown = TITLE_ICONS - optional.iter().filter(|on| !**on).count() as i32;
        let x0 = w - self.px(12.0) - shown * btn - (shown - 1) * icon_gap;
        let slot = |i: i32| Rect::new(x0 + i * (btn + icon_gap), by, btn, btn);
        let back_button = slot(0);
        let files_button = slot(1);
        let preview_button = slot(2);
        let find_button = slot(3);
        // The icons whose plugin is off take no place, and Settings stays
        // flush right.
        let mut next = 4;
        let mut place = |on: bool| {
            if on {
                next += 1;
                slot(next - 1)
            } else {
                Rect::default()
            }
        };
        let light_button = place(self.light_on);
        let drawer_button = place(self.panes_on());
        let graph_button = place(self.graph_on);
        let wifi_button = place(self.wifi_on);
        let options_button = slot(next);
        // Help opens from Settings → Help & about; the far-right slot is the
        // Settings button, and Close takes its place on the other screens.
        let close_button = options_button;
        let first = if self.back_on {
            back_button
        } else {
            files_button
        };
        let name_rect = Rect::new(0, 0, (first.x - self.px(8.0)).max(0), title.h);
        let column = if self.landscape() && self.cfg.line_on {
            let cw = w.min(h);
            Rect::new(page.x + (page.w - cw) / 2, page.y, cw, page.h)
        } else {
            page
        };
        let btn = self.px(44.0).max(btn / 2);
        let okw = self.px(72.0);
        let cw = self.px(96.0);
        let ok_rect = Rect::new(w - self.px(12.0) - okw, by, okw, btn);
        let cancel_rect = Rect::new(ok_rect.x - self.px(8.0) - cw, by, cw, btn);
        Layout {
            title,
            page,
            status,
            keys_button,
            ink_button,
            keyboard: keyboard_rect,
            keys,
            files_button,
            back_button,
            preview_button,
            close_button,
            name_rect,
            ok_rect,
            cancel_rect,
            options_button,
            drawer_button,
            light_button,
            wifi_button,
            find_button,
            graph_button,
            find_bar,
            column,
            kb_buttons,
        }
    }

    /// Title-bar icon size and spacing (S9.2): 88 brief px (twice the
    /// brief's 44) with 8 px gaps. Overflow rule if the bar is too narrow
    /// to keep a title of at least 200 brief px: first the gaps shrink to
    /// 2 px, then the icons shrink, never below 44 brief px (the minimum
    /// target). The Forma (1440 portrait, 1920 landscape) needs neither.
    #[must_use]
    pub fn title_icons(&self, w: i32) -> (i32, i32) {
        let n = TITLE_ICONS;
        let room = w - self.px(12.0) - self.px(100.0);
        // v1.1: 44 units at the UI scale (200 % = the S9.2 size).
        let (mut btn, mut gap) = (self.px(44.0), self.px(4.0));
        if n * btn + (n - 1) * gap > room {
            gap = self.px(1.0);
        }
        if n * btn + (n - 1) * gap > room {
            btn = ((room - (n - 1) * gap) / n).max(self.page_s.px(44.0));
        }
        (btn, gap)
    }

    /// Scale for title-bar glyphs: twice the page scale (S9.2), or less
    /// when the overflow rule shrank the icons.
    #[must_use]
    pub fn icon_scale(&self, l: &Layout) -> Scale {
        Scale(self.s.0 * l.files_button.w as f32 / self.px(44.0) as f32)
    }

    /// Keys and landscape bar buttons inside the keyboard rectangle.
    ///
    /// Grid units (10 Compact, 16 Full); rows are centred, columns align
    /// because every key starts at its cumulative unit offset. The unit comes
    /// from the short side, so keys keep their portrait size in landscape.
    /// Landscape: **Centre** centres the board; **Split** cuts it in two with
    /// a gap (a key across the middle goes to the side of its centre; Space
    /// is cut in two so both halves have one); **Full** adds a number or
    /// direction pad on the left or right.
    #[allow(clippy::too_many_lines)]
    fn board(
        &self,
        keyboard: &Keyboard,
        kb: Rect,
        key_h: i32,
        gap: i32,
        pad: i32,
        bar_h: i32,
    ) -> Board {
        let w = self.width as i32;
        let short = w.min(self.height as i32);
        let land = self.landscape();
        let rows = keyboard.rows();
        let grid = keyboard.grid_units();
        let kobo = keyboard.style == sigma_core::keyboard::Style::Kobo;
        let mode = if land && !kobo {
            self.cfg.land_keys
        } else {
            LandKeys::Centre
        };
        // Portrait key size; Full shrinks it slightly if board + pad would
        // not fit the long side.
        let mut unit = (short - 2 * pad + gap) as f32 / grid;
        // The number pad has a fourth column for + - * /; arrows have three.
        let pad_cols = if self.cfg.pad == Pad::Numbers {
            4.0
        } else {
            3.0
        };
        if mode == LandKeys::Full {
            unit = unit.min((w - 2 * pad + gap) as f32 / (grid + pad_cols + 0.5));
        }
        let board_w = grid * unit;
        let split_gap = if mode == LandKeys::Split {
            (3.0 * unit).min((w - 2 * pad) as f32 - board_w).max(0.0)
        } else {
            0.0
        };
        let (pad_w, pad_gap) = if mode == LandKeys::Full {
            (pad_cols * unit, unit / 2.0)
        } else {
            (0.0, 0.0)
        };
        let total = board_w + split_gap + pad_w + pad_gap;
        let x_start = if land {
            ((w as f32 - total) / 2.0).max(0.0)
        } else {
            pad as f32
        };
        let pad_left = mode == LandKeys::Full && self.cfg.pad_side == Side::Left;
        let board_x = x_start + if pad_left { pad_w + pad_gap } else { 0.0 };
        let top = kb.y + pad + bar_h;
        let mid = grid / 2.0;
        let mut keys = Vec::new();
        let mut push = |x: f32, kw: f32, y: i32, h: i32, k: Key| {
            let x0 = x.round() as i32;
            let x1 = (x + kw).round() as i32 - gap;
            keys.push((Rect::new(x0, y, x1 - x0, h), k));
        };
        for (r, row) in rows.iter().enumerate() {
            let row = if split_gap > 0.0 {
                space_across(keyboard, r, row, grid)
            } else {
                row.clone()
            };
            let total_u: f32 = row
                .iter()
                .enumerate()
                .map(|(c, &k)| keyboard.units(k, r, c))
                .sum();
            let mut u = (grid - total_u) / 2.0;
            let y = top + r as i32 * (key_h + gap);
            for (c, &k) in row.iter().enumerate() {
                let ku = keyboard.units(k, r, c);
                let across = u + 1e-3 < mid && u + ku > mid + 1e-3;
                if split_gap > 0.0 && across && k == Key::Space {
                    push(board_x + u * unit, (mid - u) * unit, y, key_h, k);
                    push(
                        board_x + mid * unit + split_gap,
                        (u + ku - mid) * unit,
                        y,
                        key_h,
                        k,
                    );
                } else {
                    let right = if across {
                        u + ku / 2.0 >= mid
                    } else {
                        u + 1e-3 >= mid
                    };
                    let off = if right { split_gap } else { 0.0 };
                    push(board_x + u * unit + off, ku * unit, y, key_h, k);
                }
                u += ku;
            }
        }
        if mode == LandKeys::Full {
            let px0 = if pad_left {
                x_start
            } else {
                board_x + board_w + pad_gap
            };
            let n = |c: char| Some(Key::Char(c));
            let cells: Vec<Vec<Option<Key>>> = match self.cfg.pad {
                Pad::Numbers => vec![
                    vec![n('7'), n('8'), n('9'), n('+')],
                    vec![n('4'), n('5'), n('6'), n('-')],
                    vec![n('1'), n('2'), n('3'), n('*')],
                    vec![n('0'), n('.'), Some(Key::Return), n('/')],
                ],
                Pad::Arrows => vec![
                    vec![None, Some(Key::Arrow(Dir::Up)), None],
                    vec![
                        Some(Key::Arrow(Dir::Left)),
                        None,
                        Some(Key::Arrow(Dir::Right)),
                    ],
                    vec![None, Some(Key::Arrow(Dir::Down)), None],
                ],
            };
            let nr = rows.len() as i32;
            let span = nr * key_h + (nr - 1) * gap;
            let m = cells.len() as i32;
            let ph = (span - (m - 1) * gap) / m;
            for (i, line) in cells.iter().enumerate() {
                for (c, k) in line.iter().enumerate() {
                    if let Some(k) = k {
                        push(
                            px0 + c as f32 * unit,
                            unit,
                            top + i as i32 * (ph + gap),
                            ph,
                            *k,
                        );
                    }
                }
            }
        }
        let mut buttons = Vec::new();
        if land && !kobo {
            let bh = self.kpx(44.0);
            let by = kb.y + pad;
            let bw = self.kpx(110.0);
            let bg = self.kpx(8.0);
            let x = (w - (3 * bw + 2 * bg)) / 2;
            for (i, m) in [LandKeys::Split, LandKeys::Full, LandKeys::Centre]
                .into_iter()
                .enumerate()
            {
                buttons.push((
                    Rect::new(x + i as i32 * (bw + bg), by, bw, bh),
                    KbButton::Mode(m),
                ));
            }
            if mode == LandKeys::Full {
                let side = Rect::new(w - pad - bw, by, bw, bh);
                buttons.push((Rect::new(side.x - bg - bw, by, bw, bh), KbButton::PadToggle));
                buttons.push((side, KbButton::PadSide));
            }
        }
        (keys, buttons)
    }

    /// Label of a landscape bar button (the action it takes).
    #[must_use]
    pub fn kb_button_label(&self, b: KbButton) -> &'static str {
        match b {
            KbButton::Mode(LandKeys::Split) => "Split",
            KbButton::Mode(LandKeys::Full) => "Full",
            KbButton::Mode(LandKeys::Centre) => "Centre",
            KbButton::PadToggle => match self.cfg.pad {
                Pad::Numbers => "Arrows",
                Pad::Arrows => "123",
            },
            KbButton::PadSide => match self.cfg.pad_side {
                Side::Left => "Pad right",
                Side::Right => "Pad left",
            },
        }
    }

    /// Scroll position bar (v1.12: the Kobo's own look): a hairline track
    /// in the page's right margin with a hollow rounded thumb sized and
    /// placed by the visible share of the note (`crate::scrollbar`). Pure
    /// 0x00 / 0xFF only, so the fast DU waveform renders it cleanly; it
    /// only changes (and is only sent to the panel, by the frame diff) when
    /// the note scrolls. Not shown when the whole note fits. Display only.
    #[must_use]
    pub fn scroll_bar(&self, page: Rect, total: usize, visible: usize) -> Option<(Rect, Rect)> {
        if total <= visible || page.h <= 0 {
            return None;
        }
        crate::scrollbar::bar(self.page_s, page, total, visible, self.scroll)
    }

    /// Where a finger grabs the scroll bar (v1.13.1): the strip widened to a
    /// touch target, out to the page edge, and the thumb on it. `None` when
    /// the whole note fits.
    #[must_use]
    pub fn scroll_bar_zone(&self, buffer: &Buffer, page: Rect) -> Option<(Rect, Rect)> {
        let rows = self.rows(buffer, page).len();
        let (strip, thumb) = self.scroll_bar(page, rows, self.visible_rows(page))?;
        Some((crate::scrollbar::zone(self.page_s, page, strip), thumb))
    }

    /// Drag: the thumb's centre follows the finger at `y`.
    pub fn scroll_bar_to(&mut self, buffer: &mut Buffer, page: Rect, y: i32) {
        let rows = self.rows(buffer, page).len();
        let visible = self.visible_rows(page);
        let Some((strip, thumb)) = self.scroll_bar(page, rows, visible) else {
            return;
        };
        let span = (strip.h - thumb.h).max(1) as f32;
        let f = ((y - thumb.h / 2 - strip.y) as f32 / span).clamp(0.0, 1.0);
        let last = rows.saturating_sub(visible);
        self.scroll = (f * last as f32).round() as usize;
        self.caret_into_view(buffer, page);
    }

    /// Tap on the bar: a screen up if the finger is above the thumb's
    /// centre, a screen down if below. The caret stays where it is.
    pub fn scroll_bar_step(&mut self, buffer: &mut Buffer, page: Rect, y: i32) {
        let rows = self.rows(buffer, page).len();
        let visible = self.visible_rows(page);
        let Some((_, thumb)) = self.scroll_bar(page, rows, visible) else {
            return;
        };
        let n = visible.saturating_sub(1).max(1);
        let last = rows.saturating_sub(visible);
        self.scroll = if y < thumb.y + thumb.h / 2 {
            self.scroll.saturating_sub(n)
        } else {
            (self.scroll + n).min(last)
        };
        self.caret_into_view(buffer, page);
    }

    /// After the bar moved the view: a caret that fell out of it goes to the
    /// nearest visible row (the page redraw would scroll back to it).
    fn caret_into_view(&self, buffer: &mut Buffer, page: Rect) {
        let rows = self.rows(buffer, page);
        let m = self.measure();
        let (row, col) = caret_cell(buffer.text(), &rows, buffer.caret(), &m);
        let last = (self.scroll + self.visible_rows(page)).saturating_sub(1);
        let target = row.clamp(self.scroll, last.min(rows.len().saturating_sub(1)));
        if target != row {
            if let Some(&r) = rows.get(target) {
                buffer.move_to(offset_at(buffer.text(), r, col, &m), false);
            }
        }
    }

    /// What the scroll bar sits on: the desk when Line is on in landscape.
    fn bar_bg(&self, t: Theme) -> u8 {
        if self.landscape() && self.cfg.line_on {
            t.gutter
        } else {
            t.paper
        }
    }

    fn draw_scroll_bar(&self, c: &mut Canvas, page: Rect, total: usize, visible: usize, t: Theme) {
        if let Some(b) = self.scroll_bar(page, total, visible) {
            // On the desk beside a locked column, else on the paper.
            crate::scrollbar::draw_bar(c, self.page_s, b, t, self.bar_bg(t));
        }
    }

    fn text_area(&self, page: Rect) -> Rect {
        let page = if self.landscape() && self.cfg.line_on {
            let cw = (self.width.min(self.height) as i32).min(page.w);
            Rect::new(page.x + (page.w - cw) / 2, page.y, cw, page.h)
        } else {
            page
        };
        let m = self.page_s.px(self.margin_px);
        let v = self.page_s.px(16.0);
        let bar = if self.cand_reserve {
            self.line_h() + self.px(20.0)
        } else {
            0
        };
        Rect::new(page.x + m, page.y + v, page.w - 2 * m, page.h - v * 2 - bar)
    }

    fn visible_rows(&self, page: Rect) -> usize {
        (self.text_area(page).h / self.line_h()).max(1) as usize
    }

    /// The note wrapped into visual rows for `page`.
    #[must_use]
    pub fn rows(&self, buffer: &Buffer, page: Rect) -> Vec<VisualLine> {
        wrap(
            buffer.text(),
            self.text_area(page).w as u32,
            &self.measure(),
        )
    }

    /// Keeps the caret row on screen.
    pub fn follow_caret(&mut self, buffer: &Buffer, page: Rect) {
        let rows = self.rows(buffer, page);
        let (row, _) = caret_cell(buffer.text(), &rows, buffer.caret(), &self.measure());
        let n = self.visible_rows(page);
        if row < self.scroll {
            self.scroll = row;
        } else if row >= self.scroll + n {
            self.scroll = row + 1 - n;
        }
        self.scroll = self.scroll.min(rows.len().saturating_sub(1));
    }

    /// Page-turn buttons: scroll one screen and put the caret at the top row.
    pub fn page_turn(&mut self, buffer: &mut Buffer, page: Rect, forward: bool) {
        self.page_turn_ext(buffer, page, forward, false);
    }

    /// As [`Self::page_turn`]; with `extend` the selection grows to the new
    /// caret position.
    pub fn page_turn_ext(&mut self, buffer: &mut Buffer, page: Rect, forward: bool, extend: bool) {
        let rows = self.rows(buffer, page);
        let n = self.visible_rows(page).saturating_sub(1).max(1);
        self.scroll = if forward {
            (self.scroll + n).min(rows.len().saturating_sub(1))
        } else {
            self.scroll.saturating_sub(n)
        };
        buffer.move_to(rows[self.scroll].start, extend);
    }

    /// Vertical caret move by one visual row (keyboard arrows later).
    pub fn caret_row(&mut self, buffer: &mut Buffer, page: Rect, down: bool) {
        self.caret_row_ext(buffer, page, down, false);
    }

    /// As [`Self::caret_row`]; with `extend` the selection grows (Shift+↑/↓).
    pub fn caret_row_ext(&mut self, buffer: &mut Buffer, page: Rect, down: bool, extend: bool) {
        let rows = self.rows(buffer, page);
        let m = self.measure();
        let (row, col) = caret_cell(buffer.text(), &rows, buffer.caret(), &m);
        let target = if down { row + 1 } else { row.wrapping_sub(1) };
        if let Some(&r) = rows.get(target) {
            buffer.move_to(offset_at(buffer.text(), r, col, &m), extend);
        } else {
            buffer.move_to(if down { buffer.text().len() } else { 0 }, extend);
        }
    }

    /// The strip of Japanese conversion candidates, along the bottom of the
    /// page (v1.10).
    fn cand_bar(&self, page: Rect) -> Rect {
        let h = self.line_h() + self.px(20.0);
        Rect::new(page.x, page.y + page.h - h, page.w, h)
    }

    /// The candidate chips that fit the bar, the chosen one in view.
    fn cand_chips(&self, bar: Rect, cands: &[String], sel: usize) -> Vec<(Rect, usize)> {
        let size = self.body() * 0.95;
        let pad = self.px(14.0);
        let gap = self.px(8.0);
        let w: Vec<i32> = cands
            .iter()
            .map(|c| self.fonts.width(Face::Atkinson, size, c).ceil() as i32 + 2 * pad)
            .collect();
        let fits = |from: usize| {
            let mut x = bar.x + gap;
            let mut out = Vec::new();
            for (i, &wi) in w.iter().enumerate().skip(from) {
                if x + wi > bar.x + bar.w - gap && !out.is_empty() {
                    break;
                }
                out.push((
                    Rect::new(x, bar.y + gap, wi.min(bar.w - 2 * gap), bar.h - 2 * gap),
                    i,
                ));
                x += wi + gap;
            }
            out
        };
        let mut from = 0;
        while from < sel && !fits(from).iter().any(|&(_, i)| i == sel) {
            from += 1;
        }
        fits(from)
    }

    fn draw_cands(&self, c: &mut Canvas, page: Rect, cands: &[String], sel: usize, t: Theme) {
        let bar = self.cand_bar(page);
        c.fill(bar, t.paper);
        c.fill(Rect::new(bar.x, bar.y, bar.w, 1), t.rule);
        let size = self.body() * 0.95;
        for (r, i) in self.cand_chips(bar, cands, sel) {
            let (fg, bg) = if i == sel {
                (t.paper, t.ink)
            } else {
                (t.ink, t.paper)
            };
            c.fill(r, bg);
            c.stroke(r, 2, t.ink);
            let tw = self.fonts.width(Face::Atkinson, size, &cands[i]);
            let base =
                r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(Face::Atkinson, size) / 2.6;
            self.fonts.draw(
                c,
                Face::Atkinson,
                size,
                r.x as f32 + (r.w as f32 - tw) / 2.0,
                base,
                &cands[i],
                fg,
            );
        }
    }

    /// Is the caret in a right-to-left row (so Left / Right swap)?
    #[must_use]
    pub fn caret_rtl(&self, buffer: &Buffer, page: Rect) -> bool {
        let rows = self.rows(buffer, page);
        let m = self.measure();
        let (row, _) = caret_cell(buffer.text(), &rows, buffer.caret(), &m);
        let r = rows[row];
        sigma_core::TextMeasure::rtl(&m, &buffer.text()[r.start..r.end])
    }

    /// Screen point (x, top of row) of a text offset, if it is on screen.
    #[must_use]
    pub fn point_of(&self, buffer: &Buffer, page: Rect, at: usize) -> Option<(i32, i32)> {
        let area = self.text_area(page);
        let rows = self.rows(buffer, page);
        let (row, col) = caret_cell(buffer.text(), &rows, at, &self.measure());
        let shift = self.row_shift(buffer.text(), rows[row], area.w);
        (row >= self.scroll && row < self.scroll + self.visible_rows(page)).then(|| {
            (
                area.x + shift + col as i32,
                area.y + (row - self.scroll) as i32 * self.line_h(),
            )
        })
    }

    /// Text offset under a screen point on the page (char-snapped).
    #[must_use]
    pub fn offset_at_point(&self, buffer: &Buffer, page: Rect, x: i32, y: i32) -> usize {
        let area = self.text_area(page);
        let rows = self.rows(buffer, page);
        let r = self.scroll + ((y - area.y).max(0) / self.line_h()) as usize;
        let Some(&row) = rows.get(r).or(rows.last()) else {
            return 0;
        };
        let shift = self.row_shift(buffer.text(), row, area.w);
        offset_at(
            buffer.text(),
            row,
            (x - area.x - shift).max(0) as u32,
            &self.measure(),
        )
    }

    /// Height of one text row, device pixels.
    #[must_use]
    pub fn line_px(&self) -> i32 {
        self.line_h()
    }

    /// Vertical distance from a text row's top to its handle's centre.
    #[must_use]
    pub fn handle_drop(&self) -> i32 {
        self.line_h() + self.px(16.0)
    }

    /// Handles under both selection ends; the action bar above the
    /// selection (below it near the top of the page).
    #[must_use]
    pub fn selection_geometry(&self, model: &Model<'_>, l: &Layout) -> SelGeom {
        let Some(ui) = model.sel_ui else {
            return SelGeom::default();
        };
        let b = model.buffer;
        // Handles sit on the text, so they follow the page scale (v1.1).
        let target = self.page_s.px(48.0);
        let mut g = SelGeom::default();
        let (a, e) = b.selection().unwrap_or((b.caret(), b.caret()));
        let pa = self.point_of(b, l.page, a);
        let pe = self.point_of(b, l.page, e);
        if b.selection().is_some() {
            for (i, p) in [pa, pe].into_iter().enumerate() {
                g.handles[i] = p.map(|(x, y)| {
                    let cy = y + self.handle_drop();
                    Rect::new(x - target / 2, cy - target / 2, target, target)
                });
            }
        }
        if !ui.bar {
            return g;
        }
        let actions: &[BarAction] = if b.selection().is_some() {
            &[
                BarAction::Cut,
                BarAction::Copy,
                BarAction::Paste,
                BarAction::SelectAll,
            ]
        } else {
            &[BarAction::Paste, BarAction::SelectAll]
        };
        let ui_px = self.ui_px();
        let pad = self.px(14.0);
        let bh = self.px(52.0);
        let widths: Vec<i32> = actions
            .iter()
            .map(|a| {
                (self.fonts.width(Face::AtkinsonBold, ui_px, &tr(a.label())) as i32 + 2 * pad)
                    .max(self.px(72.0))
            })
            .collect();
        let bw: i32 = widths.iter().sum();
        let Some((ax, ay)) = pa.or(pe) else { return g };
        let gap = self.px(10.0);
        let mut by = ay - gap - bh;
        if by < l.page.y + self.px(4.0) {
            // No room above: go below the end handle.
            let ey = pe.map_or(ay, |p| p.1);
            by = ey + self.handle_drop() + target / 2 + gap;
        }
        let bx = (ax - bw / 2).clamp(self.px(8.0), self.width as i32 - bw - self.px(8.0));
        let bar = Rect::new(bx, by, bw, bh);
        let mut x = bx;
        for (w, a) in widths.iter().zip(actions) {
            g.buttons.push((Rect::new(x, by, *w, bh), *a));
            x += w;
        }
        g.bar = Some(bar);
        g
    }

    /// What a tap at (`x`, `y`) lands on, tested against the same layout the
    /// screen draws.
    #[must_use]
    pub fn hit(&self, model: &Model<'_>, x: i32, y: i32) -> Hit {
        let l = self.layout(model.keys_on, model.keyboard);
        if model.sel_ui.is_some() && model.rename.is_none() {
            let g = self.selection_geometry(model, &l);
            if let Some((_, a)) = g.buttons.iter().find(|(r, _)| r.contains(x, y)) {
                return Hit::Bar(*a);
            }
            for (i, h) in g.handles.iter().enumerate() {
                if h.is_some_and(|r| r.inset(-self.px(6.0)).contains(x, y)) {
                    return Hit::Handle(i);
                }
            }
        }
        if let Some((cands, sel)) = model.keyboard.composer.candidates() {
            let bar = self.cand_bar(l.page);
            if bar.contains(x, y) {
                return self
                    .cand_chips(bar, cands, sel)
                    .into_iter()
                    .find(|(r, _)| r.contains(x, y))
                    .map_or(Hit::Nothing, |(_, i)| Hit::Candidate(i));
            }
        }
        if model.rename.is_some() {
            if l.ok_rect.contains(x, y) {
                return Hit::RenameOk;
            }
            if l.cancel_rect.contains(x, y) {
                return Hit::RenameCancel;
            }
            if let Some(&(_, k)) = l
                .keys
                .iter()
                .find(|(r, _)| r.inset(-self.px(3.0) / 2).contains(x, y))
            {
                return Hit::Key(k);
            }
            // Brief: blur confirms.
            return if l.title.contains(x, y) {
                Hit::Nothing
            } else {
                Hit::RenameOk
            };
        }
        if self.back_on && l.back_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Back;
        }
        if l.files_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Files;
        }
        if l.preview_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::TogglePreview;
        }
        if l.options_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Options;
        }
        if l.find_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Find;
        }
        if self.graph_on && l.graph_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Graph;
        }
        if self.panes_on() && l.drawer_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::DrawerMenu;
        }
        if self.wifi_on && l.wifi_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::Wifi;
        }
        if self.light_on && l.light_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::LightMenu;
        }
        if let Some((_, b)) = l.kb_buttons.iter().find(|(r, _)| r.contains(x, y)) {
            return Hit::KbButton(*b);
        }
        if l.name_rect.contains(x, y) {
            return Hit::StartRename;
        }
        if l.keys_button.inset(-self.px(4.0)).contains(x, y) {
            return Hit::ToggleKeys;
        }
        if l.ink_button.inset(-self.px(2.0)).contains(x, y) {
            return Hit::Ink;
        }
        if let Some(&(_, k)) = l
            .keys
            .iter()
            .find(|(r, _)| r.inset(-self.px(3.0) / 2).contains(x, y))
        {
            return Hit::Key(k);
        }
        if l.page.contains(x, y) {
            let area = self.text_area(l.page);
            let rows = self.rows(model.buffer, l.page);
            let r = self.scroll + ((y - area.y).max(0) / self.line_h()) as usize;
            let Some(&row) = rows.get(r).or(rows.last()) else {
                return Hit::Nothing;
            };
            let col = (x - area.x - self.row_shift(model.buffer.text(), row, area.w)).max(0) as u32;
            return Hit::Caret(offset_at(model.buffer.text(), row, col, &self.measure()));
        }
        Hit::Nothing
    }

    /// Draws the whole screen; returns the layout used.
    pub fn draw(&mut self, c: &mut Canvas, model: &Model<'_>) -> Layout {
        let t = model.theme;
        let l = self.layout(model.keys_on, model.keyboard);
        self.cand_reserve = model.keyboard.composer.candidates().is_some();
        self.follow_caret(model.buffer, l.page);
        c.fill(c.bounds(), t.paper);
        draw_gutters(c, &l, t);
        let ui = 17.0 * self.s.0;

        self.draw_title(c, &l, model.file_name, model.rename, t, true);
        if model.rename.is_none() {
            crate::icons::draw_icon(c, crate::icons::Icon::Eye, l.preview_button, t.ink);
            self.draw_options_icon(c, l.options_button, t);
            if self.find_open {
                c.fill(l.find_button, t.wash);
                c.stroke(l.find_button, 2, t.ink);
            }
            crate::icons::draw_icon(c, crate::icons::Icon::Search, l.find_button, t.ink);
            if self.graph_on {
                crate::icons::draw_icon(c, crate::icons::Icon::Graph, l.graph_button, t.ink);
            }
            self.draw_drawer_button(c, &l, t);
            self.draw_light_button(c, &l, t);
            self.draw_wifi_button(c, &l, t);
        }

        // Page.
        let area = self.text_area(l.page);
        let rows = self.rows(model.buffer, l.page);
        let text = model.buffer.text();
        let lh = self.line_h();
        let asc = self.fonts.ascent(self.face, self.body());
        let top_pad = ((lh as f32 - self.body() * 1.2) / 2.0).max(0.0);
        let sel = model.buffer.selection();
        for (i, r) in rows
            .iter()
            .skip(self.scroll)
            .take(self.visible_rows(l.page))
            .enumerate()
        {
            if let Some((a, b)) = sel {
                let (s0, s1) = (a.max(r.start), b.min(r.end.max(r.start)));
                let through_newline = b > r.end && a <= r.end;
                if s0 < s1 || through_newline {
                    let m = self.measure();
                    let row_text = &text[r.start..r.end];
                    let xa = sigma_core::TextMeasure::caret_x(
                        &m,
                        row_text,
                        s0.min(r.end).max(r.start) - r.start,
                    ) as f32;
                    let xb = sigma_core::TextMeasure::caret_x(
                        &m,
                        row_text,
                        s1.max(s0).min(r.end) - r.start,
                    ) as f32;
                    let nl = if through_newline {
                        self.body() * 0.4
                    } else {
                        0.0
                    };
                    let rtl = sigma_core::TextMeasure::rtl(&m, row_text);
                    let (mut x0, mut x1) = (xa.min(xb), xa.max(xb));
                    if rtl {
                        x0 -= nl;
                    } else {
                        x1 += nl;
                    }
                    let shift = self.row_shift(text, *r, area.w) as f32;
                    c.fill(
                        Rect::new(
                            area.x + (shift + x0) as i32,
                            area.y + i as i32 * lh,
                            (x1 - x0).ceil() as i32,
                            lh,
                        ),
                        t.wash,
                    );
                }
            }
            let y = area.y as f32 + (i as i32 * lh) as f32 + top_pad + asc;
            self.fonts.draw(
                c,
                self.face,
                self.body(),
                (area.x + self.row_shift(text, *r, area.w)) as f32,
                y,
                &text[r.start..r.end],
                t.ink,
            );
        }
        let (crow, ccol) = caret_cell(text, &rows, model.buffer.caret(), &self.measure());
        if crow >= self.scroll && crow < self.scroll + self.visible_rows(l.page) {
            let cy =
                area.y + (crow - self.scroll) as i32 * lh + (lh - (self.body() * 1.25) as i32) / 2;
            c.fill(
                Rect::new(
                    area.x + self.row_shift(text, rows[crow], area.w) + ccol as i32,
                    cy,
                    self.px(2.0).max(2),
                    (self.body() * 1.25) as i32,
                ),
                t.ink,
            );
        }

        // The text still being composed is underlined (v1.10).
        // The phrase being chosen, in a sentence of several, is underlined heavier.
        let marks = [
            (model.keyboard.composer.composing(model.buffer), 2.0),
            (model.keyboard.composer.current_phrase(model.buffer), 4.0),
        ];
        for (range, thick) in marks {
            let Some((a, b)) = range else { continue };
            let m = self.measure();
            for (i, r) in rows
                .iter()
                .enumerate()
                .skip(self.scroll)
                .take(self.visible_rows(l.page))
            {
                let (s0, s1) = (a.max(r.start), b.min(r.end));
                if s0 < s1 || (a == b && s0 == r.start) {
                    let row_text = &text[r.start..r.end];
                    let xa = sigma_core::TextMeasure::caret_x(&m, row_text, s0 - r.start);
                    let xb = sigma_core::TextMeasure::caret_x(&m, row_text, s1 - r.start);
                    let shift = self.row_shift(text, *r, area.w);
                    let y = (area.y + (i - self.scroll) as i32 * lh) as f32 + top_pad + asc;
                    let y = y as i32 + self.px(4.0).max(3);
                    c.fill(
                        Rect::new(
                            area.x + shift + xa.min(xb) as i32,
                            y,
                            (xa.max(xb) - xa.min(xb)) as i32,
                            self.px(thick).max(thick as i32),
                        ),
                        t.ink,
                    );
                }
            }
        }
        // v1.16: a drawer on the right stops short of the bar's strip; the
        // rows it covers would show in that strip, so it is cleared.
        if self.drawer_open.iter().any(|&o| o)
            && self.cfg.drawer == sigma_core::settings::Side::Right
        {
            let r = crate::scrollbar::reserve(self.page_s);
            let p = l.page;
            c.fill(Rect::new(p.x + p.w - r, p.y, r, p.h), self.bar_bg(t));
        }
        self.draw_scroll_bar(c, l.page, rows.len(), self.visible_rows(l.page), t);
        if let Some((cands, sel)) = model.keyboard.composer.candidates() {
            self.draw_cands(c, l.page, cands, sel, t);
        }

        self.draw_selection_ui(c, model, &l);

        // Status bar.
        c.fill(Rect::new(0, l.status.y, l.status.w, 1), t.rule);
        let sb = l.status.y as f32
            + l.status.h as f32 / 2.0
            + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
        let left = tr(&format!(
            "{} words \u{b7} line {}",
            sigma_core::text::word_count(text),
            model.buffer.line_number()
        ))
        .into_owned();
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            self.px(20.0) as f32,
            sb,
            &left,
            t.mute,
        );
        let right = self.status_right(t, model.save);
        let rw = self.fonts.width(Face::Atkinson, ui, &right);
        let rx = l.ink_button.x as f32 - self.px(16.0) as f32 - rw;
        self.fonts
            .draw(c, Face::Atkinson, ui, rx, sb, &right, t.mute);
        let kb = l.keys_button;
        if model.keys_on {
            c.fill(kb, t.wash);
            c.stroke(kb, 1, t.ink);
        } else {
            c.stroke(kb, 1, t.rule);
        }
        let kw = self.fonts.width(Face::Atkinson, ui, &tr("Keys"));
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            kb.x as f32 + (kb.w as f32 - kw) / 2.0,
            sb,
            &tr("Keys"),
            t.ink,
        );

        let ib = l.ink_button;
        c.stroke(ib, 1, t.rule);
        let iw = self.fonts.width(Face::Atkinson, ui, &tr("Write"));
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            ib.x as f32 + (ib.w as f32 - iw) / 2.0,
            sb,
            &tr("Write"),
            t.ink,
        );

        if model.keys_on {
            self.draw_keys(c, &l, model.keyboard, t);
        }
        l
    }

    fn draw_selection_ui(&self, c: &mut Canvas, model: &Model<'_>, l: &Layout) {
        if model.rename.is_some() {
            return;
        }
        let t = model.theme;
        let g = self.selection_geometry(model, l);
        let r = self.px(9.0);
        for h in g.handles.iter().flatten() {
            let (cx, cy) = (h.x + h.w / 2, h.y + h.h / 2);
            // Stem from the text row down to a solid dot: no animation,
            // a few hundred pixels per handle for the partial refresh.
            c.fill(
                Rect::new(
                    cx - 1,
                    cy - self.handle_drop() + self.line_h(),
                    2,
                    self.handle_drop() - self.line_h() - r + 1,
                ),
                t.ink,
            );
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        c.blend(cx + dx, cy + dy, t.ink, 255);
                    }
                }
            }
        }
        if let Some(bar) = g.bar {
            let ui = self.ui_px();
            c.fill(bar.inset(-2), t.paper);
            c.fill(bar, t.ink);
            for (i, (br, a)) in g.buttons.iter().enumerate() {
                if i > 0 {
                    c.fill(
                        Rect::new(br.x, br.y + self.px(10.0), 1, br.h - self.px(20.0)),
                        t.mute,
                    );
                }
                let w = self.fonts.width(Face::AtkinsonBold, ui, &tr(a.label()));
                let base = br.y as f32
                    + br.h as f32 / 2.0
                    + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6;
                self.fonts.draw(
                    c,
                    Face::AtkinsonBold,
                    ui,
                    br.x as f32 + (br.w as f32 - w) / 2.0,
                    base,
                    &tr(a.label()),
                    t.paper,
                );
            }
        }
    }

    /// Status bar right side (brief §3): `USB` when a USB keyboard is
    /// connected, `no USB` when the setting is on but none is found (v0.8),
    /// then `Paper` or
    /// `Night`, then the save state.
    fn status_right(&self, t: Theme, save: &SaveState) -> String {
        let s = format!(
            "{}{} \u{b7} {}",
            match (self.usb_live, self.cfg.usb) {
                (true, _) => "USB \u{b7} ",
                (false, true) => "no USB \u{b7} ",
                (false, false) => "",
            },
            if t == Theme::NIGHT { "Night" } else { "Paper" },
            save.label()
        );
        let s = tr(&s).into_owned();
        if self.overwrite {
            format!("OVR \u{b7} {s}")
        } else {
            s
        }
    }

    /// "Aa" in a 44 px target: the Options sheet.
    pub fn draw_options_icon(&self, c: &mut Canvas, r: Rect, t: Theme) {
        crate::icons::draw_icon(c, crate::icons::Icon::Settings, r, t.ink);
    }

    /// Interface text size, device pixels: 17 brief px at the UI scale.
    #[must_use]
    pub fn ui_px(&self) -> f32 {
        17.0 * self.s.0
    }

    /// The UI scale (title bar, status bar, sheets).
    #[must_use]
    pub fn scale(&self) -> Scale {
        self.s
    }

    /// Device scale from the short side, independent of the UI scale (v1.1).
    #[must_use]
    pub fn page_scale(&self) -> Scale {
        self.page_s
    }

    /// Title bar: a name (tap to rename) and the Files icon, or the rename
    /// field with Cancel / OK. Hairline under.
    pub fn draw_title(
        &self,
        c: &mut Canvas,
        l: &Layout,
        name: &str,
        rename: Option<&Buffer>,
        t: Theme,
        files_icon: bool,
    ) {
        let ui = self.ui_px();
        c.fill(l.title, t.paper);
        let base = l.title.y as f32
            + l.title.h as f32 / 2.0
            + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6;
        if let Some(field) = rename {
            let fr = Rect::new(
                self.px(12.0),
                l.ok_rect.y,
                l.cancel_rect.x - self.px(24.0),
                l.ok_rect.h,
            );
            c.fill(fr, t.wash);
            c.stroke(fr, 1, t.ink);
            let x0 = fr.x as f32 + self.px(10.0) as f32;
            let end = self
                .fonts
                .draw(c, Face::Atkinson, ui, x0, base, field.text(), t.ink);
            let before = self
                .fonts
                .width(Face::Atkinson, ui, &field.text()[..field.caret()]);
            let _ = end;
            c.fill(
                Rect::new(
                    (x0 + before) as i32,
                    fr.y + self.px(8.0),
                    2,
                    fr.h - self.px(16.0),
                ),
                t.ink,
            );
            for (r, label, primary) in [(l.cancel_rect, "Cancel", false), (l.ok_rect, "OK", true)] {
                let label: &str = &tr(label);
                c.fill(r, if primary { t.ink } else { t.paper });
                c.stroke(r, 1, t.ink);
                let w = self.fonts.width(Face::AtkinsonBold, ui, label);
                self.fonts.draw(
                    c,
                    Face::AtkinsonBold,
                    ui,
                    r.x as f32 + (r.w as f32 - w) / 2.0,
                    base,
                    label,
                    if primary { t.paper } else { t.ink },
                );
            }
        } else {
            let shown = ellipsize(
                &self.fonts,
                Face::AtkinsonBold,
                ui,
                name,
                l.name_rect.w as f32 - self.px(28.0) as f32,
            );
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                self.px(20.0) as f32,
                base,
                &shown,
                t.ink,
            );
            if files_icon {
                crate::icons::draw_icon(c, crate::icons::Icon::Folder, l.files_button, t.ink);
                if self.back_on {
                    crate::icons::draw_icon(c, crate::icons::Icon::Back, l.back_button, t.ink);
                }
            }
        }
        c.fill(
            Rect::new(0, l.title.y + l.title.h - 1, l.title.w, 1),
            t.rule,
        );
    }

    /// The on-screen keyboard in `l.keyboard`.
    pub fn draw_keys(&self, c: &mut Canvas, l: &Layout, keyboard: &Keyboard, t: Theme) {
        let t = self.key_theme.unwrap_or(t);
        c.fill(l.keyboard, t.paper);
        c.fill(Rect::new(0, l.keyboard.y, l.keyboard.w, 1), t.rule);
        let ks = 20.0 * self.k.0;
        let ui = 17.0 * self.k.0;
        if keyboard.style == sigma_core::keyboard::Style::Kobo {
            self.draw_kobo_keys(c, l, keyboard, t);
            return;
        }
        for &(r, b) in &l.kb_buttons {
            let active = matches!(b, KbButton::Mode(m) if m == self.cfg.land_keys);
            let (bg, fg) = if active {
                (t.ink, t.paper)
            } else {
                (t.paper, t.ink)
            };
            c.fill(r, bg);
            c.stroke(r, 1, if active { t.ink } else { t.rule });
            let label: &str = &tr(self.kb_button_label(b));
            let lw = self.fonts.width(Face::AtkinsonBold, ui, label);
            let by =
                r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(Face::AtkinsonBold, ui) / 2.6;
            self.fonts.draw(
                c,
                Face::AtkinsonBold,
                ui,
                r.x as f32 + (r.w as f32 - lw) / 2.0,
                by,
                label,
                fg,
            );
        }
        for &(r, k) in &l.keys {
            let active = keyboard.is_active(k);
            let (_, fg) = if active {
                (t.ink, t.paper)
            } else {
                (t.paper, t.ink)
            };
            // Same language as the Write panel: rounded outlined keys.
            let rad = self.kpx(8.0).max(4);
            crate::ink::round_outline_in(
                c,
                r,
                rad,
                if active { 0 } else { self.kpx(1.5).max(1) },
                t.ink,
            );
            let icon = match k {
                Key::Back => Some(crate::icons::Icon::Backspace),
                Key::Shift => Some(crate::icons::Icon::Shift),
                Key::Return => Some(crate::icons::Icon::Return),
                Key::Space => Some(crate::icons::Icon::Space),
                Key::Home => Some(crate::icons::Icon::LineStart),
                Key::End => Some(crate::icons::Icon::LineEnd),
                Key::PgUp => Some(crate::icons::Icon::PageUp),
                Key::PgDn => Some(crate::icons::Icon::PageDown),
                Key::Tab => Some(crate::icons::Icon::Tab),
                _ => None,
            };
            if let Some(i) = icon {
                crate::icons::draw_icon(c, i, r, fg);
            }
            if k == Key::Shift && keyboard.shift_lock {
                // Locked: a bar under the label (one-shot has none).
                c.fill(
                    Rect::new(
                        r.x + r.w / 4,
                        r.y + r.h - self.kpx(10.0),
                        r.w / 2,
                        self.kpx(3.0),
                    ),
                    fg,
                );
            }
            if let Key::Arrow(d) = k {
                // Atkinson has no arrow glyphs: draw a 1.75-stroke arrow.
                draw_arrow(c, r, d, self.k, fg);
                continue;
            }
            if icon.is_some() {
                continue;
            }
            let label = keyboard.label(k);
            let label = if matches!(k, Key::Char(_)) {
                // A vowel sign or mark alone sits on a dotted circle (v1.9).
                use unicode_properties::{GeneralCategory as G, UnicodeGeneralCategory};
                match label.chars().next() {
                    Some(c)
                        if matches!(c.general_category(), G::NonspacingMark | G::SpacingMark) =>
                    {
                        format!("\u{25CC}{label}")
                    }
                    _ => label,
                }
            } else if k == Key::SwitchLayout {
                // "Compact" is too wide for its key: four letters and a dot.
                let l = tr(&label).into_owned();
                if l.chars().count() > 5 {
                    format!("{}.", l.chars().take(4).collect::<String>())
                } else {
                    l
                }
            } else {
                tr(&label).into_owned()
            };
            let (face, size) = if matches!(k, Key::Char(_)) {
                (Face::Atkinson, ks * 1.15)
            } else {
                (Face::AtkinsonBold, ks * 0.85)
            };
            let lw = self.fonts.width(face, size, &label);
            let by = r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6;
            self.fonts.draw(
                c,
                face,
                size,
                r.x as f32 + (r.w as f32 - lw) / 2.0,
                by,
                &label,
                fg,
            );
        }
    }
}

impl EditorScreen {
    /// The Kobo style board (v1.13): flat keys, no outlines, plain glyphs
    /// in the sans, Shift and Back as icons, Space as a thin bracket, ← → as
    /// arrows, and Return as the one dark key (the stock reader's action
    /// key). A key that is on (Shift) is inverted.
    fn draw_kobo_keys(&self, c: &mut Canvas, l: &Layout, keyboard: &Keyboard, t: Theme) {
        use crate::icons::{draw_icon, Icon};
        let ks = 20.0 * self.k.0;
        for &(r, k) in &l.keys {
            let on = keyboard.is_active(k) || k == Key::Return;
            let (bg, fg) = if on {
                (t.ink, t.paper)
            } else {
                (t.paper, t.ink)
            };
            if on {
                c.fill(r, bg);
            }
            match k {
                Key::Arrow(d) => {
                    draw_arrow(c, r, d, self.k, fg);
                    continue;
                }
                Key::Back => {
                    draw_icon(c, Icon::Backspace, r, fg);
                    continue;
                }
                Key::Shift => {
                    draw_icon(c, Icon::Shift, r, fg);
                    if keyboard.shift_lock {
                        c.fill(
                            Rect::new(
                                r.x + r.w / 4,
                                r.y + r.h - self.kpx(10.0),
                                r.w / 2,
                                self.kpx(3.0),
                            ),
                            fg,
                        );
                    }
                    continue;
                }
                Key::Space => {
                    // A thin bracket: a line with a short tick at each end.
                    let line = self.kpx(2.0).max(2);
                    let tick = self.kpx(10.0);
                    let y = r.y + r.h * 11 / 20;
                    let (x0, x1) = (r.x + r.w / 8, r.x + r.w * 7 / 8);
                    c.fill(Rect::new(x0, y, x1 - x0, line), fg);
                    c.fill(Rect::new(x0, y - tick, line, tick + line), fg);
                    c.fill(Rect::new(x1 - line, y - tick, line, tick + line), fg);
                    continue;
                }
                _ => {}
            }
            let label = keyboard.label(k);
            let label = if matches!(k, Key::Char(_) | Key::Layer(_)) {
                label
            } else {
                tr(&label).into_owned()
            };
            let (face, size) = match k {
                Key::Char(_) => (Face::Atkinson, ks * 1.3),
                _ => (Face::AtkinsonBold, ks * 0.85),
            };
            let lw = self.fonts.width(face, size, &label);
            let by = r.y as f32 + r.h as f32 / 2.0 + self.fonts.ascent(face, size) / 2.6;
            self.fonts.draw(
                c,
                face,
                size,
                r.x as f32 + (r.w as f32 - lw) / 2.0,
                by,
                &label,
                fg,
            );
        }
    }
}

/// Split: both halves need a Space (brief §7). If the row's Space sits
/// wholly on one side, move it to the slot where it crosses the middle, as
/// evenly as possible; the other keys keep their order.
fn space_across(keyboard: &Keyboard, r: usize, row: &[Key], grid: f32) -> Vec<Key> {
    let Some(at) = row.iter().position(|&k| k == Key::Space) else {
        return row.to_vec();
    };
    let mid = grid / 2.0;
    let others: Vec<Key> = row.iter().copied().filter(|&k| k != Key::Space).collect();
    let mut best = (f32::MAX, at, row.to_vec());
    for i in 0..=others.len() {
        let mut cand = others.clone();
        cand.insert(i, Key::Space);
        let total: f32 = cand
            .iter()
            .enumerate()
            .map(|(c, &k)| keyboard.units(k, r, c))
            .sum();
        let start = (grid - total) / 2.0
            + cand[..i]
                .iter()
                .enumerate()
                .map(|(c, &k)| keyboard.units(k, r, c))
                .sum::<f32>();
        let ku = keyboard.units(Key::Space, r, i);
        let imbalance = ((mid - start) - (start + ku - mid)).abs();
        let crosses = start < mid && start + ku > mid;
        // Keep the original slot when it already crosses.
        let score = if !crosses {
            f32::MAX / 2.0
        } else if i == at {
            -1.0
        } else {
            imbalance
        };
        if score < best.0 {
            best = (score, i, cand);
        }
    }
    best.2
}

/// Landscape with Line on: the document is a page on a desk. The strips
/// beside the locked column are the desk: the gutter tone with a fine,
/// fixed grain (the same pixels every frame, so a redraw never flickers).
pub fn draw_gutters(c: &mut Canvas, l: &Layout, t: Theme) {
    if l.column.w > 0 && l.column.w < l.page.w {
        let left = Rect::new(l.page.x, l.page.y, l.column.x - l.page.x, l.page.h);
        let right_x = l.column.x + l.column.w;
        let right = Rect::new(right_x, l.page.y, l.page.x + l.page.w - right_x, l.page.h);
        for r in [left, right] {
            grain_fill(c, r, t.gutter);
        }
    }
}

/// Fills `r` with `base` ± a few levels of fixed noise (a desk guard's
/// texture). The noise is a hash of the pixel's place, so it is stable.
fn grain_fill(c: &mut Canvas, r: Rect, base: u8) {
    let b = c.bounds();
    for y in r.y.max(b.y)..(r.y + r.h).min(b.y + b.h) {
        for x in r.x.max(b.x)..(r.x + r.w).min(b.x + b.w) {
            let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
            h ^= h >> 15;
            h = h.wrapping_mul(0x2C1B_3C6D);
            h ^= h >> 12;
            let n = (h % 11) as i32 - 5;
            c.pixels[(y * c.width as i32 + x) as usize] = (i32::from(base) + n).clamp(0, 255) as u8;
        }
    }
}

fn draw_arrow(c: &mut Canvas, r: Rect, d: Dir, s: Scale, ink: u8) {
    let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
    let len = s.px(10.0);
    let head = s.px(6.0);
    let st = s.px(1.75).max(2);
    // Points along the shaft and the two head strokes, in arrow space
    // (pointing +x), rotated per direction.
    let rot = |x: i32, y: i32| match d {
        Dir::Right => (cx + x, cy + y),
        Dir::Left => (cx - x, cy + y),
        Dir::Down => (cx + y, cy + x),
        Dir::Up => (cx + y, cy - x),
    };
    let mut dot = |x: i32, y: i32| {
        for a in 0..st {
            for b in 0..st {
                let (px, py) = rot(x, y);
                c.blend(px + a - st / 2, py + b - st / 2, ink, 255);
            }
        }
    };
    for x in -len..=len {
        dot(x, 0);
    }
    for k in 0..=head {
        dot(len - k, k);
        dot(len - k, -k);
    }
}

/// What preview mode shows (v0.3, read-only).
pub struct PreviewModel<'a> {
    /// The note's file name, shown in the title bar.
    pub file_name: &'a str,
    /// The parsed and laid-out note.
    pub doc: &'a Doc,
    /// Index of the preview page shown (clamped to the last page).
    pub page: usize,
    /// Word count, shown in the status bar.
    pub words: usize,
    /// Save state, shown in the status bar.
    pub save: &'a SaveState,
    /// The keyboard, for the layout (it stays hidden in preview).
    pub keyboard: &'a Keyboard,
    /// Page colours.
    pub theme: Theme,
}

impl EditorScreen {
    /// The text column of the preview page (the keyboard is hidden).
    #[must_use]
    pub fn preview_area(&self, keyboard: &Keyboard) -> Rect {
        let page = self.layout(false, keyboard).page;
        let a = self.text_area(page);
        let Some(d) = self.preview_drawer else {
            return a;
        };
        // Beside the drawer, with the page margin on the drawer side too.
        let m = self.page_s.px(self.margin_px);
        if d.x <= page.x {
            let x = a.x.max(d.x + d.w + m);
            Rect::new(x, a.y, (a.x + a.w - x).max(1), a.h)
        } else {
            let r = (a.x + a.w).min(d.x - m);
            Rect::new(a.x, a.y, (r - a.x).max(1), a.h)
        }
    }

    /// Parses and lays out `text` for preview at the current type settings.
    #[must_use]
    pub fn build_preview(
        &self,
        text: &str,
        keyboard: &Keyboard,
        known: &dyn Fn(&str) -> bool,
    ) -> Doc {
        self.build_preview_with(text, keyboard, known, None)
    }

    /// As [`Self::build_preview`], with a Backlinks section at the end
    /// (brief §4): `(label, target)` per note that links here.
    #[must_use]
    pub fn build_preview_with(
        &self,
        text: &str,
        keyboard: &Keyboard,
        known: &dyn Fn(&str) -> bool,
        backlinks: Option<&[(String, String)]>,
    ) -> Doc {
        self.build_preview_in(text, keyboard, known, backlinks, None)
    }

    /// As [`Self::build_preview_with`]; `note_dir` is where `.ink/` image
    /// links resolve (v1.5.1). Unreadable images stay `[image: …]`.
    #[must_use]
    pub fn build_preview_in(
        &self,
        text: &str,
        keyboard: &Keyboard,
        known: &dyn Fn(&str) -> bool,
        backlinks: Option<&[(String, String)]>,
        note_dir: Option<&std::path::Path>,
    ) -> Doc {
        use sigma_core::markdown::{Block, BlockKind, LinkKind, Run, Style};
        let area = self.preview_area(keyboard);
        let ty = Type {
            body: self.body(),
            leading: self.leading,
            scale: self.page_s,
        };
        let mut blocks = sigma_core::markdown::parse(text);
        if let Some(bl) = backlinks {
            // Past every source line, so it never moves the write position.
            let line = text.lines().count() + 1;
            let run = |text: &str, style: Style, target: Option<&str>| Run {
                text: text.to_owned(),
                style,
                target: target.map(str::to_owned),
            };
            let block = |kind, runs| Block { kind, runs, line };
            blocks.push(block(BlockKind::Rule, Vec::new()));
            blocks.push(block(
                BlockKind::Paragraph,
                vec![run(
                    "Backlinks",
                    Style {
                        strong: true,
                        ..Style::default()
                    },
                    None,
                )],
            ));
            if bl.is_empty() {
                blocks.push(block(
                    BlockKind::Paragraph,
                    vec![run(
                        "No other notes link here.",
                        Style {
                            em: true,
                            ..Style::default()
                        },
                        None,
                    )],
                ));
            }
            for (label, target) in bl {
                blocks.push(block(
                    BlockKind::Item {
                        depth: 0,
                        marker: None,
                        task: None,
                    },
                    vec![run(
                        label,
                        Style {
                            link: Some(LinkKind::Wiki),
                            ..Style::default()
                        },
                        Some(target),
                    )],
                ));
            }
        }
        let images = |src: &str, max_w: i32| {
            let svg = std::fs::read_to_string(note_dir?.join(src)).ok()?;
            crate::ink::Raster::from_svg(&svg, max_w).map(std::sync::Arc::new)
        };
        let lines = preview::layout_with_images(&blocks, &self.fonts, area.w, ty, known, &images);
        Doc {
            pages: preview::paginate(&lines, area.h),
            lines,
            width: area.w,
        }
    }

    /// Source line at the top of the write-mode page (keeps the place when
    /// entering preview).
    #[must_use]
    pub fn top_source_line(&self, buffer: &Buffer, page: Rect) -> usize {
        let rows = self.rows(buffer, page);
        let at = rows.get(self.scroll).map_or(0, |r| r.start);
        buffer.text()[..at.min(buffer.text().len())]
            .matches('\n')
            .count()
    }

    /// The `[[wiki]]` target under a tap on the shown preview page.
    #[must_use]
    pub fn preview_link_at(
        &self,
        keyboard: &Keyboard,
        doc: &Doc,
        page: usize,
        x: i32,
        y: i32,
    ) -> Option<String> {
        let area = self.preview_area(keyboard);
        let r = doc.pages.get(page)?.clone();
        preview::link_at(&self.fonts, area, &doc.lines, r, x, y, self.px(8.0))
    }

    /// Screen point of the first link labelled `label` on the shown page
    /// (emulator scripts).
    #[must_use]
    pub fn preview_link_point(
        &self,
        keyboard: &Keyboard,
        doc: &Doc,
        page: usize,
        label: &str,
    ) -> Option<(i32, i32)> {
        let area = self.preview_area(keyboard);
        let mut y = area.y;
        for l in &doc.lines[doc.pages.get(page)?.clone()] {
            for p in &l.pieces {
                if p.target.is_some() && p.text.trim() == label {
                    let w = self.fonts.width(p.face, p.size, &p.text) as i32;
                    return Some((area.x + p.x + w / 2, y + l.h / 2));
                }
            }
            y += l.h;
        }
        None
    }

    /// Taps in preview: only the title-bar buttons do something in v0.3.
    #[must_use]
    pub fn preview_hit(&self, keyboard: &Keyboard, x: i32, y: i32) -> Hit {
        let l = self.layout(false, keyboard);
        if self.back_on && l.back_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Back
        } else if l.files_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Files
        } else if l.preview_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::TogglePreview
        } else if l.options_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Options
        } else if l.find_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Find
        } else if self.graph_on && l.graph_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Graph
        } else if self.panes_on() && l.drawer_button.inset(-self.px(4.0)).contains(x, y) {
            // v1.0.3: the drawer panes open in preview too.
            Hit::DrawerMenu
        } else if self.light_on && l.light_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::LightMenu
        } else if self.wifi_on && l.wifi_button.inset(-self.px(4.0)).contains(x, y) {
            Hit::Wifi
        } else {
            Hit::Nothing
        }
    }

    /// The preview's scroll bar as a paged bar (v1.13.1): the page's place
    /// among `n` pages; `None` with one page.
    #[must_use]
    pub fn preview_paged_bar(
        &self,
        keyboard: &Keyboard,
        n: usize,
        at: usize,
    ) -> Option<crate::scrollbar::Paged> {
        let l = self.layout(false, keyboard);
        crate::scrollbar::Paged::new(self.page_s, l.page, n, at)
    }

    /// Draws preview mode: title with the pencil, one page, status bar.
    pub fn draw_preview(&self, c: &mut Canvas, m: &PreviewModel<'_>) -> Layout {
        let t = m.theme;
        let l = self.layout(false, m.keyboard);
        c.fill(c.bounds(), t.paper);
        draw_gutters(c, &l, t);
        self.draw_title(c, &l, m.file_name, None, t, true);
        crate::icons::draw_icon(c, crate::icons::Icon::Pencil, l.preview_button, t.ink);
        self.draw_options_icon(c, l.options_button, t);
        crate::icons::draw_icon(c, crate::icons::Icon::Search, l.find_button, t.ink);
        if self.graph_on {
            crate::icons::draw_icon(c, crate::icons::Icon::Graph, l.graph_button, t.ink);
        }
        // v1.0.3: the drawer button, active while a pane is open (as in write).
        self.draw_drawer_button(c, &l, t);
        self.draw_light_button(c, &l, t);
        self.draw_wifi_button(c, &l, t);
        let area = self.preview_area(m.keyboard);
        let page = m.page.min(m.doc.pages.len().saturating_sub(1));
        // The same bar as the editor's: the page's place among the pages.
        if let Some(b) = crate::scrollbar::bar(self.page_s, l.page, m.doc.pages.len(), 1, page) {
            crate::scrollbar::draw_bar(c, self.page_s, b, t, self.bar_bg(t));
        }
        if let Some(r) = m.doc.pages.get(page) {
            preview::draw_page(
                c,
                &self.fonts,
                area,
                &m.doc.lines,
                r.clone(),
                t,
                self.page_s,
            );
        }
        let ui = self.ui_px();
        c.fill(Rect::new(0, l.status.y, l.status.w, 1), t.rule);
        let sb = l.status.y as f32
            + l.status.h as f32 / 2.0
            + self.fonts.ascent(Face::Atkinson, ui) / 2.6;
        let n = m.doc.pages.len();
        let mut left = format!("{} words \u{b7} preview", m.words);
        if n > 1 {
            left.push_str(&format!(" \u{b7} page {} of {n}", page + 1));
        }
        let left = tr(&left).into_owned();
        self.fonts.draw(
            c,
            Face::Atkinson,
            ui,
            self.px(20.0) as f32,
            sb,
            &left,
            t.mute,
        );
        let right = self.status_right(t, m.save);
        let rw = self.fonts.width(Face::Atkinson, ui, &right);
        let rx = (l.status.w - self.px(20.0)) as f32 - rw;
        self.fonts
            .draw(c, Face::Atkinson, ui, rx, sb, &right, t.mute);
        l
    }
}

pub(crate) fn ellipsize(f: &Fonts, face: Face, size: f32, s: &str, max: f32) -> String {
    if f.width(face, size, s) <= max {
        return s.to_owned();
    }
    let mut out: String = s.to_owned();
    while !out.is_empty() && f.width(face, size, &format!("{out}\u{2026}")) > max {
        out.pop();
    }
    format!("{out}\u{2026}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 1440;
    const H: u32 = 1920;

    fn model<'a>(b: &'a Buffer, k: &'a Keyboard, s: &'a SaveState) -> Model<'a> {
        Model {
            file_name: "field-notes.md",
            buffer: b,
            keyboard: k,
            keys_on: true,
            save: s,
            theme: Theme::DAY,
            rename: None,
            sel_ui: None,
        }
    }

    #[test]
    fn layout_fits_forma_and_targets_are_large() {
        let e = EditorScreen::new(W, H);
        let k = Keyboard::with_layout(sigma_core::keyboard::Layout::Compact);
        let l = e.layout(true, &k);
        assert!(l.page.h > 900, "page height {}", l.page.h);
        assert_eq!(l.keys.len(), 10 + 9 + 9 + 6);
        for (r, _) in &l.keys {
            assert!(r.w >= 50 && r.h >= 50, "target {r:?} under 44 brief px");
            assert!(r.x >= 0 && r.x + r.w <= W as i32 && r.y + r.h <= H as i32);
        }
        assert!(l.keys_button.w >= 50 && l.keys_button.h >= 50);
    }

    #[test]
    fn full_layout_fits_portrait_with_big_keys_and_usable_page() {
        let e = EditorScreen::new(W, H);
        let k = Keyboard::with_layout(sigma_core::keyboard::Layout::Full);
        let l = e.layout(true, &k);
        assert_eq!(l.keys.len(), 15 + 15 + 14 + 14 + 9);
        let narrowest = l.keys.iter().map(|(r, _)| r.w).min().unwrap();
        // v1.1: keyboard capped at 42 % of the screen height.
        assert!(narrowest >= 70, "1-unit key {narrowest}px");
        for (r, _) in &l.keys {
            assert!(
                r.h >= 60 && r.x >= 0 && r.x + r.w <= W as i32 && r.y + r.h <= H as i32,
                "{r:?}"
            );
        }
        // No overlaps, and ↑ sits above ↓.
        for (i, (a, _)) in l.keys.iter().enumerate() {
            for (b, _) in &l.keys[i + 1..] {
                assert!(
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y,
                    "{a:?} {b:?}"
                );
            }
        }
        let find = |d| {
            l.keys
                .iter()
                .find(|(_, key)| *key == Key::Arrow(d))
                .unwrap()
                .0
        };
        use sigma_core::keyboard::Dir;
        assert_eq!(find(Dir::Up).x, find(Dir::Down).x);
        // v1.1: the keyboard is capped at 42 % of the height.
        assert!(
            l.keyboard.h <= H as i32 * 42 / 100,
            "keyboard {}px",
            l.keyboard.h
        );
        assert!(l.page.h >= 800, "page {}px", l.page.h);
        let c = e.layout(
            true,
            &Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
        );
        let none = e.layout(false, &k);
        println!(
            "METRICS full kb {} page {} key {}x{} | compact kb {} page {} key {}x{} | hidden page {}",
            l.keyboard.h, l.page.h, narrowest, l.keys[0].0.h,
            c.keyboard.h, c.page.h, c.keys[0].0.w, c.keys[0].0.h, none.page.h
        );
        let (b, s) = (Buffer::new(""), SaveState::Saved);
        let m = model(&b, &k, &s);
        for (r, key) in &l.keys {
            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Key(*key));
        }
    }

    #[test]
    fn selection_handles_and_bar_are_placed_and_hittable() {
        let mut e = EditorScreen::new(W, H);
        let k = Keyboard::default();
        let s = SaveState::Saved;
        let mut b = Buffer::new("one\ntwo\nthree\nfour\nsecond line with words\nthird");
        let at = b.text().find("with").unwrap();
        b.select(at, at + 4);
        let mut m = model(&b, &k, &s);
        m.sel_ui = Some(SelUi { bar: true });
        let mut c = Canvas::new(W, H, 255);
        let l = e.draw(&mut c, &m);
        let g = e.selection_geometry(&m, &l);
        let [Some(h0), Some(h1)] = g.handles else {
            panic!("handles {g:?}")
        };
        assert!(h0.w >= 50 && h0.x < h1.x && h0.y == h1.y);
        let labels: Vec<_> = g.buttons.iter().map(|(_, a)| a.label()).collect();
        assert_eq!(labels, ["Cut", "Copy", "Paste", "Select all"]);
        let bar = g.bar.unwrap();
        assert!(bar.y + bar.h <= h0.y && bar.x >= 0 && bar.x + bar.w <= W as i32 && bar.h >= 50);
        for (r, a) in &g.buttons {
            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Bar(*a));
        }
        assert_eq!(e.hit(&m, h1.x + h1.w / 2, h1.y + h1.h / 2), Hit::Handle(1));
        // Offsets round-trip through screen points (char snapping).
        let (x, y) = e.point_of(&b, l.page, at).unwrap();
        assert_eq!(e.offset_at_point(&b, l.page, x + 1, y + 5), at);
        // First line: no room above, bar goes below the handles.
        let mut b2 = Buffer::new("first line of text");
        b2.select(0, 5);
        let mut m2 = model(&b2, &k, &s);
        m2.sel_ui = Some(SelUi { bar: true });
        let g2 = e.selection_geometry(&m2, &l);
        assert!(g2.bar.unwrap().y > g2.handles[1].unwrap().y);
        // Caret only: Paste and Select all, no handles.
        let b3 = Buffer::new("x  y");
        let mut m3 = model(&b3, &k, &s);
        m3.sel_ui = Some(SelUi { bar: true });
        let g3 = e.selection_geometry(&m3, &l);
        assert_eq!(g3.handles, [None, None]);
        assert_eq!(
            g3.buttons.iter().map(|(_, a)| *a).collect::<Vec<_>>(),
            [BarAction::Paste, BarAction::SelectAll]
        );
    }

    #[test]
    fn every_key_is_hittable_at_its_centre() {
        let e = EditorScreen::new(W, H);
        let (b, k, s) = (
            Buffer::new(""),
            Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
            SaveState::Saved,
        );
        let m = model(&b, &k, &s);
        for (r, key) in e.layout(true, &k).keys {
            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Key(key));
        }
    }

    #[test]
    fn tap_on_page_places_caret() {
        let mut e = EditorScreen::new(W, H);
        let (b, k, s) = (
            Buffer::new("first line\nsecond line"),
            Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
            SaveState::Saved,
        );
        let m = model(&b, &k, &s);
        let mut c = Canvas::new(W, H, 255);
        let l = e.draw(&mut c, &m);
        let area = e.text_area(l.page);
        match e.hit(&m, area.x + 1, area.y + e.line_h() + e.line_h() / 2) {
            Hit::Caret(at) => assert_eq!(at, 11),
            other => panic!("{other:?}"),
        }
        match e.hit(&m, area.x + area.w - 1, area.y + 2) {
            Hit::Caret(at) => assert_eq!(at, 10),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn about_twelve_words_per_line_at_defaults() {
        let e = EditorScreen::new(W, H);
        let text =
            "the quick brown fox jumps over the lazy dog and keeps on running far away ".repeat(5);
        let b = Buffer::new(text.clone());
        let rows = e.rows(
            &b,
            e.layout(
                true,
                &Keyboard::with_layout(sigma_core::keyboard::Layout::Compact),
            )
            .page,
        );
        let words = text[rows[0].start..rows[0].end].split_whitespace().count();
        // Brief defaults (20 px at 1264, margin 28) with Line off give a long
        // measure on the 8" Forma; Line mode (S4) fits 8–16 words.
        assert!((12..=30).contains(&words), "{words} words on the first row");
    }

    #[test]
    fn long_note_scrolls_with_caret_and_page_turn() {
        let mut e = EditorScreen::new(W, H);
        let mut b = Buffer::new("line\n".repeat(300));
        let k = Keyboard::with_layout(sigma_core::keyboard::Layout::Compact);
        let page = e.layout(true, &k).page;
        e.follow_caret(&b, page);
        assert!(e.scroll > 250);
        let before = e.scroll;
        e.page_turn(&mut b, page, false);
        assert!(e.scroll < before);
        e.page_turn(&mut b, page, true);
        assert_eq!(e.scroll, before.min(e.scroll));
    }

    #[test]
    fn without_the_graph_icon_the_rest_close_up_and_settings_stays_flush_right() {
        let k = Keyboard::default();
        for (w, h) in [(W, H), (H, W)] {
            let mut e = EditorScreen::new(w, h);
            let with = e.layout(true, &k);
            e.graph_on = false;
            let l = e.layout(true, &k);
            let s = e.scale();
            let order = [
                l.files_button,
                l.preview_button,
                l.find_button,
                l.drawer_button,
                l.options_button,
            ];
            for r in order {
                assert_eq!(
                    (r.w, r.h),
                    (s.px(44.0), s.px(44.0)),
                    "{w}x{h}: same icon size"
                );
            }
            for p in order.windows(2) {
                assert!(p[0].x + p[0].w < p[1].x, "left to right, no overlap");
            }
            assert_eq!(
                l.options_button, with.options_button,
                "Settings does not move"
            );
            assert_eq!(l.close_button, l.options_button);
            assert_eq!(l.graph_button, Rect::default(), "no button");
            // The place the Graph icon had is the drawer button's now.
            assert!(l.drawer_button.x > with.drawer_button.x);
            // Nothing answers where the Graph icon was, nor in the corner a
            // default rectangle would cover.
            let b = Buffer::new("x");
            let st = SaveState::Saved;
            let m = model(&b, &k, &st);
            let g = with.graph_button;
            assert_ne!(e.hit(&m, g.x + g.w / 2, g.y + g.h / 2), Hit::Graph);
            assert_ne!(e.hit(&m, 0, 0), Hit::Graph);
            assert_ne!(e.preview_hit(&k, g.x + g.w / 2, g.y + g.h / 2), Hit::Graph);
            assert_ne!(e.preview_hit(&k, 0, 0), Hit::Graph);
            // Every remaining icon still hits itself.
            let want = [
                Hit::Files,
                Hit::TogglePreview,
                Hit::Find,
                Hit::DrawerMenu,
                Hit::Options,
            ];
            for (r, h) in order.iter().zip(want) {
                assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), h, "{w}x{h:?}");
            }
        }
    }

    #[test]
    fn title_icons_in_mockup_order_at_ui_scale_both_orientations() {
        let k = Keyboard::default();
        for (w, h) in [(W, H), (H, W)] {
            let e = EditorScreen::new(w, h);
            let s = e.scale();
            let l = e.layout(true, &k);
            let order = [
                l.files_button,
                l.preview_button,
                l.find_button,
                l.drawer_button,
                l.graph_button,
                l.options_button,
            ];
            for r in order {
                assert_eq!(
                    (r.w, r.h),
                    (s.px(44.0), s.px(44.0)),
                    "{w}x{h}: 44 units at UI scale"
                );
                assert!(r.w >= 44 && r.y >= 0 && r.y + r.h <= l.title.h);
            }
            for p in order.windows(2) {
                assert!(p[0].x + p[0].w < p[1].x, "left to right, no overlap");
            }
            assert_eq!(l.close_button, l.options_button);
            assert!(l.options_button.x + l.options_button.w <= w as i32 - s.px(12.0));
            assert!(l.name_rect.w >= s.px(100.0), "{w}: title keeps room");
            assert!(l.title.h >= s.px(44.0) && l.page.y >= l.title.h);
            // Every icon hits itself in write and preview mode.
            let b = Buffer::new("x");
            let st = SaveState::Saved;
            let m = model(&b, &k, &st);
            let want = [
                Hit::Files,
                Hit::TogglePreview,
                Hit::Find,
                Hit::DrawerMenu,
                Hit::Graph,
                Hit::Options,
            ];
            for (r, h) in order.iter().zip(want) {
                let (x, y) = (r.x + r.w / 2, r.y + r.h / 2);
                assert_eq!(e.hit(&m, x, y), h, "{w}x{h:?}");
            }
            for (r, h) in order.iter().zip(want) {
                assert_eq!(e.preview_hit(&k, r.x + r.w / 2, r.y + r.h / 2), h);
            }
        }
    }

    #[test]
    fn title_icon_overflow_shrinks_gaps_then_icons_never_below_44() {
        let e = EditorScreen::new(W, H);
        let s = e.scale();
        assert_eq!(e.title_icons(W as i32), (s.px(44.0), s.px(4.0)));
        let n = TITLE_ICONS;
        let full = n * s.px(44.0) + (n - 1) * s.px(4.0) + s.px(12.0) + s.px(100.0);
        let (b, g) = e.title_icons(full - 8);
        assert_eq!((b, g), (s.px(44.0), s.px(1.0)), "gaps shrink first");
        let (b2, _) = e.title_icons(full - s.px(80.0));
        assert!(b2 < s.px(44.0), "then icons");
        let floor = Scale::for_width(W).px(44.0);
        assert!(b2 >= floor);
        assert_eq!(e.title_icons(300).0, floor);
    }

    #[test]
    fn eye_and_pencil_hit_and_preview_draws_without_keys() {
        let b = Buffer::new("# Title\n\nBody *em*");
        let k = Keyboard::default();
        let st = SaveState::Saved;
        let mut e = EditorScreen::new(W, H);
        let m = model(&b, &k, &st);
        let l = e.layout(true, &k);
        assert!(l.files_button.x + l.files_button.w < l.preview_button.x);
        assert!(l.files_button.x > l.name_rect.x + l.name_rect.w);
        let (cx, cy) = (l.preview_button.x + 20, l.preview_button.y + 20);
        assert_eq!(e.hit(&m, cx, cy), Hit::TogglePreview);
        assert_eq!(e.preview_hit(&k, cx, cy), Hit::TogglePreview);
        assert_eq!(e.preview_hit(&k, 700, 900), Hit::Nothing);
        let doc = e.build_preview(b.text(), &k, &|_| false);
        assert_eq!(doc.pages.len(), 1);
        let mut c = Canvas::new(W, H, 0);
        let pl = e.draw_preview(
            &mut c,
            &PreviewModel {
                file_name: "a.md",
                doc: &doc,
                page: 0,
                words: 3,
                save: &st,
                keyboard: &k,
                theme: Theme::DAY,
            },
        );
        assert!(pl.keys.is_empty(), "no keyboard in preview");
        // Page and status bar are drawn; nothing below the status bar.
        assert!(c.pixels.iter().any(|&p| p < 80));
        let _ = e.draw(&mut c, &m);
    }

    fn no_overlap(keys: &[(Rect, Key)]) {
        for (i, (a, _)) in keys.iter().enumerate() {
            for (b, _) in &keys[i + 1..] {
                assert!(
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y,
                    "{a:?} {b:?}"
                );
            }
        }
    }

    fn landscape(
        mode: LandKeys,
        pad: Pad,
        side: Side,
        layout: sigma_core::keyboard::Layout,
    ) -> (EditorScreen, Keyboard, Layout) {
        let mut e = EditorScreen::new(H, W);
        let mut st = Settings {
            land_keys: mode,
            pad,
            pad_side: side,
            ..Settings::default()
        };
        e.configure(&mut st, None);
        let k = Keyboard::with_layout(layout);
        let l = e.layout(true, &k);
        (e, k, l)
    }

    #[test]
    fn landscape_keyboards_fit_hit_and_keep_portrait_key_size() {
        use sigma_core::keyboard::Layout as L;
        let portrait = EditorScreen::new(W, H).layout(true, &Keyboard::with_layout(L::Compact));
        let unit_w = portrait.keys[0].0.w;
        for layout in [L::Compact, L::Full] {
            for mode in [LandKeys::Centre, LandKeys::Split, LandKeys::Full] {
                for pad in [Pad::Numbers, Pad::Arrows] {
                    for side in [Side::Left, Side::Right] {
                        let (e, k, l) = landscape(mode, pad, side, layout);
                        no_overlap(&l.keys);
                        let (b, s) = (Buffer::new(""), SaveState::Saved);
                        let m = model(&b, &k, &s);
                        for (r, key) in &l.keys {
                            assert!(
                                r.x >= 5 && r.x + r.w <= H as i32 - 5 && r.y + r.h <= W as i32,
                                "{r:?}"
                            );
                            assert!(r.w >= 50 && r.h >= 50, "{r:?}");
                            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::Key(*key));
                        }
                        for (r, b) in &l.kb_buttons {
                            assert!(r.y >= l.keyboard.y && r.x + r.w <= H as i32);
                            assert_eq!(e.hit(&m, r.x + r.w / 2, r.y + r.h / 2), Hit::KbButton(*b));
                        }
                        // v1.1: 200 % UI + 45 % keyboard cap leaves a smaller page.
                        assert!(l.page.h >= 500, "{mode:?} page {}", l.page.h);
                        if layout == L::Compact {
                            let q = l.keys.iter().find(|(_, k)| *k == Key::Char('q')).unwrap().0;
                            // Full with the + - * / column shrinks keys a little.
                            let tol = if mode == LandKeys::Full {
                                unit_w / 10
                            } else {
                                1
                            };
                            assert!((q.w - unit_w).abs() <= tol, "unit {} vs {unit_w}", q.w);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn split_has_a_gap_and_space_on_both_halves() {
        use sigma_core::keyboard::Layout as L;
        let (_, _, l) = landscape(LandKeys::Split, Pad::Numbers, Side::Right, L::Compact);
        let spaces: Vec<_> = l.keys.iter().filter(|(_, k)| *k == Key::Space).collect();
        assert_eq!(spaces.len(), 2);
        let t = l.keys.iter().find(|(_, k)| *k == Key::Char('t')).unwrap().0;
        let y = l.keys.iter().find(|(_, k)| *k == Key::Char('y')).unwrap().0;
        assert!(y.x - (t.x + t.w) > 2 * t.w, "clear gap between halves");
        let shift = l.keys.iter().find(|(_, k)| *k == Key::Shift).unwrap().0;
        let back = l.keys.iter().find(|(_, k)| *k == Key::Back).unwrap().0;
        assert!(shift.x < t.x && back.x > y.x);
    }

    #[test]
    fn full_pad_sits_on_the_chosen_side_with_the_right_keys() {
        use sigma_core::keyboard::Layout as L;
        let (_, _, l) = landscape(LandKeys::Full, Pad::Numbers, Side::Right, L::Compact);
        let seven = l.keys.iter().find(|(_, k)| *k == Key::Char('7')).unwrap().0;
        let p = l.keys.iter().find(|(_, k)| *k == Key::Char('p')).unwrap().0;
        assert!(seven.x > p.x + p.w);
        let (_, _, l) = landscape(LandKeys::Full, Pad::Arrows, Side::Left, L::Compact);
        let up = l
            .keys
            .iter()
            .find(|(_, k)| *k == Key::Arrow(Dir::Up))
            .unwrap()
            .0;
        let down = l
            .keys
            .iter()
            .find(|(_, k)| *k == Key::Arrow(Dir::Down))
            .unwrap()
            .0;
        let q = l.keys.iter().find(|(_, k)| *k == Key::Char('q')).unwrap().0;
        assert!(up.x + up.w < q.x && up.x == down.x && up.y < down.y);
        assert_eq!(l.kb_buttons.len(), 5);
    }

    #[test]
    fn line_mode_holds_n_words_in_both_orientations_with_real_fonts() {
        for face in [Typeface::Literata, Typeface::Atkinson] {
            for n in 8..=16u8 {
                let mut st = Settings {
                    line_on: true,
                    line_words: n,
                    face,
                    ..Settings::default()
                };
                let k = Keyboard::default();
                let mut p = EditorScreen::new(W, H);
                p.configure(&mut st, None);
                let pp = p.layout(true, &k).page;
                assert_eq!(
                    p.sample_words_per_row(pp),
                    usize::from(n),
                    "portrait {face:?} {n}"
                );
                let mut l = EditorScreen::new(H, W);
                l.configure(&mut st, None);
                let lp = l.layout(true, &k);
                assert_eq!(
                    l.sample_words_per_row(lp.page),
                    usize::from(n),
                    "landscape {face:?} {n}"
                );
                assert_eq!(lp.column.w, W as i32, "column locked to the portrait width");
                assert!(lp.column.x > 0, "gutters beside the column");
            }
        }
    }

    #[test]
    fn night_draws_inverted_and_key_tone_can_differ() {
        let mut e = EditorScreen::new(W, H);
        let mut st = Settings {
            paper: sigma_core::settings::Paper::Night,
            key_tone: KeyTone::Light,
            ..Settings::default()
        };
        e.configure(&mut st, None);
        let (b, k, s) = (Buffer::new("night"), Keyboard::default(), SaveState::Saved);
        let mut m = model(&b, &k, &s);
        m.theme = Theme::NIGHT;
        let mut c = Canvas::new(W, H, 128);
        let l = e.draw(&mut c, &m);
        let px = |x: i32, y: i32| c.pixels[(y * W as i32 + x) as usize];
        assert_eq!(
            px(W as i32 / 2, l.page.y + 5),
            Theme::NIGHT.paper,
            "night page, not a grey card"
        );
        assert_eq!(
            px(5, l.page.y + 5),
            Theme::NIGHT.paper,
            "no desk without Line"
        );
        assert_eq!(px(2, l.keyboard.y + 3), Theme::DAY.paper, "light keyboard");
        assert!(e.status_right(Theme::NIGHT, &s).starts_with("Night"));
        st.usb = true;
        e.configure(&mut st, None);
        assert!(e.status_right(Theme::NIGHT, &s).starts_with("no USB"));
        e.usb_live = true;
        assert!(e.status_right(Theme::NIGHT, &s).starts_with("USB"));
    }
}

#[cfg(test)]
mod scroll_bar_tests {
    use super::*;

    #[test]
    fn a_drawer_on_the_right_stops_short_of_the_bars_strip() {
        use sigma_core::settings::Side;
        let mut e = EditorScreen::new(1440, 1920);
        let b = Buffer::new("line\n".repeat(300));
        let page = e.layout(true, &Keyboard::default()).page;
        let (alone, _) = e.scroll_bar_zone(&b, page).unwrap();
        assert_eq!(alone.x + alone.w, page.x + page.w, "at the page edge");
        e.drawer_open = [true, false, false];
        let base = Layout {
            page,
            ..Layout::default()
        };
        let d = e.drawer_rect(&base, Side::Right);
        let (zone, _) = e.scroll_bar_zone(&b, page).unwrap();
        assert_eq!(zone, alone, "the bar does not move");
        let (strip, _) = e.scroll_bar(page, 300, 10).unwrap();
        let edge_gap = page.x + page.w - (strip.x + strip.w);
        assert_eq!(
            strip.x - (d.x + d.w),
            edge_gap,
            "as far from the drawer as from the edge"
        );
        assert!(d.x + d.w < zone.x, "the drawer is off the bar's touch zone");
        // Closed from the grip: the tab stays where the drawer's edge was.
        let tab = e.drawer_tab(&base, Side::Right);
        assert_eq!(tab.x + tab.w, d.x + d.w, "left of the bar and its margin");
        // On the left the drawer never meets the bar: flush with the page.
        e.cfg.drawer = Side::Left;
        assert_eq!(e.drawer_rect(&base, Side::Left).x, page.x);
        assert_eq!(e.scroll_bar_zone(&b, page).unwrap().0, alone);
    }

    #[test]
    fn thumb_tracks_the_note_and_is_pure_black_and_white() {
        let mut e = EditorScreen::new(1440, 1920);
        let page = Rect::new(0, 100, 1440, 1000);
        assert!(e.scroll_bar(page, 10, 20).is_none(), "fits: no bar");
        let (strip, top) = e.scroll_bar(page, 400, 40).unwrap();
        assert_eq!(top.y, strip.y, "at the top");
        assert!(
            top.h >= strip.h / 10 - 1,
            "a tenth of the note, or the minimum"
        );
        e.scroll = 360;
        let (_, end) = e.scroll_bar(page, 400, 40).unwrap();
        assert_eq!(end.y + end.h, strip.y + strip.h, "at the end");
        e.scroll = 180;
        let (_, mid) = e.scroll_bar(page, 400, 40).unwrap();
        assert!(mid.y > top.y && mid.y < end.y);
        for t in [Theme::DAY, Theme::NIGHT] {
            let mut c = Canvas::new(1440, 1920, 128);
            e.draw_scroll_bar(&mut c, page, 400, 40, t);
            for y in strip.y..strip.y + strip.h {
                for x in strip.x..strip.x + strip.w {
                    let v = c.pixels[(y * 1440 + x) as usize];
                    assert!(v == 0 || v == 255 || v == t.paper, "{v} at {x},{y}");
                }
            }
        }
    }
}
