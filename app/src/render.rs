//! Drawing the frame and sending it to the panel.

use crate::{App, RenameTarget};
use sigma_core::settings::Rotation;
use sigma_device::{Device, Refresh};
use sigma_render::editor::Model;
use sigma_render::{Canvas, FindModel, PreviewModel, Rect};

/// Grows a region to at least 16×16 (refresh rule), inside `b`.
fn min16(r: Rect, b: Rect) -> Rect {
    let w = r.w.max(16).min(b.w);
    let h = r.h.max(16).min(b.h);
    let x = r.x.min(b.x + b.w - w).max(b.x);
    let y = r.y.min(b.y + b.h - h).max(b.y);
    Rect::new(x, y, w, h)
}

/// Copies rectangle `r` of `src` into `dst` (same size canvases).
fn copy_rect(dst: &mut Canvas, src: &Canvas, r: Rect) {
    let r = r.intersect(&dst.bounds());
    if r.w <= 0 || r.h <= 0 || dst.width != src.width || dst.height != src.height {
        return;
    }
    let w = dst.width as usize;
    for y in r.y..r.y + r.h {
        let a = y as usize * w + r.x as usize;
        let e = a + r.w as usize;
        dst.pixels[a..e].copy_from_slice(&src.pixels[a..e]);
    }
}

impl App {
    pub(crate) fn present(
        &mut self,
        dev: &mut dyn Device,
        r: Rect,
        refresh: Refresh,
    ) -> std::io::Result<()> {
        let rot = self.settings.rotation;
        if rot == Rotation::R0 {
            return dev.present(&self.canvas, r, refresh);
        }
        let (lw, lh) = (self.canvas.width as i32, self.canvas.height as i32);
        let pw = self.phys_canvas.width as usize;
        let (x0, y0) = (r.x.max(0), r.y.max(0));
        let (x1, y1) = ((r.x + r.w).min(lw), (r.y + r.h).min(lh));
        for y in y0..y1 {
            for x in x0..x1 {
                let v = self.canvas.pixels[(y * lw + x) as usize];
                let (px, py) = self.to_panel(x, y);
                self.phys_canvas.pixels[py as usize * pw + px as usize] = v;
            }
        }
        // Landscape swaps width and height, so the >= 16 px minimum holds.
        let pr = match rot {
            Rotation::R0 => r,
            Rotation::R90 => Rect::new(lh - y1, x0, y1 - y0, x1 - x0),
            Rotation::R180 => Rect::new(lw - x1, lh - y1, x1 - x0, y1 - y0),
            Rotation::R270 => Rect::new(y0, lw - x1, y1 - y0, x1 - x0),
        };
        dev.present(&self.phys_canvas, pr, refresh)
    }

    /// Redraws and sends only what changed, with the given refresh class.
    ///
    /// # Errors
    /// The device could not present the frame.
    pub fn render(&mut self, dev: &mut dyn Device, refresh: Refresh) -> std::io::Result<()> {
        self.sync_pane_plugins();
        self.screen.overwrite = self.buffer.overwrite();
        if refresh == Refresh::Pointer
            && std::mem::take(&mut self.pointer_fast)
            && self.base.pixels.len() == self.canvas.pixels.len()
        {
            return self.render_pointer_only(dev);
        }
        if let Some(ink) = &mut self.ink {
            if matches!(refresh, Refresh::Ink | Refresh::InkA2) && std::mem::take(&mut ink.fast) {
                if let Some(r) = ink.pending.take() {
                    let r = min16(r, self.canvas.bounds());
                    self.present(dev, r, refresh)?;
                    copy_rect(&mut self.shown, &self.canvas, r);
                }
                return Ok(());
            }
            if refresh == Refresh::PointerCleanup && std::mem::take(&mut ink.cleanup) {
                if let Some(r) = ink.dirty.take() {
                    let r = min16(r, self.canvas.bounds());
                    self.present(dev, r, Refresh::PointerCleanup)?;
                }
                return Ok(());
            }
        }
        self.pointer_fast = false;
        self.screen.back_on = !self.history.is_empty();
        let mut c = std::mem::replace(&mut self.canvas, Canvas::new(1, 1, 0));
        if self.options {
            self.screen.draw_options(&mut c, &self.options_model());
        } else if let Some(h) = &self.help {
            self.screen.draw_help(&mut c, &self.help_model(h));
        } else if let Some((frame, title)) = self.plugin_drawable() {
            self.screen
                .draw_plugin_screen(&mut c, frame, title, self.theme(), &self.keyboard);
        } else if let Some(g) = &self.graph {
            self.screen.draw_graph(&mut c, &self.graph_model(g));
        } else if let Some(p) = &self.preview {
            let m = PreviewModel {
                file_name: self.current.name(),
                doc: &p.doc,
                page: p.page,
                words: sigma_core::text::word_count(self.buffer.text()),
                save: &self.autosave.state,
                keyboard: &self.keyboard,
                theme: self.theme(),
            };
            self.screen.drawer_open = if self.drawer_visible() {
                self.drawer
            } else {
                [false; 3]
            };
            let l = self.screen.draw_preview(&mut c, &m);
            if self.drawer_visible() {
                let (hs, todos) = (self.drawer_headings(), self.todo_list());
                self.screen
                    .draw_drawer(&mut c, &l, &self.drawer_model(&hs, &todos));
            }
        } else if self.files.is_some() {
            let rows = self.rows();
            if let Some(m) = self.files_model(&rows) {
                self.screen.draw_files(&mut c, &m);
            }
        } else {
            let renaming_current = matches!(&self.rename, Some((RenameTarget::Current, _)));
            let model = Model {
                file_name: self.current.name(),
                buffer: &self.buffer,
                keyboard: &self.keyboard,
                keys_on: self.keys_visible(),
                save: &self.autosave.state,
                theme: self.theme(),
                sel_ui: if self.rename.is_some() {
                    None
                } else {
                    self.sel_ui
                },
                rename: if renaming_current {
                    self.rename.as_ref().map(|(_, b)| b)
                } else {
                    None
                },
            };
            self.screen.drawer_open = if self.drawer_visible() {
                self.drawer
            } else {
                [false; 3]
            };
            let l = self.screen.draw(&mut c, &model);
            if let Some(f) = &self.find {
                let label = self.find_label();
                self.screen.draw_find(
                    &mut c,
                    &l,
                    &FindModel {
                        query: &f.query,
                        replace: &f.replace,
                        focus: f.focus,
                        case: f.case,
                        label: &label,
                        theme: self.theme(),
                    },
                );
            }
            if self.drawer_visible() {
                let (hs, todos) = (self.headings(), self.todo_list());
                self.screen
                    .draw_drawer(&mut c, &l, &self.drawer_model(&hs, &todos));
            }
        }
        if self.drawer_visible() {
            let base = self.drawer_base();
            self.screen
                .draw_drawer_handle(&mut c, &base, self.settings.drawer, self.theme());
        } else if self.drawer_tab_shown() {
            let base = self.drawer_base();
            self.screen
                .draw_drawer_tab(&mut c, &base, self.settings.drawer, self.theme());
        }
        self.draw_drawer_menu(&mut c);
        let v = self.ink_view();
        if let (Some(ink), Some(v)) = (&mut self.ink, v) {
            let l = self.screen.ink_layout_for(ink.mode, &v);
            if ink.mode == sigma_render::ink::InkMode::Draw {
                self.screen
                    .draw_ink(&mut c, &l, &ink.ink, self.settings.ink_pen);
            } else if ink.mode == sigma_render::ink::InkMode::Free {
                self.screen
                    .draw_free(&mut c, &l, &ink.ink, self.settings.ink_pen, &v);
            } else {
                self.screen
                    .draw_letters(&mut c, &l, &ink.ink, self.settings.ink_pen, ink.mode, &v);
            }
            ink.pending = None;
            ink.fast = false;
        }
        if self.pointer.live {
            self.base.clone_from(&c);
        }
        self.pointer.drawn = None;
        if self.pointer.visible && self.pointer.live {
            let cell = sigma_render::pointer::pointer_cell(self.screen.scale().px(1.0) as f32);
            sigma_render::pointer::draw_pointer(
                &mut c,
                self.pointer.x,
                self.pointer.y,
                cell,
                self.theme(),
            );
            let pr = sigma_render::pointer::pointer_rect(self.pointer.x, self.pointer.y, cell)
                .intersect(&c.bounds());
            self.pointer.trail = Some(self.pointer.trail.map_or(pr, |t| t.union(&pr)));
            self.pointer.drawn = Some(pr);
        }
        self.canvas = c;
        // Only what changed goes to the panel: a pressed modifier redraws its
        // cap, a typed letter its text row and the word count.
        let mut regions = if refresh == Refresh::Full {
            vec![self.canvas.bounds()]
        } else {
            self.canvas.diff_regions(&self.shown, 24, 8)
        };
        // v1.0.1: the arrow's erase and draw go out as ONE update, so the
        // panel never shows the old arrow's residue next to the new one.
        if matches!(refresh, Refresh::Pointer | Refresh::PointerCleanup) && regions.len() > 1 {
            let u = regions.iter().skip(1).fold(regions[0], |a, r| a.union(r));
            regions = vec![u];
        }
        let regions: Vec<Rect> = regions
            .into_iter()
            .map(|r| min16(r, self.canvas.bounds()))
            .collect();
        for r in &regions {
            self.present(dev, *r, refresh)?;
        }
        // When the arrow hides: one non-flashing GL16 pass over its path
        // since the last cleanup, clearing the DU trails (v1.0.3, S10.3-D2).
        if std::mem::take(&mut self.pointer.cleanup) {
            if let Some(t) = self.pointer.trail.take() {
                if refresh != Refresh::Full {
                    self.present(dev, min16(t, self.canvas.bounds()), Refresh::PointerCleanup)?;
                }
            }
        }
        if !regions.is_empty() {
            self.shown.pixels.copy_from_slice(&self.canvas.pixels);
        }
        Ok(())
    }

    /// v1.0.4: a pointer-only frame. Restores the old arrow's pixels from
    /// the last full frame and draws the new one; no full redraw (the
    /// 1.0.3 device log showed ~170 ms of CPU per move from full redraws,
    /// which backed up the event queue).
    fn render_pointer_only(&mut self, dev: &mut dyn Device) -> std::io::Result<()> {
        let b = self.canvas.bounds();
        let mut region = self.pointer.drawn.take();
        if let Some(o) = region {
            copy_rect(&mut self.canvas, &self.base, o);
        }
        if self.pointer.visible && self.pointer.live {
            let cell = sigma_render::pointer::pointer_cell(self.screen.scale().px(1.0) as f32);
            let theme = self.theme();
            sigma_render::pointer::draw_pointer(
                &mut self.canvas,
                self.pointer.x,
                self.pointer.y,
                cell,
                theme,
            );
            let pr = sigma_render::pointer::pointer_rect(self.pointer.x, self.pointer.y, cell)
                .intersect(&b);
            self.pointer.trail = Some(self.pointer.trail.map_or(pr, |t| t.union(&pr)));
            self.pointer.drawn = Some(pr);
            region = Some(region.map_or(pr, |r| r.union(&pr)));
        }
        if let Some(r) = region {
            let r = min16(r, b);
            self.present(dev, r, Refresh::Pointer)?;
            copy_rect(&mut self.shown, &self.canvas, r);
        }
        if std::mem::take(&mut self.pointer.cleanup) {
            if let Some(t) = self.pointer.trail.take() {
                let t = min16(t, b);
                self.present(dev, t, Refresh::PointerCleanup)?;
            }
        }
        Ok(())
    }
}
