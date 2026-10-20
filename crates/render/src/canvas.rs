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
}

#[cfg(test)]
mod tests {
    use super::*;

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
