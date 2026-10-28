//! The Wi-Fi button and its menu (v1.18): the switch, the network list, a
//! password typed in the title bar, forgetting a network, and Syncthing
//! following the radio. The radio is the pretend one from `sigma-device`.
//!
//! The logger is global to the process, so these tests take turns.
#![cfg(unix)]

use sigma_core::log::{Level, FILE_NAME};
use sigma_device::desktop::DesktopDevice;
use sigma_device::wifi::fake::{FakeWifi, WRONG_PASSWORD};
use sigma_device::wifi::{JoinError, Service};
use sigma_device::{Event, HwKey, Refresh};
use sigma_scribe::App;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

static TURN: Mutex<()> = Mutex::new(());

struct Rig {
    app: App,
    dev: DesktopDevice,
    dir: PathBuf,
    t: u64,
    _turn: MutexGuard<'static, ()>,
}

impl Rig {
    fn new(tag: &str, radio: Option<FakeWifi>) -> Self {
        let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let d = std::env::temp_dir().join(format!("sigma-wifi-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let notes = d.join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::write(notes.join("n.md"), "# Title\n\nSome words on the page.\n").unwrap();
        let mut app = App::new(1440, 1920, &notes);
        app.set_builtin_panes(true);
        app.open(&sigma_core::NotePath::parse("n.md").unwrap());
        app.buffer.set_caret(0);
        if let Some(r) = radio {
            app.set_wifi(Service::inline(r));
        }
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        Self {
            app,
            dev,
            dir: d,
            t: 0,
            _turn: turn,
        }
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

    fn has(&self, label: &str) -> bool {
        self.app.control_centre(label).is_some()
    }

    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.send(Event::Key(HwKey::Char(c)));
        }
    }

    fn tick(&mut self) {
        self.t += 50;
        let t = self.t;
        self.send(Event::Tick { t });
    }

    fn ssid(&self) -> Option<String> {
        self.app.wifi_view().ssid.clone()
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn with_no_radio_there_is_no_button() {
    let r = Rig::new("none", None);
    assert!(!r.has("Wi-Fi"));
    assert!(!r.app.screen.wifi_on);
    let l = r.app.screen.layout(true, &r.app.keyboard);
    assert_eq!(l.wifi_button, sigma_render::Rect::default());
}

#[test]
fn the_button_opens_the_menu_and_the_switch_turns_the_radio_on_and_off() {
    let mut r = Rig::new("switch", Some(FakeWifi::demo()));
    assert!(r.has("Wi-Fi"));
    assert!(!r.has("WifiSwitch"), "no menu while closed");
    assert_eq!(r.tap_on("Wi-Fi"), Some(Refresh::Partial));
    assert!(r.app.screen.wifi_menu);
    assert!(!r.app.wifi_view().powered);
    assert!(!r.has("Net:Home"), "the list waits for the radio");
    r.tap_on("WifiSwitch");
    let v = r.app.wifi_view();
    assert!(v.powered && v.scanned);
    assert_eq!(r.ssid().as_deref(), Some("Home"), "back on a saved network");
    assert!(r.app.screen.wifi_menu, "the menu stays open");
    for n in [
        "Home",
        "Café Central",
        "Office-5G",
        "Neighbour",
        "Campus",
        "Holiday house",
    ] {
        assert!(r.has(&format!("Net:{n}")), "{n} is listed");
    }
    assert!(r.has("Forget:Home") && r.has("Forget:Holiday house"));
    assert!(
        !r.has("Forget:Office-5G"),
        "only saved networks can be forgotten"
    );
    assert!(r.has("Rescan"));
    r.tap_on("WifiSwitch");
    assert!(!r.app.wifi_view().powered && r.ssid().is_none());
    assert!(!r.has("Net:Home") && !r.has("Rescan"));
    // The button closes it.
    r.tap_on("Wi-Fi");
    assert!(!r.app.screen.wifi_menu);
}

#[test]
fn a_tap_elsewhere_or_escape_closes_the_menu_and_does_nothing_else() {
    let mut r = Rig::new("outside", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    let caret = r.app.buffer.caret();
    r.send(Event::Tap { x: 400, y: 700 });
    assert!(!r.app.screen.wifi_menu);
    assert_eq!(r.app.buffer.caret(), caret, "the tap went no further");
    r.tap_on("Wi-Fi");
    assert!(r.app.screen.wifi_menu);
    r.send(Event::Key(HwKey::Escape));
    assert!(!r.app.screen.wifi_menu);
    // A touch takes the same road as a tap.
    r.tap_on("Wi-Fi");
    let (x, y) = r.at("WifiSwitch");
    for (phase, t) in [
        (sigma_device::Phase::Down, 100),
        (sigma_device::Phase::Up, 130),
    ] {
        r.send(Event::Touch { phase, x, y, t });
    }
    assert!(
        !r.app.wifi_view().powered,
        "a touch on the switch turns it off"
    );
}

#[test]
fn the_wifi_menu_and_the_drawer_menu_are_never_open_together() {
    let mut r = Rig::new("exclusive", Some(FakeWifi::demo_online()));
    r.tap_on("Panels");
    assert!(r.app.screen.drawer_menu);
    // While the drawer menu is open a tap on the Wi-Fi button is "elsewhere":
    // it closes that menu and goes no further.
    r.tap_on("Wi-Fi");
    assert!(!r.app.screen.drawer_menu && !r.app.screen.wifi_menu);
    r.tap_on("Wi-Fi");
    assert!(r.app.screen.wifi_menu);
    r.tap_on("Panels");
    assert!(!r.app.screen.wifi_menu && !r.app.screen.drawer_menu);
    r.tap_on("Panels");
    assert!(r.app.screen.drawer_menu);
}

#[test]
fn an_open_or_saved_network_joins_with_a_tap_and_the_joined_one_is_left_alone() {
    let mut r = Rig::new("join", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    r.tap_on("Net:Café Central");
    assert!(!r.app.is_renaming(), "an open network has no password");
    assert_eq!(r.ssid().as_deref(), Some("Café Central"));
    assert!(r
        .app
        .wifi_view()
        .networks
        .iter()
        .any(|n| n.ssid == "Café Central" && n.saved));
    r.tap_on("Net:Home");
    assert!(!r.app.is_renaming(), "a saved network keeps its password");
    assert_eq!(r.ssid().as_deref(), Some("Home"));
    let g = r.app.wifi_view().clone();
    r.tap_on("Net:Home");
    assert_eq!(
        *r.app.wifi_view(),
        g,
        "a tap on the joined one changes nothing"
    );
    assert!(r.app.screen.wifi_menu);
}

#[test]
fn a_network_the_app_cannot_join_says_so() {
    let mut r = Rig::new("unsupported", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    r.tap_on("Net:Campus");
    assert!(!r.app.is_renaming());
    assert_eq!(r.app.wifi_view().failure, Some(JoinError::Unsupported));
    assert_eq!(r.ssid().as_deref(), Some("Home"));
    // The next action clears it.
    r.tap_on("Net:Café Central");
    assert_eq!(r.app.wifi_view().failure, None);
}

#[test]
fn a_password_is_typed_in_the_title_bar_and_joins_with_ok() {
    let mut r = Rig::new("password", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    r.tap_on("Net:Office-5G");
    assert!(r.app.is_renaming(), "the title bar is the field");
    assert!(!r.app.screen.wifi_menu, "the menu steps aside");
    assert!(r.has("ShowPassword") && r.has("OK") && r.has("Cancel"));
    assert!(r.app.key_centre("a").is_some(), "the keyboard is up");
    // The first letters on the on-screen keys, the rest on a physical one.
    for c in ["c", "o"] {
        let (x, y) = r.app.key_centre(c).unwrap();
        r.send(Event::Tap { x, y });
    }
    r.type_text("rrect horse");
    let hidden = r.app.canvas.pixels.clone();
    r.tap_on("ShowPassword");
    assert_ne!(r.app.canvas.pixels, hidden, "the eye shows what was typed");
    r.tap_on("ShowPassword");
    assert_eq!(r.app.canvas.pixels, hidden, "and hides it again");
    assert_eq!(r.ssid().as_deref(), Some("Home"), "nothing joined yet");
    r.tap_on("OK");
    assert!(!r.app.is_renaming());
    assert!(r.app.screen.wifi_menu, "the menu is back");
    assert_eq!(r.ssid().as_deref(), Some("Office-5G"));
    assert!(r
        .app
        .wifi_view()
        .networks
        .iter()
        .any(|n| n.ssid == "Office-5G" && n.saved));
    assert_eq!(
        r.app.buffer.text(),
        "# Title\n\nSome words on the page.\n",
        "the note is untouched"
    );
}

#[test]
fn return_joins_and_escape_or_cancel_do_not() {
    let mut r = Rig::new("keys", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    r.tap_on("Net:Office-5G");
    r.type_text("correct horse");
    r.send(Event::Key(HwKey::Return));
    assert_eq!(r.ssid().as_deref(), Some("Office-5G"));
    assert!(r.app.screen.wifi_menu);
    r.tap_on("Net:Neighbour");
    r.type_text("whatever else");
    r.send(Event::Key(HwKey::Escape));
    assert!(!r.app.is_renaming() && r.app.screen.wifi_menu);
    assert_eq!(
        r.ssid().as_deref(),
        Some("Office-5G"),
        "Escape joins nothing"
    );
    r.tap_on("Net:Neighbour");
    r.type_text("whatever else");
    r.tap_on("Cancel");
    assert!(!r.app.is_renaming() && r.app.screen.wifi_menu);
    assert_eq!(
        r.ssid().as_deref(),
        Some("Office-5G"),
        "Cancel joins nothing"
    );
    assert!(r
        .app
        .wifi_view()
        .networks
        .iter()
        .all(|n| n.ssid != "Neighbour" || !n.saved));
}

#[test]
fn a_wrong_or_short_password_fails_and_leaves_the_current_network() {
    let mut r = Rig::new("wrong", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    r.tap_on("Net:Office-5G");
    r.type_text(WRONG_PASSWORD);
    r.tap_on("OK");
    assert_eq!(r.app.wifi_view().failure, Some(JoinError::WrongKey));
    assert_eq!(r.ssid().as_deref(), Some("Home"));
    assert!(r
        .app
        .wifi_view()
        .networks
        .iter()
        .all(|n| n.ssid != "Office-5G" || !n.saved));
    r.tap_on("Net:Office-5G");
    r.type_text("short");
    r.tap_on("OK");
    assert_eq!(r.app.wifi_view().failure, Some(JoinError::BadKey));
}

#[test]
fn the_password_field_starts_empty_and_edits_like_any_field() {
    let mut r = Rig::new("edit", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    r.tap_on("Net:Office-5G");
    r.type_text("correct horsX");
    r.send(Event::Key(HwKey::Backspace));
    r.type_text("e");
    r.tap_on("OK");
    assert_eq!(r.ssid().as_deref(), Some("Office-5G"));
    // A second network: the field is empty again, the first one's text gone.
    r.tap_on("Net:Neighbour");
    r.send(Event::Key(HwKey::Return));
    assert_eq!(
        r.app.wifi_view().failure,
        Some(JoinError::BadKey),
        "an empty password is refused, not remembered from before"
    );
}

#[test]
fn a_password_from_preview_leaves_preview_as_find_does() {
    let mut r = Rig::new("preview", Some(FakeWifi::demo_online()));
    r.tap_on("Eye");
    assert!(r.app.preview.is_some());
    assert!(r.has("Wi-Fi"));
    r.tap_on("Wi-Fi");
    assert!(
        r.app.screen.wifi_menu && r.app.preview.is_some(),
        "the menu opens in preview"
    );
    r.tap_on("Net:Café Central");
    assert!(r.app.preview.is_some(), "an open network needs no field");
    r.tap_on("Net:Office-5G");
    assert!(r.app.preview.is_none() && r.app.is_renaming());
    r.tap_on("Cancel");
    assert!(r.app.screen.wifi_menu);
}

#[test]
fn forgetting_a_network_removes_its_password_and_a_far_one_goes_from_the_list() {
    let mut r = Rig::new("forget", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    r.tap_on("Forget:Holiday house");
    assert!(
        !r.has("Net:Holiday house"),
        "out of range and forgotten: gone"
    );
    r.tap_on("Forget:Home");
    assert!(
        r.ssid().is_none(),
        "forgetting the joined network leaves it"
    );
    assert!(
        r.has("Net:Home") && !r.has("Forget:Home"),
        "still in range, no longer saved"
    );
    // Joining it again needs the password again.
    r.tap_on("Net:Home");
    assert!(r.app.is_renaming());
}

#[test]
fn scan_again_asks_the_radio_again() {
    let mut r = Rig::new("rescan", Some(FakeWifi::demo_online()));
    r.tap_on("Wi-Fi");
    let g = r.app.wifi_view().clone();
    assert_eq!(r.tap_on("Rescan"), Some(Refresh::Partial));
    assert_eq!(r.app.wifi_view().networks, g.networks);
    assert!(r.app.screen.wifi_menu);
}

#[test]
fn a_failing_turn_on_is_shown_and_the_next_try_works() {
    let mut f = FakeWifi::demo();
    f.fail_power = true;
    let mut r = Rig::new("power", Some(f));
    r.tap_on("Wi-Fi");
    r.tap_on("WifiSwitch");
    assert!(!r.app.wifi_view().powered);
    assert!(matches!(
        r.app.wifi_view().failure,
        Some(JoinError::Failed(_))
    ));
    r.tap_on("WifiSwitch");
    assert!(r.app.wifi_view().powered);
}

#[test]
fn the_button_follows_the_radio_through_the_ticks_of_a_threaded_service() {
    let mut r = Rig::new("threaded", None);
    r.app.set_wifi(Service::spawn(FakeWifi::demo()));
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    let before = r.app.canvas.pixels.clone();
    r.tap_on("Wi-Fi");
    r.tap_on("WifiSwitch");
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !r.app.wifi_view().scanned {
        assert!(std::time::Instant::now() < end, "the radio never came on");
        std::thread::sleep(std::time::Duration::from_millis(10));
        r.tick();
    }
    assert_eq!(r.ssid().as_deref(), Some("Home"));
    r.tap_on("Wi-Fi");
    assert_ne!(
        r.app.canvas.pixels, before,
        "the icon changed with the radio"
    );
}

#[test]
fn syncthing_stops_with_the_radio_and_may_start_again_with_it() {
    use std::os::unix::fs::PermissionsExt;
    let mut r = Rig::new("sync", Some(FakeWifi::demo_online()));
    let d = r.dir.clone();
    let bin = d.join("syncthing");
    let starts = d.join("starts");
    let pid = d.join("pid");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/sh\necho started >> '{}'\necho $$ > '{}'\nexec sleep 120\n",
            starts.display(),
            pid.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut cfg = sigma_core::sync::Config::new(
        bin,
        d.join("home"),
        d.join("notes"),
        d.join("log/syncthing.log"),
    );
    cfg.port = 1;
    cfg.stop_wait = std::time::Duration::from_millis(300);
    r.app.set_sync_config(cfg);
    r.app.settings.sync = true;
    let count = || std::fs::read_to_string(&starts).map_or(0, |s| s.lines().count());
    let wait = |what: &str, f: &dyn Fn() -> bool| {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !f() {
            assert!(std::time::Instant::now() < end, "{what}");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    let alive = || {
        std::fs::read_to_string(&pid)
            .ok()
            .and_then(|p| p.trim().parse::<u32>().ok())
            .is_some_and(|p| std::path::Path::new(&format!("/proc/{p}")).exists())
    };
    r.app.apply_sync();
    wait("Syncthing never started", &|| count() == 1 && alive());
    r.tap_on("Wi-Fi");
    r.tap_on("WifiSwitch");
    assert!(!r.app.wifi_view().powered);
    assert!(!alive(), "Wi-Fi off stops Syncthing");
    assert!(r.app.settings.sync, "the setting itself is not touched");
    r.tap_on("WifiSwitch");
    assert!(r.app.wifi_view().powered);
    wait("Syncthing did not start again", &|| count() == 2 && alive());
    // Off in Settings stays off with the radio on.
    r.app.settings.sync = false;
    r.app.apply_sync();
    assert!(!alive());
    r.tap_on("WifiSwitch");
    r.tap_on("WifiSwitch");
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(
        count(),
        2,
        "radio on does not start what Settings turned off"
    );
}

#[test]
fn the_log_has_wifi_lines_and_never_a_password_or_an_address() {
    let mut r = Rig::new("log", Some(FakeWifi::demo()));
    let log = r.dir.join("log");
    r.app.set_log_dir(log.clone());
    sigma_core::log::configure(true, Level::Info, Some(log.clone()));
    r.tap_on("Wi-Fi");
    r.tap_on("WifiSwitch");
    r.tap_on("Net:Office-5G");
    r.type_text("hunter2-hunter2");
    r.tap_on("OK");
    r.tap_on("Net:Campus");
    r.tap_on("Forget:Home");
    r.tap_on("WifiSwitch");
    sigma_core::log::configure(false, Level::Info, None);
    let text = std::fs::read_to_string(log.join(FILE_NAME)).unwrap_or_default();
    let wifi: Vec<&str> = text.lines().filter(|l| l.contains("wifi:")).collect();
    let has = |s: &str| wifi.iter().any(|l| l.contains(s));
    assert!(has("INFO") && has("wifi: on"), "{wifi:?}");
    assert!(has("joined Home"), "{wifi:?}");
    assert!(
        has("joining Office-5G") && has("joined Office-5G"),
        "{wifi:?}"
    );
    assert!(
        has("WARNING") && has("not supported"),
        "a failure is a warning: {wifi:?}"
    );
    assert!(has("wifi: off"), "{wifi:?}");
    assert!(!text.contains("hunter2"), "no password in the log");
    assert!(!text.contains("192.168"), "no address in the log");
}
