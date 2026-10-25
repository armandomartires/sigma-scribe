//! Wire format between Sigma Scribe and its WASM plugins.
//!
//! A screen plugin owns a full-screen body; a pane plugin owns the body of
//! one side-drawer pane (Contents, To-dos, Scratch) and draws it beside the page. The host sends it [`Event`]s
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
//! `cache_get`, `cache_put`, `log` and `log_level` (see `sigma-plugin-host`).
//!
//! # Example
//!
//! A plugin decodes the host's event and answers with a frame; the host
//! decodes that frame. Both sides use the same types:
//!
//! ```
//! use sigma_plugin_api::{Action, Event, Frame, Op, Rect, Redraw, Tone};
//!
//! # fn main() -> Result<(), sigma_plugin_api::DecodeError> {
//! // Host side: a tap arrives.
//! let bytes = Event::Tap { x: 10, y: 20 }.encode();
//!
//! // Plugin side: decode it and answer.
//! let frame = match Event::decode(&bytes)? {
//!     Event::Back => Frame { actions: vec![Action::Close], ..Frame::default() },
//!     _ => Frame {
//!         ops: vec![Op::Fill { r: Rect::new(0, 0, 100, 50), c: Tone::Ink }],
//!         redraw: Redraw::Partial,
//!         ..Frame::default()
//!     },
//! };
//!
//! // Host side: decode the answer.
//! assert_eq!(Frame::decode(&frame.encode())?, frame);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

/// Wire format version. Bumped on any incompatible change.
pub const ABI: u32 = 4;

// Decoding limits. A count above its limit fails decoding before anything
// is allocated, so a hostile or broken message cannot exhaust memory.

/// Most drawing operations in one [`Frame`].
pub const MAX_OPS: usize = 200_000;
/// Most hit rectangles in one [`Frame`]; also the most actions.
pub const MAX_HITS: usize = 50_000;
/// Most notes in a [`Scene`].
pub const MAX_NOTES: usize = 50_000;
/// Most links in [`Scene::edges`].
pub const MAX_EDGES: usize = 500_000;
/// Longest string in bytes. [`Writer::str`] cuts longer strings on a
/// character boundary; [`Reader::str`] rejects them.
pub const MAX_STR: usize = 8 * 1024;
/// Most entries in [`Scene::settings`].
pub const MAX_SETTINGS: usize = 64;
/// Most rows in [`Scene::todos`].
pub const MAX_TODOS: usize = 20_000;
/// Most rows in [`Scene::headings`].
pub const MAX_HEADINGS: usize = 5_000;

/// Bytes that are not a valid message: truncated, with trailing bytes, an
/// unknown tag, invalid UTF-8, or a count or string above its limit.
///
/// It carries no detail on purpose: the host treats any decode failure the
/// same way (the plugin is dropped), and a plugin cannot do better than
/// ignore a bad event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeError;

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid plugin message")
    }
}

impl std::error::Error for DecodeError {}

/// Result of decoding a message.
pub type Result<T> = std::result::Result<T, DecodeError>;

/// Appends values to a byte buffer in the wire encoding: integers and
/// floats little-endian, strings as a `u32` byte length then UTF-8 bytes.
///
/// The [`Frame::encode`] and [`Event::encode`] methods use it; plugins that
/// build messages by hand can too.
#[derive(Default)]
pub struct Writer(pub Vec<u8>);

impl Writer {
    /// Appends one byte.
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    /// Appends a little-endian `u32`.
    pub fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends a little-endian `i32`.
    pub fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends a little-endian `f32`.
    pub fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    /// Appends a string, cut to [`MAX_STR`] bytes on a character boundary
    /// so that the reader always accepts it.
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

/// Reads values written by [`Writer`], checking bounds on every read.
///
/// Every method fails with [`DecodeError`] instead of panicking when the
/// buffer is too short or a value is invalid.
///
/// ```
/// use sigma_plugin_api::{DecodeError, Reader, Writer};
///
/// let mut w = Writer::default();
/// w.u32(7);
/// w.str("hello");
///
/// let mut r = Reader::new(&w.0);
/// assert_eq!(r.u32(), Ok(7));
/// assert_eq!(r.str().as_deref(), Ok("hello"));
/// r.finish()?; // nothing left over
///
/// // A truncated buffer is an error, not a panic.
/// assert_eq!(Reader::new(&[1, 0]).u32(), Err(DecodeError));
/// # Ok::<(), DecodeError>(())
/// ```
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// A reader at the start of `buf`.
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
    /// Reads one byte.
    ///
    /// # Errors
    /// [`DecodeError`] when the buffer has too few bytes left.
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    /// Reads a little-endian `u32`.
    ///
    /// # Errors
    /// [`DecodeError`] when the buffer has too few bytes left.
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| DecodeError)?,
        ))
    }
    /// Reads a little-endian `i32`.
    ///
    /// # Errors
    /// [`DecodeError`] when the buffer has too few bytes left.
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| DecodeError)?,
        ))
    }
    /// Reads a little-endian `f32`.
    ///
    /// # Errors
    /// [`DecodeError`] when the buffer has too few bytes left.
    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| DecodeError)?,
        ))
    }
    /// Reads a length-prefixed UTF-8 string.
    ///
    /// # Errors
    /// [`DecodeError`] when the length is above [`MAX_STR`], the buffer is
    /// too short, or the bytes are not UTF-8.
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
    /// Checks that the whole buffer was read.
    ///
    /// # Errors
    /// [`DecodeError`] when bytes are left over.
    pub fn finish(self) -> Result<()> {
        if self.pos == self.buf.len() {
            Ok(())
        } else {
            Err(DecodeError)
        }
    }
}

/// A rectangle in device pixels: top-left corner, width and height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl Rect {
    /// A rectangle from its corner and size.
    #[must_use]
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    /// Whether the point is inside. The left and top edges are inside, the
    /// right and bottom edges are not.
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
    /// The page background.
    Paper,
    /// Body text and strong marks.
    Ink,
    /// Secondary text.
    Mute,
    /// Hairlines and dividers.
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

/// A weight of the host's UI font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// The regular weight.
    Regular,
    /// The bold weight.
    Bold,
}

impl Face {
    /// The face for its wire tag, or `None` for an unknown tag.
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
#[allow(missing_docs)] // Short field names are part of the wire format; variant docs explain them.
pub enum Op {
    /// A filled rectangle `r` in tone `c`.
    Fill { r: Rect, c: Tone },
    /// The outline of rectangle `r`, `t` pixels thick, in tone `c`.
    Stroke { r: Rect, t: i32, c: Tone },
    /// A pill: a rectangle with fully rounded ends (a circle when w == h).
    Pill { r: Rect, c: Tone },
    /// A straight line from `a` to `b` of width `w`, stamped as squares.
    Line {
        a: (i32, i32),
        b: (i32, i32),
        w: i32,
        c: Tone,
    },
    /// Text `s` in `face` at `size` pixels, with its baseline at `y`,
    /// starting at `x`.
    Text {
        face: Face,
        size: f32,
        x: f32,
        y: f32,
        s: String,
        c: Tone,
    },
}

/// A labelled tappable area of a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// Plugin-defined id, echoed nowhere: hits are for host tap routing of
    /// emulator controls and accessibility; the plugin does its own routing.
    pub id: u32,
    /// The tappable area.
    pub rect: Rect,
    /// A name for the area, such as `Node:a.md`.
    pub label: String,
}

/// Something the plugin asks the host to do after showing a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Leave the screen.
    Close,
    /// Open note number `n` (index into [`Scene::notes`]) in the editor.
    OpenNote(u32),
    /// Open note number `n` in preview (what tapping a map node twice does).
    PreviewNote(u32),
    /// Pane plugins: check or uncheck to-do number `n` (index into
    /// [`Scene::todos`]). The pane stays open.
    ToggleTodo(u32),
    /// Pane plugins: open the note of to-do `n` at its line.
    GotoTodo(u32),
    /// Pane plugins: jump to heading `n` (index into [`Scene::headings`]) of
    /// the open note.
    GotoHeading(u32),
}

/// How much of the screen the host should refresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Redraw {
    /// Leave the screen as it is.
    #[default]
    None,
    /// Repaint with a fast partial refresh.
    Partial,
    /// Repaint and clear pointer ghosts (a slower, cleaner refresh).
    Clean,
    /// Repaint the whole screen (a flashing refresh): for big changes.
    Full,
}

/// The plugin's answer to an [`Event`]: what to draw and what to do.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Frame {
    /// Drawing commands, painted in order.
    pub ops: Vec<Op>,
    /// Labelled tappable areas.
    pub hits: Vec<Hit>,
    /// A screen's actions all leave it, so the host applies only the first;
    /// a pane stays open, so the host applies every action in order.
    pub actions: Vec<Action>,
    /// How much of the screen to refresh.
    pub redraw: Redraw,
    /// The plugin has more to do (building a layout, say): the host shows
    /// this frame and, when idle, sends [`Event::Step`] for the next one.
    /// The frame is the progress picture meanwhile.
    pub pending: bool,
    /// Panes: where the host draws its text field (the Scratch pad), with
    /// the placeholder shown while the field is empty. The host owns the
    /// text, the keyboard, handwriting and saving; the plugin only places it.
    pub field: Option<Field>,
}

/// A text field the host draws and edits inside a pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Where the field goes.
    pub rect: Rect,
    /// Text shown while the field is empty.
    pub placeholder: String,
}

impl Frame {
    /// The frame in the wire encoding. Strings longer than [`MAX_STR`] are
    /// cut; the result always decodes with [`Frame::decode`] unless a list
    /// is longer than its limit.
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
                Action::ToggleTodo(n) => {
                    w.u8(3);
                    w.u32(*n);
                }
                Action::GotoTodo(n) => {
                    w.u8(4);
                    w.u32(*n);
                }
                Action::GotoHeading(n) => {
                    w.u8(5);
                    w.u32(*n);
                }
            }
        }
        w.u8(self.redraw as u8);
        w.u8(u8::from(self.pending));
        match &self.field {
            Some(f) => {
                w.u8(1);
                f.rect.put(&mut w);
                w.str(&f.placeholder);
            }
            None => w.u8(0),
        }
        w.0
    }

    /// Decodes a frame written by [`Frame::encode`].
    ///
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
                3 => Action::ToggleTodo(r.u32()?),
                4 => Action::GotoTodo(r.u32()?),
                5 => Action::GotoHeading(r.u32()?),
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
        f.pending = match r.u8()? {
            0 => false,
            1 => true,
            _ => return Err(DecodeError),
        };
        f.field = match r.u8()? {
            0 => None,
            1 => Some(Field {
                rect: Rect::get(&mut r)?,
                placeholder: r.str()?,
            }),
            _ => return Err(DecodeError),
        };
        r.finish()?;
        Ok(f)
    }
}

/// One note of the library, as the host lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The note's path relative to the library, such as `b/c.md`.
    pub name: String,
    /// Whether this is the note open in the editor.
    pub current: bool,
    /// Text for list rows, such as "3 links".
    pub detail: String,
}

/// Everything the plugin needs to build its screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// Screen width in device pixels.
    pub width: i32,
    /// Screen height in device pixels.
    pub height: i32,
    /// The host's exact UI scale factor, so `round(brief * scale)` gives the
    /// same pixels as the host.
    pub scale: f32,
    /// The host's UI font size in pixels.
    pub ui_px: f32,
    /// Bottom edge of the top bar the host draws itself; the plugin paints
    /// below it.
    pub top: i32,
    /// Top edge of the bottom bar the host draws itself; the plugin paints
    /// above it.
    pub bottom: i32,
    /// Every note in the library.
    pub notes: Vec<Note>,
    /// Directed links between notes, as indexes into `notes`.
    pub edges: Vec<(u32, u32)>,
    /// How many links reach a note (what footers say), which counts
    /// repeated links that `edges` lists once.
    pub links: u32,
    /// The plugin's own settings as the reader set them, by key: the chosen
    /// option's text as the plugin declared it ("List", "All", ...). Every
    /// declared setting is present.
    pub settings: Vec<(String, String)>,
    /// Panes: the pane's left edge. Zero for a screen.
    pub left: i32,
    /// Panes: the host's live to-do list (every box in the library, the
    /// open note and the scratch pad taken from their buffers). Empty for a
    /// screen.
    pub todos: Vec<TodoRow>,
    /// Panes: the headings of the open note, in order.
    pub headings: Vec<HeadingRow>,
}

/// One heading of the open note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingRow {
    /// 1 for `#` up to 6.
    pub level: u8,
    /// The line it starts on (0-based).
    pub line: u32,
    /// The heading text without the `#` marks.
    pub text: String,
}

/// One to-do as the host lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoRow {
    /// Index into [`Scene::notes`].
    pub note: u32,
    /// The line of the box in its note (0-based).
    pub line: u32,
    /// Whether the box is checked.
    pub done: bool,
    /// The to-do text after the box.
    pub text: String,
}

/// The stage of a drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The finger or pointer went down.
    Start,
    /// It moved while down.
    Move,
    /// It went up.
    End,
}

/// A message from the host to the plugin. The plugin answers each with a
/// [`Frame`].
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)] // Coordinates `x`, `y` are device pixels throughout.
pub enum Event {
    /// The screen opens (or the library changed while it was open).
    Open(Scene),
    /// A tap at a point.
    Tap { x: i32, y: i32 },
    /// A drag stage at a point.
    Drag { phase: Phase, x: i32, y: i32 },
    /// Mouse wheel notches, already adjusted for the reader's scroll
    /// direction setting; positive is down. `x`, `y` is the pointer (the
    /// screen centre when the host has no pointer position).
    Wheel { x: i32, y: i32, dy: i32 },
    /// A page-turn button; `forward` is the next page.
    PageTurn { forward: bool },
    /// The reader asked to go back (the back control or Esc).
    Back,
    /// The host is idle and the last frame was `pending`: do the next piece.
    Step,
}

impl Event {
    /// The event in the wire encoding. Strings longer than [`MAX_STR`] are
    /// cut.
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
                w.u32(s.settings.len() as u32);
                for (k, v) in &s.settings {
                    w.str(k);
                    w.str(v);
                }
                w.i32(s.left);
                w.u32(s.todos.len() as u32);
                for t in &s.todos {
                    w.u32(t.note);
                    w.u32(t.line);
                    w.u8(u8::from(t.done));
                    w.str(&t.text);
                }
                w.u32(s.headings.len() as u32);
                for h in &s.headings {
                    w.u8(h.level);
                    w.u32(h.line);
                    w.str(&h.text);
                }
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
            Event::Step => w.u8(6),
        }
        w.0
    }

    /// Decodes an event written by [`Event::encode`].
    ///
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
                let mut settings = Vec::new();
                for _ in 0..r.count(MAX_SETTINGS, 8)? {
                    settings.push((r.str()?, r.str()?));
                }
                let left = r.i32()?;
                let mut todos = Vec::new();
                for _ in 0..r.count(MAX_TODOS, 13)? {
                    todos.push(TodoRow {
                        note: r.u32()?,
                        line: r.u32()?,
                        done: match r.u8()? {
                            0 => false,
                            1 => true,
                            _ => return Err(DecodeError),
                        },
                        text: r.str()?,
                    });
                }
                let mut headings = Vec::new();
                for _ in 0..r.count(MAX_HEADINGS, 9)? {
                    headings.push(HeadingRow {
                        level: r.u8()?,
                        line: r.u32()?,
                        text: r.str()?,
                    });
                }
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
                    settings,
                    left,
                    todos,
                    headings,
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
            6 => Event::Step,
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
            actions: vec![
                Action::OpenNote(3),
                Action::PreviewNote(1),
                Action::ToggleTodo(2),
                Action::GotoTodo(0),
                Action::GotoHeading(0),
                Action::Close,
            ],
            redraw: Redraw::Clean,
            pending: true,
            field: Some(Field {
                rect: Rect::new(4, 60, 300, 500),
                placeholder: "A side note.".into(),
            }),
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
            settings: vec![
                ("view".into(), "List".into()),
                ("labels".into(), "All".into()),
            ],
            left: 640,
            todos: vec![
                TodoRow {
                    note: 1,
                    line: 4,
                    done: false,
                    text: "call Ana".into(),
                },
                TodoRow {
                    note: 0,
                    line: 0,
                    done: true,
                    text: "ship 日本".into(),
                },
            ],
            headings: vec![HeadingRow {
                level: 2,
                line: 7,
                text: "Part two".into(),
            }],
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
            Event::Step,
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
        // An Open event claiming MAX_TODOS + 1 to-dos.
        let mut w = Writer::default();
        w.u8(0);
        for _ in 0..4 {
            w.i32(0);
        }
        w.f32(1.0);
        w.f32(1.0);
        w.u32(0); // notes
        w.u32(0); // edges
        w.u32(0); // links
        w.u32(0); // settings
        w.i32(0); // left
        w.u32(MAX_TODOS as u32 + 1);
        assert!(Event::decode(&w.0).is_err());
        // An Open event claiming MAX_HEADINGS + 1 headings.
        let mut w = Writer::default();
        w.u8(0);
        for _ in 0..4 {
            w.i32(0);
        }
        w.f32(1.0);
        w.f32(1.0);
        for _ in 0..4 {
            w.u32(0); // notes, edges, links, settings
        }
        w.i32(0); // left
        w.u32(0); // todos
        w.u32(MAX_HEADINGS as u32 + 1);
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
