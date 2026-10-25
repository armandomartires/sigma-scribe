//! v1.0.2 Back navigation: history stack, Back arrow, Esc / Alt+Left.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Mods, Refresh};
use sigma_scribe::{App, HISTORY_CAP};

struct Rig {
    app: App,
    dev: DesktopDevice,
}

impl Rig {
    fn new(tag: &str, w: u32, h: u32) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-hist-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(w, h, d.clone());
        let long: String = (0..400).map(|i| format!("line {i}\n")).collect();
        for (n, body) in [
            ("a.md", format!("# A\n\nGo to [[b]].\n\n{long}")),
            ("b.md", "# B\n\nOn to [[c]].".to_owned()),
            ("c.md", "# C\n\nBack to [[a]].".to_owned()),
        ] {
            std::fs::write(d.join(n), body).unwrap();
        }
        app.open(&sigma_core::NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(w, h);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev }
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(k) = r {
            self.app.render(&mut self.dev, k).unwrap();
        }
        r
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.app.control_centre(label).expect(label);
        self.send(Event::Tap { x, y })
    }
}

#[test]
fn links_push_and_back_restores_note_mode_and_position() {
    let mut r = Rig::new("links", 1440, 1920);
    assert!(
        r.app.control_centre("Back").is_none(),
        "no Back while empty"
    );
    // Write mode, caret deep in a.md.
    let at = r.app.buffer.text().find("line 300").unwrap();
    r.app.buffer.set_caret(at);
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    r.app.enter_preview();
    let page = r.app.preview.as_ref().unwrap().page;
    r.app.exit_preview();
    r.app.follow_link("b");
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert_eq!(r.app.current.as_str(), "b.md");
    assert!(r.app.preview.is_some());
    r.app.follow_link("c");
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert_eq!(r.app.history.len(), 2);
    assert!(r.app.control_centre("Back").is_some());
    // Back button: to b.md in preview.
    assert_eq!(r.tap_on("Back"), Some(Refresh::Full));
    assert_eq!(r.app.current.as_str(), "b.md");
    assert!(r.app.preview.is_some());
    // Esc: to a.md in write mode at the old caret.
    r.send(Event::Key(HwKey::Escape));
    assert_eq!(r.app.current.as_str(), "a.md");
    assert!(r.app.preview.is_none());
    assert_eq!(r.app.buffer.caret(), at);
    assert!(r.app.history.is_empty());
    assert!(r.app.control_centre("Back").is_none());
    let _ = page;
}

#[test]
fn alt_left_goes_back_and_history_is_capped() {
    let mut r = Rig::new("cap", 1440, 1920);
    for i in 0..(HISTORY_CAP + 10) {
        r.app.follow_link(if i % 2 == 0 { "b" } else { "c" });
    }
    assert_eq!(r.app.history.len(), HISTORY_CAP);
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    let was = r.app.current.clone();
    r.send(Event::Chord {
        key: HwKey::Left,
        mods: Mods {
            alt: true,
            ..Mods::default()
        },
    });
    assert_ne!(r.app.current, was);
    assert_eq!(r.app.history.len(), HISTORY_CAP - 1);
}

#[test]
fn files_and_contents_push() {
    let mut r = Rig::new("files", 1440, 1920);
    r.tap_on("Files");
    r.tap_on("b.md");
    assert_eq!(r.app.current.as_str(), "b.md");
    assert_eq!(r.app.history.len(), 1);
    r.tap_on("Back");
    assert_eq!(r.app.current.as_str(), "a.md");
}

#[test]
fn title_bar_fits_with_back_in_both_orientations() {
    for land in [false, true] {
        let mut r = Rig::new(&format!("fit{land}"), 1440, 1920);
        if land {
            r.app.change(sigma_core::settings::Change::Orientation(
                sigma_core::settings::Orientation::Landscape,
            ));
        }
        let w = r.app.canvas.width;
        r.app.follow_link("b");
        r.app.render(&mut r.dev, Refresh::Full).unwrap();
        let l = r.app.screen.layout(false, &r.app.keyboard);
        let s = r.app.screen.scale().px(44.0);
        assert!(l.back_button.w >= s && l.back_button.h >= s);
        assert!(l.back_button.x + l.back_button.w < l.files_button.x);
        assert!(l.name_rect.x + l.name_rect.w <= l.back_button.x);
        assert!(
            l.name_rect.w >= r.app.screen.page_scale().px(200.0),
            "{land}: {:?}",
            l.name_rect
        );
        assert!(l.options_button.x + l.options_button.w <= w as i32);
    }
}
