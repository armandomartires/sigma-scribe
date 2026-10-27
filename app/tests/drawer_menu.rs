//! The drawer button and its menu (v1.16): one title-bar icon opens a menu
//! with a checkbox per pane; rows show or hide panes and the menu stays
//! open; the button, Escape or a tap anywhere else closes it, and that tap
//! does nothing else.

use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Phase, Refresh};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: std::path::PathBuf,
}

impl Rig {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-dm-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d.clone());
        app.set_builtin_panes(true);
        let mut body = String::from("# Top\n\n- [ ] first task\n\n");
        for i in 0..40 {
            body.push_str(&format!("Paragraph {i} with enough words to wrap onto a second line beside the drawer and fill the preview page.\n\n"));
        }
        std::fs::write(d.join("n.md"), &body).unwrap();
        app.open(&sigma_core::NotePath::parse("n.md").unwrap());
        app.buffer.set_caret(0);
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev, dir: d }
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        if let Some(r) = r {
            self.app.render(&mut self.dev, r).unwrap();
        }
        r
    }
    fn at(&self, label: &str) -> (i32, i32) {
        self.app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no control {label}"))
    }
    fn tap_on(&mut self, label: &str) -> Option<Refresh> {
        let (x, y) = self.at(label);
        self.send(Event::Tap { x, y })
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn rows_show_and_hide_panes_and_the_menu_stays_open() {
    let mut r = Rig::new("rows");
    assert!(
        r.app.control_centre("Contents").is_none(),
        "no row while closed"
    );
    assert_eq!(r.tap_on("Panels"), Some(Refresh::Partial));
    assert!(r.app.screen.drawer_menu);
    r.tap_on("Contents");
    r.tap_on("To-dos");
    assert!(r.app.screen.drawer_menu, "still open after two rows");
    assert_eq!(r.app.drawer, [true, true, false]);
    assert!(r.app.drawer_visible());
    r.tap_on("Contents");
    assert_eq!(r.app.drawer, [false, true, false], "a second tap hides it");
    r.tap_on("Panels");
    assert!(!r.app.screen.drawer_menu, "the button closes it");
    assert_eq!(
        r.app.drawer,
        [false, true, false],
        "closing changes no pane"
    );
}

#[test]
fn a_tap_outside_only_closes_the_menu() {
    let mut r = Rig::new("outside");
    let text = r.app.buffer.text().to_owned();
    let caret = r.app.buffer.caret();
    r.tap_on("Panels");
    // A key of the on-screen keyboard: no letter typed.
    let (x, y) = r.app.key_centre("a").unwrap();
    r.send(Event::Tap { x, y });
    assert!(!r.app.screen.drawer_menu);
    assert_eq!(r.app.buffer.text(), text);
    // The page: the caret stays.
    r.tap_on("Panels");
    let page = r.app.page_rect();
    r.send(Event::Tap {
        x: page.x + page.w / 4,
        y: page.y + page.h / 2,
    });
    assert!(!r.app.screen.drawer_menu);
    assert_eq!(r.app.buffer.caret(), caret);
    // Another title-bar icon: Files does not open.
    r.tap_on("Panels");
    r.tap_on("Files");
    assert!(!r.app.screen.drawer_menu);
    assert!(r.app.files.is_none());
    // With the menu closed, the same tap does its usual thing again.
    r.send(Event::Tap {
        x: page.x + page.w / 4,
        y: page.y + page.h / 2,
    });
    assert_ne!(r.app.buffer.caret(), caret);
}

#[test]
fn a_long_press_under_the_menu_selects_nothing_and_escape_closes_it() {
    let mut r = Rig::new("hold");
    r.tap_on("Panels");
    let page = r.app.page_rect();
    let (x, y) = (page.x + page.w / 4, page.y + page.h / 2);
    r.send(Event::Touch {
        phase: Phase::Down,
        x,
        y,
        t: 1000,
    });
    for k in 1..20 {
        r.send(Event::Tick { t: 1000 + k * 50 });
    }
    r.send(Event::Touch {
        phase: Phase::Up,
        x,
        y,
        t: 2000,
    });
    assert!(r.app.buffer.selection().is_none(), "no word selected");
    assert!(r.app.sel_ui.is_none());
    assert!(
        !r.app.screen.drawer_menu,
        "the hold counts as a tap outside"
    );
    r.tap_on("Panels");
    assert_eq!(r.send(Event::Key(HwKey::Escape)), Some(Refresh::Partial));
    assert!(!r.app.screen.drawer_menu);
    assert!(r.app.preview.is_none(), "Escape did nothing else");
}

#[test]
fn in_preview_a_row_lays_the_page_out_beside_the_drawer() {
    let mut r = Rig::new("preview");
    r.tap_on("Preview");
    let full = r.app.preview.as_ref().unwrap().doc.pages.len();
    r.tap_on("Panels");
    r.tap_on("Contents");
    assert!(r.app.screen.drawer_menu && r.app.drawer_visible());
    let narrow = r.app.preview.as_ref().unwrap().doc.pages.len();
    assert!(
        narrow > full,
        "{narrow} pages beside the drawer, {full} without"
    );
    r.tap_on("Panels");
    assert!(r.app.preview.is_some(), "still in preview");
    assert!(r.app.control_centre("Heading:Top").is_some());
}

#[test]
fn the_menu_lists_only_panes_on_offer_and_goes_with_the_last_one() {
    let mut r = Rig::new("offer");
    r.tap_on("Panels");
    r.app.set_builtin_panes(false);
    assert!(
        r.app.control_centre("Panels").is_none(),
        "no pane plugin on: no drawer button"
    );
    assert!(!r.app.screen.drawer_menu, "and no menu");
}

#[test]
fn in_preview_closing_from_the_grip_widens_the_page_and_the_tab_narrows_it_again() {
    let mut r = Rig::new("gripview");
    r.tap_on("Preview");
    let full = r.app.preview.as_ref().unwrap().doc.pages.len();
    r.tap_on("Panels");
    r.tap_on("Contents");
    r.tap_on("Panels");
    let narrow = r.app.preview.as_ref().unwrap().doc.pages.len();
    let (x, y) = r.at("DrawerHandle");
    for t in [1000, 1150] {
        r.send(Event::Touch {
            phase: Phase::Down,
            x,
            y,
            t,
        });
        r.send(Event::Touch {
            phase: Phase::Up,
            x,
            y,
            t: t + 50,
        });
    }
    assert!(!r.app.drawer_visible());
    assert_eq!(r.app.preview.as_ref().unwrap().doc.pages.len(), full);
    r.tap_on("DrawerTab");
    assert!(r.app.drawer_visible() && r.app.preview.is_some());
    assert_eq!(r.app.preview.as_ref().unwrap().doc.pages.len(), narrow);
}

#[test]
fn the_scroll_bar_stays_in_view_and_works_beside_the_open_drawer() {
    let mut r = Rig::new("bar");
    r.tap_on("Panels");
    r.tap_on("Contents");
    r.tap_on("Panels");
    let page = r.app.page_rect();
    let grip = r.at("DrawerHandle");
    let edge = page.x + page.w - r.app.drawer_width_px();
    // Just left of the drawer, well below the grip: the bar's lower half.
    let (x, y) = (edge - 10, (grip.1 + page.y + page.h) / 2 + 100);
    assert_eq!(r.app.screen.scroll, 0);
    r.send(Event::Tap { x, y });
    assert!(r.app.screen.scroll > 0, "a tap below the thumb pages down");
    assert!(r.app.drawer_visible(), "the drawer stays open");
}
