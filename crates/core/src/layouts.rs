//! Latin keyboard layouts (v1.3, S13) shared by the on-screen Full board
//! and USB keyboards, plus the dead-key composer.
//!
//! Each layout is a table over the 48 printable key positions of a PC
//! keyboard (evdev codes): the number row (TLDE, AE01–AE12), the top row
//! (AD01–AD12) and BKSL, the home row (AC01–AC11), the ISO key left of Z
//! (LSGT, absent on US) and the bottom row (AB01–AB10). Each position has
//! base, Shift and AltGr characters. The tables were written from the xkb
//! `symbols/` files (us, gb, fr, de, es, pt, it — basic variants) and are
//! checked in as data; there is no runtime xkb.
//!
//! Dead keys are stored as Unicode combining marks (U+0300 grave, U+0301
//! acute, U+0302 circumflex, U+0303 tilde, U+0308 diaeresis): the composer
//! joins them with the next letter, or writes the spacing accent when no
//! composition exists (then the next character).

/// A keyboard layout (Options → Layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KeyLayout {
    #[default]
    Us,
    Uk,
    Fr,
    De,
    Es,
    Pt,
    It,
    UsIntl,
    Be,
    Dk,
    No,
    Se,
    Pl,
    Cz,
    Ro,
    Tr,
    Latam,
    Br,
}

pub const ALL: [KeyLayout; 18] = [
    KeyLayout::Us,
    KeyLayout::Uk,
    KeyLayout::Fr,
    KeyLayout::De,
    KeyLayout::Es,
    KeyLayout::Pt,
    KeyLayout::It,
    KeyLayout::UsIntl,
    KeyLayout::Be,
    KeyLayout::Dk,
    KeyLayout::No,
    KeyLayout::Se,
    KeyLayout::Pl,
    KeyLayout::Cz,
    KeyLayout::Ro,
    KeyLayout::Tr,
    KeyLayout::Latam,
    KeyLayout::Br,
];

/// evdev codes of the 48 positions, in table order.
pub const CODES: [u16; 48] = [
    41, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, // TLDE AE01..AE12
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 43, // AD01..AD12 BKSL
    30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 86, // AC01..AC11 LSGT
    44, 45, 46, 47, 48, 49, 50, 51, 52, 53, // AB01..AB10
];

/// Index of the ISO key (LSGT) in [`CODES`].
pub const LSGT: usize = 37;

// Rows: number (13), top (12) + BKSL, home (11) + LSGT, bottom (10).
// A space in an AltGr (or LSGT) slot means "nothing there".
struct Table {
    base: [&'static str; 4],
    shift: [&'static str; 4],
    altgr: [&'static str; 4],
}

const US: Table = Table {
    base: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
    shift: [
        "~!@#$%^&*()_+",
        "QWERTYUIOP{}|",
        "ASDFGHJKL:\" ",
        "ZXCVBNM<>?",
    ],
    altgr: [
        "             ",
        "             ",
        "            ",
        "          ",
    ],
};

const UK: Table = Table {
    base: [
        "`1234567890-=",
        "qwertyuiop[]#",
        "asdfghjkl;'\\",
        "zxcvbnm,./",
    ],
    shift: [
        "¬!\"£$%^&*()_+",
        "QWERTYUIOP{}~",
        "ASDFGHJKL:@|",
        "ZXCVBNM<>?",
    ],
    altgr: [
        "¦   €        ",
        "  é  ú í ó   ",
        "á           ",
        "          ",
    ],
};

const FR: Table = Table {
    base: [
        "²&é\"'(-è_çà)=",
        "azertyuiop\u{302}$*",
        "qsdfghjklmù<",
        "wxcvbn,;:!",
    ],
    shift: [
        "~1234567890°+",
        "AZERTYUIOP\u{308}£µ",
        "QSDFGHJKLM%>",
        "WXCVBN?./§",
    ],
    altgr: [
        "  ~#{[|`\\^@]}",
        "  €        ¤ ",
        "            ",
        "          ",
    ],
};

const DE: Table = Table {
    base: [
        "\u{302}1234567890ß\u{301}",
        "qwertzuiopü+#",
        "asdfghjklöä<",
        "yxcvbnm,.-",
    ],
    shift: [
        "°!\"§$%&/()=?\u{300}",
        "QWERTZUIOPÜ*'",
        "ASDFGHJKLÖÄ>",
        "YXCVBNM;:_",
    ],
    altgr: [
        "  ²³   {[]}\\ ",
        "@ €        ~ ",
        "           |",
        "      µ   ",
    ],
};

const ES: Table = Table {
    base: [
        "º1234567890'¡",
        "qwertyuiop\u{300}+ç",
        "asdfghjklñ\u{301}<",
        "zxcvbnm,.-",
    ],
    shift: [
        "ª!\"·$%&/()=?¿",
        "QWERTYUIOP\u{302}*Ç",
        "ASDFGHJKLÑ\u{308}>",
        "ZXCVBNM;:_",
    ],
    altgr: [
        "\\|@#~€¬      ",
        "  €       []}",
        "          { ",
        "          ",
    ],
};

const PT: Table = Table {
    base: [
        "\\1234567890'«",
        "qwertyuiop+\u{301}\u{303}",
        "asdfghjklçº<",
        "zxcvbnm,.-",
    ],
    shift: [
        "|!\"#$%&/()=?»",
        "QWERTYUIOP*\u{300}\u{302}",
        "ASDFGHJKLÇª>",
        "ZXCVBNM;:_",
    ],
    altgr: [
        "  @£§€ {[]}  ",
        "  €       \u{308}  ",
        "            ",
        "          ",
    ],
};

const IT: Table = Table {
    base: [
        "\\1234567890'ì",
        "qwertyuiopè+ù",
        "asdfghjklòà<",
        "zxcvbnm,.-",
    ],
    shift: [
        "|!\"£$%&/()=?^",
        "QWERTYUIOPé*§",
        "ASDFGHJKLç°>",
        "ZXCVBNM;:_",
    ],
    altgr: [
        "             ",
        "  €       [] ",
        "         @# ",
        "          ",
    ],
};

const USINTL: Table = Table {
    base: [
        "\u{300}1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;\u{301} ",
        "zxcvbnm,./",
    ],
    shift: [
        "\u{303}!@#$%\u{302}&*()_+",
        "QWERTYUIOP{}|",
        "ASDFGHJKL:\u{308} ",
        "ZXCVBNM<>?",
    ],
    altgr: [
        " ¡²³¤€¼½¾‘’¥×",
        "äåé®þüúíóö«»¬",
        "áßð     ø¶´ ",
        "æ ©  ñµç ¿",
    ],
};

const BE: Table = Table {
    base: [
        "²&é\"'(§è!çà)-",
        "azertyuiop\u{302}$µ",
        "qsdfghjklmù<",
        "wxcvbn,;:=",
    ],
    shift: [
        "³1234567890°_",
        "AZERTYUIOP\u{308}*£",
        "QSDFGHJKLM%>",
        "WXCVBN?./+",
    ],
    altgr: [
        " |@#  ^  {}  ",
        "  €       []`",
        "          ´\\",
        "         ~",
    ],
};

const DK: Table = Table {
    base: [
        "½1234567890+\u{301}",
        "qwertyuiopå\u{308}'",
        "asdfghjklæø<",
        "zxcvbnm,.-",
    ],
    shift: [
        "§!\"#¤%&/()=?\u{300}",
        "QWERTYUIOPÅ\u{302}*",
        "ASDFGHJKLÆØ>",
        "ZXCVBNM;:_",
    ],
    altgr: [
        "  @£$€ {[]} |",
        "  €        ~ ",
        "           \\",
        "      µ   ",
    ],
};

const NO: Table = Table {
    base: [
        "|1234567890+\\",
        "qwertyuiopå\u{308}'",
        "asdfghjkløæ<",
        "zxcvbnm,.-",
    ],
    shift: [
        "§!\"#¤%&/()=?\u{300}",
        "QWERTYUIOPÅ\u{302}*",
        "ASDFGHJKLØÆ>",
        "ZXCVBNM;:_",
    ],
    altgr: [
        "  @£$€ {[]} \u{301}",
        "  €        ~ ",
        "           \\",
        "      µ   ",
    ],
};

const SE: Table = Table {
    base: [
        "§1234567890+\u{301}",
        "qwertyuiopå\u{308}'",
        "asdfghjklöä<",
        "zxcvbnm,.-",
    ],
    shift: [
        "½!\"#¤%&/()=?\u{300}",
        "QWERTYUIOPÅ\u{302}*",
        "ASDFGHJKLÖÄ>",
        "ZXCVBNM;:_",
    ],
    altgr: [
        "  @£$€ {[]}\\ ",
        "  €        ~ ",
        "           |",
        "      µ   ",
    ],
};

const PL: Table = Table {
    base: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
    shift: [
        "~!@#$%^&*()_+",
        "QWERTYUIOP{}|",
        "ASDFGHJKL:\" ",
        "ZXCVBNM<>?",
    ],
    altgr: [
        "             ",
        "  ę   € ó    ",
        "ąś      ł   ",
        "żźć  ń    ",
    ],
};

const CZ: Table = Table {
    base: [
        ";+ěščřžýáíé=\u{301}",
        "qwertzuiopú)\u{308}",
        "asdfghjklů§\\",
        "yxcvbnm,.-",
    ],
    shift: [
        "°1234567890%\u{30c}",
        "QWERTZUIOP/('",
        "ASDFGHJKL\"!|",
        "YXCVBNM?:_",
    ],
    altgr: [
        "`!@#$%^&*{}\\ ",
        "\\|€       [] ",
        "            ",
        "       <> ",
    ],
};

const RO: Table = Table {
    base: [
        "„1234567890-=",
        "qwertyuiopăîâ",
        "asdfghjklșț\\",
        "zxcvbnm,./",
    ],
    shift: [
        "”!@#$%^&*()_+",
        "QWERTYUIOPĂÎÂ",
        "ASDFGHJKLȘȚ|",
        "ZXCVBNM;:?",
    ],
    altgr: [
        "             ",
        "  €       []\\",
        "         ;' ",
        "       <> ",
    ],
};

const TR: Table = Table {
    base: [
        "\"1234567890*-",
        "qwertyuıopğü,",
        "asdfghjklşi<",
        "zxcvbnmöç.",
    ],
    shift: [
        "é!'^+%&/()=?_",
        "QWERTYUIOPĞÜ;",
        "ASDFGHJKLŞİ>",
        "ZXCVBNMÖÇ:",
    ],
    altgr: [
        "  £#$½ {[]}\\|",
        "@ €    i   ~`",
        "æß         |",
        "          ",
    ],
};

const LATAM: Table = Table {
    base: [
        "|1234567890'¿",
        "qwertyuiop\u{301}+}",
        "asdfghjklñ{<",
        "zxcvbnm,.-",
    ],
    shift: [
        "°!\"#$%&/()=?¡",
        "QWERTYUIOP\u{308}*]",
        "ASDFGHJKLÑ[>",
        "ZXCVBNM;:_",
    ],
    altgr: [
        "¬            ",
        "@          ~`",
        "          ^ ",
        "          ",
    ],
};

const BR: Table = Table {
    base: [
        "'1234567890-=",
        "qwertyuiop\u{301}[]",
        "asdfghjklç\u{303}\\",
        "zxcvbnm,.;",
    ],
    shift: [
        "\"!@#$%\u{308}&*()_+",
        "QWERTYUIOP\u{300}{}",
        "ASDFGHJKLÇ\u{302}|",
        "ZXCVBNM<>:",
    ],
    altgr: [
        " ¹²³£¢¬     §",
        "/?°        ªº",
        "            ",
        "          ",
    ],
};

fn table(l: KeyLayout) -> &'static Table {
    match l {
        KeyLayout::Us => &US,
        KeyLayout::Uk => &UK,
        KeyLayout::Fr => &FR,
        KeyLayout::De => &DE,
        KeyLayout::Es => &ES,
        KeyLayout::Pt => &PT,
        KeyLayout::It => &IT,
        KeyLayout::UsIntl => &USINTL,
        KeyLayout::Be => &BE,
        KeyLayout::Dk => &DK,
        KeyLayout::No => &NO,
        KeyLayout::Se => &SE,
        KeyLayout::Pl => &PL,
        KeyLayout::Cz => &CZ,
        KeyLayout::Ro => &RO,
        KeyLayout::Tr => &TR,
        KeyLayout::Latam => &LATAM,
        KeyLayout::Br => &BR,
    }
}

fn at(rows: &[&'static str; 4], i: usize) -> Option<char> {
    let (r, j) = match i {
        0..=12 => (0, i),
        13..=25 => (1, i - 13),
        26..=37 => (2, i - 26),
        _ => (3, i - 38),
    };
    rows[r].chars().nth(j).filter(|&c| c != ' ')
}

impl KeyLayout {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Us => "us",
            Self::Uk => "uk",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Es => "es",
            Self::Pt => "pt",
            Self::It => "it",
            Self::UsIntl => "us-intl",
            Self::Be => "be",
            Self::Dk => "dk",
            Self::No => "no",
            Self::Se => "se",
            Self::Pl => "pl",
            Self::Cz => "cz",
            Self::Ro => "ro",
            Self::Tr => "tr",
            Self::Latam => "latam",
            Self::Br => "br",
        }
    }

    /// Label in Options.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Us => "US",
            Self::Uk => "UK",
            Self::Fr => "French AZERTY",
            Self::De => "German QWERTZ",
            Self::Es => "Spanish",
            Self::Pt => "Portuguese",
            Self::It => "Italian",
            Self::UsIntl => "US International",
            Self::Be => "Belgian AZERTY",
            Self::Dk => "Danish",
            Self::No => "Norwegian",
            Self::Se => "Swedish / Finnish",
            Self::Pl => "Polish (programmer)",
            Self::Cz => "Czech QWERTZ",
            Self::Ro => "Romanian",
            Self::Tr => "Turkish Q",
            Self::Latam => "Spanish (Latin America)",
            Self::Br => "Portuguese (Brazil)",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        ALL.into_iter().find(|l| l.name() == s.trim())
    }

    /// The ISO key left of Z exists on this layout.
    #[must_use]
    pub fn has_iso_key(self) -> bool {
        at(&table(self).base, LSGT).is_some()
    }

    /// Characters at position `i` ([`CODES`] order): base, Shift, AltGr.
    #[must_use]
    pub fn chars(self, i: usize) -> (Option<char>, Option<char>, Option<char>) {
        let t = table(self);
        (at(&t.base, i), at(&t.shift, i), at(&t.altgr, i))
    }

    /// The character for an evdev code with Shift / AltGr / Caps Lock.
    /// Caps affects letters only.
    #[must_use]
    pub fn char_for_code(self, code: u16, shift: bool, altgr: bool, caps: bool) -> Option<char> {
        let i = CODES.iter().position(|&c| c == code)?;
        let (b, s, a) = self.chars(i);
        if altgr {
            // Shift+AltGr capitalises an AltGr letter (Polish ą → Ą).
            return a.map(|c| {
                if shift && c.is_lowercase() {
                    c.to_uppercase().next().unwrap_or(c)
                } else {
                    c
                }
            });
        }
        let b = b?;
        let upper = shift != (caps && b.is_alphabetic());
        if upper {
            s
        } else {
            Some(b)
        }
    }

    /// The letter a Ctrl shortcut means on this key (Ctrl+Z is Z, wherever
    /// Z sits), or the US base character for non-letters.
    #[must_use]
    pub fn shortcut_char(self, code: u16) -> Option<char> {
        let i = CODES.iter().position(|&c| c == code)?;
        match self.chars(i).0 {
            Some(c) if c.is_ascii_alphabetic() => Some(c),
            _ => KeyLayout::Us.chars(i).0,
        }
    }

    /// The position of a character (base, Shift or AltGr) — emulator and
    /// tests type through this.
    #[must_use]
    pub fn position_of(self, c: char) -> Option<(usize, bool, bool)> {
        (0..CODES.len()).find_map(|i| {
            let (b, s, a) = self.chars(i);
            if b == Some(c) {
                Some((i, false, false))
            } else if s == Some(c) {
                Some((i, true, false))
            } else if a == Some(c) {
                Some((i, false, true))
            } else if a.is_some_and(|a| a.is_lowercase() && a.to_uppercase().eq([c])) {
                // Shift+AltGr capital (Polish Ą).
                Some((i, true, true))
            } else {
                None
            }
        })
    }

    /// Compact letter rows (QWERTY, AZERTY or QWERTZ).
    #[must_use]
    pub fn compact_letters(self) -> [&'static str; 3] {
        match self {
            Self::Fr | Self::Be => ["azertyuiop", "qsdfghjklm", "wxcvbn"],
            Self::De | Self::Cz => ["qwertzuiop", "asdfghjkl", "yxcvbnm"],
            _ => ["qwertyuiop", "asdfghjkl", "zxcvbnm"],
        }
    }
}

/// Is `c` a dead key (a combining accent)?
#[must_use]
pub fn is_dead(c: char) -> bool {
    matches!(
        c,
        '\u{300}' | '\u{301}' | '\u{302}' | '\u{303}' | '\u{308}' | '\u{30A}' | '\u{30C}'
    )
}

/// The spacing form of a dead key, for labels and the fallback.
#[must_use]
pub fn spacing(c: char) -> char {
    match c {
        '\u{300}' => '`',
        '\u{301}' => '´',
        '\u{302}' => '^',
        '\u{303}' => '~',
        '\u{308}' => '¨',
        '\u{30A}' => '°',
        '\u{30C}' => 'ˇ',
        c => c,
    }
}

const COMPOSE: &[(char, &str, &str)] = &[
    ('\u{300}', "aeiouAEIOU", "àèìòùÀÈÌÒÙ"),
    (
        '\u{301}',
        "aeiouyAEIOUYcCnNsSzZlLrR",
        "áéíóúýÁÉÍÓÚÝćĆńŃśŚźŹĺĹŕŔ",
    ),
    ('\u{302}', "aeiouAEIOU", "âêîôûÂÊÎÔÛ"),
    ('\u{303}', "anoANO", "ãñõÃÑÕ"),
    ('\u{308}', "aeiouyAEIOUY", "äëïöüÿÄËÏÖÜŸ"),
    ('\u{30A}', "auAU", "åůÅŮ"),
    ('\u{30C}', "cdenrstzCDENRSTZ", "čďěňřšťžČĎĚŇŘŠŤŽ"),
];

/// Dead key + letter → the accented letter, if it exists.
#[must_use]
pub fn compose(dead: char, c: char) -> Option<char> {
    let (_, from, to) = COMPOSE.iter().find(|x| x.0 == dead)?;
    let i = from.chars().position(|x| x == c)?;
    to.chars().nth(i)
}

/// Dead-key state between keys (the input pipeline's composer, S13).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Composer {
    pub pending: Option<char>,
}

impl Composer {
    /// Feeds one character; returns the text to insert (possibly empty
    /// while a dead key waits).
    pub fn feed(&mut self, c: char) -> String {
        match self.pending.take() {
            None if is_dead(c) => {
                self.pending = Some(c);
                String::new()
            }
            None => c.to_string(),
            // Space after a dead key writes the accent itself.
            Some(d) if c == ' ' => spacing(d).to_string(),
            // Two dead keys: the first is written, the second waits.
            Some(d) if is_dead(c) => {
                self.pending = Some(c);
                spacing(d).to_string()
            }
            Some(d) => match compose(d, c) {
                Some(x) => x.to_string(),
                None => format!("{}{c}", spacing(d)),
            },
        }
    }

    /// A non-character key (Enter, arrows…) flushes a waiting accent.
    pub fn flush(&mut self) -> String {
        self.pending
            .take()
            .map(|d| spacing(d).to_string())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_row_has_the_right_length() {
        for l in ALL {
            let t = table(l);
            for rows in [&t.base, &t.shift, &t.altgr] {
                let n: Vec<usize> = rows.iter().map(|r| r.chars().count()).collect();
                assert_eq!(n, [13, 13, 12, 10], "{l:?}");
            }
            // Every base key has a Shift character.
            for i in 0..CODES.len() {
                let (b, s, _) = l.chars(i);
                assert_eq!(b.is_some(), s.is_some(), "{l:?} position {i}");
            }
        }
    }

    #[test]
    fn letters_sit_where_the_layout_puts_them() {
        // Code 16 is Q on QWERTY, A on AZERTY; 21 is Y / Z on QWERTZ.
        assert_eq!(
            KeyLayout::Fr.char_for_code(16, false, false, false),
            Some('a')
        );
        assert_eq!(
            KeyLayout::De.char_for_code(21, false, false, false),
            Some('z')
        );
        assert_eq!(
            KeyLayout::De.char_for_code(44, true, false, false),
            Some('Y')
        );
        assert_eq!(
            KeyLayout::Es.char_for_code(39, false, false, false),
            Some('ñ')
        );
        assert_eq!(
            KeyLayout::Pt.char_for_code(39, false, false, true),
            Some('Ç')
        );
        assert_eq!(
            KeyLayout::De.char_for_code(12, false, false, false),
            Some('ß')
        );
        assert_eq!(
            KeyLayout::Fr.char_for_code(3, false, false, false),
            Some('é')
        );
        assert_eq!(
            KeyLayout::Fr.char_for_code(3, true, false, false),
            Some('2')
        );
        assert_eq!(
            KeyLayout::De.char_for_code(16, false, true, false),
            Some('@')
        );
        assert_eq!(
            KeyLayout::Fr.char_for_code(18, false, true, false),
            Some('€')
        );
        assert_eq!(KeyLayout::Us.char_for_code(86, false, false, false), None);
        assert!(KeyLayout::Fr.has_iso_key() && !KeyLayout::Us.has_iso_key());
    }

    #[test]
    fn ctrl_shortcuts_keep_their_letter() {
        // Ctrl+Z on AZERTY is the key labelled Z (code 17), not code 44.
        assert_eq!(KeyLayout::Fr.shortcut_char(17), Some('z'));
        assert_eq!(KeyLayout::Fr.shortcut_char(44), Some('w'));
        assert_eq!(KeyLayout::De.shortcut_char(21), Some('z'));
        // Non-letters fall back to the US meaning.
        assert_eq!(KeyLayout::Fr.shortcut_char(3), Some('2'));
    }

    #[test]
    fn dead_keys_compose_or_fall_back() {
        let mut c = Composer::default();
        assert_eq!(c.feed('\u{302}'), "");
        assert_eq!(c.feed('e'), "ê");
        assert_eq!(c.feed('\u{308}'), "");
        assert_eq!(c.feed('u'), "ü");
        assert_eq!(c.feed('\u{301}'), "");
        assert_eq!(c.feed(' '), "´");
        assert_eq!(c.feed('\u{303}'), "");
        assert_eq!(c.feed('x'), "~x");
        assert_eq!(c.feed('\u{300}'), "");
        assert_eq!(c.feed('\u{301}'), "`");
        assert_eq!(c.feed('a'), "á");
        assert_eq!(c.feed('\u{303}'), "");
        assert_eq!(c.flush(), "~");
        assert_eq!(c.flush(), "");
        assert_eq!(c.feed('n'), "n");
    }

    #[test]
    fn names_round_trip_and_every_char_is_typeable() {
        for l in ALL {
            assert_eq!(KeyLayout::parse(l.name()), Some(l));
            for c in "abcdefghijklmnopqrstuvwxyz0123456789".chars() {
                assert!(l.position_of(c).is_some(), "{l:?} cannot type {c}");
            }
        }
    }
}
