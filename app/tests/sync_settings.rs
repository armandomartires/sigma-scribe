//! Settings → Sync: the page, its switch, and Accept / Ignore, first on a
//! view the test supplies, then against the real supervisor with a fake
//! Syncthing (a shell script that sleeps, and a few canned REST answers).
#![cfg(unix)]

use sigma_core::sync::{Config, Peer, PendingDevice, PendingFolder, Run, View};
use sigma_device::desktop::DesktopDevice;
use sigma_device::{Event, Refresh};
use sigma_render::options::SYNC_CATEGORY;
use sigma_render::SettingsPage;
use sigma_scribe::App;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ME: &str = "MFZWI3D-BONSGYC-YLTMRWG-C43ENR5-QXGZDMM-FZWI3DP-BONSGYY-LTMRWAD";
const LAPTOP: &str = "P56IOI7-MZJNU2Y-IQGDREY-DM2MGTI-MGL3BXN-PQ6W5BM-TBBZ4TJ-XZWICQ2";

struct Rig {
    app: App,
    dev: DesktopDevice,
    t: u64,
}

impl Rig {
    fn new(tag: &str) -> (Self, PathBuf) {
        let d = std::env::temp_dir().join(format!("sigma-syncset-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let notes = d.join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::write(notes.join("a.md"), "hi").unwrap();
        let mut app = App::new(1440, 1920, &notes);
        app.open(&sigma_core::NotePath::parse("a.md").unwrap());
        let mut dev = DesktopDevice::new(1440, 1920);
        app.render(&mut dev, Refresh::Full).unwrap();
        (Self { app, dev, t: 0 }, d)
    }

    fn tap_on(&mut self, label: &str) {
        let (x, y) = self
            .app
            .control_centre(label)
            .unwrap_or_else(|| panic!("no control {label}"));
        if let Some(r) = self.app.handle(Event::Tap { x, y }) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }

    fn tick(&mut self) {
        self.t += 50;
        if let Some(r) = self.app.handle(Event::Tick { t: self.t }) {
            self.app.render(&mut self.dev, r).unwrap();
        }
    }

    fn open_sync_page(&mut self) {
        self.tap_on("Options");
        self.tap_on("Sync");
        assert_eq!(
            self.app.settings_page,
            SettingsPage::Category(SYNC_CATEGORY)
        );
    }
}

fn running() -> View {
    View {
        run: Run::Running,
        device_id: ME.into(),
        ..View::default()
    }
}

#[test]
fn the_switch_is_saved_and_the_rest_waits_for_it() {
    let (mut r, d) = Rig::new("switch");
    r.app.set_sync_config(Config::new(
        d.join("no-such-syncthing"),
        d.join("home"),
        d.join("notes"),
        d.join("log/syncthing.log"),
    ));
    r.open_sync_page();
    // Off: only the switch.
    assert!(r.app.control_centre("Sync notes/On").is_some());
    assert!(r.app.control_centre("Status").is_none());
    // A view with no switch on shows nothing more.
    r.app.set_sync_view(running());
    r.tap_on("Sync notes/On");
    assert!(r.app.settings.sync);
    assert!(std::fs::read_to_string(d.join("notes/.sigma-settings"))
        .unwrap()
        .contains("sync=on"));
    // The binary is not there: the supervisor says so, and the page shows it.
    let mut t = 0;
    while !matches!(r.app.sync_view().run, Run::NotInstalled) && t < 200 {
        std::thread::sleep(Duration::from_millis(25));
        r.tick();
        t += 1;
    }
    assert_eq!(r.app.sync_view().run, Run::NotInstalled);
    r.tap_on("Sync notes/Off");
    assert!(!r.app.settings.sync);
    assert_eq!(
        *r.app.sync_view(),
        View::default(),
        "off clears what it showed"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn accept_and_ignore_leave_the_list_at_once() {
    let (mut r, d) = Rig::new("accept");
    r.open_sync_page();
    r.app.settings.sync = true;
    r.app.set_sync_view(View {
        pending_devices: vec![PendingDevice {
            id: LAPTOP.into(),
            name: "Laptop".into(),
            address: "10.0.0.2:22000".into(),
        }],
        pending_folders: vec![PendingFolder {
            id: "notes-1".into(),
            label: "Shared notes".into(),
            device: LAPTOP.into(),
            device_name: String::new(),
        }],
        peers: vec![Peer {
            id: LAPTOP.into(),
            name: "Laptop".into(),
            connected: true,
        }],
        ..running()
    });
    r.app.render(&mut r.dev, Refresh::Full).unwrap();
    assert!(r.app.control_centre("Laptop/Accept").is_some());
    assert!(r.app.control_centre("Shared notes/Ignore").is_some());
    r.tap_on("Laptop/Accept");
    assert!(r.app.control_centre("Laptop/Accept").is_none());
    assert!(r.app.sync_view().pending_devices.is_empty());
    assert_eq!(r.app.sync_view().pending_folders.len(), 1);
    r.tap_on("Shared notes/Ignore");
    assert!(r.app.sync_view().pending_folders.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

// --- against the real supervisor ---

type Seen = Arc<Mutex<Vec<(String, String)>>>;

/// A fake REST API: canned answers by "METHOD path"; every request is kept.
fn serve(routes: Vec<(&'static str, String)>) -> (u16, Seen) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let seen: Seen = Arc::default();
    let s = Arc::clone(&seen);
    std::thread::spawn(move || {
        for c in l.incoming() {
            let Ok(mut c) = c else { continue };
            let mut raw = Vec::new();
            let mut buf = [0u8; 2048];
            let (head, mut body) = loop {
                let n = c.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break (String::new(), Vec::new());
                }
                raw.extend_from_slice(&buf[..n]);
                if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    break (
                        String::from_utf8_lossy(&raw[..i]).into_owned(),
                        raw[i + 4..].to_vec(),
                    );
                }
            };
            let want = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            while body.len() < want {
                let n = c.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..n]);
            }
            let mut it = head.lines().next().unwrap_or("").split_whitespace();
            let key = format!("{} {}", it.next().unwrap_or(""), it.next().unwrap_or(""));
            s.lock()
                .unwrap()
                .push((key.clone(), String::from_utf8_lossy(&body).into_owned()));
            let (code, text) = routes
                .iter()
                .find(|r| r.0 == key)
                .map_or((404, "not found".to_owned()), |r| (200, r.1.clone()));
            let _ = write!(
                c,
                "HTTP/1.1 {code} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            );
        }
    });
    (port, seen)
}

#[test]
fn turning_sync_on_runs_it_and_accepting_reaches_syncthing() {
    let (mut r, d) = Rig::new("e2e");
    let notes = d.join("notes");
    let bin = d.join("syncthing");
    std::fs::write(&bin, "#!/bin/sh\nexec sleep 120\n").unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (port, seen) = serve(vec![
        ("GET /rest/system/ping", r#"{"ping":"pong"}"#.into()),
        ("PATCH /rest/config/options", String::new()),
        ("GET /rest/system/status", format!(r#"{{"myID":"{ME}"}}"#)),
        (
            "GET /rest/config/devices",
            format!(r#"[{{"deviceID":"{ME}","name":"forma"}}]"#),
        ),
        (
            "GET /rest/system/connections",
            r#"{"connections":{}}"#.into(),
        ),
        ("GET /rest/config/folders", "[]".into()),
        ("GET /rest/cluster/pending/folders", "{}".into()),
        (
            "GET /rest/cluster/pending/devices",
            format!(r#"{{"{LAPTOP}":{{"name":"Laptop","address":"10.0.0.2:22000"}}}}"#),
        ),
        (
            "GET /rest/config/defaults/device",
            r#"{"deviceID":"","name":""}"#.into(),
        ),
        ("POST /rest/config/devices", String::new()),
        ("POST /rest/system/shutdown", String::new()),
    ]);
    let mut cfg = Config::new(bin, d.join("home"), notes, d.join("log/syncthing.log"));
    cfg.port = port;
    cfg.stop_wait = Duration::from_millis(300);
    r.app.set_sync_config(cfg);

    r.open_sync_page();
    r.tap_on("Sync notes/On");
    let end = Instant::now() + Duration::from_secs(20);
    while r.app.control_centre("Laptop/Accept").is_none() {
        assert!(
            Instant::now() < end,
            "the pending device never showed: {:?}",
            r.app.sync_view()
        );
        std::thread::sleep(Duration::from_millis(50));
        r.tick();
    }
    assert_eq!(r.app.sync_view().device_id, ME);
    r.tap_on("Laptop/Accept");
    let end = Instant::now() + Duration::from_secs(10);
    while !seen
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.0 == "POST /rest/config/devices")
    {
        assert!(Instant::now() < end, "Accept never reached Syncthing");
        std::thread::sleep(Duration::from_millis(50));
    }
    let body = seen
        .lock()
        .unwrap()
        .iter()
        .find(|s| s.0 == "POST /rest/config/devices")
        .unwrap()
        .1
        .clone();
    assert!(body.contains(LAPTOP) && body.contains("Laptop"), "{body}");
    // Leaving the app stops it.
    r.app.stop_sync();
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.0 == "POST /rest/system/shutdown"));
    let _ = std::fs::remove_dir_all(&d);
}
