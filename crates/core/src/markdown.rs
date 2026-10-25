//! A small Markdown parser for the preview (brief §4, S3). Line based and
//! dependency free: it understands exactly the syntax the brief lists for
//! v0.3 and treats everything else as text, so no note can fail to render.
//!
//! Blocks: ATX headings `#`–`######`, paragraphs (soft-wrapped lines are
//! joined), `-`/`*`/`+` and `1.`/`1)` lists with nesting by indent, task
//! items `[ ]`/`[x]`, `>` quotes, fenced code ```` ``` ```` / `~~~`, and rules
//! `---` / `***` / `___`. Inline: `**strong**`/`__strong__`, `*em*`/`_em_`,
//! `~~strike~~`, `` `code` ``, `[label](url)`, `[[note]]`/`[[note|label]]`,
//! `![alt](src)` (alt text only in v0.3) and backslash escapes. A delimiter
//! without a closing partner is kept as a literal character.

/// Inline style of a run of text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    /// `**strong**` or `__strong__`.
    pub strong: bool,
    /// `*em*` or `_em_`.
    pub em: bool,
    /// `~~strike~~`.
    pub strike: bool,
    /// `` `code` ``, or a line of a fenced code block.
    pub code: bool,
    /// Part of a `[label](url)` or `[[wiki]]` link, or an image's alt text.
    pub link: Option<LinkKind>,
}

/// What a link run points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// `[label](url)`: a web link.
    Web,
    /// `[[note]]`: shown in v0.3; navigation, backlinks and the graph are S6.
    Wiki,
    /// `![alt](src)`: the alt text, until images are drawn.
    Image,
}

/// A run of text in one [`Style`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    /// The text, with the Markdown syntax removed.
    pub text: String,
    /// How the text is styled.
    pub style: Style,
    /// URL for web links and images, note name for wiki links.
    pub target: Option<String>,
}

/// What a [`Block`] is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockKind {
    /// An ATX heading, with its level (1 to 6).
    Heading(u8),
    /// A paragraph; soft-wrapped lines are joined with a space.
    Paragraph,
    /// A list item. `marker` is `None` for bullets, the number for ordered
    /// items; `task` is `Some(done)` for `- [ ]` / `- [x]`.
    Item {
        /// Nesting level: the item's indent over 2 (a tab counts 4), at most 6.
        depth: u8,
        /// The item's number, or `None` for a bullet.
        marker: Option<u32>,
        /// `Some(done)` for a task item, `None` otherwise.
        task: Option<bool>,
    },
    /// A `>` quote; nested `>>` is shown as one level.
    Quote,
    /// Fenced code: `runs` holds one plain run per source line.
    Code,
    /// A thematic break, such as `---`, `***` or `___`.
    Rule,
}

/// One block of a parsed note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// What the block is.
    pub kind: BlockKind,
    /// The block's text as styled runs (empty for a rule).
    pub runs: Vec<Run>,
    /// 0-based source line where the block starts (keeps the position when
    /// switching between write and preview).
    pub line: usize,
}

/// Parses a whole note.
#[must_use]
pub fn parse(src: &str) -> Vec<Block> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out: Vec<Block> = Vec::new();
    // Text of the paragraph-like block being built (paragraph, quote, item).
    let mut open: Option<(BlockKind, String, usize)> = None;
    let flush = |open: &mut Option<(BlockKind, String, usize)>, out: &mut Vec<Block>| {
        if let Some((kind, text, line)) = open.take() {
            out.push(Block {
                kind,
                runs: inline(&text),
                line,
            });
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        let indent = indent_of(line);
        if let Some(fence) = fence_of(trimmed).filter(|_| indent < 4) {
            flush(&mut open, &mut out);
            let start = i;
            let mut runs = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with(fence) {
                runs.push(Run {
                    text: lines[i].trim_end().replace('\t', "    "),
                    style: Style {
                        code: true,
                        ..Style::default()
                    },
                    target: None,
                });
                i += 1;
            }
            out.push(Block {
                kind: BlockKind::Code,
                runs,
                line: start,
            });
            i += 1;
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut open, &mut out);
        } else if let Some((level, text)) = heading_of(trimmed).filter(|_| indent < 4) {
            flush(&mut open, &mut out);
            out.push(Block {
                kind: BlockKind::Heading(level),
                runs: inline(text),
                line: i,
            });
        } else if is_rule(trimmed) && indent < 4 {
            flush(&mut open, &mut out);
            out.push(Block {
                kind: BlockKind::Rule,
                runs: Vec::new(),
                line: i,
            });
        } else if let Some(rest) = quote_of(trimmed) {
            match &mut open {
                Some((BlockKind::Quote, text, _)) if !rest.is_empty() => {
                    text.push(' ');
                    text.push_str(rest);
                }
                Some((BlockKind::Quote, _, _)) => flush(&mut open, &mut out),
                _ => {
                    flush(&mut open, &mut out);
                    if !rest.is_empty() {
                        open = Some((BlockKind::Quote, rest.to_owned(), i));
                    }
                }
            }
        } else if let Some((marker, task, text)) = item_of(trimmed) {
            flush(&mut open, &mut out);
            let depth = (indent / 2).min(6) as u8;
            open = Some((
                BlockKind::Item {
                    depth,
                    marker,
                    task,
                },
                text.to_owned(),
                i,
            ));
        } else {
            match &mut open {
                // Lazy continuation: the line belongs to the open block.
                Some((_, text, _)) => {
                    text.push(' ');
                    text.push_str(trimmed);
                }
                None => open = Some((BlockKind::Paragraph, trimmed.to_owned(), i)),
            }
        }
        i += 1;
    }
    flush(&mut open, &mut out);
    out
}

fn indent_of(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

fn fence_of(t: &str) -> Option<&'static str> {
    if t.starts_with("```") {
        Some("```")
    } else if t.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn heading_of(t: &str) -> Option<(u8, &str)> {
    let n = t.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&n) {
        return None;
    }
    let rest = &t[n..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    // Optional closing hashes: `## Title ##`.
    let text = rest.trim();
    let text = text.trim_end_matches('#');
    let text = if text.len() < rest.trim().len() && !text.ends_with(' ') && !text.is_empty() {
        rest.trim()
    } else {
        text.trim_end()
    };
    Some((n as u8, text))
}

fn is_rule(t: &str) -> bool {
    let c: Vec<char> = t.chars().filter(|c| !c.is_whitespace()).collect();
    c.len() >= 3 && matches!(c[0], '-' | '*' | '_') && c.iter().all(|&x| x == c[0])
}

fn quote_of(t: &str) -> Option<&str> {
    let rest = t.strip_prefix('>')?;
    // `>> nested` is shown as one level.
    let rest = rest.trim_start_matches('>');
    Some(rest.strip_prefix(' ').unwrap_or(rest).trim_start())
}

pub(crate) fn item_of(t: &str) -> Option<(Option<u32>, Option<bool>, &str)> {
    let (marker, rest) = if let Some(r) = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("+ "))
    {
        (None, r)
    } else if matches!(t, "-" | "*" | "+") {
        (None, "")
    } else {
        let digits = t.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || digits > 9 {
            return None;
        }
        let after = &t[digits..];
        let r = after
            .strip_prefix(". ")
            .or_else(|| after.strip_prefix(") "))?;
        (t[..digits].parse().ok(), r)
    };
    let rest = rest.trim_start();
    for (p, done) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
        if let Some(r) = rest.strip_prefix(p) {
            if r.is_empty() || r.starts_with(' ') {
                return Some((marker, Some(done), r.trim_start()));
            }
        }
    }
    Some((marker, None, rest))
}

/// Parses inline syntax into styled runs (adjacent runs of the same style
/// and target are merged).
#[must_use]
pub fn inline(src: &str) -> Vec<Run> {
    let mut out = Vec::new();
    inline_into(src, Style::default(), None, &mut out);
    out
}

fn push(out: &mut Vec<Run>, text: &str, style: Style, target: Option<&str>) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.style == style && last.target.as_deref() == target {
            last.text.push_str(text);
            return;
        }
    }
    out.push(Run {
        text: text.to_owned(),
        style,
        target: target.map(str::to_owned),
    });
}

fn inline_into(src: &str, base: Style, target: Option<&str>, out: &mut Vec<Run>) {
    let mut st = base;
    let b = src.as_bytes();
    let mut i = 0;
    let mut lit = String::new();
    let emit = |lit: &mut String, out: &mut Vec<Run>, st: Style| {
        push(out, lit, st, target);
        lit.clear();
    };
    while i < b.len() {
        let rest = &src[i..];
        let c = rest.chars().next().unwrap_or(' ');
        // Escapes.
        if c == '\\' {
            if let Some(n) = rest[1..].chars().next().filter(char::is_ascii_punctuation) {
                lit.push(n);
                i += 1 + n.len_utf8();
                continue;
            }
        }
        if c == '`' && !st.code {
            let ticks = rest.bytes().take_while(|&x| x == b'`').count();
            let fence = &rest[..ticks];
            if let Some(end) = rest[ticks..].find(fence) {
                emit(&mut lit, out, st);
                let code = rest[ticks..ticks + end].trim();
                push(out, code, Style { code: true, ..st }, target);
                i += ticks + end + ticks;
                continue;
            }
        }
        if target.is_none() {
            if let Some(r) = rest.strip_prefix("[[") {
                if let Some(end) = r.find("]]").filter(|&e| e > 0 && !r[..e].contains('\n')) {
                    emit(&mut lit, out, st);
                    let inner = &r[..end];
                    let (note, label) = inner.split_once('|').unwrap_or((inner, inner));
                    let s = Style {
                        link: Some(LinkKind::Wiki),
                        ..st
                    };
                    push(out, label.trim(), s, Some(note.trim()));
                    i += 2 + end + 2;
                    continue;
                }
            }
            let image = rest.starts_with("![");
            if image || c == '[' {
                let open = if image { 2 } else { 1 };
                if let Some((label, url, used)) = link_at(&rest[open..]) {
                    emit(&mut lit, out, st);
                    let kind = if image {
                        LinkKind::Image
                    } else {
                        LinkKind::Web
                    };
                    let s = Style {
                        link: Some(kind),
                        ..st
                    };
                    if image {
                        let alt = if label.is_empty() { "image" } else { label };
                        push(out, alt, s, Some(url));
                    } else {
                        inline_into(label, s, Some(url), out);
                    }
                    i += open + used;
                    continue;
                }
            }
        }
        if !st.code {
            let toggles: [(&str, Toggle); 5] = [
                ("**", |s| &mut s.strong),
                ("__", |s| &mut s.strong),
                ("~~", |s| &mut s.strike),
                ("*", |s| &mut s.em),
                ("_", |s| &mut s.em),
            ];
            let mut done = false;
            for (d, field) in toggles {
                if !rest.starts_with(d) {
                    continue;
                }
                let prev = src[..i].chars().next_back();
                let next = rest[d.len()..].chars().next();
                let on = *field(&mut st);
                let opens = !on
                    && next.is_some_and(|n| !n.is_whitespace())
                    && closes_later(&rest[d.len()..], d)
                    && !(d.starts_with('_') && prev.is_some_and(char::is_alphanumeric));
                let closes = on
                    && prev.is_some_and(|p| !p.is_whitespace())
                    && !(d.starts_with('_') && next.is_some_and(char::is_alphanumeric));
                if opens || closes {
                    emit(&mut lit, out, st);
                    *field(&mut st) = !on;
                    i += d.len();
                    done = true;
                }
                break;
            }
            if done {
                continue;
            }
        }
        lit.push(c);
        i += c.len_utf8();
    }
    emit(&mut lit, out, st);
}

type Toggle = fn(&mut Style) -> &mut bool;

/// After an opening delimiter: is there a closing one (not preceded by a
/// space) later on?
fn closes_later(rest: &str, d: &str) -> bool {
    let mut from = 0;
    while let Some(p) = rest[from..].find(d) {
        let at = from + p;
        let before = rest[..at].chars().next_back();
        let doubled = d.len() == 1 && rest[at + 1..].starts_with(d);
        if at > 0 && before.is_some_and(|b| !b.is_whitespace()) && !doubled {
            return true;
        }
        if at > 0 && before.is_some_and(|b| !b.is_whitespace()) && doubled {
            // `*a**b*`: skip the double and keep looking.
            from = at + 2;
            continue;
        }
        from = at + d.len();
    }
    false
}

/// `label](url)` after the opening bracket: (label, url, bytes used).
fn link_at(s: &str) -> Option<(&str, &str, usize)> {
    let mut depth = 0usize;
    let mut close = None;
    for (j, ch) in s.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' if depth == 0 => {
                close = Some(j);
                break;
            }
            ']' => depth -= 1,
            _ => {}
        }
    }
    let close = close?;
    let after = s[close + 1..].strip_prefix('(')?;
    let end = after.find(')')?;
    let url = after[..end].trim();
    let url = url.split_once(' ').map_or(url, |(u, _)| u);
    Some((&s[..close], url, close + 2 + end + 1))
}

/// Plain text of the runs (tests, word counts).
#[must_use]
pub fn plain(runs: &[Run]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<BlockKind> {
        parse(src).into_iter().map(|b| b.kind).collect()
    }

    fn styled(runs: &[Run]) -> Vec<(String, Style)> {
        runs.iter().map(|r| (r.text.clone(), r.style)).collect()
    }

    #[test]
    fn headings_levels_and_not_headings() {
        let b = parse("# One\n## Two ##\n###Three\n####### seven\n   # indented");
        assert_eq!(b[0].kind, BlockKind::Heading(1));
        assert_eq!(plain(&b[0].runs), "One");
        assert_eq!(b[1].kind, BlockKind::Heading(2));
        assert_eq!(plain(&b[1].runs), "Two");
        // `###Three` and seven hashes are text, joined into one paragraph.
        assert_eq!(b[2].kind, BlockKind::Paragraph);
        assert_eq!(plain(&b[2].runs), "###Three ####### seven");
        assert_eq!(b[3].kind, BlockKind::Heading(1));
        assert_eq!(b[3].line, 4);
    }

    #[test]
    fn paragraphs_join_soft_lines_and_split_on_blank() {
        let b = parse("one\ntwo\n\nthree");
        assert_eq!(b.len(), 2);
        assert_eq!(plain(&b[0].runs), "one two");
        assert_eq!(b[1].line, 3);
    }

    #[test]
    fn lists_tasks_and_nesting() {
        let k = kinds("- a\n* b\n+ c\n1. d\n12) e\n  - [ ] f\n    - [x] g\n- [X] h\n- [y] i");
        let item = |depth, marker, task| BlockKind::Item {
            depth,
            marker,
            task,
        };
        assert_eq!(
            k,
            vec![
                item(0, None, None),
                item(0, None, None),
                item(0, None, None),
                item(0, Some(1), None),
                item(0, Some(12), None),
                item(1, None, Some(false)),
                item(2, None, Some(true)),
                item(0, None, Some(true)),
                item(0, None, None),
            ]
        );
        let b = parse("- [y] i\n- [ ] task text\n  continues");
        assert_eq!(plain(&b[0].runs), "[y] i");
        assert_eq!(plain(&b[1].runs), "task text continues");
    }

    #[test]
    fn quotes_merge_and_rules() {
        let b = parse("> one\n> two\n>\n> three\n---\n* * *\n___\n--");
        assert_eq!(b[0].kind, BlockKind::Quote);
        assert_eq!(plain(&b[0].runs), "one two");
        assert_eq!(plain(&b[1].runs), "three");
        assert_eq!(b[2].kind, BlockKind::Rule);
        assert_eq!(b[3].kind, BlockKind::Rule);
        assert_eq!(b[4].kind, BlockKind::Rule);
        assert_eq!(b[5].kind, BlockKind::Paragraph);
    }

    #[test]
    fn fenced_code_keeps_lines_verbatim() {
        let b = parse("```rust\nlet *a* = 1;\n\n# not a heading\n```\nafter\n~~~\nx\n");
        assert_eq!(b[0].kind, BlockKind::Code);
        let lines: Vec<_> = b[0].runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(lines, vec!["let *a* = 1;", "", "# not a heading"]);
        assert!(b[0].runs.iter().all(|r| r.style.code));
        assert_eq!(plain(&b[1].runs), "after");
        // An unclosed fence runs to the end of the note.
        assert_eq!(b[2].kind, BlockKind::Code);
        assert_eq!(b[2].runs.len(), 1);
    }

    #[test]
    fn emphasis_strong_strike_code() {
        let r = inline("a **b** *c* ~~d~~ `e*f*` __g__ _h_");
        let s = styled(&r);
        let find = |t: &str| s.iter().find(|(x, _)| x == t).unwrap().1;
        assert!(find("b").strong && !find("b").em);
        assert!(find("c").em);
        assert!(find("d").strike);
        assert!(find("e*f*").code && !find("e*f*").em);
        assert!(find("g").strong);
        assert!(find("h").em);
        assert_eq!(plain(&r), "a b c d e*f* g h");
    }

    #[test]
    fn nested_emphasis() {
        let r = inline("**bold *both* bold**");
        let s = styled(&r);
        assert_eq!(s[0].0, "bold ");
        assert!(s[0].1.strong && !s[0].1.em);
        assert_eq!(s[1].0, "both");
        assert!(s[1].1.strong && s[1].1.em);
        assert!(s[2].1.strong && !s[2].1.em);
    }

    #[test]
    fn unmatched_delimiters_stay_literal() {
        for src in [
            "2 * 3 = 6",
            "a *b",
            "snake_case_name",
            "**open",
            "~~ x",
            "`tick",
            "[x] not a link",
            "[a](b",
        ] {
            let r = inline(src);
            assert_eq!(plain(&r), src, "{src}");
            assert!(r.iter().all(|r| r.style == Style::default()), "{src}");
        }
    }

    #[test]
    fn links_wiki_and_images() {
        let r = inline(
            "see [the *site*](https://x.org \"t\"), [[chapter-01]], [[a b|label]] ![pic](p.png)",
        );
        let web: Vec<_> = r
            .iter()
            .filter(|r| r.style.link == Some(LinkKind::Web))
            .collect();
        assert_eq!(web.len(), 2);
        assert_eq!(web[1].text, "site");
        assert!(web[1].style.em);
        assert_eq!(web[0].target.as_deref(), Some("https://x.org"));
        let wiki: Vec<_> = r
            .iter()
            .filter(|r| r.style.link == Some(LinkKind::Wiki))
            .map(|r| (r.text.as_str(), r.target.as_deref()))
            .collect();
        assert_eq!(
            wiki,
            vec![("chapter-01", Some("chapter-01")), ("label", Some("a b"))]
        );
        let img = r
            .iter()
            .find(|r| r.style.link == Some(LinkKind::Image))
            .unwrap();
        assert_eq!(
            (img.text.as_str(), img.target.as_deref()),
            ("pic", Some("p.png"))
        );
        assert_eq!(plain(&r), "see the site, chapter-01, label pic");
    }

    #[test]
    fn escapes() {
        assert_eq!(plain(&inline(r"\*not em\* \[[x]]")), "*not em* [[x]]");
        assert!(inline(r"\*not em\*").iter().all(|r| !r.style.em));
    }

    #[test]
    fn seed_note() {
        let b = parse("# Field notes\n\nA page.\n\n## Today\n\n- [ ] Try\n- [x] Open [[chapter-01]]\n\nType here.");
        let k: Vec<_> = b.iter().map(|b| &b.kind).collect();
        assert_eq!(k.len(), 6);
        assert_eq!(*k[0], BlockKind::Heading(1));
        assert_eq!(
            *k[4],
            BlockKind::Item {
                depth: 0,
                marker: None,
                task: Some(true)
            }
        );
        assert_eq!(b[4].runs[1].style.link, Some(LinkKind::Wiki));
    }

    /// Never panics and never loses non-syntax characters, whatever the input
    /// (a cheap deterministic fuzz over byte soup of Markdown punctuation).
    #[test]
    fn fuzz_no_panic() {
        let alphabet: Vec<char> = "*_~`[]()!#>-+1. x|\\\n\té".chars().collect();
        let mut seed: u64 = 0x5eed;
        for _ in 0..4000 {
            let len = (seed % 40) as usize;
            let mut s = String::new();
            for _ in 0..len {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                s.push(alphabet[(seed >> 33) as usize % alphabet.len()]);
            }
            let blocks = parse(&s);
            for b in &blocks {
                for r in &b.runs {
                    assert!(!r.text.is_empty() || b.kind == BlockKind::Code, "{s:?}");
                }
            }
            seed = seed.wrapping_add(7);
        }
    }

    #[test]
    fn large_note_parses_quickly() {
        let para = "Some **bold** and *em* text with a [[link]] and `code`.\n";
        let src: String = (0..4000)
            .map(|i| match i % 5 {
                0 => format!("## Heading {i}\n"),
                1 => format!("- [ ] task {i}\n"),
                _ => para.to_owned(),
            })
            .collect();
        assert!(src.len() > 100_000);
        let t = std::time::Instant::now();
        let b = parse(&src);
        assert!(b.len() > 1500);
        assert!(t.elapsed().as_millis() < 500, "{:?}", t.elapsed());
    }
}
