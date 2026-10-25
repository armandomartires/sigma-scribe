//! v1.13.1: the scroll bar of Settings, Help and Preview answers a finger
//! the way the editor's does: drag the thumb, or tap above or below it.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Phase, Refresh};
use sigma_render::{HelpPage, SettingsPage};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sigma-pbar-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(1440, 1920, dir.clone());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self {
            app,
            dev,
            t: 0,
            dir,
        }
    }
    fn touch(&mut self, phase: Phase, x: i32, y: i32) {
        self.t += 30;
        let t = self.t;
        if let Some(r) = self.app.handle(Event::Touch { phase, x, y, t }) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }
    fn tap(&mut self, x: i32, y: i32) {
        self.touch(Phase::Down, x, y);
        self.touch(Phase::Up, x, y);
    }
    fn drag(&mut self, x: i32, y0: i32, y1: i32) {
        self.touch(Phase::Down, x, y0);
        for k in 1..=8 {
            self.touch(Phase::Move, x, y0 + (y1 - y0) * k / 8);
        }
        self.touch(Phase::Up, x, y1);
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The bar's thumb centre and a point below it, from the layout on screen.
fn settings_bar(r: &Rig) -> sigma_render::scrollbar::Paged {
    let l = r.app.screen.options_layout(&r.app.options_model());
    l.bar.expect("a long list has a bar")
}

#[test]
fn settings_bar_drags_and_taps_between_pages() {
    let mut r = Rig::new("settings");
    r.app.open_options();
    r.app.settings_page = SettingsPage::Choice(2, "Layout");
    let b = settings_bar(&r);
    assert!(b.n >= 2 && r.app.settings_sub == 0);
    let x = b.strip.x + b.strip.w / 2;
    // A tap below the thumb turns to the next page, above it back.
    r.tap(x, b.thumb.y + b.thumb.h + 60);
    assert_eq!(r.app.settings_sub, 1, "tap below");
    let b = settings_bar(&r);
    r.tap(x, b.thumb.y - 40);
    assert_eq!(r.app.settings_sub, 0, "tap above");
    // A drag to the bottom lands on the last page.
    let b = settings_bar(&r);
    r.drag(x, b.thumb.y + b.thumb.h / 2, b.strip.y + b.strip.h);
    assert_eq!(r.app.settings_sub, b.n - 1, "drag to the end");
}

#[test]
fn help_bar_drags_and_taps_between_pages() {
    let mut r = Rig::new("help");
    r.app.open_help();
    let doc = r
        .app
        .screen
        .build_help(&r.app.keyboard, HelpPage::Markdown, &[]);
    r.app.help = Some(sigma_scribe::HelpState {
        page: HelpPage::Markdown,
        doc: Some(doc),
        at: 0,
    });
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    let bar = |r: &Rig| {
        let h = r.app.help.as_ref().unwrap();
        r.app
            .screen
            .help_paged_bar(&r.app.help_model(h))
            .expect("the reference has several pages")
    };
    let b = bar(&r);
    let x = b.strip.x + b.strip.w / 2;
    r.tap(x, b.thumb.y + b.thumb.h + 60);
    assert_eq!(r.app.help.as_ref().unwrap().at, 1);
    let b = bar(&r);
    r.drag(x, b.thumb.y + b.thumb.h / 2, b.strip.y + b.strip.h);
    assert_eq!(r.app.help.as_ref().unwrap().at, b.n - 1);
}

#[test]
fn preview_bar_drags_and_taps_between_pages() {
    let mut r = Rig::new("preview");
    r.app.buffer.select_all();
    r.app.buffer.backspace();
    r.app.buffer.insert(&"a line of the note\n\n".repeat(250));
    r.app.enter_preview();
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    let bar = |r: &Rig| {
        let p = r.app.preview.as_ref().unwrap();
        r.app
            .screen
            .preview_paged_bar(&r.app.keyboard, p.doc.pages.len(), p.page)
            .expect("a long note has several preview pages")
    };
    let b = bar(&r);
    let x = b.strip.x + b.strip.w / 2;
    r.tap(x, b.thumb.y + b.thumb.h + 60);
    assert_eq!(r.app.preview.as_ref().unwrap().page, 1);
    let b = bar(&r);
    r.drag(x, b.thumb.y + b.thumb.h / 2, b.strip.y + b.strip.h);
    assert_eq!(r.app.preview.as_ref().unwrap().page, b.n - 1);
}
