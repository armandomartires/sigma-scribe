//! USB-OTG keyboards (v0.8): find them in `/proc/bus/input/devices` and turn
//! their evdev key codes into `HwKey`s with a US layout. Pure logic, tested
//! on the desktop; the reader threads live in `fbink.rs`.
//!
//! Unverified on the Forma until the first device session: whether the
//! kernel enumerates a keyboard behind a powered hub (OTG host mode, `usbhid`
//! / `hid-generic` built in), and the `eventN` number it gets. Everything
//! here degrades to "no keyboard": the touch screen and buttons are unchanged.

use crate::evdev::{RawEvent, EV_KEY, EV_REL};
use crate::{Event, HwKey, Mods};

/// One `/proc/bus/input/devices` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDev {
    pub name: String,
    /// `eventN` handler number, if any.
    pub event: Option<u32>,
    pub keyboard: bool,
    /// Relative pointer: mouse, touchpad, or the mouse half of a combined
    /// device such as a Logitech Unifying receiver (v0.8.1).
    pub pointer: bool,
}

const KEY_A: u32 = 30;
const KEY_Z: u32 = 44;
const KEY_SPACE: u32 = 57;
const KEY_ENTER: u32 = 28;

/// Parses `/proc/bus/input/devices`. A device is a keyboard when it reports
/// `EV_KEY` and its key bitmap has A, Z, Space and Enter (the Kobo's own
/// gpio keys and touch panel do not).
#[must_use]
pub fn parse_devices(text: &str) -> Vec<InputDev> {
    let mut out = Vec::new();
    for block in text.split("\n\n") {
        let mut name = String::new();
        let mut event = None;
        let mut ev_key = false;
        let mut keys: Vec<u64> = Vec::new();
        let (mut ev_rel, mut rel_xy) = (false, false);
        for line in block.lines() {
            if let Some(n) = line.strip_prefix("N: Name=") {
                name = n.trim().trim_matches('"').to_owned();
            } else if let Some(h) = line.strip_prefix("H: Handlers=") {
                event = h
                    .split_whitespace()
                    .find_map(|w| w.strip_prefix("event").and_then(|n| n.parse().ok()));
            } else if let Some(b) = line.strip_prefix("B: EV=") {
                let v = u64::from_str_radix(b.trim(), 16).unwrap_or(0);
                ev_key = v & (1 << EV_KEY) != 0;
                ev_rel = v & (1 << EV_REL) != 0;
            } else if let Some(b) = line.strip_prefix("B: REL=") {
                // REL_X (0) and REL_Y (1) live in the lowest word.
                rel_xy = b
                    .split_whitespace()
                    .last()
                    .and_then(|w| u64::from_str_radix(w, 16).ok())
                    .is_some_and(|v| v & 0b11 == 0b11);
            } else if let Some(b) = line.strip_prefix("B: KEY=") {
                // Words are printed most significant first; each is one
                // `long` (32 bits on the Kobo, 64 on a desktop). Reading them
                // as 32-bit words is exact on the Kobo; on 64-bit hosts the
                // words are wider, so detect the width from the digit count.
                let words: Vec<&str> = b.split_whitespace().collect();
                let wide = words.iter().any(|w| w.len() > 8);
                keys = words
                    .iter()
                    .rev()
                    .flat_map(|w| {
                        let v = u64::from_str_radix(w, 16).unwrap_or(0);
                        if wide {
                            vec![v & 0xffff_ffff, v >> 32]
                        } else {
                            vec![v]
                        }
                    })
                    .collect();
            }
        }
        if name.is_empty() && event.is_none() {
            continue;
        }
        let has = |code: u32| {
            keys.get((code / 32) as usize)
                .is_some_and(|w| w & (1 << (code % 32)) != 0)
        };
        let keyboard = ev_key && [KEY_A, KEY_Z, KEY_SPACE, KEY_ENTER].iter().all(|&k| has(k));
        out.push(InputDev {
            name,
            event,
            keyboard,
            pointer: ev_rel && rel_xy,
        });
    }
    out
}

/// evdev key codes → characters (US layout): (code, plain, shifted).
const CHARS: &[(u16, char, char)] = &[
    (2, '1', '!'),
    (3, '2', '@'),
    (4, '3', '#'),
    (5, '4', '$'),
    (6, '5', '%'),
    (7, '6', '^'),
    (8, '7', '&'),
    (9, '8', '*'),
    (10, '9', '('),
    (11, '0', ')'),
    (12, '-', '_'),
    (13, '=', '+'),
    (16, 'q', 'Q'),
    (17, 'w', 'W'),
    (18, 'e', 'E'),
    (19, 'r', 'R'),
    (20, 't', 'T'),
    (21, 'y', 'Y'),
    (22, 'u', 'U'),
    (23, 'i', 'I'),
    (24, 'o', 'O'),
    (25, 'p', 'P'),
    (26, '[', '{'),
    (27, ']', '}'),
    (30, 'a', 'A'),
    (31, 's', 'S'),
    (32, 'd', 'D'),
    (33, 'f', 'F'),
    (34, 'g', 'G'),
    (35, 'h', 'H'),
    (36, 'j', 'J'),
    (37, 'k', 'K'),
    (38, 'l', 'L'),
    (39, ';', ':'),
    (40, '\'', '"'),
    (41, '`', '~'),
    (43, '\\', '|'),
    (44, 'z', 'Z'),
    (45, 'x', 'X'),
    (46, 'c', 'C'),
    (47, 'v', 'V'),
    (48, 'b', 'B'),
    (49, 'n', 'N'),
    (50, 'm', 'M'),
    (51, ',', '<'),
    (52, '.', '>'),
    (53, '/', '?'),
    (57, ' ', ' '),
];

const KEY_LEFTCTRL: u16 = 29;
const KEY_RIGHTCTRL: u16 = 97;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_LEFTALT: u16 = 56;
const KEY_RIGHTALT: u16 = 100;
const KEY_CAPSLOCK: u16 = 58;

fn special(code: u16) -> Option<HwKey> {
    Some(match code {
        1 => HwKey::Escape,
        14 => HwKey::Backspace,
        15 => HwKey::Tab,
        28 | 96 => HwKey::Return,
        102 => HwKey::Home,
        103 => HwKey::Up,
        104 => HwKey::PageUp,
        105 => HwKey::Left,
        106 => HwKey::Right,
        107 => HwKey::End,
        108 => HwKey::Down,
        109 => HwKey::PageDown,
        111 => HwKey::Delete,
        _ => return None,
    })
}

/// Modifier state and key mapping for one keyboard.
#[derive(Clone, Debug, Default)]
pub struct KeyMapper {
    lctrl: bool,
    rctrl: bool,
    lshift: bool,
    rshift: bool,
    lalt: bool,
    ralt: bool,
    caps: bool,
}

impl KeyMapper {
    #[must_use]
    pub fn mods(&self) -> Mods {
        Mods {
            ctrl: self.lctrl || self.rctrl,
            shift: self.lshift || self.rshift,
            alt: self.lalt || self.ralt,
        }
    }

    /// Feeds one raw event. Press (1) and auto-repeat (2) produce keys;
    /// releases only update modifiers. Plain and Shift-ed characters arrive
    /// as `Key(Char)`; anything with Ctrl/Alt, and Shift with a non-character
    /// key (Shift+arrows selects), as `Chord`.
    pub fn feed(&mut self, e: RawEvent) -> Option<Event> {
        if e.kind != EV_KEY {
            return None;
        }
        let down = e.value != 0;
        match e.code {
            KEY_LEFTCTRL => self.lctrl = down,
            KEY_RIGHTCTRL => self.rctrl = down,
            KEY_LEFTSHIFT => self.lshift = down,
            KEY_RIGHTSHIFT => self.rshift = down,
            KEY_LEFTALT => self.lalt = down,
            KEY_RIGHTALT => self.ralt = down,
            KEY_CAPSLOCK if e.value == 1 => self.caps = !self.caps,
            _ if down => return self.key(e.code),
            _ => {}
        }
        None
    }

    fn key(&self, code: u16) -> Option<Event> {
        let m = self.mods();
        if let Some(&(_, plain, shifted)) = CHARS.iter().find(|c| c.0 == code) {
            let upper = m.shift != (self.caps && plain.is_ascii_alphabetic());
            let c = if upper { shifted } else { plain };
            return Some(if m.ctrl || m.alt {
                Event::Chord {
                    key: HwKey::Char(plain),
                    mods: m,
                }
            } else {
                Event::Key(HwKey::Char(c))
            });
        }
        let k = special(code)?;
        Some(if m == Mods::default() {
            Event::Key(k)
        } else {
            Event::Chord { key: k, mods: m }
        })
    }
}

/// Characters → raw press/release events (emulator `usb-type`): the inverse
/// of the mapper for printable ASCII.
#[must_use]
pub fn raw_for_char(c: char) -> Vec<RawEvent> {
    let ev = |code, value| RawEvent {
        kind: EV_KEY,
        code,
        value,
    };
    if c == '\n' {
        return vec![ev(28, 1), ev(28, 0)];
    }
    let Some(&(code, plain, _)) = CHARS.iter().find(|x| x.1 == c || x.2 == c) else {
        return Vec::new();
    };
    if c == plain {
        vec![ev(code, 1), ev(code, 0)]
    } else {
        vec![
            ev(KEY_LEFTSHIFT, 1),
            ev(code, 1),
            ev(code, 0),
            ev(KEY_LEFTSHIFT, 0),
        ]
    }
}

/// Key name → evdev code, for the emulator (`usb-key ctrl+f`).
#[must_use]
pub fn code_for_name(name: &str) -> Option<u16> {
    let n = name.to_ascii_lowercase();
    Some(match n.as_str() {
        "ctrl" => KEY_LEFTCTRL,
        "shift" => KEY_LEFTSHIFT,
        "alt" => KEY_LEFTALT,
        "caps" => KEY_CAPSLOCK,
        "esc" => 1,
        "backspace" => 14,
        "tab" => 15,
        "enter" => 28,
        "home" => 102,
        "up" => 103,
        "pgup" => 104,
        "left" => 105,
        "right" => 106,
        "end" => 107,
        "down" => 108,
        "pgdn" => 109,
        "delete" => 111,
        _ => {
            let mut cs = n.chars();
            let c = cs.next()?;
            if cs.next().is_some() {
                return None;
            }
            CHARS.iter().find(|x| x.1 == c)?.0
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROC: &str = "I: Bus=0000 Vendor=0000 Product=0000 Version=0000
N: Name=\"gpio-keys\"
P: Phys=gpio-keys/input0
H: Handlers=kbd event0
B: PROP=0
B: EV=3
B: KEY=180000 0 0 10000000000000 0

I: Bus=0018 Vendor=0000 Product=0000 Version=0000
N: Name=\"cyttsp5_mt\"
H: Handlers=event1
B: PROP=2
B: EV=b
B: KEY=400 0 0 0 0 0 0 0 0 0 0
B: ABS=6e18000 0

I: Bus=0003 Vendor=046d Product=c31c Version=0110
N: Name=\"Logitech USB Keyboard\"
H: Handlers=sysrq kbd leds event3
B: PROP=0
B: EV=120013
B: KEY=1000000000007 ff9f207ac14057ff febeffdfffefffff fffffffffffffffe
";

    #[test]
    fn finds_only_the_real_keyboard() {
        let d = parse_devices(PROC);
        assert_eq!(d.len(), 3);
        assert!(!d[0].keyboard && !d[1].keyboard);
        assert!(d[2].keyboard);
        assert_eq!(d[2].event, Some(3));
        assert_eq!(d[2].name, "Logitech USB Keyboard");
    }

    #[test]
    fn finds_pointers_and_combined_receivers() {
        // A Unifying receiver with hid-logitech-dj: one device per paired
        // product, the K400 Plus being keys + touchpad in one; and the same
        // receiver without the dj driver: separate keyboard and mouse nodes.
        let t = "N: Name=\"Logitech K400 Plus\"
H: Handlers=sysrq kbd leds mouse0 event3
B: EV=12001f
B: KEY=1f0000 0 0 0 0 0 0 0 7 ff9f207ac14057ff febeffdfffefffff fffffffffffffffe
B: REL=1943

N: Name=\"Logitech USB Receiver Mouse\"
H: Handlers=mouse1 event5
B: EV=17
B: KEY=ffff0000 0 0 0 0
B: REL=1943
B: MSC=10

N: Name=\"cyttsp5_mt\"
H: Handlers=event1
B: EV=b
B: KEY=400 0 0 0 0 0 0 0 0 0 0
B: ABS=6e18000 0
";
        let d = parse_devices(t);
        assert!(d[0].keyboard && d[0].pointer, "{:?}", d[0]);
        assert!(!d[1].keyboard && d[1].pointer);
        assert_eq!(d[1].event, Some(5));
        assert!(!d[2].keyboard && !d[2].pointer, "touch is never a pointer");
        assert!(parse_devices(PROC).iter().all(|d| !d.pointer));
    }

    #[test]
    fn keyboard_bitmap_in_32_bit_words() {
        // The same keyboard as printed by a 32-bit kernel (the Kobo).
        let t = "N: Name=\"kb\"\nH: Handlers=kbd event4\nB: EV=120013\nB: KEY=10000 7 ff9f207a c14057ff febeffdf ffefffff ffffffff fffffffe\n";
        let d = parse_devices(t);
        assert!(d[0].keyboard, "{d:?}");
        assert_eq!(parse_devices("")[..], []);
    }

    fn feed(m: &mut KeyMapper, evs: &[(u16, i32)]) -> Vec<Event> {
        evs.iter()
            .filter_map(|&(code, value)| {
                m.feed(RawEvent {
                    kind: EV_KEY,
                    code,
                    value,
                })
            })
            .collect()
    }

    #[test]
    fn maps_shift_caps_ctrl_and_repeat() {
        let mut m = KeyMapper::default();
        let ev = feed(
            &mut m,
            &[
                (30, 1),
                (30, 2),
                (30, 0),
                (42, 1),
                (3, 1),
                (3, 0),
                (42, 0),
                (58, 1),
                (58, 0),
                (31, 1),
                (31, 0),
                (29, 1),
                (33, 1),
                (33, 0),
                (29, 0),
                (42, 1),
                (105, 1),
                (42, 0),
                (102, 1),
            ],
        );
        assert_eq!(
            ev,
            vec![
                Event::Key(HwKey::Char('a')),
                Event::Key(HwKey::Char('a')),
                Event::Key(HwKey::Char('@')),
                Event::Key(HwKey::Char('S')),
                Event::Chord {
                    key: HwKey::Char('f'),
                    mods: Mods {
                        ctrl: true,
                        shift: false,
                        alt: false
                    }
                },
                Event::Chord {
                    key: HwKey::Left,
                    mods: Mods {
                        ctrl: false,
                        shift: true,
                        alt: false
                    }
                },
                Event::Key(HwKey::Home),
            ]
        );
    }

    #[test]
    fn raw_round_trip() {
        let mut m = KeyMapper::default();
        let text = "Hello, World! 1+1=2 \"ok\"";
        let out: String = text
            .chars()
            .flat_map(raw_for_char)
            .filter_map(|e| m.feed(e))
            .map(|e| match e {
                Event::Key(HwKey::Char(c)) => c,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(out, text);
        assert_eq!(code_for_name("F"), Some(33));
        assert_eq!(code_for_name("pgdn"), Some(109));
    }
}
