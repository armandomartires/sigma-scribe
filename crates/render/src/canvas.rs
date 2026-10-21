//! A plain greyscale framebuffer (0 = black, 255 = white).

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    #[must_use]
    pub fn union(&self, o: &Rect) -> Rect {
        if self.w <= 0 || self.h <= 0 {
            return *o;
        }
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect::new(
            x,
            y,
            (self.x + self.w).max(o.x + o.w) - x,
            (self.y + self.h).max(o.y + o.h) - y,
        )
    }

    /// The overlap of two rectangles (empty when they do not meet).
    #[must_use]
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        Rect::new(x0, y0, (x1 - x0).max(0), (y1 - y0).max(0))
    }

    #[must_use]
    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(self.x + d, self.y + d, self.w - 2 * d, self.h - 2 * d)
    }
}

#[derive(Clone, Debug)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    #[must_use]
    pub fn new(width: u32, height: u32, fill: u8) -> Self {
        Self {
            width,
            height,
            pixels: vec![fill; (width * height) as usize],
        }
    }

    #[must_use]
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width as i32, self.height as i32)
    }

    fn clip(&self, r: Rect) -> Option<(usize, usize, usize, usize)> {
        let x0 = r.x.max(0);
        let y0 = r.y.max(0);
        let x1 = (r.x + r.w).min(self.width as i32);
        let y1 = (r.y + r.h).min(self.height as i32);
        (x1 > x0 && y1 > y0).then_some((x0 as usize, y0 as usize, x1 as usize, y1 as usize))
    }

    pub fn fill(&mut self, r: Rect, v: u8) {
        if let Some((x0, y0, x1, y1)) = self.clip(r) {
            let w = self.width as usize;
            for y in y0..y1 {
                self.pixels[y * w + x0..y * w + x1].fill(v);
            }
        }
    }

    /// 1 px (or `t` px) outline.
    pub fn stroke(&mut self, r: Rect, t: i32, v: u8) {
        self.fill(Rect::new(r.x, r.y, r.w, t), v);
        self.fill(Rect::new(r.x, r.y + r.h - t, r.w, t), v);
        self.fill(Rect::new(r.x, r.y, t, r.h), v);
        self.fill(Rect::new(r.x + r.w - t, r.y, t, r.h), v);
    }

    /// Blends coverage `a` (0..=255) of colour `v` onto a pixel.
    pub fn blend(&mut self, x: i32, y: i32, v: u8, a: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 || a == 0 {
            return;
        }
        let i = y as usize * self.width as usize + x as usize;
        let bg = u32::from(self.pixels[i]);
        let a32 = u32::from(a);
        self.pixels[i] = ((u32::from(v) * a32 + bg * (255 - a32)) / 255) as u8;
    }

    /// Copies a sub-rectangle out (for partial-refresh blits).
    #[must_use]
    pub fn crop(&self, r: Rect) -> (Rect, Vec<u8>) {
        let Some((x0, y0, x1, y1)) = self.clip(r) else {
            return (Rect::default(), Vec::new());
        };
        let w = self.width as usize;
        let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0));
        for y in y0..y1 {
            out.extend_from_slice(&self.pixels[y * w + x0..y * w + x1]);
        }
        (
            Rect::new(x0 as i32, y0 as i32, (x1 - x0) as i32, (y1 - y0) as i32),
            out,
        )
    }

    /// Bounding box of pixels that differ from `other` (same size), if any.
    #[must_use]
    pub fn diff(&self, other: &Canvas) -> Option<Rect> {
        let w = self.width as usize;
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for (i, (a, b)) in self.pixels.iter().zip(&other.pixels).enumerate() {
            if a != b {
                let (x, y) = (i % w, i / w);
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
        (x0 != usize::MAX).then(|| {
            Rect::new(
                x0 as i32,
                y0 as i32,
                (x1 - x0 + 1) as i32,
                (y1 - y0 + 1) as i32,
            )
        })
    }

    /// Changed areas as separate rectangles, so e-ink only refreshes what
    /// changed (a key cap and a text row, not everything between them).
    /// Changed rows closer than `gap` px merge into one band; each band is
    /// then split into column clusters closer than `gap`. More than `max`
    /// rectangles collapse into the single bounding box.
    #[must_use]
    pub fn diff_regions(&self, other: &Canvas, gap: i32, max: usize) -> Vec<Rect> {
        let w = self.width as usize;
        let row_span = |y: usize| -> Option<(usize, usize)> {
            let a = &self.pixels[y * w..(y + 1) * w];
            let b = &other.pixels[y * w..(y + 1) * w];
            let x0 = a.iter().zip(b).position(|(p, q)| p != q)?;
            let x1 = w
                - 1
                - a.iter()
                    .rev()
                    .zip(b.iter().rev())
                    .position(|(p, q)| p != q)?;
            Some((x0, x1))
        };
        let mut bands: Vec<(usize, usize)> = Vec::new();
        for y in 0..self.height as usize {
            if row_span(y).is_some() {
                match bands.last_mut() {
                    Some((_, end)) if y as i32 - *end as i32 <= gap => *end = y,
                    _ => bands.push((y, y)),
                }
            }
        }
        let mut out = Vec::new();
        for (y0, y1) in bands {
            // Changed columns within the band.
            let mut cols = vec![false; w];
            for y in y0..=y1 {
                let (a, b) = (
                    &self.pixels[y * w..(y + 1) * w],
                    &other.pixels[y * w..(y + 1) * w],
                );
                for x in 0..w {
                    if a[x] != b[x] {
                        cols[x] = true;
                    }
                }
            }
            let mut start: Option<usize> = None;
            let mut last = 0usize;
            for (x, &on) in cols.iter().enumerate() {
                if on {
                    if let Some(s0) = start {
                        if x as i32 - last as i32 > gap {
                            out.push(Rect::new(
                                s0 as i32,
                                y0 as i32,
                                (last - s0 + 1) as i32,
                                (y1 - y0 + 1) as i32,
                            ));
                            start = Some(x);
                        }
                    } else {
                        start = Some(x);
                    }
                    last = x;
                }
            }
            if let Some(s0) = start {
                out.push(Rect::new(
                    s0 as i32,
                    y0 as i32,
                    (last - s0 + 1) as i32,
                    (y1 - y0 + 1) as i32,
                ));
            }
        }
        if out.len() > max {
            return self
                .diff(other)
                .into_iter()
                .map(|r| self.min_region(r))
                .collect();
        }
        out.into_iter().map(|r| self.min_region(r)).collect()
    }

    /// Grows a region to at least [`MIN_REGION`] px each way, inside the
    /// canvas. FBInk on the Forma rejects 1-px-wide regions ("bogus empty
    /// region") and the refresh fails (seen on device 2026-10-21 00:25).
    #[must_use]
    pub fn min_region(&self, r: Rect) -> Rect {
        let grow = |p: i32, len: i32, max: i32| {
            if len >= MIN_REGION {
                return (p, len);
            }
            let len2 = MIN_REGION.min(max);
            let p2 = (p - (len2 - len) / 2).clamp(0, (max - len2).max(0));
            (p2, len2)
        };
        let (x, w) = grow(r.x, r.w, self.width as i32);
        let (y, h) = grow(r.y, r.h, self.height as i32);
        Rect::new(x, y, w, h)
    }
}

/// Smallest refresh region sent to the panel, in px.
pub const MIN_REGION: i32 = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thin_regions_are_padded_inside_the_canvas() {
        let a = Canvas::new(100, 100, 255);
        let mut b = a.clone();
        b.fill(Rect::new(40, 10, 1, 34), 0); // the 1x34 region FBInk refused
        b.fill(Rect::new(99, 99, 1, 1), 0); // a corner pixel
        let r = b.diff_regions(&a, 4, 8);
        assert_eq!(
            r,
            vec![Rect::new(33, 10, 16, 34), Rect::new(84, 84, 16, 16)]
        );
        for x in r {
            assert!(
                x.w >= MIN_REGION
                    && x.h >= MIN_REGION
                    && x.x >= 0
                    && x.y >= 0
                    && x.x + x.w <= 100
                    && x.y + x.h <= 100
            );
        }
    }

    #[test]
    fn diff_regions_split_distant_changes() {
        let a = Canvas::new(100, 100, 255);
        let mut b = a.clone();
        b.fill(Rect::new(5, 5, 10, 4), 0); // a text row
        b.fill(Rect::new(60, 80, 8, 8), 0); // a key cap
        b.fill(Rect::new(70, 82, 2, 2), 10); // next to the cap: same rect
        let r = b.diff_regions(&a, 4, 8);
        assert_eq!(
            r,
            vec![Rect::new(2, 0, 16, 16), Rect::new(58, 76, 16, 16)],
            "padded to MIN_REGION"
        );
        assert_eq!(b.diff_regions(&a, 4, 1), vec![Rect::new(5, 5, 67, 83)]);
        assert!(a.diff_regions(&a, 4, 8).is_empty());
    }

    #[test]
    fn fill_clips_and_diff_finds_change() {
        let mut a = Canvas::new(10, 10, 255);
        let b = a.clone();
        a.fill(Rect::new(-5, 8, 7, 9), 0);
        assert_eq!(a.diff(&b), Some(Rect::new(0, 8, 2, 2)));
        let (r, px) = a.crop(Rect::new(0, 8, 2, 2));
        assert_eq!(r.w * r.h, 4);
        assert!(px.iter().all(|&p| p == 0));
    }
}
