//! The on-screen keyboard model (brief §7, portrait / landscape Centre).
//!
//! Two layouts. **Compact** is the brief's four-row board with a symbol
//! layer. **Full** (v0.2.1) is a Windows On-Screen-Keyboard style board:
//! Esc and a number row, brackets, Caps, Enter, both Shifts, Ctrl, Alt,
//! arrows, Home/End/PgUp/PgDn/Del.
//!
//! Positional: a key is addressed by row and column and keeps its place when
//! Shift or the symbol layer relabels it. Widths are in grid units
//! ([`Keyboard::grid_units`] per row) so the renderer can align columns.
//!
//! Modifiers are sticky for touch: Shift is one-shot, a second tap locks it,
//! a third releases; Caps toggles letters only; Ctrl and Alt are one-shot.

use crate::buffer::{Buffer, TabKind};
use crate::layouts::{self, Composer, KeyLayout};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Layer {
    #[default]
    Letters,
    Symbols,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Layout {
    Compact,
    /// Default since v0.2.1 (user decision 2026-10-21).
    #[default]
    Full,
}

impl Layout {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Full => "full",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "compact" => Some(Self::Compact),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Shift,
    Back,
    Del,
    Tab,
    Space,
    Return,
    /// `123` on letters, `ABC` on symbols (Compact).
    SwapLayer,
    /// Switch Full ⇄ Compact.
    SwitchLayout,
    Esc,
    Caps,
    Ctrl,
    Alt,
    Arrow(Dir),
    Home,
    End,
    PgUp,
    PgDn,
}

/// What the app should do after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    /// The buffer changed: repaint the page and autosave.
    Edited,
    /// Only the keyboard changed (shift / layer): repaint the keys.
    Relabelled,
    /// Caret or selection moved; nothing to save.
    Moved,
    /// Needs the screen (visual rows): move one row; `extend` = Shift.
    Vertical {
        down: bool,
        extend: bool,
    },
    /// PgUp / PgDn: one screen.
    Page {
        forward: bool,
        extend: bool,
    },
    /// Layout switched: re-layout and remember the choice.
    Relayout,
    Escape,
    /// Ctrl+F: open the Find bar (v0.6).
    Find,
    /// A Ctrl shortcut with no binding; nothing happened.
    Ignored,
}

const SYMBOLS: [&str; 3] = ["1234567890", "#*-[]()`/", ".,!?:|~>"];

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Keyboard {
    pub layer: Layer,
    pub layout: Layout,
    /// One-shot Shift (or locked when `shift_lock`).
    pub shift: bool,
    pub shift_lock: bool,
    pub caps: bool,
    pub ctrl: bool,
    pub alt: bool,
    /// Internal clipboard for Ctrl+C / X / V.
    pub clipboard: String,
    /// Options → Layout (v1.3): letter positions, Shift / AltGr characters.
    pub lang: KeyLayout,
    /// Dead-key state, shared with USB typing (v1.3).
    pub composer: Composer,
    /// The caret is in a right-to-left row: Left / Right swap (v1.8). The
    /// app sets it before an arrow key.
    pub rtl: bool,
}

/// Compact symbol-layer shift map (brief).
fn shifted(c: char) -> char {
    match c {
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '/' => '?',
        '.' => '>',
        ',' => '<',
        ':' => ';',
        '`' => '~',
        '[' => '{',
        ']' => '}',
        '|' => '\\',
        '?' => '"',
        '!' => '\'',
        '~' => '=',
        '>' => '+',
        // Doubled consonants and vowels of Dubeolsik (v1.7.1).
        'ㅂ' => 'ㅃ',
        'ㅈ' => 'ㅉ',
        'ㄷ' => 'ㄸ',
        'ㄱ' => 'ㄲ',
        'ㅅ' => 'ㅆ',
        'ㅐ' => 'ㅒ',
        'ㅔ' => 'ㅖ',
        // Unicode capitals (Cyrillic, Greek; v1.6.1).
        c => c.to_uppercase().next().unwrap_or(c),
    }
}

fn chars(s: &str) -> Vec<Key> {
    s.chars().map(Key::Char).collect()
}

impl Keyboard {
    #[must_use]
    pub fn with_layout(layout: Layout) -> Self {
        Self {
            layout,
            ..Self::default()
        }
    }

    /// Rows of keys for the current layout and layer, top to bottom.
    #[must_use]
    pub fn rows(&self) -> Vec<Vec<Key>> {
        match self.layout {
            Layout::Full => self.full_rows(),
            Layout::Compact => {
                let src = match self.layer {
                    Layer::Letters => self.lang.compact_letters(),
                    Layer::Symbols => SYMBOLS,
                };
                let mut third = vec![Key::Shift];
                third.extend(chars(src[2]));
                third.push(Key::Back);
                vec![
                    chars(src[0]),
                    chars(src[1]),
                    third,
                    vec![
                        Key::SwapLayer,
                        Key::SwitchLayout,
                        Key::Tab,
                        Key::Del,
                        Key::Space,
                        Key::Return,
                    ],
                ]
            }
        }
    }

    fn full_rows(&self) -> Vec<Vec<Key>> {
        let base = |r: std::ops::Range<usize>| -> Vec<Key> {
            r.filter_map(|i| self.lang.chars(i).0)
                .map(Key::Char)
                .collect()
        };
        let mut r0 = vec![Key::Esc];
        r0.extend(base(0..13));
        r0.push(Key::Back);
        let mut r1 = vec![Key::Tab];
        r1.extend(base(13..26));
        r1.push(Key::Del);
        let mut r2 = vec![Key::Caps];
        r2.extend(base(26..37));
        r2.extend([Key::Return, Key::PgUp]);
        let mut r3 = vec![Key::Shift];
        r3.extend(base(layouts::LSGT..layouts::LSGT + 1));
        r3.extend(base(38..48));
        r3.extend([Key::Shift, Key::Arrow(Dir::Up), Key::PgDn]);
        let r4 = vec![
            Key::Ctrl,
            Key::Alt,
            Key::SwitchLayout,
            Key::Space,
            Key::Home,
            Key::End,
            Key::Arrow(Dir::Left),
            Key::Arrow(Dir::Down),
            Key::Arrow(Dir::Right),
        ];
        vec![r0, r1, r2, r3, r4]
    }

    /// Row width in units (every row adds up to this).
    #[must_use]
    pub fn grid_units(&self) -> f32 {
        match self.layout {
            Layout::Compact => 10.0,
            Layout::Full => 16.0,
        }
    }

    /// Width of a key in grid units; `col` disambiguates the two Shifts.
    #[must_use]
    pub fn units(&self, key: Key, row: usize, col: usize) -> f32 {
        match self.layout {
            Layout::Compact => match key {
                Key::Shift | Key::Back => 1.5,
                Key::Space => 3.5,
                Key::Return => 1.5,
                Key::SwapLayer | Key::SwitchLayout | Key::Tab | Key::Del => 1.25,
                // Rows with more letters than fit (Cyrillic) share the row.
                Key::Char(_) => {
                    let n = self.rows().get(row).map_or(0, |r| {
                        r.iter().filter(|k| matches!(k, Key::Char(_))).count()
                    }) as f32;
                    let room = if row == 2 { 7.0 } else { 10.0 };
                    if n > room {
                        room / n
                    } else {
                        1.0
                    }
                }
                _ => 1.0,
            },
            Layout::Full => match (key, row) {
                (Key::Back, _) => 2.0,
                (Key::Tab, _) => 1.5,
                (Key::Char(_), 1) if col == 13 => 1.5,
                (Key::Caps, _) => 1.75,
                (Key::Return, _) => 2.25,
                (Key::Shift, _) if col == 0 && self.lang.has_iso_key() => 1.25,
                (Key::Shift, _) if col == 0 => 2.25,
                (Key::Shift, _) => 1.75,
                (Key::Ctrl | Key::Alt | Key::SwitchLayout, _) => 1.5,
                (Key::Space, _) => 6.5,
                _ => 1.0,
            },
        }
    }

    fn shift_on(&self) -> bool {
        self.shift || self.shift_lock
    }

    #[must_use]
    pub fn label(&self, key: Key) -> String {
        match key {
            Key::Char(c) if self.layout == Layout::Full => {
                let ch = self.char_for(c);
                if ch == '半' {
                    // The あ / A switch shows the mode it is in (v1.10).
                    return if self.composer.japanese_direct() {
                        "A"
                    } else {
                        "あ"
                    }
                    .into();
                }
                if ch == '中' {
                    // The 中 / A switch of the Chinese layouts (v1.11).
                    return if self.composer.chinese_direct() {
                        "A"
                    } else {
                        "中"
                    }
                    .into();
                }
                layouts::spacing(ch).to_string()
            }
            Key::Char(c) => (if self.shift_on() { shifted(c) } else { c }).to_string(),
            Key::Shift => "Shift".into(),
            Key::Back => "Back".into(),
            Key::Del => "Del".into(),
            Key::Tab => "Tab".into(),
            Key::Space => "Space".into(),
            Key::Return if self.layout == Layout::Full => "Enter".into(),
            Key::Return => "Return".into(),
            Key::SwapLayer => match self.layer {
                Layer::Letters => "123".into(),
                Layer::Symbols => "ABC".into(),
            },
            Key::SwitchLayout => match self.layout {
                Layout::Compact => "Full".into(),
                Layout::Full => "Compact".into(),
            },
            Key::Esc => "Esc".into(),
            Key::Caps => "Caps".into(),
            Key::Ctrl => "Ctrl".into(),
            Key::Alt => "Alt".into(),
            Key::Arrow(Dir::Left) => "\u{2190}".into(),
            Key::Arrow(Dir::Up) => "\u{2191}".into(),
            Key::Arrow(Dir::Right) => "\u{2192}".into(),
            Key::Arrow(Dir::Down) => "\u{2193}".into(),
            Key::Home => "Home".into(),
            Key::End => "End".into(),
            Key::PgUp => "PgUp".into(),
            Key::PgDn => "PgDn".into(),
        }
    }

    /// Modifier keys drawn inverted while active.
    #[must_use]
    pub fn is_active(&self, key: Key) -> bool {
        match key {
            Key::Shift => self.shift_on(),
            Key::Caps => self.caps,
            Key::Ctrl => self.ctrl,
            Key::Alt => self.alt,
            _ => false,
        }
    }

    /// Key at a position, if any.
    #[must_use]
    pub fn key_at(&self, row: usize, col: usize) -> Option<Key> {
        self.rows().get(row).and_then(|r| r.get(col)).copied()
    }

    fn release(&mut self) {
        self.shift = false;
        self.ctrl = false;
        self.alt = false;
    }

    /// The character a Full key types now: its layout position with
    /// Shift / Caps, or the AltGr character while Alt is on (v1.3).
    fn char_for(&self, c: char) -> char {
        match self.layout {
            Layout::Full => {
                // Only a key's own base character is remapped; anything
                // else (USB text, an already shifted char) passes through.
                let Some(i) = (0..layouts::CODES.len()).find(|&i| self.lang.chars(i).0 == Some(c))
                else {
                    return c;
                };
                let (b, sh, a) = self.lang.chars(i);
                if self.alt && self.lang.latin_altgr() {
                    // US layer with Shift (S16.1-D1).
                    if let Some(u) = layouts::KeyLayout::Us.char_for_code(
                        layouts::CODES[i],
                        self.shift_on(),
                        false,
                        self.caps,
                    ) {
                        return u;
                    }
                }
                if self.alt {
                    if let Some(a) = a {
                        return a;
                    }
                }
                let b = b.unwrap_or(c);
                if self.shift_on() != (self.caps && b.is_lowercase()) {
                    sh.unwrap_or(b)
                } else {
                    b
                }
            }
            Layout::Compact if self.shift_on() => shifted(c),
            Layout::Compact => c,
        }
    }

    /// Applies a key to the focused buffer.
    pub fn press(&mut self, key: Key, buffer: &mut Buffer, tab: TabKind) -> KeyAction {
        // Any key that is not a character or a modifier writes a waiting
        // accent first (v1.3 composer).
        if self.composer.pending.is_some()
            && !matches!(
                key,
                Key::Char(_) | Key::Space | Key::Shift | Key::Caps | Key::Alt | Key::SwapLayer
            )
        {
            let t = self.composer.flush();
            if !t.is_empty() {
                buffer.insert(&t);
            }
        }
        // Anything else that is not typing ends the Japanese composition
        // (Return, Backspace and Esc act on it themselves).
        let phrase_arrow = self.composer.candidates().is_some()
            && matches!(key, Key::Arrow(Dir::Left | Dir::Right))
            && !self.ctrl;
        if self.composer.composing_ja()
            && !phrase_arrow
            && !matches!(
                key,
                Key::Char(_)
                    | Key::Space
                    | Key::Shift
                    | Key::Caps
                    | Key::Alt
                    | Key::SwapLayer
                    | Key::Back
                    | Key::Return
                    | Key::Esc
            )
        {
            self.composer.forget();
        }
        let extend = self.shift_on();
        let ctrl = self.ctrl;
        let action = match key {
            Key::Shift => {
                // off → one-shot → locked → off
                if self.shift_lock {
                    self.shift_lock = false;
                    self.shift = false;
                } else if self.shift {
                    self.shift = false;
                    self.shift_lock = true;
                } else {
                    self.shift = true;
                }
                return KeyAction::Relabelled;
            }
            Key::Caps => {
                self.caps = !self.caps;
                return KeyAction::Relabelled;
            }
            Key::Ctrl => {
                self.ctrl = !self.ctrl;
                return KeyAction::Relabelled;
            }
            Key::Alt => {
                self.alt = !self.alt;
                return KeyAction::Relabelled;
            }
            Key::SwapLayer => {
                self.layer = match self.layer {
                    Layer::Letters => Layer::Symbols,
                    Layer::Symbols => Layer::Letters,
                };
                self.shift = false;
                return KeyAction::Relabelled;
            }
            Key::SwitchLayout if !self.lang.has_compact() => return KeyAction::Ignored,
            Key::SwitchLayout => {
                self.layout = match self.layout {
                    Layout::Compact => Layout::Full,
                    Layout::Full => Layout::Compact,
                };
                self.layer = Layer::Letters;
                self.shift = false;
                self.shift_lock = false;
                self.caps = false;
                self.release();
                return KeyAction::Relayout;
            }
            Key::Char(c) if ctrl => {
                self.composer.forget();
                self.shortcut(c.to_ascii_lowercase(), buffer)
            }
            Key::Char(c) => {
                let ch = self.char_for(c);
                if !self.composer.type_char(buffer, self.lang, ch) {
                    // A dead key waits for the next letter.
                    self.release();
                    return KeyAction::Relabelled;
                }
                KeyAction::Edited
            }
            Key::Back => {
                self.composer.backspace(buffer, self.lang);
                KeyAction::Edited
            }
            // Brief: "Del is Delete; with Shift held it is Backspace" (Compact).
            Key::Del if extend && self.layout == Layout::Compact => {
                buffer.backspace();
                KeyAction::Edited
            }
            Key::Del => {
                buffer.delete_forward();
                KeyAction::Edited
            }
            Key::Tab => {
                buffer.insert_tab(tab);
                KeyAction::Edited
            }
            Key::Space => {
                self.composer.type_char(buffer, self.lang, ' ');
                KeyAction::Edited
            }
            Key::Return => {
                if !self.composer.enter(buffer) {
                    buffer.newline();
                }
                KeyAction::Edited
            }
            Key::Esc if self.composer.escape(buffer) => KeyAction::Edited,
            Key::Esc => KeyAction::Escape,
            // Converting Japanese: the arrows choose the phrase (Shift: its length).
            Key::Arrow(d @ (Dir::Left | Dir::Right)) if phrase_arrow => {
                self.composer.phrase_arrow(buffer, d == Dir::Right, extend);
                KeyAction::Edited
            }
            Key::Arrow(d @ (Dir::Left | Dir::Right)) => {
                // In a right-to-left row (v1.8) the left arrow moves to
                // the next letter, not the previous one.
                let back = (d == Dir::Left) != self.rtl;
                let at = match (ctrl, extend, buffer.selection(), back) {
                    (true, _, _, true) => buffer.word_left(buffer.caret()),
                    (true, _, _, false) => buffer.word_right(buffer.caret()),
                    (false, false, Some((a, _)), true) => a,
                    (false, false, Some((_, b)), false) => b,
                    (_, _, _, true) => buffer.prev_pos(buffer.caret()),
                    (_, _, _, false) => buffer.next_pos(buffer.caret()),
                };
                buffer.move_to(at, extend);
                KeyAction::Moved
            }
            Key::Arrow(d) => KeyAction::Vertical {
                down: d == Dir::Down,
                extend,
            },
            Key::Home => {
                let at = if ctrl {
                    0
                } else {
                    buffer.line_start(buffer.caret())
                };
                buffer.move_to(at, extend);
                KeyAction::Moved
            }
            Key::End => {
                let at = if ctrl {
                    buffer.text().len()
                } else {
                    buffer.line_end(buffer.caret())
                };
                buffer.move_to(at, extend);
                KeyAction::Moved
            }
            Key::PgUp => KeyAction::Page {
                forward: false,
                extend,
            },
            Key::PgDn => KeyAction::Page {
                forward: true,
                extend,
            },
        };
        self.release();
        action
    }

    fn shortcut(&mut self, c: char, buffer: &mut Buffer) -> KeyAction {
        match c {
            'a' => {
                buffer.select_all();
                KeyAction::Moved
            }
            'c' => {
                if let Some(t) = buffer.selected_text() {
                    self.clipboard = t.to_owned();
                }
                KeyAction::Moved
            }
            'x' => {
                if let Some(t) = buffer.selected_text() {
                    self.clipboard = t.to_owned();
                    buffer.backspace();
                    return KeyAction::Edited;
                }
                KeyAction::Moved
            }
            'v' if !self.clipboard.is_empty() => {
                let t = self.clipboard.clone();
                buffer.insert(&t);
                KeyAction::Edited
            }
            'z' => {
                if buffer.undo() {
                    KeyAction::Edited
                } else {
                    KeyAction::Ignored
                }
            }
            'f' => KeyAction::Find,
            'y' => {
                if buffer.redo() {
                    KeyAction::Edited
                } else {
                    KeyAction::Ignored
                }
            }
            _ => KeyAction::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_brief() {
        let k = Keyboard::with_layout(Layout::Compact);
        let rows = k.rows();
        let labels: Vec<Vec<String>> = rows
            .iter()
            .map(|r| r.iter().map(|&x| k.label(x)).collect())
            .collect();
        assert_eq!(labels[0].join(" "), "q w e r t y u i o p");
        assert_eq!(labels[1].join(" "), "a s d f g h j k l");
        assert_eq!(labels[2].join(" "), "Shift z x c v b n m Back");
        assert_eq!(labels[3].join(" "), "123 Full Tab Del Space Return");
    }

    #[test]
    fn symbol_layer_and_back() {
        let mut k = Keyboard::with_layout(Layout::Compact);
        let mut b = Buffer::new("");
        k.press(Key::SwapLayer, &mut b, TabKind::default());
        assert_eq!(k.label(k.key_at(0, 0).unwrap()), "1");
        assert_eq!(k.label(k.key_at(3, 0).unwrap()), "ABC");
        k.press(k.key_at(1, 3).unwrap(), &mut b, TabKind::default());
        assert_eq!(b.text(), "[");
        k.press(Key::SwapLayer, &mut b, TabKind::default());
        assert_eq!(k.layer, Layer::Letters);
    }

    #[test]
    fn shift_is_one_shot() {
        let mut k = Keyboard::with_layout(Layout::Compact);
        let mut b = Buffer::new("");
        k.press(Key::Shift, &mut b, TabKind::default());
        k.press(Key::Char('h'), &mut b, TabKind::default());
        k.press(Key::Char('i'), &mut b, TabKind::default());
        assert_eq!(b.text(), "Hi");
    }

    #[test]
    fn del_and_shift_del() {
        let mut k = Keyboard::with_layout(Layout::Compact);
        let mut b = Buffer::new("abc");
        b.set_caret(1);
        k.press(Key::Del, &mut b, TabKind::default());
        assert_eq!(b.text(), "ac");
        k.press(Key::Shift, &mut b, TabKind::default());
        k.press(Key::Del, &mut b, TabKind::default());
        assert_eq!(b.text(), "c");
    }

    #[test]
    fn return_tab_space() {
        let mut k = Keyboard::with_layout(Layout::Compact);
        let mut b = Buffer::new("");
        for key in [Key::Char('a'), Key::Space, Key::Tab, Key::Return] {
            assert_eq!(k.press(key, &mut b, TabKind::Spaces(2)), KeyAction::Edited);
        }
        assert_eq!(b.text(), "a   \n");
    }

    #[test]
    fn positions_are_stable_across_shift() {
        let mut k = Keyboard::with_layout(Layout::Compact);
        let before = k.rows().iter().map(Vec::len).collect::<Vec<_>>();
        k.shift = true;
        k.caps = true;
        let after = k.rows().iter().map(Vec::len).collect::<Vec<_>>();
        assert_eq!(before, after);
    }

    fn full() -> (Keyboard, Buffer) {
        (Keyboard::with_layout(Layout::Full), Buffer::new(""))
    }

    fn labels(k: &Keyboard) -> Vec<String> {
        k.rows()
            .iter()
            .map(|r| r.iter().map(|&x| k.label(x)).collect::<Vec<_>>().join(" "))
            .collect()
    }

    #[test]
    fn full_layout_matches_windows_osk_and_rows_fill_the_grid() {
        let k = Keyboard::with_layout(Layout::Full);
        assert_eq!(
            labels(&k),
            [
                "Esc ` 1 2 3 4 5 6 7 8 9 0 - = Back",
                "Tab q w e r t y u i o p [ ] \\ Del",
                "Caps a s d f g h j k l ; ' Enter PgUp",
                "Shift z x c v b n m , . / Shift \u{2191} PgDn",
                "Ctrl Alt Compact Space Home End \u{2190} \u{2193} \u{2192}",
            ]
        );
        for (r, row) in k.rows().iter().enumerate() {
            let sum: f32 = row
                .iter()
                .enumerate()
                .map(|(c, &key)| k.units(key, r, c))
                .sum();
            assert!((sum - k.grid_units()).abs() < 1e-3, "row {r} = {sum}");
        }
        for lang in crate::layouts::ALL {
            let mut c = Keyboard::with_layout(Layout::Compact);
            c.lang = lang;
            for (r, row) in c.rows().iter().enumerate() {
                let sum: f32 = row
                    .iter()
                    .enumerate()
                    .map(|(i, &key)| c.units(key, r, i))
                    .sum();
                assert!(
                    sum <= c.grid_units() + 1e-3,
                    "{lang:?} compact row {r} = {sum}"
                );
            }
        }
    }

    #[test]
    fn full_shift_one_shot_double_tap_lock_and_caps() {
        let (mut k, mut b) = full();
        let t = TabKind::default();
        k.press(Key::Shift, &mut b, t);
        assert_eq!(labels(&k)[0], "Esc ~ ! @ # $ % ^ & * ( ) _ + Back");
        for c in "1ab".chars() {
            k.press(Key::Char(c), &mut b, t);
        }
        assert_eq!(b.text(), "!ab", "one-shot");
        k.press(Key::Shift, &mut b, t);
        k.press(Key::Shift, &mut b, t);
        assert!(k.shift_lock);
        for c in "ab;".chars() {
            k.press(Key::Char(c), &mut b, t);
        }
        assert_eq!(b.text(), "!abAB:", "locked");
        k.press(Key::Shift, &mut b, t);
        assert!(!k.is_active(Key::Shift));
        k.press(Key::Caps, &mut b, t);
        for c in "a1".chars() {
            k.press(Key::Char(c), &mut b, t);
        }
        k.press(Key::Shift, &mut b, t);
        k.press(Key::Char('a'), &mut b, t);
        assert_eq!(
            b.text(),
            "!abAB:A1a",
            "caps letters only; shift inverts caps"
        );
        assert!(k.caps && k.is_active(Key::Caps));
    }

    #[test]
    fn ctrl_shortcuts_clipboard_undo() {
        let (mut k, mut b) = full();
        let t = TabKind::default();
        for c in "hello world".chars() {
            k.press(if c == ' ' { Key::Space } else { Key::Char(c) }, &mut b, t);
        }
        // Shift+Ctrl+Left selects the last word.
        k.press(Key::Shift, &mut b, t);
        k.press(Key::Ctrl, &mut b, t);
        assert_eq!(k.press(Key::Arrow(Dir::Left), &mut b, t), KeyAction::Moved);
        assert_eq!(b.selected_text(), Some("world"));
        assert!(!k.ctrl && !k.shift, "modifiers are one-shot");
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Char('x'), &mut b, t);
        assert_eq!((b.text(), k.clipboard.as_str()), ("hello ", "world"));
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Char('v'), &mut b, t);
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Char('v'), &mut b, t);
        assert_eq!(b.text(), "hello worldworld");
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Char('z'), &mut b, t);
        assert_eq!(b.text(), "hello world");
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Char('y'), &mut b, t);
        assert_eq!(b.text(), "hello worldworld");
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Char('a'), &mut b, t);
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Char('c'), &mut b, t);
        assert_eq!(k.clipboard, "hello worldworld");
        k.press(Key::Ctrl, &mut b, t);
        assert_eq!(k.press(Key::Char('q'), &mut b, t), KeyAction::Ignored);
        assert_eq!(b.text(), "hello worldworld");
    }

    #[test]
    fn nav_keys_and_selection() {
        let mut k = Keyboard::with_layout(Layout::Full);
        let t = TabKind::default();
        let mut b = Buffer::new("ab\ncd");
        k.press(Key::Home, &mut b, t);
        assert_eq!(b.caret(), 3);
        k.press(Key::Shift, &mut b, t);
        k.press(Key::End, &mut b, t);
        assert_eq!(b.selected_text(), Some("cd"));
        k.press(Key::Arrow(Dir::Left), &mut b, t);
        assert_eq!(
            (b.caret(), b.selection()),
            (3, None),
            "Left collapses a selection"
        );
        k.press(Key::Shift, &mut b, t);
        k.press(Key::Shift, &mut b, t); // lock: extend repeatedly
        k.press(Key::Arrow(Dir::Right), &mut b, t);
        k.press(Key::Arrow(Dir::Right), &mut b, t);
        assert_eq!(b.selected_text(), Some("cd"));
        assert_eq!(
            k.press(Key::Arrow(Dir::Up), &mut b, t),
            KeyAction::Vertical {
                down: false,
                extend: true
            }
        );
        assert_eq!(
            k.press(Key::PgDn, &mut b, t),
            KeyAction::Page {
                forward: true,
                extend: true
            }
        );
        k.press(Key::Shift, &mut b, t);
        k.press(Key::Ctrl, &mut b, t);
        k.press(Key::Home, &mut b, t);
        assert_eq!(b.caret(), 0);
        k.press(Key::Del, &mut b, t);
        assert_eq!(b.text(), "b\ncd", "Full Del is always forward delete");
        assert_eq!(k.press(Key::Esc, &mut b, t), KeyAction::Escape);
    }

    #[test]
    fn full_is_the_default() {
        assert_eq!(Keyboard::default().layout, Layout::Full);
    }

    #[test]
    fn switch_layout_round_trip() {
        let mut k = Keyboard::with_layout(Layout::Compact);
        let mut b = Buffer::new("");
        k.shift = true;
        assert_eq!(
            k.press(Key::SwitchLayout, &mut b, TabKind::default()),
            KeyAction::Relayout
        );
        assert_eq!((k.layout, k.shift), (Layout::Full, false));
        k.press(Key::SwitchLayout, &mut b, TabKind::default());
        assert_eq!(k.layout, Layout::Compact);
        assert_eq!(Layout::parse("full"), Some(Layout::Full));
        assert_eq!(Layout::parse(Layout::Compact.name()), Some(Layout::Compact));
        assert_eq!(Layout::parse("x"), None);
    }
}
