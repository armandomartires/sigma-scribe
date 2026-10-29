//! A small, bounded JSON reader for Syncthing's REST answers.
//!
//! The app has no serde, and the answers are small objects, so this reads
//! only what it needs: nesting is limited to [`MAX_DEPTH`] and the caller
//! limits the size of the text before it gets here. Numbers are `f64`.
//! Anything malformed is `None`, never a panic.

/// Deepest nesting read.
pub const MAX_DEPTH: usize = 24;

/// A JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number.
    Num(f64),
    /// A string, unescaped.
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object, keys in the order read (a repeated key keeps the first).
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Reads `text` as one JSON value, or `None` when it is not one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Json> {
        let mut p = P {
            b: text.as_bytes(),
            i: 0,
        };
        p.ws();
        let v = p.value(0)?;
        p.ws();
        (p.i == p.b.len()).then_some(v)
    }

    /// The member `key` of an object.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(o) => o.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The text of a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A whole, non-negative number.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            // `as` saturates; the range check keeps a huge or fractional
            // number from passing as something it is not.
            Json::Num(n) if *n >= 0.0 && *n < 1.8e19 && n.fract() == 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    /// A boolean.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The items of an array (empty for anything else).
    #[must_use]
    pub fn items(&self) -> &[Json] {
        match self {
            Json::Arr(a) => a,
            _ => &[],
        }
    }

    /// The members of an object (empty for anything else).
    #[must_use]
    pub fn members(&self) -> &[(String, Json)] {
        match self {
            Json::Obj(o) => o,
            _ => &[],
        }
    }
}

impl Json {
    /// A string value.
    #[must_use]
    pub fn string(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// Sets member `key` of an object, replacing it or adding it at the
    /// end. Anything that is not an object becomes one.
    pub fn set(&mut self, key: &str, v: Json) {
        if !matches!(self, Json::Obj(_)) {
            *self = Json::Obj(Vec::new());
        }
        if let Json::Obj(o) = self {
            match o.iter_mut().find(|(k, _)| k == key) {
                Some(slot) => slot.1 = v,
                None => o.push((key.to_owned(), v)),
            }
        }
    }

    /// The value as JSON text.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            Json::Null => "null".to_owned(),
            Json::Bool(b) => b.to_string(),
            Json::Num(n) if n.fract() == 0.0 && n.abs() < 9e15 => format!("{}", *n as i64),
            Json::Num(n) => n.to_string(),
            Json::Str(s) => quote(s),
            Json::Arr(a) => format!(
                "[{}]",
                a.iter().map(Json::render).collect::<Vec<_>>().join(",")
            ),
            Json::Obj(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!("{}:{}", quote(k), v.render()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

/// `s` as a JSON string literal, quotes included.
#[must_use]
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Option<()> {
        (self.b.get(self.i) == Some(&c)).then(|| self.i += 1)
    }

    fn word(&mut self, w: &str, v: Json) -> Option<Json> {
        self.b[self.i..].starts_with(w.as_bytes()).then(|| {
            self.i += w.len();
            v
        })
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > MAX_DEPTH {
            return None;
        }
        match *self.b.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut out: Vec<(String, Json)> = Vec::new();
                self.ws();
                if self.eat(b'}').is_some() {
                    return Some(Json::Obj(out));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    self.eat(b':')?;
                    self.ws();
                    let v = self.value(depth + 1)?;
                    if !out.iter().any(|(e, _)| *e == k) {
                        out.push((k, v));
                    }
                    self.ws();
                    if self.eat(b',').is_some() {
                        continue;
                    }
                    self.eat(b'}')?;
                    return Some(Json::Obj(out));
                }
            }
            b'[' => {
                self.i += 1;
                let mut out = Vec::new();
                self.ws();
                if self.eat(b']').is_some() {
                    return Some(Json::Arr(out));
                }
                loop {
                    self.ws();
                    out.push(self.value(depth + 1)?);
                    self.ws();
                    if self.eat(b',').is_some() {
                        continue;
                    }
                    self.eat(b']')?;
                    return Some(Json::Arr(out));
                }
            }
            b'"' => self.string().map(Json::Str),
            b't' => self.word("true", Json::Bool(true)),
            b'f' => self.word("false", Json::Bool(false)),
            b'n' => self.word("null", Json::Null),
            _ => self.number(),
        }
    }

    fn number(&mut self) -> Option<Json> {
        let start = self.i;
        while matches!(
            self.b.get(self.i),
            Some(b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
        ) {
            self.i += 1;
        }
        std::str::from_utf8(&self.b[start..self.i])
            .ok()?
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(Json::Num)
    }

    fn hex4(&mut self) -> Option<u32> {
        let s = std::str::from_utf8(self.b.get(self.i..self.i + 4)?).ok()?;
        let v = u32::from_str_radix(s, 16).ok()?;
        self.i += 4;
        Some(v)
    }

    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let mut out = Vec::new();
        loop {
            let c = *self.b.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *self.b.get(self.i)?;
                    self.i += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let mut u = self.hex4()?;
                            // A surrogate pair is two escapes.
                            if (0xD800..0xDC00).contains(&u) {
                                self.eat(b'\\')?;
                                self.eat(b'u')?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return None;
                                }
                                u = 0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00);
                            }
                            char::from_u32(u)?
                        }
                        _ => return None,
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c if c < 0x20 => return None,
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_shapes_syncthing_sends() {
        let j = Json::parse(
            r#" {"a": [1, 2.5, -3e2], "b": {"c": null, "d": true}, "e": "x\u00e9\n\"y\"\ud83d\ude00"} "#,
        )
        .unwrap();
        assert_eq!(j.get("a").unwrap().items().len(), 3);
        assert_eq!(j.get("a").unwrap().items()[0].as_u64(), Some(1));
        assert_eq!(j.get("a").unwrap().items()[1].as_u64(), None);
        assert_eq!(j.get("b").unwrap().get("d").unwrap().as_bool(), Some(true));
        assert_eq!(
            j.get("e").unwrap().as_str(),
            Some("x\u{e9}\n\"y\"\u{1f600}")
        );
    }

    #[test]
    fn malformed_text_is_none_and_never_panics() {
        for t in [
            "",
            "{",
            "[1,]",
            "{\"a\"}",
            "\"\\x\"",
            "tru",
            "1 2",
            "\"\\ud800\"",
            "{\"a\":1,}",
            "nan",
            "\"a\nb\"",
            "-",
            "\u{0}",
        ] {
            assert_eq!(Json::parse(t), None, "{t:?}");
        }
        // Too deep.
        let deep = "[".repeat(MAX_DEPTH + 5) + &"]".repeat(MAX_DEPTH + 5);
        assert_eq!(Json::parse(&deep), None);
    }

    #[test]
    fn a_repeated_key_keeps_the_first_and_quote_round_trips() {
        let j = Json::parse(r#"{"k":1,"k":2}"#).unwrap();
        assert_eq!(j.get("k").unwrap().as_u64(), Some(1));
        let s = "a\"b\\c\n\u{1}\u{e9}";
        assert_eq!(Json::parse(&quote(s)).unwrap().as_str(), Some(s));
    }

    #[test]
    fn a_value_renders_and_reads_back() {
        let mut j = Json::parse(r#"{"a": 1, "b": [true, null, 2.5], "c": {"d": "x"}}"#).unwrap();
        j.set("a", Json::Num(7.0));
        j.set("new", Json::string("q\"uote"));
        let text = j.render();
        assert_eq!(Json::parse(&text), Some(j.clone()));
        assert_eq!(j.get("a").unwrap().as_u64(), Some(7));
        assert!(text.starts_with(r#"{"a":7,"b":[true,null,2.5]"#), "{text}");
        let mut n = Json::Null;
        n.set("k", Json::Bool(true));
        assert_eq!(n.render(), r#"{"k":true}"#);
    }
}
