//! The Syncthing supervisor against a fake REST server and a fake binary.
//!
//! No Syncthing is needed: the "binary" is a shell script that sleeps (or
//! exits), and the REST API is a few canned answers on the loopback. What is
//! checked is what the app does with the answers and what it sends back.
#![cfg(unix)]

use sigma_core::sync::json::Json;
use sigma_core::sync::{Cmd, Config, Run, Status, Supervisor, View};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ME: &str = "MFZWI3D-BONSGYC-YLTMRWG-C43ENR5-QXGZDMM-FZWI3DP-BONSGYY-LTMRWAD";
const JOE: &str = "P56IOI7-MZJNU2Y-IQGDREY-DM2MGTI-MGL3BXN-PQ6W5BM-TBBZ4TJ-XZWICQ2";

#[derive(Default)]
struct Fake {
    routes: Vec<(String, u16, String)>,
    seen: Vec<(String, String, String)>, // "METHOD path", X-API-Key, body
}

type Shared = Arc<Mutex<Fake>>;

fn serve() -> (u16, Shared) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let fake: Shared = Arc::default();
    let f = Arc::clone(&fake);
    std::thread::spawn(move || {
        for s in l.incoming() {
            let Ok(mut s) = s else { continue };
            let mut raw = Vec::new();
            let mut buf = [0u8; 1024];
            let (head, mut body) = loop {
                let n = s.read(&mut buf).unwrap_or(0);
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
            let len = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            while body.len() < len {
                let n = s.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..n]);
            }
            let first = head.lines().next().unwrap_or("").to_owned();
            let mut it = first.split_whitespace();
            let key = format!("{} {}", it.next().unwrap_or(""), it.next().unwrap_or(""));
            let apikey = head
                .lines()
                .find_map(|l| l.strip_prefix("X-API-Key: "))
                .unwrap_or("")
                .to_owned();
            let (code, text) = {
                let mut g = f.lock().unwrap();
                g.seen.push((
                    key.clone(),
                    apikey,
                    String::from_utf8_lossy(&body).into_owned(),
                ));
                g.routes
                    .iter()
                    .find(|r| r.0 == key)
                    .map_or((404, "not found".to_owned()), |r| (r.1, r.2.clone()))
            };
            let _ = write!(
                s,
                "HTTP/1.1 {code} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            );
        }
    });
    (port, fake)
}

fn route(f: &Shared, key: &str, body: &str) {
    let mut g = f.lock().unwrap();
    g.routes.retain(|r| r.0 != key);
    g.routes.push((key.to_owned(), 200, body.to_owned()));
}

fn seen(f: &Shared, key: &str) -> Vec<String> {
    f.lock()
        .unwrap()
        .seen
        .iter()
        .filter(|s| s.0 == key)
        .map(|s| s.2.clone())
        .collect()
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sigma-sup-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A "syncthing" that records its pid and sleeps, or runs `script`.
fn fake_binary(dir: &Path, script: &str) -> PathBuf {
    let p = dir.join("syncthing");
    std::fs::write(&p, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn config(dir: &Path, port: u16, script: &str) -> Config {
    let notes = dir.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    let mut c = Config::new(
        fake_binary(dir, script),
        dir.join("home"),
        notes,
        dir.join("log/syncthing.log"),
    );
    c.port = port;
    c.stop_wait = Duration::from_millis(300);
    c.retry_wait = Duration::from_millis(50);
    c
}

fn until(sup: &Supervisor, what: &str, ok: impl Fn(&View) -> bool) -> View {
    let end = Instant::now() + Duration::from_secs(20);
    loop {
        let v = sup.view();
        if ok(&v) {
            return v;
        }
        assert!(Instant::now() < end, "timed out waiting for {what}: {v:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn basic_routes(f: &Shared, notes: &Path, folder_configured: bool) {
    route(f, "GET /rest/system/ping", r#"{"ping":"pong"}"#);
    route(f, "PATCH /rest/config/options", "");
    route(
        f,
        "GET /rest/system/status",
        &format!(r#"{{"myID":"{ME}"}}"#),
    );
    route(
        f,
        "GET /rest/config/devices",
        &format!(r#"[{{"deviceID":"{ME}","name":"forma"}},{{"deviceID":"{JOE}","name":"Joe"}}]"#),
    );
    route(
        f,
        "GET /rest/system/connections",
        &format!(r#"{{"connections":{{"{JOE}":{{"connected":true}}}}}}"#),
    );
    let folders = if folder_configured {
        format!(
            r#"[{{"id":"notes","label":"Notes","path":"{}/","devices":[{{"deviceID":"{ME}"}}]}}]"#,
            notes.display()
        )
    } else {
        "[]".to_owned()
    };
    route(f, "GET /rest/config/folders", &folders);
    route(
        f,
        "GET /rest/db/status?folder=notes",
        r#"{"state":"syncing","needFiles":3,"needBytes":4096,"pullErrors":0,"invalid":""}"#,
    );
    route(
        f,
        "GET /rest/stats/folder",
        r#"{"notes":{"lastFile":{"filename":"a.md","at":"2026-10-28T14:05:33+01:00"}}}"#,
    );
    route(f, "GET /rest/cluster/pending/devices", "{}");
    route(f, "GET /rest/cluster/pending/folders", "{}");
    route(f, "POST /rest/system/shutdown", "");
}

#[test]
fn the_view_follows_the_rest_answers() {
    let dir = tmp("view");
    let (port, fake) = serve();
    let cfg = config(&dir, port, "exec sleep 120");
    basic_routes(&fake, &cfg.notes, true);
    route(
        &fake,
        "GET /rest/cluster/pending/devices",
        r#"{"AAAAAAA-BBBBBBB":{"name":"Laptop","address":"10.0.0.5:22000"}}"#,
    );
    let notes = cfg.notes.clone();
    std::fs::write(
        notes.join("a.sync-conflict-20261028-141500-ABCDEFG.md"),
        "x",
    )
    .unwrap();
    let sup = Supervisor::start(cfg);
    let v = until(&sup, "a running view", |v| {
        v.run == Run::Running && !v.device_id.is_empty()
    });
    assert_eq!(v.device_id, ME);
    assert_eq!(v.peers.len(), 1);
    assert_eq!(
        (v.peers[0].name.as_str(), v.peers[0].connected),
        ("Joe", true)
    );
    assert_eq!(v.last_sync.as_deref(), Some("2026-10-28 14:05"));
    assert_eq!(v.status(), Status::Syncing(3, 4096));
    assert_eq!(v.pending_devices.len(), 1);
    assert_eq!(v.pending_devices[0].name, "Laptop");
    assert_eq!(v.conflicts, 1);
    // The ignore list is in the folder, and the options were set once.
    assert!(notes.join(".stignore").exists());
    assert_eq!(seen(&fake, "PATCH /rest/config/options").len(), 1);
    assert!(seen(&fake, "PATCH /rest/config/options")[0].contains("\"urAccepted\":-1"));
    // Every request carries the same fresh key.
    let keys: Vec<String> = fake
        .lock()
        .unwrap()
        .seen
        .iter()
        .map(|s| s.1.clone())
        .collect();
    assert!(
        keys.iter().all(|k| k == &keys[0]) && keys[0].len() == 64,
        "{keys:?}"
    );
    // A change bumps the generation; a poll with the same answers does not.
    let g = sup.generation();
    sup.set_fast(true);
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(sup.generation(), g);
    sup.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn accepting_posts_the_documented_objects() {
    let dir = tmp("accept");
    let (port, fake) = serve();
    let cfg = config(&dir, port, "exec sleep 120");
    let notes = cfg.notes.clone();
    basic_routes(&fake, &notes, false);
    route(
        &fake,
        "GET /rest/config/defaults/device",
        r#"{"deviceID":"","name":"","compression":"metadata"}"#,
    );
    route(
        &fake,
        "GET /rest/config/defaults/folder",
        r#"{"id":"","path":"~","type":"sendreceive","devices":[]}"#,
    );
    route(&fake, "POST /rest/config/devices", "");
    route(&fake, "POST /rest/config/folders", "");
    route(
        &fake,
        "DELETE /rest/cluster/pending/devices?device=OLD-ID",
        "",
    );
    route(
        &fake,
        "DELETE /rest/cluster/pending/folders?folder=a%20b&device=OLD-ID",
        "",
    );
    let sup = Supervisor::start(cfg);
    until(&sup, "running", |v| v.run == Run::Running);

    sup.command(Cmd::AcceptDevice {
        id: JOE.into(),
        name: "Joe's laptop".into(),
    });
    sup.command(Cmd::AcceptFolder {
        id: "abcde-fghij".into(),
        label: "Notes".into(),
        device: JOE.into(),
    });
    sup.command(Cmd::IgnoreDevice {
        id: "OLD-ID".into(),
    });
    sup.command(Cmd::IgnoreFolder {
        id: "a b".into(),
        device: "OLD-ID".into(),
    });
    let end = Instant::now() + Duration::from_secs(10);
    while seen(
        &fake,
        "DELETE /rest/cluster/pending/folders?folder=a%20b&device=OLD-ID",
    )
    .is_empty()
    {
        assert!(Instant::now() < end, "commands were not carried out");
        std::thread::sleep(Duration::from_millis(50));
    }

    let dev = Json::parse(&seen(&fake, "POST /rest/config/devices")[0]).unwrap();
    assert_eq!(dev.get("deviceID").unwrap().as_str(), Some(JOE));
    assert_eq!(dev.get("name").unwrap().as_str(), Some("Joe's laptop"));
    assert_eq!(
        dev.get("addresses").unwrap().items()[0].as_str(),
        Some("dynamic")
    );
    assert_eq!(dev.get("autoAcceptFolders").unwrap().as_bool(), Some(false));
    assert_eq!(
        dev.get("compression").unwrap().as_str(),
        Some("metadata"),
        "the template is kept"
    );

    let f = Json::parse(&seen(&fake, "POST /rest/config/folders")[0]).unwrap();
    assert_eq!(f.get("id").unwrap().as_str(), Some("abcde-fghij"));
    assert_eq!(
        f.get("path").unwrap().as_str(),
        Some(notes.to_str().unwrap())
    );
    assert_eq!(f.get("type").unwrap().as_str(), Some("sendreceive"));
    assert_eq!(f.get("ignorePerms").unwrap().as_bool(), Some(true));
    assert_eq!(
        f.get("devices").unwrap().items()[0]
            .get("deviceID")
            .unwrap()
            .as_str(),
        Some(JOE)
    );
    assert!(notes.join(".stignore").exists());
    sup.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_folder_offer_for_the_notes_folder_shares_it_once_more() {
    let dir = tmp("share");
    let (port, fake) = serve();
    let cfg = config(&dir, port, "exec sleep 120");
    let notes = cfg.notes.clone();
    basic_routes(&fake, &notes, true);
    route(&fake, "PUT /rest/config/folders/notes", "");
    route(
        &fake,
        "GET /rest/cluster/pending/folders",
        r#"{"notes":{"offeredBy":{"AAAAAAA-BBBBBBB":{"label":"Notes"}}},"other":{"offeredBy":{"AAAAAAA-BBBBBBB":{"label":"Other"}}}}"#,
    );
    let sup = Supervisor::start(cfg);
    let v = until(&sup, "pending folders", |v| !v.pending_folders.is_empty());
    // The offer for another folder cannot be the notes folder: not listed.
    assert_eq!(v.pending_folders.len(), 1);
    assert_eq!(v.pending_folders[0].id, "notes");
    for _ in 0..2 {
        sup.command(Cmd::AcceptFolder {
            id: "notes".into(),
            label: "Notes".into(),
            device: "AAAAAAA-BBBBBBB".into(),
        });
    }
    let end = Instant::now() + Duration::from_secs(10);
    while seen(&fake, "PUT /rest/config/folders/notes").len() < 2 {
        assert!(Instant::now() < end, "no PUT");
        std::thread::sleep(Duration::from_millis(50));
    }
    let body = Json::parse(&seen(&fake, "PUT /rest/config/folders/notes")[0]).unwrap();
    let ids: Vec<&str> = body
        .get("devices")
        .unwrap()
        .items()
        .iter()
        .filter_map(|d| d.get("deviceID")?.as_str())
        .collect();
    assert_eq!(ids, [ME, "AAAAAAA-BBBBBBB"]);
    sup.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stopping_asks_for_a_shutdown_and_leaves_no_process() {
    let dir = tmp("stop");
    let (port, fake) = serve();
    let pidfile = dir.join("pid");
    let cfg = config(
        &dir,
        port,
        &format!("echo $$ > {}\nexec sleep 120", pidfile.display()),
    );
    basic_routes(&fake, &cfg.notes, true);
    let sup = Supervisor::start(cfg);
    until(&sup, "running", |v| v.run == Run::Running);
    let pid = std::fs::read_to_string(&pidfile).unwrap().trim().to_owned();
    assert!(Path::new(&format!("/proc/{pid}")).exists());
    let t = Instant::now();
    sup.stop();
    assert!(t.elapsed() < Duration::from_secs(5));
    assert_eq!(seen(&fake, "POST /rest/system/shutdown").len(), 1);
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "the process is still there"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dropping_the_supervisor_stops_it_too() {
    let dir = tmp("drop");
    let (port, fake) = serve();
    let pidfile = dir.join("pid");
    let cfg = config(
        &dir,
        port,
        &format!("echo $$ > {}\nexec sleep 120", pidfile.display()),
    );
    basic_routes(&fake, &cfg.notes, true);
    let pid;
    {
        let sup = Supervisor::start(cfg);
        until(&sup, "running", |v| v.run == Run::Running);
        pid = std::fs::read_to_string(&pidfile).unwrap().trim().to_owned();
    }
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_binary_is_reported_not_retried() {
    let dir = tmp("missing");
    let mut cfg = config(&dir, 1, "true");
    std::fs::remove_file(&cfg.binary).unwrap();
    cfg.binary = dir.join("nope");
    let sup = Supervisor::start(cfg);
    until(&sup, "not installed", |v| v.run == Run::NotInstalled);
    assert_eq!(sup.view().status(), Status::NotInstalled);
    sup.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_process_that_keeps_exiting_ends_failed() {
    let dir = tmp("dies");
    let cfg = config(&dir, 1, "exit 3");
    let sup = Supervisor::start(cfg);
    let v = until(
        &sup,
        "a failure that stays",
        |v| matches!(&v.run, Run::Failed(w) if w.contains("exited")),
    );
    assert!(matches!(v.status(), Status::Failed(_)));
    std::thread::sleep(Duration::from_secs(2));
    // Three tries, then it stays failed (no fourth start, no flapping).
    assert!(matches!(sup.view().run, Run::Failed(_)));
    sup.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stopping_while_it_is_still_starting_does_not_wait_for_it() {
    let dir = tmp("starting");
    let (port, _fake) = serve(); // answers 404 to everything: never "up"
    let pidfile = dir.join("pid");
    let mut cfg = config(
        &dir,
        port,
        &format!("echo $$ > {}\nexec sleep 120", pidfile.display()),
    );
    cfg.stop_wait = Duration::from_secs(30);
    let sup = Supervisor::start(cfg);
    let end = Instant::now() + Duration::from_secs(10);
    while !std::fs::read_to_string(&pidfile).is_ok_and(|t| !t.trim().is_empty()) {
        assert!(Instant::now() < end, "the process never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(sup.view().run, Run::Starting);
    let pid = std::fs::read_to_string(&pidfile).unwrap().trim().to_owned();
    let t = Instant::now();
    sup.stop();
    assert!(
        t.elapsed() < Duration::from_secs(5),
        "stop waited {:?}",
        t.elapsed()
    );
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_down_loopback_is_reported_and_nothing_is_started() {
    let dir = tmp("noloop");
    let pidfile = dir.join("pid");
    let mut cfg = config(
        &dir,
        1,
        &format!("echo $$ > {}\nexec sleep 120", pidfile.display()),
    );
    cfg.loopback_probe = || false;
    let sup = Supervisor::start(cfg);
    let v = until(&sup, "no loopback", |v| v.run == Run::NoLoopback);
    assert_eq!(v.status(), Status::NoLoopback);
    // Still looking (retry_wait is 50 ms in tests), and never spawned.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(sup.view().run, Run::NoLoopback);
    assert!(
        !pidfile.exists(),
        "Syncthing was started without a loopback"
    );
    sup.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_last_error_syncthing_logged_explains_the_failure() {
    let dir = tmp("reason");
    let log = dir.join("log/syncthing.log");
    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    // An old error from an earlier run must not be blamed on this one.
    std::fs::write(&log, "2026-10-27 09:00:00 ERR Old problem\n").unwrap();
    let script = r#"for a in "$@"; do case $a in --log-file=*) f=${a#--log-file=};; esac; done
echo '2026-10-28 15:23:44 INF Starting' >> "$f"
echo '2026-10-28 15:23:45 ERR Failed to start API/GUI (error="listen tcp 127.0.0.1:8484: bind: cannot assign requested address")' >> "$f"
exit 1"#;
    let cfg = config(&dir, 1, script);
    let sup = Supervisor::start(cfg);
    let v = until(
        &sup,
        "a failure with its reason",
        |v| matches!(&v.run, Run::Failed(w) if w.contains("cannot assign requested address")),
    );
    let Run::Failed(why) = v.run else {
        unreachable!()
    };
    // The reason is Syncthing's own words, not the exit status.
    assert!(!why.starts_with("exited"), "{why}");
    assert!(why.contains("Failed to start API/GUI"), "{why}");
    assert!(!why.contains("Old problem"), "{why}");
    sup.stop();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn last_error_reads_only_the_tail_after_the_offset_and_is_short() {
    use sigma_core::sync::last_error;
    let dir = tmp("tail");
    let log = dir.join("l.log");
    std::fs::write(&log, "t ERR first\nt INF ok\n").unwrap();
    assert_eq!(last_error(&log, 0).as_deref(), Some("first"));
    let len = std::fs::metadata(&log).unwrap().len();
    assert_eq!(last_error(&log, len), None);
    // A log shorter than the offset was rotated: read it all.
    assert_eq!(last_error(&log, len + 100).as_deref(), Some("first"));
    assert_eq!(last_error(&dir.join("missing"), 0), None);
    std::fs::write(&log, format!("t ERR {}\n", "x".repeat(500))).unwrap();
    let long = last_error(&log, 0).unwrap();
    assert!(long.chars().count() <= 111 && long.ends_with('…'), "{long}");
    let _ = std::fs::remove_dir_all(&dir);
}
