//! Runs Syncthing and keeps [`View`] current.
//!
//! One worker thread owns the process: it starts it, waits for the REST API,
//! sets the options, polls, carries out the reader's Accept / Ignore, and
//! stops it. The app only reads the latest [`View`] and sends [`Cmd`]s; it
//! never waits for Syncthing (except in [`Supervisor::stop`], at exit).

use super::json::Json;
use super::model::{self, Run, View};
use super::rest::Client;
use super::{count_conflicts, ensure_ignore, percent_encode, random_key};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The loopback port of the REST API.
pub const DEFAULT_PORT: u16 = 8484;

/// What sets Syncthing up on every start: no usage or crash reports, no
/// browser, no self-upgrade, no router probing. Global discovery and relays
/// stay on.
const OPTIONS: &str = r#"{"urAccepted":-1,"startBrowser":false,"autoUpgradeIntervalH":0,"crashReportingEnabled":false,"natEnabled":false}"#;

/// Where and how to run Syncthing.
#[derive(Clone, Debug)]
pub struct Config {
    /// The Syncthing executable.
    pub binary: PathBuf,
    /// Its `--home`: config, keys and database.
    pub home: PathBuf,
    /// The notes folder it syncs.
    pub notes: PathBuf,
    /// Where it writes its own log.
    pub log_file: PathBuf,
    /// The loopback port of the REST API.
    pub port: u16,
    /// How long the API may take to answer after the start.
    pub start_wait: Duration,
    /// How long a shutdown request may take before the process is killed.
    pub stop_wait: Duration,
    /// How long to wait before starting it again after it died.
    pub retry_wait: Duration,
    /// After a kill, also `killall syncthing`: the real Syncthing starts a
    /// child of its own, which a kill of the parent would leave behind. Only
    /// for the Kobo; on a desktop it could end the reader's own Syncthing.
    pub kill_by_name: bool,
    /// Whether 127.0.0.1 can be bound; checked before every start. Tests
    /// replace it.
    pub loopback_probe: fn() -> bool,
    /// Root certificates for Syncthing's HTTPS (relays, global discovery),
    /// given to it as `SSL_CERT_FILE`. The Kobo has none it finds; on a
    /// desktop `None` leaves it the system's.
    pub ca_file: Option<PathBuf>,
}

/// Whether a socket can be bound on 127.0.0.1. On the Kobo the loopback
/// interface is down unless the launcher brings it up; Syncthing then dies at
/// start with "cannot assign requested address".
#[must_use]
pub fn loopback_up() -> bool {
    std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).is_ok()
}

impl Config {
    /// A configuration with the usual time limits and port.
    #[must_use]
    pub fn new(binary: PathBuf, home: PathBuf, notes: PathBuf, log_file: PathBuf) -> Self {
        Self {
            binary,
            home,
            notes,
            log_file,
            port: DEFAULT_PORT,
            start_wait: Duration::from_secs(90),
            stop_wait: Duration::from_secs(8),
            retry_wait: Duration::from_secs(15),
            kill_by_name: false,
            loopback_probe: loopback_up,
            ca_file: None,
        }
    }
}

/// What the reader can ask of Syncthing from Settings → Sync.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// Add a device that tried to connect.
    AcceptDevice {
        /// Its ID.
        id: String,
        /// The name it announced (may be empty).
        name: String,
    },
    /// Forget that a device tried to connect (until it tries again).
    IgnoreDevice {
        /// Its ID.
        id: String,
    },
    /// Share the notes folder as the folder a device offers.
    AcceptFolder {
        /// The folder ID.
        id: String,
        /// Its label.
        label: String,
        /// The offering device.
        device: String,
    },
    /// Forget a folder offer.
    IgnoreFolder {
        /// The folder ID.
        id: String,
        /// The offering device.
        device: String,
    },
}

enum Msg {
    Cmd(Cmd),
    Poll,
    Stop,
}

struct Shared {
    view: Mutex<View>,
    generation: AtomicU64,
    fast: AtomicBool,
}

impl Shared {
    /// Replaces the view, counting a change only when there is one.
    fn put(&self, v: View) {
        if let Ok(mut g) = self.view.lock() {
            if *g != v {
                *g = v;
                self.generation.fetch_add(1, Ordering::Release);
            }
        }
    }

    fn run(&self, r: Run) {
        let mut v = self.view.lock().map(|g| g.clone()).unwrap_or_default();
        v.run = r;
        self.put(v);
    }
}

/// The running (or failed) Syncthing and its worker thread.
pub struct Supervisor {
    tx: Sender<Msg>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Supervisor {
    /// Starts the worker; the process starts in it. Returns at once.
    #[must_use]
    pub fn start(cfg: Config) -> Self {
        let shared = Arc::new(Shared {
            view: Mutex::new(View {
                run: Run::Starting,
                ..View::default()
            }),
            generation: AtomicU64::new(1),
            fast: AtomicBool::new(false),
        });
        let (tx, rx) = channel();
        let sh = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("sync".into())
            .spawn(move || worker(&cfg, &sh, &rx))
            .ok();
        if thread.is_none() {
            shared.run(Run::Failed("could not start a thread".into()));
        }
        Self { tx, shared, thread }
    }

    /// A number that changes whenever [`Supervisor::view`] would.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.shared.generation.load(Ordering::Acquire)
    }

    /// What was last read.
    #[must_use]
    pub fn view(&self) -> View {
        self.shared
            .view
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Asks for something; the answer shows in the view.
    pub fn command(&self, c: Cmd) {
        let _ = self.tx.send(Msg::Cmd(c));
    }

    /// Poll every second (the Sync page is open) or every ten.
    pub fn set_fast(&self, fast: bool) {
        if self.shared.fast.swap(fast, Ordering::Relaxed) != fast && fast {
            let _ = self.tx.send(Msg::Poll);
        }
    }

    /// Stops Syncthing and waits for the worker (at most `stop_wait` plus a
    /// moment). Dropping the supervisor does the same.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        if let Some(t) = self.thread.take() {
            let _ = self.tx.send(Msg::Stop);
            let _ = t.join();
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.shutdown();
    }
}

enum Ended {
    Stopped,
    Died(String),
}

fn worker(cfg: &Config, sh: &Shared, rx: &Receiver<Msg>) {
    if !cfg.binary.is_file() {
        crate::warn!("sync", "{} not found", cfg.binary.display());
        sh.run(Run::NotInstalled);
        wait_stop(rx);
        return;
    }
    let mut failures = 0u32;
    loop {
        // No point starting what cannot listen: say why, and look again in
        // a while (it costs nothing and a later fix needs no restart).
        if !(cfg.loopback_probe)() {
            crate::warn!(
                "sync",
                "127.0.0.1 cannot be bound: the loopback network is down"
            );
            sh.put(View {
                run: Run::NoLoopback,
                ..View::default()
            });
            if wait_or_stop(rx, cfg.retry_wait) {
                return;
            }
            continue;
        }
        sh.put(View {
            run: Run::Starting,
            ..View::default()
        });
        let began = Instant::now();
        match session(cfg, sh, rx) {
            Ended::Stopped => {
                sh.put(View::default());
                return;
            }
            Ended::Died(why) => {
                if began.elapsed() > Duration::from_secs(300) {
                    failures = 0;
                }
                failures += 1;
                crate::warn!("sync", "stopped: {why} (failure {failures} of 3)");
                sh.put(View {
                    run: Run::Failed(why),
                    ..View::default()
                });
                if failures >= 3 || wait_or_stop(rx, cfg.retry_wait) {
                    // Out of retries: the reason stays up until sync is
                    // turned off.
                    if failures >= 3 {
                        wait_stop(rx);
                    }
                    return;
                }
            }
        }
    }
}

/// Blocks until a stop is asked for.
fn wait_stop(rx: &Receiver<Msg>) {
    while let Ok(m) = rx.recv() {
        if matches!(m, Msg::Stop) {
            return;
        }
    }
}

/// Waits `d`; true when a stop came meanwhile.
fn wait_or_stop(rx: &Receiver<Msg>, d: Duration) -> bool {
    let end = Instant::now() + d;
    loop {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        match rx.recv_timeout(left) {
            Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => return true,
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => return false,
        }
    }
}

fn spawn(cfg: &Config, key: &str) -> io::Result<Child> {
    std::fs::create_dir_all(&cfg.home)?;
    if let Some(d) = cfg.log_file.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut cmd = Command::new(&cfg.binary);
    if let Some(f) = &cfg.ca_file {
        cmd.env("SSL_CERT_FILE", f);
    }
    cmd.arg("serve")
        .arg(format!("--home={}", cfg.home.display()))
        .arg(format!("--gui-address=127.0.0.1:{}", cfg.port))
        .arg("--no-browser")
        .arg("--no-restart")
        .arg("--no-upgrade")
        .arg("--no-port-probing")
        .arg(format!("--log-file={}", cfg.log_file.display()))
        .arg("--log-max-size=262144")
        .arg("--log-max-old-files=1")
        // The key is a secret: the environment, not the command line.
        .env("STGUIAPIKEY", key)
        .env("STNODEFAULTFOLDER", "1")
        // A soft memory limit and one core: the Forma has 512 MB and a
        // single Cortex-A9. See docs/sync.md §11.
        .env("GOMEMLIMIT", "96MiB")
        .env("GOGC", "50")
        .env("GOMAXPROCS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

/// One run of the process, from the start to its end.
fn session(cfg: &Config, sh: &Shared, rx: &Receiver<Msg>) -> Ended {
    let key = random_key();
    let client = Client::new(cfg.port, &key);
    // Only what Syncthing logs from now on says why this start failed.
    let log_from = std::fs::metadata(&cfg.log_file).map_or(0, |m| m.len());
    let mut child = match spawn(cfg, &key) {
        Ok(c) => c,
        Err(e) => return Ended::Died(format!("cannot start: {e}")),
    };
    crate::info!("sync", "started pid {}", child.id());
    let deadline = Instant::now() + cfg.start_wait;
    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => {
                stop_child(cfg, &client, &mut child);
                return Ended::Stopped;
            }
            Ok(_) | Err(RecvTimeoutError::Timeout) => {}
        }
        if let Ok(Some(st)) = child.try_wait() {
            return Ended::Died(with_reason(
                format!("exited at start ({st})"),
                &cfg.log_file,
                log_from,
            ));
        }
        if client.get_json("/rest/system/ping").is_ok() {
            break;
        }
        if Instant::now() > deadline {
            kill(cfg, &mut child);
            return Ended::Died(format!("no answer in {} s", cfg.start_wait.as_secs()));
        }
    }
    let mut poller = Poller::default();
    if let Err(e) = client.send("PATCH", "/rest/config/options", OPTIONS) {
        crate::warn!("sync", "options: {e}");
    }
    if let Err(e) = ensure_ignore(&cfg.notes) {
        crate::warn!("sync", ".stignore: {e}");
    }
    poller.poll(cfg, &client, sh);
    loop {
        let wait = if sh.fast.load(Ordering::Relaxed) {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(10)
        };
        match rx.recv_timeout(wait) {
            Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => {
                stop_child(cfg, &client, &mut child);
                return Ended::Stopped;
            }
            Ok(Msg::Cmd(c)) => match apply(cfg, &client, &c) {
                Ok(()) => crate::info!("sync", "{}", describe(&c)),
                Err(e) => crate::warn!("sync", "{}: {e}", describe(&c)),
            },
            Ok(Msg::Poll) | Err(RecvTimeoutError::Timeout) => {}
        }
        if let Ok(Some(st)) = child.try_wait() {
            return Ended::Died(with_reason(
                format!("exited ({st})"),
                &cfg.log_file,
                log_from,
            ));
        }
        poller.poll(cfg, &client, sh);
    }
}

/// The last error Syncthing logged after byte `from` of its log (the one
/// that explains the exit), else `what`. The exit status goes to the log.
fn with_reason(what: String, log: &Path, from: u64) -> String {
    crate::warn!("sync", "{what}");
    last_error(log, from).unwrap_or(what)
}

/// Longest reason shown on the Sync page.
const REASON_MAX: usize = 110;

/// The message of the last `ERR` line Syncthing wrote at or after byte
/// `from` of `log` (a log that is shorter now was rotated: read it all).
/// Only the tail is read. `2026-10-28 15:23:44 ERR Failed to start (…)`
/// gives `Failed to start (…)`.
#[must_use]
pub fn last_error(log: &Path, from: u64) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(log).ok()?;
    let len = f.metadata().ok()?.len();
    let from = if from > len { 0 } else { from };
    let start = from.max(len.saturating_sub(16 * 1024));
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut raw = Vec::new();
    f.take(16 * 1024).read_to_end(&mut raw).ok()?;
    let text = String::from_utf8_lossy(&raw);
    let line = text.lines().rev().find_map(|l| {
        let i = l.find(" ERR ")?;
        Some(l[i + 5..].trim())
    })?;
    let mut out: String = line.chars().take(REASON_MAX).collect();
    if line.chars().count() > REASON_MAX {
        out.push('…');
    }
    (!out.is_empty()).then_some(out)
}

/// Asks Syncthing to shut down; kills it when it does not.
fn stop_child(cfg: &Config, client: &Client, child: &mut Child) {
    if client.send("POST", "/rest/system/shutdown", "").is_err() {
        // The API does not answer (still starting, or hung): nothing to
        // wait for.
        crate::warn!("sync", "no answer to the shutdown request: killed");
        kill(cfg, child);
        return;
    }
    let end = Instant::now() + cfg.stop_wait;
    while Instant::now() < end {
        if matches!(child.try_wait(), Ok(Some(_))) {
            crate::info!("sync", "stopped");
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    crate::warn!(
        "sync",
        "did not stop in {} s: killed",
        cfg.stop_wait.as_secs()
    );
    kill(cfg, child);
}

fn kill(cfg: &Config, child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
    if cfg.kill_by_name {
        let _ = Command::new("killall")
            .args(["-q", "syncthing"])
            .stderr(Stdio::null())
            .status();
    }
}

/// The log line for a command (no names: they are the reader's).
fn describe(c: &Cmd) -> &'static str {
    match c {
        Cmd::AcceptDevice { .. } => "accepted a device",
        Cmd::IgnoreDevice { .. } => "ignored a device",
        Cmd::AcceptFolder { .. } => "accepted a folder",
        Cmd::IgnoreFolder { .. } => "ignored a folder",
    }
}

fn same_path(a: &str, b: &Path) -> bool {
    Path::new(a.trim_end_matches('/')) == Path::new(b.to_string_lossy().trim_end_matches('/'))
}

/// The ID of the configured folder at `notes`, if there is one.
fn folder_at(folders: &Json, notes: &Path) -> Option<String> {
    folders.items().iter().find_map(|f| {
        let path = f.get("path")?.as_str()?;
        same_path(path, notes).then(|| f.get("id")?.as_str().map(str::to_owned))?
    })
}

fn device_entry(id: &str) -> Json {
    let mut d = Json::Null;
    d.set("deviceID", Json::string(id));
    d.set("introducedBy", Json::string(""));
    d.set("encryptionPassword", Json::string(""));
    d
}

/// Carries out one command against the REST API.
fn apply(cfg: &Config, client: &Client, c: &Cmd) -> io::Result<()> {
    match c {
        Cmd::AcceptDevice { id, name } => {
            let mut d = client.get_json("/rest/config/defaults/device")?;
            d.set("deviceID", Json::string(id.as_str()));
            let name = if name.is_empty() {
                id.split('-').next().unwrap_or(id)
            } else {
                name
            };
            d.set("name", Json::string(name));
            d.set("addresses", Json::Arr(vec![Json::string("dynamic")]));
            d.set("autoAcceptFolders", Json::Bool(false));
            d.set("introducer", Json::Bool(false));
            client.send("POST", "/rest/config/devices", &d.render())
        }
        Cmd::IgnoreDevice { id } => client.send(
            "DELETE",
            &format!(
                "/rest/cluster/pending/devices?device={}",
                percent_encode(id)
            ),
            "",
        ),
        Cmd::IgnoreFolder { id, device } => client.send(
            "DELETE",
            &format!(
                "/rest/cluster/pending/folders?folder={}&device={}",
                percent_encode(id),
                percent_encode(device)
            ),
            "",
        ),
        Cmd::AcceptFolder { id, label, device } => {
            ensure_ignore(&cfg.notes)?;
            let folders = client.get_json("/rest/config/folders")?;
            if let Some(f) = folders
                .items()
                .iter()
                .find(|f| f.get("id").and_then(Json::as_str) == Some(id))
            {
                // Already the notes folder: share it with one more device.
                let mut f = f.clone();
                let mut devices = f
                    .get("devices")
                    .map_or_else(Vec::new, |d| d.items().to_vec());
                if !devices
                    .iter()
                    .any(|d| d.get("deviceID").and_then(Json::as_str) == Some(device))
                {
                    devices.push(device_entry(device));
                }
                f.set("devices", Json::Arr(devices));
                return client.send(
                    "PUT",
                    &format!("/rest/config/folders/{}", percent_encode(id)),
                    &f.render(),
                );
            }
            if folder_at(&folders, &cfg.notes).is_some() {
                return Err(io::Error::other(
                    "the notes folder is already shared under another folder ID",
                ));
            }
            let mut f = client.get_json("/rest/config/defaults/folder")?;
            f.set("id", Json::string(id.as_str()));
            f.set(
                "label",
                Json::string(if label.is_empty() { id } else { label }.as_str()),
            );
            f.set("path", Json::string(cfg.notes.to_string_lossy()));
            f.set("type", Json::string("sendreceive"));
            f.set("devices", Json::Arr(vec![device_entry(device)]));
            f.set("fsWatcherEnabled", Json::Bool(true));
            f.set("fsWatcherDelayS", Json::Num(10.0));
            f.set("rescanIntervalS", Json::Num(3600.0));
            // The Kobo's volume is FAT: no permission bits, 2 s time stamps.
            f.set("ignorePerms", Json::Bool(true));
            f.set("modTimeWindowS", Json::Num(2.0));
            f.set("maxConflicts", Json::Num(5.0));
            client.send("POST", "/rest/config/folders", &f.render())
        }
    }
}

/// Reads Syncthing's state into the view; remembers what it last complained
/// about so a repeated failure is logged once.
#[derive(Default)]
struct Poller {
    last_error: String,
    conflicts: usize,
    conflicts_at: Option<Instant>,
}

impl Poller {
    fn poll(&mut self, cfg: &Config, client: &Client, sh: &Shared) {
        match self.read(cfg, client, sh) {
            Ok(()) => self.last_error.clear(),
            Err(e) => {
                let text = e.to_string();
                if text != self.last_error {
                    crate::warn!("sync", "poll: {text}");
                    self.last_error = text;
                }
            }
        }
    }

    fn read(&mut self, cfg: &Config, client: &Client, sh: &Shared) -> io::Result<()> {
        let id = model::parse_my_id(&client.get_json("/rest/system/status")?).unwrap_or_default();
        let devices = client.get_json("/rest/config/devices")?;
        let conns = client.get_json("/rest/system/connections")?;
        let peers = model::parse_peers(&devices, &conns, &id);
        let folders = client.get_json("/rest/config/folders")?;
        let folder_id = folder_at(&folders, &cfg.notes);
        let (folder, last_sync) = match &folder_id {
            Some(fid) => {
                let db =
                    client.get_json(&format!("/rest/db/status?folder={}", percent_encode(fid)))?;
                let stats = client.get_json("/rest/stats/folder")?;
                (
                    Some(model::parse_folder(fid, &db)),
                    model::parse_last_sync(fid, &stats),
                )
            }
            None => (None, None),
        };
        let pending_devices =
            model::parse_pending_devices(&client.get_json("/rest/cluster/pending/devices")?);
        // Offers fit only while there is no notes folder, or for that folder.
        let pending_folders = model::parse_pending_folders(
            &client.get_json("/rest/cluster/pending/folders")?,
            &peers,
        )
        .into_iter()
        .filter(|p| folder_id.as_deref().is_none_or(|f| f == p.id))
        .collect();
        if self
            .conflicts_at
            .is_none_or(|t| t.elapsed() > Duration::from_secs(60))
        {
            self.conflicts = count_conflicts(&cfg.notes);
            self.conflicts_at = Some(Instant::now());
        }
        sh.put(View {
            run: Run::Running,
            device_id: id,
            peers,
            folder,
            last_sync,
            pending_devices,
            pending_folders,
            conflicts: self.conflicts,
        });
        Ok(())
    }
}
