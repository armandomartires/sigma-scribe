//! Ink (v1.5.0, plan §12.3): strokes drawn in the writing panel and the
//! files they are saved as. No recognition yet.
//!
//! A saved ink block is two files next to the note:
//! `<note dir>/.ink/<note stem>/ink-NNN.svg` (black polylines in mm, any
//! viewer shows it) and `ink-NNN.strokes.json` (raw points with times, for
//! re-recognition later). The note gets `![ink](.ink/<stem>/ink-NNN.svg)`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// One touch report inside the panel, panel pixels and ms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    /// Horizontal position, panel pixels.
    pub x: i32,
    /// Vertical position, panel pixels.
    pub y: i32,
    /// Time in ms.
    pub t: u64,
    /// Contact size (`ABS_MT_TOUCH_MAJOR`, device units) when the panel
    /// reports it (v1.5.1); `None` otherwise.
    pub major: Option<u16>,
}

/// One stroke: the points from touch down to touch up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stroke {
    /// The points kept (jitter is dropped), in the order they came.
    pub points: Vec<Point>,
}

/// Pen width: a finger or a passive capacitive stylus (same contacts on
/// the Forma; only the drawn width differs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pen {
    /// A finger: the wider line.
    #[default]
    Finger,
    /// A stylus: the thinner line.
    Stylus,
}

impl Pen {
    /// Drawn width in mm (plan §12.3: 1.2 / 0.6 mm).
    #[must_use]
    pub fn width_mm(self) -> f32 {
        match self {
            Self::Finger => 1.2,
            Self::Stylus => 0.6,
        }
    }
}

/// Panel pixels per mm: the Forma is 300 ppi.
pub const PX_PER_MM: f32 = 300.0 / 25.4;

/// Points closer than this (px) to the last one are dropped (jitter).
pub const MIN_STEP_PX: i32 = 2;

/// The strokes in the panel, with undo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ink {
    /// Finished strokes, oldest first.
    pub strokes: Vec<Stroke>,
    /// Stroke being drawn (finger still down).
    pub live: Option<Stroke>,
    /// Strokes taken by Clear, so Undo can bring them back.
    cleared: Vec<Vec<Stroke>>,
}

impl Ink {
    /// Whether there are no strokes, finished or live.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty() && self.live.is_none()
    }

    /// Starts a live stroke at `p`, replacing any live one.
    pub fn down(&mut self, p: Point) {
        self.live = Some(Stroke { points: vec![p] });
    }

    /// Adds a point; returns the new segment (from, to) to draw, or `None`
    /// if it was jitter.
    pub fn moved(&mut self, p: Point) -> Option<(Point, Point)> {
        let s = self.live.as_mut()?;
        let last = *s.points.last()?;
        if (p.x - last.x).abs().max((p.y - last.y).abs()) < MIN_STEP_PX {
            return None;
        }
        s.points.push(p);
        Some((last, p))
    }

    /// Ends the live stroke (a tap leaves a dot).
    pub fn up(&mut self) {
        if let Some(s) = self.live.take() {
            self.strokes.push(s);
            self.cleared.clear();
        }
    }

    /// Undo: the last stroke, or a Clear. Returns whether anything changed.
    pub fn undo(&mut self) -> bool {
        self.live = None;
        if let Some(back) = self.cleared.pop() {
            self.strokes = back;
            return true;
        }
        self.strokes.pop().is_some()
    }

    /// Clear all strokes (undoable).
    pub fn clear(&mut self) -> bool {
        self.live = None;
        if self.strokes.is_empty() {
            return false;
        }
        self.cleared.push(std::mem::take(&mut self.strokes));
        true
    }

    /// Whether any point carries a contact size.
    #[must_use]
    pub fn has_contact(&self) -> bool {
        self.strokes
            .iter()
            .flat_map(|s| s.points.iter())
            .any(|p| p.major.is_some())
    }

    /// Bounding box of all points: (x0, y0, x1, y1).
    #[must_use]
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut it = self.strokes.iter().flat_map(|s| s.points.iter());
        let f = it.next()?;
        Some(it.fold((f.x, f.y, f.x, f.y), |(a, b, c, d), p| {
            (a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y))
        }))
    }

    /// The SVG file: black round polylines in mm, cropped to the ink with
    /// a margin of the pen width.
    #[must_use]
    pub fn to_svg(&self, pen: Pen) -> Option<String> {
        let (x0, y0, x1, y1) = self.bounds()?;
        let w = pen.width_mm();
        let mm = |v: i32, o: i32| (v - o) as f32 / PX_PER_MM + w;
        let (vw, vh) = (mm(x1, x0) + w, mm(y1, y0) + w);
        let mut s = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{vw:.2}mm\" height=\"{vh:.2}mm\" \
             viewBox=\"0 0 {vw:.2} {vh:.2}\">\n<g fill=\"none\" stroke=\"#000\" stroke-width=\"{w:.2}\" \
             stroke-linecap=\"round\" stroke-linejoin=\"round\">\n"
        );
        for st in &self.strokes {
            let mut d = String::new();
            for (i, p) in st.points.iter().enumerate() {
                let _ = write!(
                    d,
                    "{}{:.2} {:.2}",
                    if i == 0 { "M" } else { " L" },
                    mm(p.x, x0),
                    mm(p.y, y0)
                );
            }
            if st.points.len() == 1 {
                d.push_str(" l0 0");
            }
            let _ = writeln!(s, "<path d=\"{d}\"/>");
        }
        s.push_str("</g>\n</svg>\n");
        Some(s)
    }

    /// The raw strokes: panel px, ms since the first point.
    #[must_use]
    pub fn to_strokes_json(&self, pen: Pen) -> String {
        let t0 = self
            .strokes
            .first()
            .and_then(|s| s.points.first())
            .map_or(0, |p| p.t);
        let mut s = format!(
            "{{\"format\":\"sigma-scribe-ink\",\"version\":2,\"px_per_mm\":{PX_PER_MM:.4},\"pen\":\"{}\",\"contact\":{},\"strokes\":[",
            match pen {
                Pen::Finger => "finger",
                Pen::Stylus => "stylus",
            },
            if self.has_contact() {
                "\"touch_major\""
            } else {
                "null"
            }
        );
        for (i, st) in self.strokes.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push('[');
            for (j, p) in st.points.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                let _ = write!(s, "[{},{},{}", p.x, p.y, p.t.saturating_sub(t0));
                if let Some(m) = p.major {
                    let _ = write!(s, ",{m}");
                }
                s.push(']');
            }
            s.push(']');
        }
        s.push_str("]}\n");
        s
    }
}

/// Where the next ink block of `note` goes: (svg path, strokes path, the
/// Markdown line). Picks the first free `ink-NNN`.
#[must_use]
pub fn next_paths(note: &Path) -> (PathBuf, PathBuf, String) {
    let dir = note.parent().unwrap_or(Path::new("."));
    let stem = note
        .file_stem()
        .map_or_else(|| "note".to_owned(), |s| s.to_string_lossy().into_owned());
    let ink_dir = dir.join(".ink").join(&stem);
    let mut n = 1;
    loop {
        let name = format!("ink-{n:03}");
        let svg = ink_dir.join(format!("{name}.svg"));
        if !svg.exists() {
            let json = ink_dir.join(format!("{name}.strokes.json"));
            let md = format!("![ink](.ink/{stem}/{name}.svg)");
            return (svg, json, md);
        }
        n += 1;
    }
}

/// Writes both files for `ink` next to `note`; returns the Markdown line
/// to insert, or `None` (and writes nothing) when there are no strokes.
///
/// # Errors
/// A filesystem error creating the folder or writing either file.
pub fn save(ink: &Ink, pen: Pen, note: &Path) -> std::io::Result<Option<String>> {
    let Some(svg) = ink.to_svg(pen) else {
        return Ok(None);
    };
    let (svg_path, json_path, md) = next_paths(note);
    if let Some(d) = svg_path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(&json_path, ink.to_strokes_json(pen))?;
    std::fs::write(&svg_path, svg)?;
    Ok(Some(md))
}

fn ink_dir(note: &Path) -> Option<(PathBuf, String)> {
    let stem = note.file_stem()?.to_string_lossy().into_owned();
    Some((note.parent()?.join(".ink").join(&stem), stem))
}

/// After a note moved from `old` to `new` (v1.5.1): moves its
/// `.ink/<stem>/` folder along and rewrites the note's `](.ink/<old>/`
/// links when the stem changed. A taken target folder gets `-2`, `-3`…
/// Nothing happens for a note without ink.
///
/// # Errors
/// A filesystem error moving the folder or rewriting the note.
pub fn carry(old: &Path, new: &Path) -> std::io::Result<()> {
    let (Some((from, old_stem)), Some((base, new_stem))) = (ink_dir(old), ink_dir(new)) else {
        return Ok(());
    };
    if !from.is_dir() || from == base {
        return Ok(());
    }
    let (mut to, mut stem) = (base.clone(), new_stem.clone());
    let mut n = 2;
    while to.exists() {
        stem = format!("{new_stem}-{n}");
        to = base.with_file_name(&stem);
        n += 1;
    }
    if let Some(d) = to.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::rename(&from, &to)?;
    if let Some(d) = from.parent() {
        // Leave no empty `.ink/` behind.
        let _ = std::fs::remove_dir(d);
    }
    if stem != old_stem {
        let text = std::fs::read_to_string(new)?;
        let fixed = text.replace(&format!("](.ink/{old_stem}/"), &format!("](.ink/{stem}/"));
        if fixed != text {
            crate::store::write_atomic(new, &fixed)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i32, y: i32, t: u64) -> Point {
        Point {
            x,
            y,
            t,
            major: None,
        }
    }

    fn two_strokes() -> Ink {
        let mut i = Ink::default();
        i.down(p(10, 10, 100));
        assert_eq!(i.moved(p(11, 10, 110)), None, "jitter dropped");
        assert_eq!(
            i.moved(p(40, 10, 120)),
            Some((p(10, 10, 100), p(40, 10, 120)))
        );
        i.up();
        i.down(p(20, 30, 200));
        i.up();
        i
    }

    #[test]
    fn undo_and_clear() {
        let mut i = two_strokes();
        assert_eq!(i.strokes.len(), 2);
        assert!(i.undo());
        assert_eq!(i.strokes.len(), 1);
        assert!(i.clear());
        assert!(i.is_empty());
        assert!(i.undo(), "undo brings a Clear back");
        assert_eq!(i.strokes.len(), 1);
        assert!(i.undo());
        assert!(!i.undo());
        assert!(!i.clear());
    }

    #[test]
    fn svg_is_black_mm_and_cropped() {
        let s = two_strokes().to_svg(Pen::Finger).unwrap();
        assert!(s.contains("stroke=\"#000\"") && s.contains("stroke-width=\"1.20\""));
        assert!(s.contains("<path d=\"M1.20 1.20 L3.74 1.20\"/>"), "{s}");
        assert!(s.contains(" l0 0"), "a tap is a dot");
        assert_eq!(s.matches("<path").count(), 2);
        assert!(Ink::default().to_svg(Pen::Finger).is_none());
    }

    #[test]
    fn strokes_json_keeps_points_and_relative_times() {
        let j = two_strokes().to_strokes_json(Pen::Stylus);
        assert!(j.contains("\"pen\":\"stylus\""));
        assert!(
            j.contains("\"strokes\":[[[10,10,0],[40,10,20]],[[20,30,100]]]"),
            "{j}"
        );
    }

    #[test]
    fn carry_moves_the_folder_and_rewrites_links() {
        let d = std::env::temp_dir().join(format!("sigma-carry-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        let note = d.join("trip.md");
        let md = save(&two_strokes(), Pen::Finger, &note).unwrap().unwrap();
        std::fs::write(&note, format!("a\n{md}\n[x](.ink/trip/no)\n")).unwrap();
        // Rename trip -> walk (the file has already moved).
        let walk = d.join("walk.md");
        std::fs::rename(&note, &walk).unwrap();
        carry(&note, &walk).unwrap();
        assert!(d.join(".ink/walk/ink-001.svg").exists() && !d.join(".ink/trip").exists());
        let t = std::fs::read_to_string(&walk).unwrap();
        assert!(t.contains("![ink](.ink/walk/ink-001.svg)") && t.contains("](.ink/walk/no)"));
        // Move into sub/ with the same stem: folder follows, text unchanged.
        let moved = d.join("sub/walk.md");
        std::fs::rename(&walk, &moved).unwrap();
        carry(&walk, &moved).unwrap();
        assert!(d.join("sub/.ink/walk/ink-001.svg").exists());
        assert!(!d.join(".ink").exists(), "empty .ink removed");
        assert_eq!(std::fs::read_to_string(&moved).unwrap(), t);
        // A taken target folder gets a suffix.
        std::fs::create_dir_all(d.join("sub/.ink/run")).unwrap();
        let run = d.join("sub/run.md");
        std::fs::rename(&moved, &run).unwrap();
        carry(&moved, &run).unwrap();
        assert!(d.join("sub/.ink/run-2/ink-001.svg").exists());
        assert!(std::fs::read_to_string(&run)
            .unwrap()
            .contains("](.ink/run-2/ink-001.svg)"));
        // No ink: nothing to do.
        std::fs::write(d.join("plain.md"), "x").unwrap();
        carry(&d.join("plain.md"), &d.join("plain.md")).unwrap();
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn save_picks_the_next_free_name() {
        let d = std::env::temp_dir().join(format!("sigma-ink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let note = d.join("trip.md");
        let ink = two_strokes();
        let a = save(&ink, Pen::Finger, &note).unwrap().unwrap();
        let b = save(&ink, Pen::Finger, &note).unwrap().unwrap();
        assert_eq!(a, "![ink](.ink/trip/ink-001.svg)");
        assert_eq!(b, "![ink](.ink/trip/ink-002.svg)");
        assert!(d.join(".ink/trip/ink-002.strokes.json").exists());
        assert_eq!(save(&Ink::default(), Pen::Finger, &note).unwrap(), None);
        let _ = std::fs::remove_dir_all(&d);
    }
}
