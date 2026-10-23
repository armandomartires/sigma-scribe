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
//! composition exists (then the next character). Vietnamese Telex and VNI
//! (`viet.rs`) re-spell the word being typed instead.

use crate::buffer::Buffer;

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
    Ru,
    Ua,
    Gr,
    /// Vietnamese on the US keys, Telex (v1.7).
    ViTelex,
    /// Vietnamese on the US keys, VNI (v1.7).
    ViVni,
    /// Korean, Dubeolsik (KS X 5002; v1.7.1).
    Ko,
    /// Arabic 101 (v1.8).
    Ar,
    /// Persian, standard (v1.8).
    Fa,
    /// Urdu, phonetic (v1.8).
    Ur,
    /// Hebrew, standard (v1.8).
    He,
    /// Devanagari InScript: Hindi, Marathi, Nepali (v1.9).
    Deva,
    /// Bengali InScript (v1.9).
    Bn,
    /// Tamil InScript (v1.9).
    Ta,
    /// Thai Kedmanee (v1.9).
    Th,
    /// Japanese: romaji to kana, kana to kanji (v1.10).
    Ja,
}

pub const ALL: [KeyLayout; 33] = [
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
    KeyLayout::Ru,
    KeyLayout::Ua,
    KeyLayout::Gr,
    KeyLayout::ViTelex,
    KeyLayout::ViVni,
    KeyLayout::Ko,
    KeyLayout::Ar,
    KeyLayout::Fa,
    KeyLayout::Ur,
    KeyLayout::He,
    KeyLayout::Deva,
    KeyLayout::Bn,
    KeyLayout::Ta,
    KeyLayout::Th,
    KeyLayout::Ja,
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

const KO: Table = Table {
    base: [
        "`1234567890-=",
        "ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔ[]\\",
        "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ;' ",
        "ㅋㅌㅊㅍㅠㅜㅡ,./",
    ],
    shift: [
        "~!@#$%^&*()_+",
        "ㅃㅉㄸㄲㅆㅛㅕㅑㅒㅖ{}|",
        "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ:\" ",
        "ㅋㅌㅊㅍㅠㅜㅡ<>?",
    ],
    // AltGr = the US keys (as Russian, Ukrainian, Greek), for Latin text,
    // Markdown and shortcuts; Ctrl shortcuts use the US letter.
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

// Right-to-left layouts (v1.8). Digits stay ASCII (Markdown lists need
// them); AltGr is the US layer. Arabic: xkb `ara` (Arabic 101); Persian:
// xkb `ir`; Urdu: a phonetic layout (letters by sound, like CRULP);
// Hebrew: xkb `il` (SI-1452). Basic variants, written as data.
const AR: Table = Table {
    base: [
        "ذ1234567890-=",
        "ضصثقفغعهخحجد\\",
        "شسيبلاتنمكط ",
        "ئءؤرﻻىةوزظ",
    ],
    shift: ["ّ!@#$%^&*()_+", "ًٌَُﻹإ‘÷×؛<>|", "ٍِ][ﻷأـ،/:\" ", "~ْ}{ﻵآ’,.؟"],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const FA: Table = Table {
    base: [
        "\u{200c}1234567890-=",
        "ضصثقفغعهخحجچپ",
        "شسیبلاتنمکگ ",
        "ظطزرذدئو./",
    ],
    shift: [
        "~!@#$%^&*()_+",
        "ًٌٍقفغعهخح}{|",
        "ؤئيإأآـة»:\" ",
        "كٓژٰ\u{200c}ٔء<>؟",
    ],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const UR: Table = Table {
    base: [
        "`1234567890-=",
        "قوعرتےءیوپ[]\\",
        "اسدفگہجکل؛' ",
        "زشچطبنم،./",
    ],
    shift: [
        "~!@#$%^&*()_+",
        "قؤٰڑٹۓئیوّ{}|",
        "آصڈفغھضخل:\" ",
        "ذژثظبںم<>؟",
    ],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const HE: Table = Table {
    base: [
        ";1234567890-=",
        "/'קראטוןםפ][\\",
        "שדגכעיחלךף, ",
        "זסבהנמצתץ.",
    ],
    shift: [
        "~!@#$%^&*()_+",
        "/'קראטוןםפ}{|",
        "שדגכעיחלך:\" ",
        "זסבהנמצ<>?",
    ],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

// Indic and Thai layouts (v1.9). InScript (Devanagari, Bengali, Tamil) puts
// vowel signs on the base layer and independent vowels on Shift; consonants
// are on the right hand; the same key positions serve every script. Digits
// stay ASCII and AltGr is the US layer (as for the right-to-left layouts);
// Shift+1..5 carry a few signs. Thai is Kedmanee (xkb `th`). Basic variants.
const DEVA: Table = Table {
    base: ["ॊ1234567890-=", "ौैाीूबहगदजड़ॉ", "ोे्िुपरकतचट ", "ॆंमनवलस,.य"],
    shift: ["ऒऍॅृःऋ^&*()_+", "औऐआईऊभङघधझढञऑ", "ओएअइउफऱखथछठ ", "ऎँणऩऴळशष।ॹ"],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const BENG: Table = Table {
    base: ["`1234567890-=", "ৌৈাীূবহগদজড়ৃ", "োে্িুপরকতচট ", "ৎংমনবলস,.য"],
    shift: ["~!@ৃঃ%^&*()_+", "ঔঐআঈঊভঙঘধঝঢঞঋ", "ওএঅইউফৰখথছঠ ", "ৎঁণনৱলশষ।য়"],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const TAMIL: Table = Table {
    base: ["ொ1234567890-=", "ௌைாீூபஹகதஜடஞ\\", "ோே்ிுபரகதசட ", "ெஃமநவலஸ,.ய"],
    shift: [
        "ஒ!@#ஃ%^&*()_+",
        "ஔஐஆஈஊபஙகதஜடஞ|",
        "ஓஏஅஇஉபறகதசட ",
        "எஃணனழளஶஷ>ய",
    ],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const THAI: Table = Table {
    base: ["_ๅ/-ภถุึคตจขช", "ๆไำพะัีรนยบลฃ", "ฟหกดเ้่าสวง ", "ผปแอิืทมใฝ"],
    shift: ["%+๑๒๓๔ู฿๕๖๗๘๙", "๐\"ฎฑธํ๊ณฯญฐ,ฅ", "ฤฆฏโฌ็๋ษศซ. ", "()ฉฮฺ์?ฒฬฦ"],
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

// Japanese: the US keys feed the romaji input; the key left of 1 is the
// あ / A switch (半 / 全 on a Japanese keyboard); AltGr types the US key.
const JA: Table = Table {
    base: [
        "半1234567890-=",
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
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const RU: Table = Table {
    base: [
        "ё1234567890-=",
        "йцукенгшщзхъ\\",
        "фывапролджэ ",
        "ячсмитьбю.",
    ],
    shift: [
        "Ё!\"№;%:?*()_+",
        "ЙЦУКЕНГШЩЗХЪ/",
        "ФЫВАПРОЛДЖЭ ",
        "ЯЧСМИТЬБЮ,",
    ],
    // AltGr = the US base keys (S16.1-D1): Latin for Markdown, links and
    // shortcuts without leaving the layout; Shift+AltGr gives capitals.
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const UA: Table = Table {
    base: [
        "'1234567890-=",
        "йцукенгшщзхїґ",
        "фівапролджє ",
        "ячсмитьбю.",
    ],
    shift: [
        "₴!\"№;%:?*()_+",
        "ЙЦУКЕНГШЩЗХЇҐ",
        "ФІВАПРОЛДЖЄ ",
        "ЯЧСМИТЬБЮ,",
    ],
    // AltGr = the US base keys (S16.1-D1): Latin for Markdown, links and
    // shortcuts without leaving the layout; Shift+AltGr gives capitals.
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
    ],
};

const GR: Table = Table {
    base: [
        "`1234567890-=",
        ";ςερτυθιοπ[]\\",
        "ασδφγηξκλ\u{301}' ",
        "ζχψωβνμ,./",
    ],
    shift: [
        "~!@#$%^&*()_+",
        ":ΣΕΡΤΥΘΙΟΠ{}|",
        "ΑΣΔΦΓΗΞΚΛ\u{308}\" ",
        "ΖΧΨΩΒΝΜ<>?",
    ],
    // AltGr = the US base keys (S16.1-D1): Latin for Markdown, links and
    // shortcuts without leaving the layout; Shift+AltGr gives capitals.
    altgr: [
        "`1234567890-=",
        "qwertyuiop[]\\",
        "asdfghjkl;' ",
        "zxcvbnm,./",
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
        KeyLayout::Ru => &RU,
        KeyLayout::Ua => &UA,
        KeyLayout::Gr => &GR,
        KeyLayout::ViTelex | KeyLayout::ViVni => &US,
        KeyLayout::Ko => &KO,
        KeyLayout::Ar => &AR,
        KeyLayout::Fa => &FA,
        KeyLayout::Ur => &UR,
        KeyLayout::He => &HE,
        KeyLayout::Deva => &DEVA,
        KeyLayout::Bn => &BENG,
        KeyLayout::Ta => &TAMIL,
        KeyLayout::Th => &THAI,
        KeyLayout::Ja => &JA,
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
            Self::Ru => "ru",
            Self::Ua => "ua",
            Self::Gr => "gr",
            Self::ViTelex => "vi",
            Self::ViVni => "vi-vni",
            Self::Ko => "ko",
            Self::Ar => "ar",
            Self::Fa => "fa",
            Self::Ur => "ur",
            Self::He => "he",
            Self::Deva => "deva",
            Self::Bn => "bn",
            Self::Ta => "ta",
            Self::Th => "th",
            Self::Ja => "ja",
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
            Self::Ru => "Russian",
            Self::Ua => "Ukrainian",
            Self::Gr => "Greek",
            Self::ViTelex => "Vietnamese (Telex)",
            Self::ViVni => "Vietnamese (VNI)",
            Self::Ko => "Korean (Dubeolsik)",
            Self::Ar => "Arabic",
            Self::Fa => "Persian (Farsi)",
            Self::Ur => "Urdu (phonetic)",
            Self::He => "Hebrew",
            Self::Deva => "Hindi, Marathi, Nepali (InScript)",
            Self::Bn => "Bengali (InScript)",
            Self::Ta => "Tamil (InScript)",
            Self::Th => "Thai (Kedmanee)",
            Self::Ja => "Japanese (romaji)",
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
        if altgr && self.latin_altgr() {
            // AltGr is the whole US layer, Shift included (S16.1-D1).
            return KeyLayout::Us.char_for_code(code, shift, false, caps);
        }
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
        let upper = shift != (caps && b.is_lowercase());
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

    /// Vietnamese typing: `Some(true)` for VNI, `Some(false)` for Telex.
    #[must_use]
    pub fn viet(self) -> Option<bool> {
        match self {
            Self::ViTelex => Some(false),
            Self::ViVni => Some(true),
            _ => None,
        }
    }

    /// Non-Latin layouts whose AltGr layer is the US layout (v1.6.1).
    #[must_use]
    pub fn latin_altgr(self) -> bool {
        matches!(
            self,
            Self::Ru
                | Self::Ua
                | Self::Gr
                | Self::Ko
                | Self::Ar
                | Self::Fa
                | Self::Ur
                | Self::He
                | Self::Deva
                | Self::Bn
                | Self::Ta
                | Self::Th
                | Self::Ja
        )
    }

    /// The position of a character (base, Shift or AltGr) — emulator and
    /// tests type through this.
    #[must_use]
    pub fn position_of(self, c: char) -> Option<(usize, bool, bool)> {
        let own = self.position_own(c);
        if own.is_none() && self.latin_altgr() {
            return KeyLayout::Us
                .position_own(c)
                .map(|(i, shift, _)| (i, shift, true));
        }
        own
    }

    fn position_own(self, c: char) -> Option<(usize, bool, bool)> {
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
            // 12 / 11 / 9 letters: the compact board narrows these keys.
            Self::Ru => ["йцукенгшщзхъ", "фывапролджэ", "ячсмитьбюё"],
            Self::Ua => ["йцукенгшщзхї", "фівапролджє", "ячсмитьбюґ"],
            // No tonos key on the compact board (S16.1-D3): Full and USB.
            Self::Gr => ["ςερτυθιοπ", "ασδφγηξκλ", "ζχψωβνμ"],
            // Dubeolsik: 10 / 9 / 7 jamo; Shift makes the doubled ones.
            Self::Ko => [
                "ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔ",
                "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ",
                "ㅋㅌㅊㅍㅠㅜㅡ",
            ],
            // Right-to-left layouts (v1.8): letters only; Full and USB have
            // the whole layout (diacritics, punctuation).
            Self::Ar => ["ضصثقفغعهخحجد", "شسيبلاتنمكط", "ئءؤرىةوزظذ"],
            Self::Fa => ["ضصثقفغعهخحجچ", "شسیبلاتنمکگ", "ظطزرذدئوپژ"],
            Self::Ur => ["قوعرتےءیوپ", "اسدفگہجکل", "زشچطبنمٹڈڑ"],
            Self::He => ["קראטוןםפ", "שדגכעיחלךף", "זסבהנמצתץ"],
            _ => ["qwertyuiop", "asdfghjkl", "zxcvbnm"],
        }
    }

    /// The compact board has room for the letters of this layout (v1.9):
    /// the Indic and Thai layouts have too many keys, so they use the Full
    /// board only.
    #[must_use]
    pub fn has_compact(self) -> bool {
        !matches!(self, Self::Deva | Self::Bn | Self::Ta | Self::Th | Self::Ja)
    }

    /// Japanese typing: romaji to kana, then kanji.
    #[must_use]
    pub fn japanese(self) -> bool {
        self == Self::Ja
    }

    /// Korean typing: jamo become syllables.
    #[must_use]
    pub fn hangul(self) -> bool {
        self == Self::Ko
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
        "aeiouyAEIOUYcCnNsSzZlLrRαεηιουωΑΕΗΙΟΥΩ",
        "áéíóúýÁÉÍÓÚÝćĆńŃśŚźŹĺĹŕŔάέήίόύώΆΈΉΊΌΎΏ",
    ),
    ('\u{302}', "aeiouAEIOU", "âêîôûÂÊÎÔÛ"),
    ('\u{303}', "anoANO", "ãñõÃÑÕ"),
    ('\u{308}', "aeiouyAEIOUYιυΙΥ", "äëïöüÿÄËÏÖÜŸϊϋΪΫ"),
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

/// Accented letter → (dead key, letter), the inverse of [`compose`].
#[must_use]
pub fn decompose(c: char) -> Option<(char, char)> {
    COMPOSE.iter().find_map(|(d, from, to)| {
        let i = to.chars().position(|x| x == c)?;
        Some((*d, from.chars().nth(i)?))
    })
}

/// Dead-key state between keys (the input pipeline's composer, S13), and
/// the keystrokes of the Vietnamese word being typed (v1.7).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Composer {
    pub pending: Option<char>,
    /// Keystrokes of the word in progress (Telex / VNI) and what they show.
    raw: String,
    shown: String,
    /// Where the caret was left, to notice that it moved.
    at: usize,
    /// Japanese input in progress (v1.10).
    ja: JaState,
    /// What `shown` is: 0 nothing, 1 Vietnamese, 2 Korean, 3 Japanese.
    kind: u8,
}

/// The Japanese preedit: the reading so far, the romaji waiting, and the
/// conversion candidates once Space has been pressed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct JaState {
    /// Direct (Latin) input instead of kana.
    direct: bool,
    kana: String,
    pend: String,
    cands: Vec<String>,
    sel: usize,
    converting: bool,
}

impl JaState {
    fn active(&self) -> bool {
        self.converting || !self.kana.is_empty() || !self.pend.is_empty()
    }

    /// The text the preedit shows now.
    fn text(&self) -> String {
        if self.converting {
            self.cands.get(self.sel).cloned().unwrap_or_default()
        } else {
            format!("{}{}", self.kana, self.pend)
        }
    }
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

    /// Types `c` into `buffer` under `lang`: dead keys for the Latin
    /// layouts, a re-spelled word for Telex / VNI. True when the text
    /// changed.
    pub fn type_char(&mut self, buffer: &mut Buffer, lang: KeyLayout, c: char) -> bool {
        if lang.japanese() {
            return self.type_ja(buffer, c);
        }
        let vni = lang.viet();
        let hangul = lang.hangul();
        if vni.is_none() && !hangul {
            let t = self.feed(c);
            if t.is_empty() {
                return false;
            }
            buffer.insert(&t);
            return true;
        }
        let takes = if hangul {
            crate::hangul::is_jamo(c)
        } else {
            c.is_ascii_alphabetic() || (vni == Some(true) && c.is_ascii_digit())
        };
        if !takes {
            self.forget();
            buffer.insert(&self.feed(c));
            return true;
        }
        self.sync(buffer);
        self.raw.push(c);
        self.show(buffer, hangul, vni == Some(true));
        true
    }

    /// Japanese: letters make kana, Space converts and cycles, digits and
    /// the 半 key commit; see `ja.rs`.
    fn type_ja(&mut self, buffer: &mut Buffer, c: char) -> bool {
        self.sync(buffer);
        if c == '半' {
            self.ja_commit();
            self.ja.direct = !self.ja.direct;
            return false;
        }
        if self.ja.direct {
            buffer.insert(&c.to_string());
            return true;
        }
        if c == ' ' {
            if !self.ja.active() {
                buffer.insert(" ");
                return true;
            }
            if self.ja.converting {
                self.ja.sel = (self.ja.sel + 1) % self.ja.cands.len().max(1);
            } else {
                crate::ja::finish(&mut self.ja.kana, &mut self.ja.pend);
                self.ja.cands = crate::ja::convert(&self.ja.kana);
                self.ja.sel = 0;
                self.ja.converting = true;
            }
            self.put_ja(buffer);
            return true;
        }
        if c.is_ascii_digit() || (!c.is_ascii() && !c.is_alphabetic()) {
            self.ja_commit();
            buffer.insert(&c.to_string());
            return true;
        }
        if self.ja.converting {
            self.ja_commit();
        }
        crate::ja::romaji_feed(&mut self.ja.kana, &mut self.ja.pend, c);
        self.put_ja(buffer);
        true
    }

    /// Shows the preedit (or the chosen candidate) in the buffer.
    fn put_ja(&mut self, buffer: &mut Buffer) {
        self.kind = 3;
        let new = self.ja.text();
        self.put(buffer, new);
    }

    /// Leaves the shown text in the buffer and starts afresh.
    fn ja_commit(&mut self) {
        self.ja.kana.clear();
        self.ja.pend.clear();
        self.ja.cands.clear();
        self.ja.sel = 0;
        self.ja.converting = false;
        self.raw.clear();
        self.shown.clear();
    }

    /// Japanese direct (Latin) input is on.
    #[must_use]
    pub fn japanese_direct(&self) -> bool {
        self.ja.direct
    }

    /// Japanese is being composed (reading or candidates on screen).
    #[must_use]
    pub fn composing_ja(&self) -> bool {
        self.ja.active()
    }

    /// The candidates and the chosen one, while converting.
    #[must_use]
    pub fn candidates(&self) -> Option<(&[String], usize)> {
        self.ja
            .converting
            .then_some((&self.ja.cands[..], self.ja.sel))
    }

    /// Chooses candidate `i` and commits it (a tap on the candidate bar).
    pub fn pick(&mut self, buffer: &mut Buffer, i: usize) {
        if self.ja.converting && i < self.ja.cands.len() {
            self.ja.sel = i;
            self.put_ja(buffer);
            self.ja_commit();
        }
    }

    /// Return: commits what is being composed (no newline then). Otherwise
    /// writes a waiting accent and returns false (the caller breaks the line).
    pub fn enter(&mut self, buffer: &mut Buffer) -> bool {
        if self.ja.active() {
            self.sync(buffer);
            crate::ja::finish(&mut self.ja.kana, &mut self.ja.pend);
            if !self.ja.converting {
                self.put_ja(buffer);
            }
            self.ja_commit();
            return true;
        }
        let t = self.flush();
        if !t.is_empty() {
            buffer.insert(&t);
        }
        false
    }

    /// Esc: from the candidates back to the reading, from the reading to
    /// nothing. False when nothing was being composed.
    pub fn escape(&mut self, buffer: &mut Buffer) -> bool {
        if !self.ja.active() {
            return false;
        }
        self.sync(buffer);
        if self.ja.converting {
            self.ja.converting = false;
            self.ja.cands.clear();
        } else {
            self.ja.kana.clear();
            self.ja.pend.clear();
        }
        self.put_ja(buffer);
        if !self.ja.active() {
            self.ja_commit();
        }
        true
    }

    /// The byte range of the buffer that is still being composed (drawn
    /// underlined): the Japanese preedit, or the Korean syllable.
    #[must_use]
    pub fn composing(&self, buffer: &Buffer) -> Option<(usize, usize)> {
        let end = buffer.caret();
        if self.shown.is_empty() || self.at != end || !buffer.text().is_char_boundary(end) {
            return None;
        }
        let start = end.checked_sub(self.shown.len())?;
        match self.kind {
            3 if self.ja.active() => Some((start, end)),
            2 => {
                let last = self.shown.chars().next_back()?;
                Some((end - last.len_utf8(), end))
            }
            _ => None,
        }
    }

    /// Backspace: Korean takes the last jamo back (한 → 하 → ㅎ); anything
    /// else deletes a character as usual.
    pub fn backspace(&mut self, buffer: &mut Buffer, lang: KeyLayout) {
        if self.pending.take().is_some() {
            // A waiting accent is dropped, as when it was written first and
            // then taken back.
            return;
        }
        if lang.japanese() && self.ja.active() {
            self.sync(buffer);
            if self.ja.converting {
                self.ja.converting = false;
                self.ja.cands.clear();
            } else if self.ja.pend.pop().is_none() {
                self.ja.kana.pop();
            }
            self.put_ja(buffer);
            if !self.ja.active() {
                self.ja_commit();
            }
            return;
        }
        if lang.hangul() {
            self.sync(buffer);
            if self.raw.pop().is_some() {
                self.show(buffer, true, false);
                return;
            }
        }
        self.forget();
        buffer.backspace();
    }

    /// Forgets the word in progress unless the buffer still ends with it.
    fn sync(&mut self, buffer: &Buffer) {
        let caret = buffer.caret();
        if buffer.selection().is_some()
            || caret != self.at
            || !buffer.text()[..caret].ends_with(&self.shown)
        {
            self.forget();
        }
    }

    /// Re-spells the word from the keystrokes, rewriting only from the
    /// first letter that changed.
    fn show(&mut self, buffer: &mut Buffer, hangul: bool, vni: bool) {
        self.kind = if hangul { 2 } else { 1 };
        let new = if hangul {
            crate::hangul::compose(&self.raw)
        } else {
            crate::viet::convert(&self.raw, vni)
        };
        self.put(buffer, new);
    }

    /// Makes the buffer show `new` where the word was, from the first
    /// letter that differs.
    fn put(&mut self, buffer: &mut Buffer, new: String) {
        let keep = self
            .shown
            .chars()
            .zip(new.chars())
            .take_while(|(a, b)| a == b)
            .count();
        for _ in keep..self.shown.chars().count() {
            buffer.backspace();
        }
        let tail: String = new.chars().skip(keep).collect();
        buffer.insert(&tail);
        self.shown = new;
        self.at = buffer.caret();
    }

    /// Starts a new word (a space, punctuation, a moved caret).
    pub fn forget(&mut self) {
        self.raw.clear();
        self.shown.clear();
        self.ja_commit();
    }

    /// A non-character key (Enter, arrows…) flushes a waiting accent.
    pub fn flush(&mut self) -> String {
        self.forget();
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
    fn cyrillic_and_greek_keep_latin_on_altgr() {
        // S16.1-D1: AltGr types the US key, Shift+AltGr its capital;
        // Ctrl+Я is Ctrl+Z.
        assert_eq!(
            KeyLayout::Ru.char_for_code(16, false, false, false),
            Some('й')
        );
        assert_eq!(
            KeyLayout::Ru.char_for_code(16, false, true, false),
            Some('q')
        );
        assert_eq!(
            KeyLayout::Ru.char_for_code(16, true, true, false),
            Some('Q')
        );
        assert_eq!(
            KeyLayout::Ru.char_for_code(44, false, false, true),
            Some('Я')
        );
        assert_eq!(KeyLayout::Ru.shortcut_char(44), Some('z'));
        assert_eq!(KeyLayout::Ru.char_for_code(4, true, true, false), Some('#'));
        assert_eq!(KeyLayout::Ru.position_of('#'), Some((3, true, true)));
        assert_eq!(
            KeyLayout::Ua.char_for_code(43, false, false, false),
            Some('ґ')
        );
        assert_eq!(
            KeyLayout::Gr.char_for_code(30, false, false, false),
            Some('α')
        );
        let mut c = Composer::default();
        assert_eq!(c.feed('\u{301}'), "");
        assert_eq!(c.feed('ε'), "έ");
        assert_eq!(c.feed('\u{308}'), "");
        assert_eq!(c.feed('Ι'), "Ϊ");
    }

    #[test]
    fn vietnamese_words_are_respelled_in_the_buffer() {
        let mut b = Buffer::new("");
        let mut c = Composer::default();
        for ch in "Vieetj nguwowif".chars() {
            c.type_char(&mut b, KeyLayout::ViTelex, ch);
        }
        assert_eq!(b.text(), "Việt người");
        // Moving the caret starts a new word: the tone key is a letter.
        b.set_caret(0);
        c.type_char(&mut b, KeyLayout::ViTelex, 'a');
        c.type_char(&mut b, KeyLayout::ViTelex, 's');
        assert_eq!(b.text(), "áViệt người");
        let mut b = Buffer::new("");
        for ch in "to1i ".chars() {
            c.type_char(&mut b, KeyLayout::ViVni, ch);
        }
        assert_eq!(b.text(), "tói ");
        // A Latin layout types the letters as they are.
        let mut b = Buffer::new("");
        c.type_char(&mut b, KeyLayout::Us, 'a');
        c.type_char(&mut b, KeyLayout::Us, 's');
        assert_eq!(b.text(), "as");
    }

    #[test]
    fn japanese_reading_conversion_and_commit() {
        let mut b = Buffer::new("");
        let mut c = Composer::default();
        let l = KeyLayout::Ja;
        for ch in "watashi".chars() {
            c.type_char(&mut b, l, ch);
        }
        assert_eq!(b.text(), "わたし");
        assert!(c.composing_ja() && c.candidates().is_none());
        assert_eq!(c.composing(&b), Some((0, b.text().len())));
        c.type_char(&mut b, l, ' ');
        assert_eq!(b.text(), "私");
        assert_eq!(c.candidates().map(|x| x.1), Some(0));
        c.type_char(&mut b, l, ' ');
        assert_eq!(b.text(), "わたし");
        // Enter commits and is not a newline; the next letter starts anew.
        assert!(c.enter(&mut b));
        assert!(!c.composing_ja());
        for ch in "ha".chars() {
            c.type_char(&mut b, l, ch);
        }
        assert_eq!(b.text(), "わたしは");
        // Backspace takes back a kana, then nothing is composing.
        c.backspace(&mut b, l);
        assert_eq!(b.text(), "わたし");
        assert!(!c.composing_ja());
        assert!(!c.enter(&mut b));
    }

    #[test]
    fn japanese_pick_escape_and_direct_input() {
        let mut b = Buffer::new("");
        let mut c = Composer::default();
        let l = KeyLayout::Ja;
        for ch in "nihongo ".chars() {
            c.type_char(&mut b, l, ch);
        }
        assert_eq!(b.text(), "日本語");
        c.pick(&mut b, 1);
        assert_eq!(b.text(), "にほんご");
        assert!(!c.composing_ja());
        for ch in "kyou ".chars() {
            c.type_char(&mut b, l, ch);
        }
        assert!(c.escape(&mut b));
        assert_eq!(b.text(), "にほんごきょう");
        assert!(c.escape(&mut b));
        assert_eq!(b.text(), "にほんご");
        assert!(!c.escape(&mut b));
        // The 半 key switches to direct input and back.
        c.type_char(&mut b, l, '半');
        assert!(c.japanese_direct());
        for ch in "ab ".chars() {
            c.type_char(&mut b, l, ch);
        }
        assert_eq!(b.text(), "にほんごab ");
        c.type_char(&mut b, l, '半');
        assert!(!c.japanese_direct());
        // Digits and a moved caret end the composition.
        c.type_char(&mut b, l, 'k');
        c.type_char(&mut b, l, 'a');
        c.type_char(&mut b, l, '1');
        assert_eq!(b.text(), "にほんごab か1");
        assert!(!c.composing_ja());
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
