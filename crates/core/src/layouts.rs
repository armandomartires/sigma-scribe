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
    /// Chinese pinyin, Simplified characters (v1.11).
    ZhHans,
    /// Chinese pinyin, Traditional characters (v1.11).
    ZhHant,
}

pub const ALL: [KeyLayout; 35] = [
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
    KeyLayout::ZhHans,
    KeyLayout::ZhHant,
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

// Chinese pinyin: the US keys feed the pinyin input; the key left of 1 is the
// 中 / A switch; AltGr types the US key.
const ZH: Table = Table {
    base: [
        "中1234567890-=",
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
        KeyLayout::ZhHans | KeyLayout::ZhHant => &ZH,
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
            Self::ZhHans => "zh-hans",
            Self::ZhHant => "zh-hant",
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
            Self::ZhHans => "Chinese Simplified (pinyin)",
            Self::ZhHant => "Chinese Traditional (pinyin)",
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
                | Self::ZhHans
                | Self::ZhHant
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

    /// Whether typing on this layout composes text (re-spelling, syllables,
    /// conjuncts, candidates), so the Insert key's overwrite does not apply.
    #[must_use]
    pub fn composes(self) -> bool {
        matches!(
            self,
            Self::ViTelex
                | Self::ViVni
                | Self::Ko
                | Self::Ja
                | Self::ZhHans
                | Self::ZhHant
                | Self::Deva
                | Self::Bn
                | Self::Ta
                | Self::Th
        )
    }

    /// The compact board has room for the letters of this layout (v1.9):
    /// the Indic and Thai layouts have too many keys, so they use the Full
    /// board only.
    #[must_use]
    pub fn has_compact(self) -> bool {
        !matches!(
            self,
            Self::Deva | Self::Bn | Self::Ta | Self::Th | Self::Ja | Self::ZhHans | Self::ZhHant
        )
    }

    /// Japanese typing: romaji to kana, then kanji.
    #[must_use]
    pub fn japanese(self) -> bool {
        self == Self::Ja
    }

    /// Chinese pinyin typing, and which characters it writes.
    #[must_use]
    pub fn chinese(self) -> Option<crate::zh::Script> {
        match self {
            Self::ZhHans => Some(crate::zh::Script::Hans),
            Self::ZhHant => Some(crate::zh::Script::Hant),
            _ => None,
        }
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
    /// Chinese input in progress (v1.11).
    zh: ZhState,
    /// What `shown` is: 0 nothing, 1 Vietnamese, 2 Korean, 3 Japanese.
    kind: u8,
}

/// The Japanese preedit: the reading so far, the romaji waiting, and the
/// phrases of the sentence once Space has been pressed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct JaState {
    /// Direct (Latin) input instead of kana.
    direct: bool,
    kana: String,
    pend: String,
    /// The phrases while converting (empty before), and the one being chosen.
    segs: Vec<Seg>,
    cur: usize,
}

/// A phrase being converted: its reading, candidates and the chosen one.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seg {
    reading: String,
    cands: Vec<String>,
    sel: usize,
}

/// The Chinese preedit: the pinyin typed, its candidates, the chosen one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ZhState {
    /// Direct (Latin) input instead of pinyin.
    direct: bool,
    input: String,
    cands: Vec<crate::zh::Cand>,
    texts: Vec<String>,
    sel: usize,
    script: Option<crate::zh::Script>,
}

impl ZhState {
    fn active(&self) -> bool {
        !self.input.is_empty()
    }

    fn refresh(&mut self) {
        let script = self.script.unwrap_or(crate::zh::Script::Hans);
        self.cands = crate::zh::candidates(script, &self.input);
        self.texts = self.cands.iter().map(|c| c.text.clone()).collect();
        self.sel = 0;
    }

    /// Forgets the input (the mode stays).
    fn clear(&mut self) {
        self.input.clear();
        self.cands.clear();
        self.texts.clear();
        self.sel = 0;
    }
}

/// The full-width mark for a typed punctuation key.
fn zh_punct(c: char) -> char {
    match c {
        ',' => '，',
        '.' => '。',
        '?' => '？',
        '!' => '！',
        ':' => '：',
        ';' => '；',
        '(' => '（',
        ')' => '）',
        '[' => '【',
        ']' => '】',
        '\\' => '、',
        '<' => '《',
        '>' => '》',
        _ => c,
    }
}

impl From<crate::ja::Phrase> for Seg {
    fn from(p: crate::ja::Phrase) -> Self {
        Self {
            reading: p.reading,
            cands: p.cands,
            sel: 0,
        }
    }
}

impl JaState {
    fn converting(&self) -> bool {
        !self.segs.is_empty()
    }

    fn active(&self) -> bool {
        self.converting() || !self.kana.is_empty() || !self.pend.is_empty()
    }

    /// The text the preedit shows now.
    fn text(&self) -> String {
        if self.converting() {
            self.segs
                .iter()
                .map(|s| {
                    s.cands
                        .get(s.sel)
                        .map_or(s.reading.as_str(), String::as_str)
                })
                .collect()
        } else {
            format!("{}{}", self.kana, self.pend)
        }
    }

    /// Back to the reading (Esc, Backspace): the phrases' readings joined.
    fn unconvert(&mut self) {
        self.kana = self.segs.iter().map(|s| s.reading.as_str()).collect();
        self.segs.clear();
        self.cur = 0;
    }

    /// Moves the boundary after the current phrase by `by` kana (Shift +
    /// arrows): the current phrase and what follows are converted again.
    fn resize(&mut self, by: isize) {
        let rest: Vec<char> = self.segs[self.cur..]
            .iter()
            .flat_map(|s| s.reading.chars())
            .collect();
        let len = self.segs[self.cur].reading.chars().count();
        let new = len.saturating_add_signed(by).clamp(1, rest.len());
        if new == len {
            return;
        }
        let head: String = rest[..new].iter().collect();
        let tail: String = rest[new..].iter().collect();
        self.segs.truncate(self.cur);
        self.segs.push(crate::ja::phrase(&head).into());
        self.segs
            .extend(crate::ja::phrases(&tail).into_iter().map(Seg::from));
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
        if let Some(script) = lang.chinese() {
            return self.type_zh(buffer, c, script);
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
            if self.ja.converting() {
                let s = &mut self.ja.segs[self.ja.cur];
                s.sel = (s.sel + 1) % s.cands.len().max(1);
            } else {
                crate::ja::finish(&mut self.ja.kana, &mut self.ja.pend);
                self.ja.segs = crate::ja::phrases(&self.ja.kana)
                    .into_iter()
                    .map(Seg::from)
                    .collect();
                self.ja.kana.clear();
                self.ja.cur = 0;
            }
            self.put_ja(buffer);
            return true;
        }
        if c.is_ascii_digit() || (!c.is_ascii() && !c.is_alphabetic()) {
            self.ja_commit();
            buffer.insert(&c.to_string());
            return true;
        }
        if self.ja.converting() {
            self.ja_commit();
        }
        crate::ja::romaji_feed(&mut self.ja.kana, &mut self.ja.pend, c);
        self.put_ja(buffer);
        true
    }

    /// Chinese: letters are pinyin with live candidates; Space writes the
    /// chosen one, 1–9 pick, punctuation writes the first and itself in
    /// full width; the 中 key switches to direct input.
    fn type_zh(&mut self, buffer: &mut Buffer, c: char, script: crate::zh::Script) -> bool {
        self.sync(buffer);
        self.zh.script = Some(script);
        if c == '中' {
            self.zh_commit_raw();
            self.zh.direct = !self.zh.direct;
            return false;
        }
        if self.zh.direct {
            buffer.insert(&c.to_string());
            return true;
        }
        let active = self.zh.active();
        if c.is_ascii_lowercase() || (c == '\'' && active) {
            self.zh.input.push(c);
            self.zh.refresh();
            self.put_zh(buffer);
            return true;
        }
        if c == ' ' && active {
            self.zh_pick(buffer, self.zh.sel);
            return true;
        }
        if c.is_ascii_digit() && c != '0' && active && !self.zh.cands.is_empty() {
            let i = c as usize - '1' as usize;
            if i < self.zh.cands.len().min(9) {
                self.zh_pick(buffer, i);
            }
            return true;
        }
        // Anything else writes what is chosen first.
        let after_digit = !active
            && buffer.text()[..buffer.caret()]
                .chars()
                .next_back()
                .is_some_and(|p| p.is_ascii_digit());
        if active {
            if self.zh.cands.is_empty() {
                self.zh_commit_raw();
            } else {
                self.zh_pick_all(buffer);
            }
        }
        let out = if after_digit && matches!(c, '.' | ',' | ':') {
            c
        } else {
            zh_punct(c)
        };
        buffer.insert(&out.to_string());
        true
    }

    /// Shows the typed pinyin as the preedit.
    fn put_zh(&mut self, buffer: &mut Buffer) {
        self.kind = 3;
        let new = self.zh.input.clone();
        self.put(buffer, new);
    }

    /// Leaves the typed letters in the buffer and starts afresh.
    fn zh_commit_raw(&mut self) {
        self.zh.clear();
        self.raw.clear();
        self.shown.clear();
    }

    /// Writes candidate `i`; what it does not cover stays as the preedit.
    fn zh_pick(&mut self, buffer: &mut Buffer, i: usize) {
        let Some(c) = self.zh.cands.get(i).cloned() else {
            self.zh_commit_raw();
            return;
        };
        let rest = self.zh.input[c.used.min(self.zh.input.len())..]
            .trim_start_matches('\'')
            .to_string();
        self.put(buffer, String::new());
        buffer.insert(&c.text);
        self.shown.clear();
        self.at = buffer.caret();
        self.zh.input = rest;
        if self.zh.input.is_empty() {
            self.zh_commit_raw();
        } else {
            self.zh.refresh();
            self.put_zh(buffer);
        }
    }

    /// Writes the chosen candidate and goes on picking until all the input
    /// is written (the first one each time).
    fn zh_pick_all(&mut self, buffer: &mut Buffer) {
        let mut first = true;
        while self.zh.active() {
            if self.zh.cands.is_empty() {
                // Letters that spell no syllable are written as typed.
                self.zh_commit_raw();
                break;
            }
            let i = if first { self.zh.sel } else { 0 };
            first = false;
            self.zh_pick(buffer, i);
        }
    }

    /// Shows the preedit (or the chosen candidate) in the buffer.
    fn put_ja(&mut self, buffer: &mut Buffer) {
        self.kind = 3;
        let new = self.ja.text();
        self.put(buffer, new);
    }

    /// Leaves the shown text in the buffer and starts afresh.
    fn ja_commit(&mut self) {
        // A phrase written other than first choice is remembered.
        for s in &self.ja.segs {
            if s.sel > 0 {
                if let Some(c) = s.cands.get(s.sel) {
                    crate::ja::learn(&s.reading, c);
                }
            }
        }
        self.ja.kana.clear();
        self.ja.pend.clear();
        self.ja.segs.clear();
        self.ja.cur = 0;
        self.raw.clear();
        self.shown.clear();
    }

    /// Japanese direct (Latin) input is on.
    #[must_use]
    pub fn japanese_direct(&self) -> bool {
        self.ja.direct
    }

    /// Chinese direct (Latin) input is on.
    #[must_use]
    pub fn chinese_direct(&self) -> bool {
        self.zh.direct
    }

    /// Japanese or Chinese is being composed (reading or candidates on
    /// screen).
    #[must_use]
    pub fn composing_ja(&self) -> bool {
        self.ja.active() || self.zh.active()
    }

    /// The candidates of the current phrase and the chosen one, while
    /// converting.
    #[must_use]
    pub fn candidates(&self) -> Option<(&[String], usize)> {
        if self.zh.active() {
            return (!self.zh.texts.is_empty()).then_some((&self.zh.texts[..], self.zh.sel));
        }
        self.ja.segs.get(self.ja.cur).map(|s| (&s.cands[..], s.sel))
    }

    /// The byte range of the current phrase inside the preedit, when the
    /// sentence has more than one phrase (drawn heavier than the rest).
    #[must_use]
    pub fn current_phrase(&self, buffer: &Buffer) -> Option<(usize, usize)> {
        let (start, end) = self.composing(buffer)?;
        if self.ja.segs.len() < 2 {
            return None;
        }
        let shown = |s: &Seg| s.cands.get(s.sel).map_or(s.reading.len(), String::len);
        let a = start + self.ja.segs[..self.ja.cur].iter().map(shown).sum::<usize>();
        let b = a + shown(&self.ja.segs[self.ja.cur]);
        (b <= end).then_some((a, b))
    }

    /// Chooses candidate `i` of the current phrase (a tap on the candidate
    /// bar); the last phrase commits the sentence, an earlier one moves on
    /// to the next.
    pub fn pick(&mut self, buffer: &mut Buffer, i: usize) {
        if self.zh.active() {
            self.sync(buffer);
            self.zh_pick(buffer, i);
            return;
        }
        let cur = self.ja.cur;
        match self.ja.segs.get_mut(cur) {
            Some(s) if i < s.cands.len() => s.sel = i,
            _ => return,
        }
        self.put_ja(buffer);
        if cur + 1 < self.ja.segs.len() {
            self.ja.cur += 1;
        } else {
            self.ja_commit();
        }
    }

    /// While converting: Left / Right choose the phrase, with `extend`
    /// (Shift) they move the boundary after it. False when not converting.
    pub fn phrase_arrow(&mut self, buffer: &mut Buffer, right: bool, extend: bool) -> bool {
        if self.zh.active() && !self.zh.cands.is_empty() {
            // Chinese: the arrows move the choice along the candidates.
            let n = self.zh.cands.len();
            self.zh.sel = if right {
                (self.zh.sel + 1) % n
            } else {
                (self.zh.sel + n - 1) % n
            };
            return true;
        }
        if !self.ja.converting() {
            return false;
        }
        self.sync(buffer);
        if extend {
            self.ja.resize(if right { 1 } else { -1 });
            self.put_ja(buffer);
        } else if right {
            self.ja.cur = (self.ja.cur + 1).min(self.ja.segs.len() - 1);
        } else {
            self.ja.cur = self.ja.cur.saturating_sub(1);
        }
        true
    }

    /// Return: commits what is being composed (no newline then). Otherwise
    /// writes a waiting accent and returns false (the caller breaks the line).
    pub fn enter(&mut self, buffer: &mut Buffer) -> bool {
        if self.zh.active() {
            // The pinyin as typed stays.
            self.zh_commit_raw();
            return true;
        }
        if self.ja.active() {
            self.sync(buffer);
            crate::ja::finish(&mut self.ja.kana, &mut self.ja.pend);
            if !self.ja.converting() {
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
        if self.zh.active() {
            self.sync(buffer);
            self.put(buffer, String::new());
            self.zh_commit_raw();
            return true;
        }
        if !self.ja.active() {
            return false;
        }
        self.sync(buffer);
        if self.ja.converting() {
            self.ja.unconvert();
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
            3 if self.ja.active() || self.zh.active() => Some((start, end)),
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
        if self.zh.active() {
            self.sync(buffer);
            if self.zh.active() {
                self.zh.input.pop();
                if self.zh.input.is_empty() {
                    self.zh.clear();
                } else {
                    self.zh.refresh();
                }
                self.put_zh(buffer);
                if !self.zh.active() {
                    self.zh_commit_raw();
                }
                return;
            }
        }
        if lang.japanese() && self.ja.active() {
            self.sync(buffer);
            if self.ja.converting() {
                self.ja.unconvert();
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
        self.zh.clear();
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
        assert_ne!(b.text(), "私", "the next candidate");
        // Esc steps back to the reading.
        assert!(c.escape(&mut b));
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
        // The hiragana is the second to last candidate.
        let n = c.candidates().map_or(0, |x| x.0.len());
        c.pick(&mut b, n - 2);
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
    fn japanese_sentences_convert_by_phrase() {
        let mut b = Buffer::new("");
        let mut c = Composer::default();
        let l = KeyLayout::Ja;
        for ch in "watashihagakuseidesu ".chars() {
            c.type_char(&mut b, l, ch);
        }
        assert_eq!(b.text(), "私は学生です");
        // Two phrases; the first is current and drawn heavier.
        let first = "私は".len();
        assert_eq!(c.current_phrase(&b), Some((0, first)));
        assert!(c.phrase_arrow(&mut b, true, false));
        assert_eq!(c.current_phrase(&b), Some((first, b.text().len())));
        // Space cycles the second phrase only.
        c.type_char(&mut b, l, ' ');
        assert!(b.text().starts_with("私は") && b.text() != "私は学生です");
        c.type_char(&mut b, l, ' ');
        c.type_char(&mut b, l, ' ');
        c.type_char(&mut b, l, ' ');
        c.type_char(&mut b, l, ' ');
        assert!(c.phrase_arrow(&mut b, false, false));
        assert_eq!(c.current_phrase(&b).map(|x| x.0), Some(0));
        // Shift + Right takes a kana from the next phrase into this one.
        assert!(c.phrase_arrow(&mut b, true, true));
        assert!(b.text().starts_with("私は"), "{}", b.text());
        // Esc gives back the whole reading; Enter commits.
        assert!(c.escape(&mut b));
        assert_eq!(b.text(), "わたしはがくせいです");
        assert!(!c.phrase_arrow(&mut b, true, false));
    }

    #[test]
    fn a_phrase_chosen_by_hand_is_remembered() {
        let mut b = Buffer::new("");
        let mut c = Composer::default();
        let l = KeyLayout::Ja;
        for ch in "kikaku ".chars() {
            c.type_char(&mut b, l, ch);
        }
        let first = b.text().to_string();
        c.type_char(&mut b, l, ' ');
        let second = b.text().to_string();
        assert_ne!(first, second);
        assert!(c.enter(&mut b));
        let mut b = Buffer::new("");
        for ch in "kikaku ".chars() {
            c.type_char(&mut b, l, ch);
        }
        assert_eq!(b.text(), second);
        crate::ja::forget_learned();
    }

    fn type_str(c: &mut Composer, b: &mut Buffer, l: KeyLayout, s: &str) {
        for ch in s.chars() {
            c.type_char(b, l, ch);
        }
    }

    #[test]
    fn chinese_pinyin_candidates_pick_and_partial_commit() {
        let mut b = Buffer::new("");
        let mut c = Composer::default();
        let l = KeyLayout::ZhHans;
        type_str(&mut c, &mut b, l, "nihao");
        // The pinyin is the underlined preedit; the candidates are live.
        assert_eq!(b.text(), "nihao");
        assert_eq!(c.composing(&b), Some((0, 5)));
        let (cands, sel) = c.candidates().expect("candidates");
        assert_eq!((cands[0].as_str(), sel), ("你好", 0));
        // Space writes the chosen candidate.
        c.type_char(&mut b, l, ' ');
        assert_eq!(b.text(), "你好");
        assert!(!c.composing_ja());
        // A candidate that covers the start leaves the rest typed on.
        type_str(&mut c, &mut b, l, "nihao");
        let i = c
            .candidates()
            .unwrap()
            .0
            .iter()
            .position(|x| x == "你")
            .unwrap();
        c.pick(&mut b, i);
        assert_eq!(b.text(), "你好你hao");
        assert_eq!(
            c.composing(&b)
                .map(|x| b.text()[x.0..x.1].to_string())
                .as_deref(),
            Some("hao")
        );
        // A digit picks by number; the rest of the input is written.
        c.type_char(&mut b, l, '1');
        assert_eq!(b.text(), "你好你好");
        assert!(!c.composing_ja());
    }

    #[test]
    fn chinese_punctuation_enter_escape_backspace_and_direct_input() {
        let mut b = Buffer::new("");
        let mut c = Composer::default();
        let l = KeyLayout::ZhHant;
        // Punctuation writes the chosen candidate, then itself in full width.
        type_str(&mut c, &mut b, l, "zhongguo,");
        assert_eq!(b.text(), "中國，");
        // After a digit, the point stays a point.
        type_str(&mut c, &mut b, l, "3.");
        assert_eq!(b.text(), "中國，3.");
        // Enter keeps the pinyin as typed; Esc takes it away; Backspace a letter.
        type_str(&mut c, &mut b, l, "ab");
        assert!(c.enter(&mut b));
        assert_eq!(b.text(), "中國，3.ab");
        type_str(&mut c, &mut b, l, "xyz");
        assert!(c.candidates().is_none());
        assert!(c.escape(&mut b));
        assert_eq!(b.text(), "中國，3.ab");
        type_str(&mut c, &mut b, l, "wo");
        c.backspace(&mut b, l);
        assert_eq!(b.text(), "中國，3.abw");
        assert!(c.escape(&mut b));
        // The arrows move the choice; the 中 key switches to direct input.
        type_str(&mut c, &mut b, l, "shi");
        let n = c.candidates().map_or(0, |x| x.0.len());
        assert!(n > 2, "{n}");
        assert!(c.phrase_arrow(&mut b, true, false));
        assert_eq!(c.candidates().map(|x| x.1), Some(1));
        let second = c.candidates().unwrap().0[1].clone();
        c.type_char(&mut b, l, ' ');
        assert!(b.text().ends_with(&second), "{}", b.text());
        c.type_char(&mut b, l, '中');
        assert!(c.chinese_direct());
        type_str(&mut c, &mut b, l, "ab ");
        assert!(b.text().ends_with("ab "));
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
