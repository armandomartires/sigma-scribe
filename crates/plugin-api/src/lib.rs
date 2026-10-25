//! Wire format between Sigma Scribe and its WASM screen plugins.
//!
//! A screen plugin owns a full-screen body. The host sends it [`Event`]s
//! and the plugin answers each with a [`Frame`]: a draw list, a hit list
//! and actions. Everything is little-endian bytes, versioned by [`ABI`],
//! and decoding is bounds-checked and size-limited, so a broken plugin or
//! host message is an error, never a crash or a huge allocation.
//!
//! The plugin side (guest) exports:
//! - `sigma_abi() -> u32`: must equal [`ABI`].
//! - `sigma_alloc(len: u32) -> u32`: a buffer in plugin memory for an event.
//! - `sigma_call(ptr: u32, len: u32) -> u64`: handles an encoded event and
//!   returns `(out_ptr << 32) | out_len`, an encoded frame valid until the
//!   next call.
//!
//! The host provides (module `sigma`): `text_width`, `ascent`, `tr`,
//! `cache_get` and `cache_put` (see `sigma-plugin-host`).

/// Wire format version. Bumped on any incompatible change.
pub const ABI: u32 = 1;

/// Decoding limits.
pub const MAX_OPS: usize = 200_000;
pub const MAX_HITS: usize = 50_000;
pub const MAX_NOTES: usize = 50_000;
pub const MAX_EDGES: usize = 500_000;
pub const MAX_STR: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError;

pub type Result<T> = std::result::Result<T, DecodeError>;

#[derive(Default)]
pub struct Writer(pub Vec<u8>);

impl Writer {
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    pub fn str(&mut self, s: &str) {
        let s = truncate(s, MAX_STR);
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
}

/// `s` cut to at most `max` bytes on a character boundary.
fn truncate(s: &str, max: usize) -> &str {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(DecodeError)?;
        let s = self.buf.get(self.pos..end).ok_or(DecodeError)?;
        self.pos = end;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| DecodeError)?,
        ))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| DecodeError)?,
        ))
    }
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| DecodeError)?,
        ))
    }
    pub fn str(&mut self) -> Result<String> {
        let n = self.u32()? as usize;
        if n > MAX_STR {
            return Err(DecodeError);
        }
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| DecodeError)
    }
    /// A count, checked against `max` and against the bytes left (each item
    /// takes at least `min_item` bytes), before anything is allocated.
    fn count(&mut self, max: usize, min_item: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n > max || n.saturating_mul(min_item) > self.buf.len() - self.pos {
            return Err(DecodeError);
        }
        Ok(n)
    }
    /// Fails when bytes are left over.
    pub fn finish(self) -> Result<()> {
        if self.pos == self.buf.len() {
            Ok(())
        } else {
            Err(DecodeError)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    #[must_use]
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    fn put(&self, w: &mut Writer) {
        for v in [self.x, self.y, self.w, self.h] {
            w.i32(v);
        }
    }
    fn get(r: &mut Reader) -> Result<Self> {
        Ok(Self::new(r.i32()?, r.i32()?, r.i32()?, r.i32()?))
    }
}

/// Colours are the host theme's tokens, so a plugin follows Day and Night.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Paper,
    Ink,
    Mute,
    Rule,
}

impl Tone {
    fn from_u8(v: u8) -> Result<Self> {
        Ok(match v {
            0 => Self::Paper,
            1 => Self::Ink,
            2 => Self::Mute,
            3 => Self::Rule,
            _ => return Err(DecodeError),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Regular,
    Bold,
}

impl Face {
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Regular),
            1 => Some(Self::Bold),
            _ => None,
        }
    }
}

/// One drawing command. All coordinates are device pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Fill {
        r: Rect,
        c: Tone,
    },
    Stroke {
        r: Rect,
        t: i32,
        c: Tone,
    },
    /// A pill: a rectangle with fully rounded ends (a circle when w == h).
    Pill {
        r: Rect,
        c: Tone,
    },
    /// A straight line of width `w`, stamped as squares.
    Line {
        a: (i32, i32),
        b: (i32, i32),
        w: i32,
        c: Tone,
    },
    /// Text with its baseline at `y`, starting at `x`.
    Text {
        face: Face,
        size: f32,
        x: f32,
        y: f32,
        s: String,
        c: Tone,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// Plugin-defined id, echoed nowhere: hits are for host tap routing of
    /// emulator controls and accessibility; the plugin does its own routing.
    pub id: u32,
    pub rect: Rect,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Leave the screen.
    Close,
    /// Open note number `n` (index into [`Scene::notes`]) in the editor.
    OpenNote(u32),
    /// Open note number `n` in preview (what tapping a map node twice does).
    PreviewNote(u32),
}

/// How much of the screen the host should refresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Redraw {
    #[default]
    None,
    /// Repaint with a fast partial refresh.
    Partial,
    /// Repaint and clear pointer ghosts (a slower, cleaner refresh).
    Clean,
    /// Repaint the whole screen (a flashing refresh): for big changes.
    Full,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Frame {
    pub ops: Vec<Op>,
    pub hits: Vec<Hit>,
    /// Every action leaves the screen, so the host applies only the first.
    pub actions: Vec<Action>,
    pub redraw: Redraw,
}

impl Frame {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.u32(self.ops.len() as u32);
        for op in &self.ops {
            match op {
                Op::Fill { r, c } => {
                    w.u8(0);
                    r.put(&mut w);
                    w.u8(*c as u8);
                }
                Op::Stroke { r, t, c } => {
                    w.u8(1);
                    r.put(&mut w);
                    w.i32(*t);
                    w.u8(*c as u8);
                }
                Op::Pill { r, c } => {
                    w.u8(2);
                    r.put(&mut w);
                    w.u8(*c as u8);
                }
                Op::Line { a, b, w: lw, c } => {
                    w.u8(3);
                    for v in [a.0, a.1, b.0, b.1, *lw] {
                        w.i32(v);
                    }
                    w.u8(*c as u8);
                }
                Op::Text {
                    face,
                    size,
                    x,
                    y,
                    s,
                    c,
                } => {
                    w.u8(4);
                    w.u8(*face as u8);
                    w.f32(*size);
                    w.f32(*x);
                    w.f32(*y);
                    w.str(s);
                    w.u8(*c as u8);
                }
            }
        }
        w.u32(self.hits.len() as u32);
        for h in &self.hits {
            w.u32(h.id);
            h.rect.put(&mut w);
            w.str(&h.label);
        }
        w.u32(self.actions.len() as u32);
        for a in &self.actions {
            match a {
                Action::Close => w.u8(0),
                Action::OpenNote(n) => {
                    w.u8(1);
                    w.u32(*n);
                }
                Action::PreviewNote(n) => {
                    w.u8(2);
                    w.u32(*n);
                }
            }
        }
        w.u8(self.redraw as u8);
        w.0
    }

    /// # Errors
    /// [`DecodeError`] for truncated, oversized, trailing or invalid bytes.
    pub fn decode(buf: &[u8]) -> Result<Self> {
        let mut r = Reader::new(buf);
        let mut f = Self::default();
        for _ in 0..r.count(MAX_OPS, 14)? {
            f.ops.push(match r.u8()? {
                0 => Op::Fill {
                    r: Rect::get(&mut r)?,
                    c: Tone::from_u8(r.u8()?)?,
                },
                1 => Op::Stroke {
                    r: Rect::get(&mut r)?,
                    t: r.i32()?,
                    c: Tone::from_u8(r.u8()?)?,
                },
                2 => Op::Pill {
                    r: Rect::get(&mut r)?,
                    c: Tone::from_u8(r.u8()?)?,
                },
                3 => Op::Line {
                    a: (r.i32()?, r.i32()?),
                    b: (r.i32()?, r.i32()?),
                    w: r.i32()?,
                    c: Tone::from_u8(r.u8()?)?,
                },
                4 => Op::Text {
                    face: Face::from_u8(r.u8()?).ok_or(DecodeError)?,
                    size: r.f32()?,
                    x: r.f32()?,
                    y: r.f32()?,
                    s: r.str()?,
                    c: Tone::from_u8(r.u8()?)?,
                },
                _ => return Err(DecodeError),
            });
        }
        for _ in 0..r.count(MAX_HITS, 24)? {
            f.hits.push(Hit {
                id: r.u32()?,
                rect: Rect::get(&mut r)?,
                label: r.str()?,
            });
        }
        for _ in 0..r.count(MAX_HITS, 1)? {
            f.actions.push(match r.u8()? {
                0 => Action::Close,
                1 => Action::OpenNote(r.u32()?),
                2 => Action::PreviewNote(r.u32()?),
                _ => return Err(DecodeError),
            });
        }
        f.redraw = match r.u8()? {
            0 => Redraw::None,
            1 => Redraw::Partial,
            2 => Redraw::Clean,
            3 => Redraw::Full,
            _ => return Err(DecodeError),
        };
        r.finish()?;
        Ok(f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub name: String,
    pub current: bool,
    /// Text for list rows, such as "3 links".
    pub detail: String,
}

/// Everything the plugin needs to build its screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub width: i32,
    pub height: i32,
    /// The host's exact UI scale factor (so `round(brief * scale)` gives the
    /// same pixels as the host) and its UI font size.
    pub scale: f32,
    pub ui_px: f32,
    /// Top and bottom bars the host draws itself: the plugin paints between.
    pub top: i32,
    pub bottom: i32,
    pub notes: Vec<Note>,
    /// Directed links between notes, as indexes into `notes`.
    pub edges: Vec<(u32, u32)>,
    /// How many links reach a note (what footers say), which counts
    /// repeated links that `edges` lists once.
    pub links: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Start,
    Move,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The screen opens (or the library changed while it was open).
    Open(Scene),
    Tap {
        x: i32,
        y: i32,
    },
    Drag {
        phase: Phase,
        x: i32,
        y: i32,
    },
    /// Mouse wheel notches, already adjusted for the reader's scroll
    /// direction setting; positive is down. `x`, `y` is the pointer (the
    /// screen centre when the host has no pointer position).
    Wheel {
        x: i32,
        y: i32,
        dy: i32,
    },
    PageTurn {
        forward: bool,
    },
    Back,
}

impl Event {
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::default();
        match self {
            Event::Open(s) => {
                w.u8(0);
                for v in [s.width, s.height, s.top, s.bottom] {
                    w.i32(v);
                }
                w.f32(s.scale);
                w.f32(s.ui_px);
                w.u32(s.notes.len() as u32);
                for n in &s.notes {
                    w.str(&n.name);
                    w.u8(u8::from(n.current));
                    w.str(&n.detail);
                }
                w.u32(s.edges.len() as u32);
                for &(a, b) in &s.edges {
                    w.u32(a);
                    w.u32(b);
                }
                w.u32(s.links);
            }
            Event::Tap { x, y } => {
                w.u8(1);
                w.i32(*x);
                w.i32(*y);
            }
            Event::Drag { phase, x, y } => {
                w.u8(2);
                w.u8(*phase as u8);
                w.i32(*x);
                w.i32(*y);
            }
            Event::Wheel { x, y, dy } => {
                w.u8(3);
                w.i32(*x);
                w.i32(*y);
                w.i32(*dy);
            }
            Event::PageTurn { forward } => {
                w.u8(4);
                w.u8(u8::from(*forward));
            }
            Event::Back => w.u8(5),
        }
        w.0
    }

    /// # Errors
    /// [`DecodeError`] for truncated, oversized, trailing or invalid bytes.
    pub fn decode(buf: &[u8]) -> Result<Self> {
        let mut r = Reader::new(buf);
        let e = match r.u8()? {
            0 => {
                let (width, height, top, bottom) = (r.i32()?, r.i32()?, r.i32()?, r.i32()?);
                let (scale, ui_px) = (r.f32()?, r.f32()?);
                let mut notes = Vec::new();
                for _ in 0..r.count(MAX_NOTES, 9)? {
                    notes.push(Note {
                        name: r.str()?,
                        current: r.u8()? != 0,
                        detail: r.str()?,
                    });
                }
                let mut edges = Vec::new();
                for _ in 0..r.count(MAX_EDGES, 8)? {
                    edges.push((r.u32()?, r.u32()?));
                }
                let links = r.u32()?;
                Event::Open(Scene {
                    width,
                    height,
                    scale,
                    ui_px,
                    top,
                    bottom,
                    notes,
                    edges,
                    links,
                })
            }
            1 => Event::Tap {
                x: r.i32()?,
                y: r.i32()?,
            },
            2 => Event::Drag {
                phase: match r.u8()? {
                    0 => Phase::Start,
                    1 => Phase::Move,
                    2 => Phase::End,
                    _ => return Err(DecodeError),
                },
                x: r.i32()?,
                y: r.i32()?,
            },
            3 => Event::Wheel {
                x: r.i32()?,
                y: r.i32()?,
                dy: r.i32()?,
            },
            4 => Event::PageTurn {
                forward: r.u8()? != 0,
            },
            5 => Event::Back,
            _ => return Err(DecodeError),
        };
        r.finish()?;
        Ok(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            ops: vec![
                Op::Fill {
                    r: Rect::new(0, 0, 10, 20),
                    c: Tone::Paper,
                },
                Op::Stroke {
                    r: Rect::new(1, 2, 3, 4),
                    t: 2,
                    c: Tone::Rule,
                },
                Op::Pill {
                    r: Rect::new(5, 6, 7, 7),
                    c: Tone::Ink,
                },
                Op::Line {
                    a: (0, 0),
                    b: (9, -9),
                    w: 3,
                    c: Tone::Mute,
                },
                Op::Text {
                    face: Face::Bold,
                    size: 21.5,
                    x: 3.25,
                    y: 40.0,
                    s: "Café — 日本".into(),
                    c: Tone::Ink,
                },
            ],
            hits: vec![Hit {
                id: 7,
                rect: Rect::new(1, 1, 44, 44),
                label: "Node:a.md".into(),
            }],
            actions: vec![Action::OpenNote(3), Action::PreviewNote(1), Action::Close],
            redraw: Redraw::Clean,
        }
    }

    fn scene() -> Scene {
        Scene {
            width: 1264,
            height: 1680,
            scale: 1.0,
            ui_px: 28.0,
            top: 96,
            bottom: 1600,
            notes: vec![
                Note {
                    name: "a.md".into(),
                    current: true,
                    detail: "2 links".into(),
                },
                Note {
                    name: "b/c.md".into(),
                    current: false,
                    detail: String::new(),
                },
            ],
            edges: vec![(0, 1), (1, 0)],
            links: 3,
        }
    }

    #[test]
    fn frame_round_trips() {
        let f = frame();
        assert_eq!(Frame::decode(&f.encode()), Ok(f));
        assert_eq!(
            Frame::decode(&Frame::default().encode()),
            Ok(Frame::default())
        );
    }

    #[test]
    fn events_round_trip() {
        for e in [
            Event::Open(scene()),
            Event::Tap { x: -3, y: 9 },
            Event::Drag {
                phase: Phase::Move,
                x: 1,
                y: 2,
            },
            Event::Wheel {
                x: 1,
                y: 2,
                dy: -120,
            },
            Event::PageTurn { forward: true },
            Event::Back,
        ] {
            assert_eq!(Event::decode(&e.encode()), Ok(e));
        }
    }

    #[test]
    fn truncated_trailing_and_invalid_bytes_are_errors() {
        let bytes = frame().encode();
        for n in 0..bytes.len() {
            assert!(Frame::decode(&bytes[..n]).is_err(), "cut at {n}");
        }
        let mut more = bytes.clone();
        more.push(0);
        assert!(Frame::decode(&more).is_err());
        assert!(Event::decode(&[9]).is_err());
        assert!(Event::decode(&[]).is_err());
        let ev = Event::Open(scene()).encode();
        for n in 0..ev.len() {
            assert!(Event::decode(&ev[..n]).is_err(), "cut at {n}");
        }
    }

    #[test]
    fn huge_counts_are_refused_before_allocating() {
        // A frame claiming 4 billion ops in a few bytes.
        let mut b = u32::MAX.to_le_bytes().to_vec();
        b.extend_from_slice(&[0; 8]);
        assert!(Frame::decode(&b).is_err());
        // An Open event claiming MAX_NOTES + 1 notes.
        let mut w = Writer::default();
        w.u8(0);
        for _ in 0..4 {
            w.i32(0);
        }
        w.f32(1.0);
        w.f32(1.0);
        w.u32(MAX_NOTES as u32 + 1);
        assert!(Event::decode(&w.0).is_err());
    }

    #[test]
    fn strings_are_cut_on_a_character_boundary() {
        let long = "é".repeat(MAX_STR); // 2 bytes each
        let mut w = Writer::default();
        w.str(&long);
        let s = Reader::new(&w.0).str().unwrap();
        assert!(s.len() <= MAX_STR && s.chars().all(|c| c == 'é'));
    }

    #[test]
    fn rect_contains_is_half_open() {
        let r = Rect::new(10, 10, 5, 5);
        assert!(r.contains(10, 10) && r.contains(14, 14));
        assert!(!r.contains(15, 10) && !r.contains(9, 10));
    }
}
