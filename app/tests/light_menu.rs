//! The front-light button and its menu: Backlight and Warmth sliders in the
//! title bar. They set the light and the saved warmth exactly as the rows of
//! Settings → Display do; the menu stays open while a finger drags and a tap
//! elsewhere or Escape closes it.

use sigma_core::settings::{Change, BACKLIGHT_MAX, WARMTH_MAX};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, HwKey, Phase, Refresh};
use sigma_render::options::{page_items, Item, Kind, SettingsPage};
use sigma_scribe::App;

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: std::path::PathBuf,
}

impl Rig {
    /// A note open in write mode; the device has a light at `light` and a
    /// mixer at `warmth`.
    fn new(tag: &str, light: Option<u8>, warmth: Option<u8>) -> Self {
        let d = std::env::temp_dir().join(format!("sigma-lm-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut app = App::new(1440, 1920, d.clone());
        std::fs::write(d.join("n.md"), "# Note\n\nSome text.\n").unwrap();
        app.open(&sigma_core::NotePath::parse("n.md").unwrap());
        app.buffer.set_caret(0);
        let mut dev = DesktopDevice::new(1440, 1920);
        dev.light = light;
        dev.warmth = warmth;
        app.sync_light(&mut dev);
        app.sync_warmth(&mut dev);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self { app, dev, dir: d }
    }
    fn send(&mut self, e: Event) -> Option<Refresh> {
        let r = self.app.handle(e);
        self.app.sync_light(&mut self.dev);
        self.app.sync_warmth(&mut self.dev);
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
    fn touch(&mut self, phase: Phase, x: i32, y: i32) -> Option<Refresh> {
        self.send(Event::Touch {
            phase,
            x,
            y,
            t: 100,
        })
    }
    /// The step Settings → Display shows for a slider.
    fn row(&mut self, label: &str) -> Option<usize> {
        let (open, page) = (self.app.options, self.app.settings_page);
        self.app.open_options();
        self.app.settings_page = SettingsPage::Category(1);
        let m = self.app.options_model();
        let v = page_items(&m).into_iter().find_map(|i| match i {
            Item::Row(r) if r.label == label => match r.kind {
                Kind::Slider { value, .. } => Some(value),
                _ => None,
            },
            _ => None,
        });
        if !open {
            self.app.options = false;
        }
        self.app.settings_page = page;
        v
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn the_button_is_there_only_with_a_light_and_warmth_only_with_a_mixer() {
    let r = Rig::new("shown", None, None);
    assert!(
        r.app.control_centre("Light").is_none(),
        "no light, no button"
    );
    assert!(!r.app.screen.light_on);

    let mut r = Rig::new("light", Some(4), None);
    assert!(r.app.screen.light_on);
    r.tap_on("Light");
    assert!(r.app.screen.light_menu);
    assert!(r.app.control_centre("Light:Backlight/Full").is_some());
    assert!(
        r.app.control_centre("Light:Warmth/Warm").is_none(),
        "no mixer, no Warmth slider"
    );

    let mut r = Rig::new("both", Some(4), Some(2));
    r.tap_on("Light");
    assert!(r.app.control_centre("Light:Warmth/Warm").is_some());
}

#[test]
fn the_sliders_drive_the_device_and_settings_like_the_rows() {
    let mut r = Rig::new("drive", Some(4), Some(2));
    r.tap_on("Light");
    // One tap per step, each way, and the menu stays open.
    for i in 0..=BACKLIGHT_MAX {
        let label = match i {
            0 => "Off".to_string(),
            BACKLIGHT_MAX => "Full".to_string(),
            i => format!("{} %", u32::from(i) * 100 / u32::from(BACKLIGHT_MAX)),
        };
        if i == 4 {
            continue; // where it already is: nothing to send
        }
        assert!(r.tap_on(&format!("Light:Backlight/{label}")).is_some());
        assert_eq!(r.dev.light, Some(i), "step {i}");
        assert_eq!(r.row("Backlight"), Some(usize::from(i)), "the row agrees");
        assert!(r.app.screen.light_menu, "still open");
    }
    assert!(
        !r.app.settings.serialize().contains("backlight"),
        "the light is not a saved setting"
    );
    for i in 0..=WARMTH_MAX {
        let label = match i {
            0 => "Cool".to_string(),
            WARMTH_MAX => "Warm".to_string(),
            i => format!("{} %", u32::from(i) * 100 / u32::from(WARMTH_MAX)),
        };
        assert!(r.tap_on(&format!("Light:Warmth/{label}")).is_some());
        assert_eq!(r.dev.warmth, Some(i), "step {i}");
        assert!(r
            .app
            .settings
            .serialize()
            .contains(&format!("warmth={i}\n")));
        assert_eq!(r.row("Warmth"), Some(usize::from(i)), "the row agrees");
    }
    // The light and its colour do not disturb each other.
    assert_eq!(r.dev.light, Some(BACKLIGHT_MAX));
}

#[test]
fn the_menu_follows_settings_and_the_icon_dims_while_off() {
    let mut r = Rig::new("follow", Some(4), Some(2));
    assert_eq!(r.app.screen.light_level, 4);
    assert_eq!(r.app.screen.warmth_level, Some(2));
    r.app.change(Change::Warmth(9));
    r.app.change(Change::Backlight(0));
    assert_eq!(r.app.screen.warmth_level, Some(9));
    assert_eq!(r.app.screen.light_level, 0, "off: the dim sun is drawn");
}

#[test]
fn a_drag_acts_once_per_step_and_the_menu_stays_open() {
    let mut r = Rig::new("drag", Some(4), Some(2));
    r.tap_on("Light");
    let (x, y) = r.at("Light:Backlight/50 %");
    let (x75, _) = r.at("Light:Backlight/75 %");
    assert_eq!(r.dev.light, Some(4), "50 % is where it already is");
    assert!(r.touch(Phase::Down, x, y).is_none(), "same step: nothing");
    assert!(r.touch(Phase::Move, x75, y).is_some());
    assert_eq!(r.dev.light, Some(6));
    for dx in [0, 1, 2, 1] {
        assert!(
            r.touch(Phase::Move, x75 + dx, y + 400).is_none(),
            "the same step again does nothing, whatever the height"
        );
    }
    // Past the end clamps to full.
    assert!(r.touch(Phase::Move, 1430, y).is_some());
    assert_eq!(r.dev.light, Some(BACKLIGHT_MAX));
    r.touch(Phase::Up, 1430, y);
    assert!(r.app.screen.light_menu, "open after the drag");
    // A touch down on a slider sets it at once.
    let (wx, wy) = r.at("Light:Warmth/Warm");
    r.touch(Phase::Down, wx, wy);
    r.touch(Phase::Up, wx, wy);
    assert_eq!(r.dev.warmth, Some(WARMTH_MAX));
}

#[test]
fn a_tap_elsewhere_the_button_or_escape_closes_it_and_does_nothing_else() {
    let mut r = Rig::new("close", Some(4), Some(2));
    let caret = r.app.buffer.caret();
    r.tap_on("Light");
    r.send(Event::Tap { x: 300, y: 800 });
    assert!(!r.app.screen.light_menu, "a tap on the page closes it");
    assert_eq!(r.app.buffer.caret(), caret, "and the caret did not move");
    r.tap_on("Light");
    r.tap_on("Light");
    assert!(!r.app.screen.light_menu, "the button closes it");
    r.tap_on("Light");
    r.send(Event::Key(HwKey::Escape));
    assert!(!r.app.screen.light_menu, "Escape closes it");
    // A touch (finger) outside closes it too.
    r.tap_on("Light");
    r.touch(Phase::Down, 300, 800);
    r.touch(Phase::Up, 300, 800);
    assert!(!r.app.screen.light_menu);
    assert_eq!(r.app.buffer.caret(), caret);
    // Nothing was sent to the light by closing it.
    assert_eq!(r.dev.light, Some(4));
    assert_eq!(r.dev.warmth, Some(2));
}

#[test]
fn the_button_works_in_preview_and_the_drawer_menu_makes_way() {
    let mut r = Rig::new("preview", Some(4), Some(2));
    r.app.set_builtin_panes(true);
    r.tap_on("Preview");
    r.tap_on("Light");
    assert!(r.app.screen.light_menu, "opens in preview");
    r.tap_on("Light:Backlight/Full");
    assert_eq!(r.dev.light, Some(BACKLIGHT_MAX));
    r.send(Event::Key(HwKey::Escape));
    r.tap_on("Panels");
    assert!(r.app.screen.drawer_menu);
    // Like any tap elsewhere, the button closes the drawer menu and no more.
    r.tap_on("Light");
    assert!(!r.app.screen.light_menu && !r.app.screen.drawer_menu);
    r.tap_on("Light");
    assert!(r.app.screen.light_menu && !r.app.screen.drawer_menu);
}
