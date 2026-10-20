//! The on-screen keyboard model (brief §7, portrait / landscape Centre).
//!
//! Positional: a key is addressed by row and column and keeps its place when
//! Shift or the symbol layer relabels it.

use crate::buffer::{Buffer, TabKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Letters,
    Symbols,
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
    /// `123` on letters, `ABC` on symbols.
    SwapLayer,
}

/// What the app should do after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    /// The buffer changed: repaint the page and autosave.
    Edited,
    /// Only the keyboard changed (shift / layer): repaint the keys.
    Relabelled,
}

const LETTERS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
const SYMBOLS: [&str; 3] = ["1234567890", "#*-[]()`/", ".,!?:|~>"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keyboard {
    pub layer: Layer,
    pub shift: bool,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self { layer: Layer::Letters, shift: false }
    }
}

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
        c => c.to_ascii_uppercase(),
    }
}

impl Keyboard {
    /// Rows of keys for the current layer, top to bottom.
    #[must_use]
    pub fn rows(&self) -> Vec<Vec<Key>> {
        let src = match self.layer {
            Layer::Letters => LETTERS,
            Layer::Symbols => SYMBOLS,
        };
        let chars = |s: &str| s.chars().map(Key::Char).collect::<Vec<_>>();
        let mut third = vec![Key::Shift];
        third.extend(chars(src[2]));
        third.push(Key::Back);
        vec![
            chars(src[0]),
            chars(src[1]),
            third,
            vec![Key::SwapLayer, Key::Tab, Key::Del, Key::Space, Key::Return],
        ]
    }

    #[must_use]
    pub fn label(&self, key: Key) -> String {
        match key {
            Key::Char(c) => (if self.shift { shifted(c) } else { c }).to_string(),
            Key::Shift => "Shift".into(),
            Key::Back => "Back".into(),
            Key::Del => "Del".into(),
            Key::Tab => "Tab".into(),
            Key::Space => "Space".into(),
            Key::Return => "Return".into(),
            Key::SwapLayer => match self.layer {
                Layer::Letters => "123".into(),
                Layer::Symbols => "ABC".into(),
            },
        }
    }

    /// Key at a position, if any.
    #[must_use]
    pub fn key_at(&self, row: usize, col: usize) -> Option<Key> {
        self.rows().get(row).and_then(|r| r.get(col)).copied()
    }

    /// Applies a key to the focused buffer. Shift releases after one key.
    pub fn press(&mut self, key: Key, buffer: &mut Buffer, tab: TabKind) -> KeyAction {
        let shift = self.shift;
        let action = match key {
            Key::Shift => {
                self.shift = !self.shift;
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
            Key::Char(c) => {
                buffer.insert(&(if shift { shifted(c) } else { c }).to_string());
                KeyAction::Edited
            }
            Key::Back => {
                buffer.backspace();
                KeyAction::Edited
            }
            // Brief: "Del is Delete; with Shift held it is Backspace."
            Key::Del if shift => {
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
                buffer.insert(" ");
                KeyAction::Edited
            }
            Key::Return => {
                buffer.newline();
                KeyAction::Edited
            }
        };
        self.shift = false;
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_brief() {
        let k = Keyboard::default();
        let rows = k.rows();
        let labels: Vec<Vec<String>> =
            rows.iter().map(|r| r.iter().map(|&x| k.label(x)).collect()).collect();
        assert_eq!(labels[0].join(" "), "q w e r t y u i o p");
        assert_eq!(labels[1].join(" "), "a s d f g h j k l");
        assert_eq!(labels[2].join(" "), "Shift z x c v b n m Back");
        assert_eq!(labels[3].join(" "), "123 Tab Del Space Return");
    }

    #[test]
    fn symbol_layer_and_back() {
        let mut k = Keyboard::default();
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
        let mut k = Keyboard::default();
        let mut b = Buffer::new("");
        k.press(Key::Shift, &mut b, TabKind::default());
        k.press(Key::Char('h'), &mut b, TabKind::default());
        k.press(Key::Char('i'), &mut b, TabKind::default());
        assert_eq!(b.text(), "Hi");
    }

    #[test]
    fn del_and_shift_del() {
        let mut k = Keyboard::default();
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
        let mut k = Keyboard::default();
        let mut b = Buffer::new("");
        for key in [Key::Char('a'), Key::Space, Key::Tab, Key::Return] {
            assert_eq!(k.press(key, &mut b, TabKind::Spaces(2)), KeyAction::Edited);
        }
        assert_eq!(b.text(), "a   \n");
    }

    #[test]
    fn positions_are_stable_across_shift() {
        let mut k = Keyboard::default();
        let before = k.rows().iter().map(Vec::len).collect::<Vec<_>>();
        k.shift = true;
        let after = k.rows().iter().map(Vec::len).collect::<Vec<_>>();
        assert_eq!(before, after);
    }
}
