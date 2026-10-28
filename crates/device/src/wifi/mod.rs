//! Wi-Fi (v1.18): the title-bar button's backend.
//!
//! With the stock reader stopped, nothing else brings the network up or
//! picks a network, so the app does. The pieces:
//!
//! - [`wpa`]: the `wpa_supplicant` control-socket protocol (parsers, a
//!   client over a Unix datagram socket, and the join / forget flows). It
//!   is plain Unix code, so it is tested on the desktop against a fake
//!   supplicant. There is no `wpa_cli` dependency.
//! - [`Backend`]: what the menu needs, as blocking calls. [`fake::FakeWifi`]
//!   serves the desktop window, the emulator and the tests; `kobo::KoboWifi`
//!   (ARM only) drives the real radio.
//! - [`Service`]: runs a backend on its own thread (never on the input
//!   loop) or inline (tests), and keeps the last [`View`] for the app to
//!   take in on its ticks, as the Syncthing supervisor does.
//!
//! Nothing here logs or keeps a Wi-Fi password beyond the call that uses it.
//! `docs/wifi.md` has the design.

pub mod fake;
#[cfg(target_arch = "arm")]
pub mod kobo;
pub mod wpa;

use std::fmt;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How a network is secured, as far as joining it is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Security {
    /// No key.
    Open,
    /// WEP: a key typed on the keyboard.
    Wep,
    /// WPA or WPA2 with a passphrase.
    Wpa,
    /// Needs something this app does not do (WPA-Enterprise, WPA3-only,
    /// enhanced open): shown with a lock, not joinable.
    Unsupported,
    /// A saved network not in range, so its security is not known.
    Unknown,
}

impl Security {
    /// The security a `SCAN_RESULTS` flags field stands for.
    #[must_use]
    pub fn from_flags(flags: &str) -> Self {
        if flags.contains("EAP") || flags.contains("OWE") {
            Self::Unsupported
        } else if flags.contains("PSK") {
            // Includes WPA2-PSK+SAE transition networks.
            Self::Wpa
        } else if flags.contains("SAE") {
            Self::Unsupported
        } else if flags.contains("WEP") {
            Self::Wep
        } else if flags.contains("WPA") || flags.contains("RSN") {
            Self::Unsupported
        } else {
            Self::Open
        }
    }

    /// A key has to be typed to join.
    #[must_use]
    pub fn needs_key(self) -> bool {
        matches!(self, Self::Wep | Self::Wpa)
    }

    /// The list shows a lock.
    #[must_use]
    pub fn locked(self) -> bool {
        matches!(self, Self::Wep | Self::Wpa | Self::Unsupported)
    }
}

/// One network in the list: found by the last scan, saved, or both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    /// The network name.
    pub ssid: String,
    /// Strongest signal seen, dBm; `None` for a saved network not in range.
    pub signal: Option<i32>,
    /// How it is secured.
    pub security: Security,
    /// Known to `wpa_supplicant` (its password is kept).
    pub saved: bool,
}

impl Network {
    /// Signal strength as 1 to 4 bars; 0 when out of range.
    #[must_use]
    pub fn bars(&self) -> u8 {
        self.signal.map_or(0, bars)
    }
}

/// 1 to 4 bars for a signal level in dBm.
#[must_use]
pub fn bars(dbm: i32) -> u8 {
    match dbm {
        d if d >= -55 => 4,
        d if d >= -65 => 3,
        d if d >= -75 => 2,
        _ => 1,
    }
}

/// What the radio is doing right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// The radio is powered and `wpa_supplicant` answers.
    pub powered: bool,
    /// The network joined, once the handshake is complete.
    pub ssid: Option<String>,
    /// The address the network gave.
    pub ip: Option<String>,
}

/// Why a join failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinError {
    /// The password is not one a network takes (length or characters).
    BadKey,
    /// The network is of a kind this app cannot join.
    Unsupported,
    /// The network refused the key, or never answered.
    WrongKey,
    /// Joined, but no address was given.
    NoAddress,
    /// The radio or `wpa_supplicant` failed; the text is for the log.
    Failed(String),
}

impl JoinError {
    /// What the menu says (English; the app translates it).
    #[must_use]
    pub fn text(&self) -> &'static str {
        match self {
            Self::BadKey => "A Wi-Fi password has 8 to 63 characters",
            Self::Unsupported => "This kind of network is not supported here",
            Self::WrongKey => "Could not join: wrong password? Forget it and try again",
            Self::NoAddress => "Joined, but the network gave no address",
            Self::Failed(_) => "Wi-Fi did not answer",
        }
    }
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(why) => write!(f, "{}: {why}", self.text()),
            _ => f.write_str(self.text()),
        }
    }
}

impl std::error::Error for JoinError {}

impl From<io::Error> for JoinError {
    fn from(e: io::Error) -> Self {
        Self::Failed(e.to_string())
    }
}

/// What a backend does, as blocking calls. Called from one thread at a time.
pub trait Backend: Send {
    /// The radio's state; cheap.
    ///
    /// # Errors
    /// The state could not be read.
    fn status(&mut self) -> io::Result<Status>;
    /// Turns the radio on (loads the driver, starts `wpa_supplicant`) or off.
    ///
    /// # Errors
    /// The radio did not change state.
    fn set_power(&mut self, on: bool) -> io::Result<()>;
    /// Scans and lists what was found together with the saved networks,
    /// strongest first.
    ///
    /// # Errors
    /// The scan could not run.
    fn scan(&mut self) -> io::Result<Vec<Network>>;
    /// Joins `ssid`: a saved network with `key` `None`, else with the key
    /// given; open networks take no key. Returns once the network gave an
    /// address, or failed.
    ///
    /// # Errors
    /// See [`JoinError`].
    fn join(&mut self, ssid: &str, key: Option<&str>) -> Result<(), JoinError>;
    /// Removes a saved network.
    ///
    /// # Errors
    /// `wpa_supplicant` refused.
    fn forget(&mut self, ssid: &str) -> io::Result<()>;
}

/// What the service is busy with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Busy {
    /// Nothing.
    Idle,
    /// Turning the radio on (`true`) or off.
    Power(bool),
    /// Scanning.
    Scanning,
    /// Joining a network.
    Joining(String),
}

/// What the menu shows: the last state taken from the backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// Radio on.
    pub powered: bool,
    /// The network joined.
    pub ssid: Option<String>,
    /// Its address.
    pub ip: Option<String>,
    /// Networks found and saved, in the order to list them.
    pub networks: Vec<Network>,
    /// A scan has finished since the radio was turned on.
    pub scanned: bool,
    /// What is running.
    pub busy: Busy,
    /// The last failure, until the next action.
    pub failure: Option<JoinError>,
}

impl Default for View {
    fn default() -> Self {
        Self {
            powered: false,
            ssid: None,
            ip: None,
            networks: Vec::new(),
            scanned: false,
            busy: Busy::Idle,
            failure: None,
        }
    }
}

/// The interface's address in the output of `ifconfig <interface>`: BusyBox
/// prints `inet addr:192.168.1.5  Bcast:…`, others `inet 192.168.1.5  netmask …`.
/// Loopback and link-local addresses (`169.254.…`, a failed DHCP) do not count.
#[must_use]
pub fn parse_ifconfig_address(out: &str) -> Option<String> {
    out.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("inet ")?;
        let a = rest.strip_prefix("addr:").unwrap_or(rest);
        let a = a.split_whitespace().next()?;
        let ok = a.split('.').count() == 4
            && a.split('.').all(|p| p.parse::<u8>().is_ok())
            && !a.starts_with("127.")
            && !a.starts_with("169.254.");
        ok.then(|| a.to_owned())
    })
}

/// A line of the Wi-Fi log (`wifi: …`, INFO); never a key or an address.
#[cfg(target_arch = "arm")]
pub(crate) fn log_line(text: &str) {
    sigma_core::info!("wifi", "{text}");
}

/// A command for the worker.
enum Cmd {
    Power(bool),
    Scan,
    Join(String, Option<String>),
    Forget(String),
    Refresh,
    Stop,
}

/// The backend and the shared view; run by the thread or inline.
struct Worker {
    backend: Box<dyn Backend>,
    shared: Arc<Shared>,
}

struct Shared {
    view: Mutex<View>,
    generation: AtomicU64,
}

impl Shared {
    fn update(&self, f: impl FnOnce(&mut View)) {
        let mut v = self
            .view
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let before = v.clone();
        f(&mut v);
        if *v != before {
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
    }
}

impl Worker {
    fn status(&mut self) {
        if let Ok(s) = self.backend.status() {
            self.shared.update(|v| {
                v.powered = s.powered;
                v.ssid = s.ssid;
                v.ip = s.ip;
                if !s.powered {
                    v.networks.clear();
                    v.scanned = false;
                }
            });
        }
    }

    fn scan(&mut self) {
        self.shared.update(|v| v.busy = Busy::Scanning);
        let found = self.backend.scan();
        self.shared.update(|v| {
            v.busy = Busy::Idle;
            match found {
                Ok(n) => {
                    v.networks = n;
                    v.scanned = true;
                }
                Err(e) => v.failure = Some(JoinError::Failed(e.to_string())),
            }
        });
        self.status();
    }

    fn run(&mut self, cmd: Cmd) -> bool {
        match cmd {
            Cmd::Stop => return false,
            Cmd::Refresh => self.status(),
            Cmd::Scan => {
                self.shared.update(|v| v.failure = None);
                self.scan();
            }
            Cmd::Power(on) => {
                self.shared.update(|v| {
                    v.failure = None;
                    v.busy = Busy::Power(on);
                });
                let r = self.backend.set_power(on);
                self.shared.update(|v| {
                    v.busy = Busy::Idle;
                    if let Err(e) = &r {
                        v.failure = Some(JoinError::Failed(e.to_string()));
                    }
                });
                self.status();
                if on && r.is_ok() {
                    self.scan();
                }
            }
            Cmd::Join(ssid, key) => {
                self.shared.update(|v| {
                    v.failure = None;
                    v.busy = Busy::Joining(ssid.clone());
                });
                let r = self.backend.join(&ssid, key.as_deref());
                self.shared.update(|v| {
                    v.busy = Busy::Idle;
                    if let Err(e) = &r {
                        v.failure = Some(e.clone());
                    }
                });
                self.status();
                // A network that took its key is saved now.
                if r.is_ok() || r == Err(JoinError::NoAddress) {
                    self.shared.update(|v| {
                        for n in v.networks.iter_mut().filter(|n| n.ssid == ssid) {
                            n.saved = true;
                        }
                    });
                }
            }
            Cmd::Forget(ssid) => {
                let r = self.backend.forget(&ssid);
                self.shared.update(|v| {
                    v.failure = r.err().map(|e| JoinError::Failed(e.to_string()));
                    v.networks.retain_mut(|n| {
                        if n.ssid == ssid {
                            n.saved = false;
                        }
                        n.saved || n.signal.is_some()
                    });
                });
                self.status();
            }
        }
        true
    }
}

enum Mode {
    Thread(Sender<Cmd>),
    Inline(Mutex<Worker>),
}

/// A backend with its thread and its last view.
pub struct Service {
    mode: Mode,
    shared: Arc<Shared>,
}

/// How often the thread looks at the radio while idle.
const POLL: Duration = Duration::from_secs(4);

impl Service {
    fn build(backend: Box<dyn Backend>, threaded: bool) -> Self {
        let shared = Arc::new(Shared {
            view: Mutex::new(View::default()),
            generation: AtomicU64::new(0),
        });
        let mut w = Worker {
            backend,
            shared: shared.clone(),
        };
        // The state is known before the first frame is drawn.
        w.status();
        let mode = if threaded {
            let (tx, rx) = channel();
            std::thread::spawn(move || loop {
                match rx.recv_timeout(POLL) {
                    Ok(cmd) => {
                        if !w.run(cmd) {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => w.status(),
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            });
            Mode::Thread(tx)
        } else {
            Mode::Inline(Mutex::new(w))
        };
        Self { mode, shared }
    }

    /// Runs `backend` on a thread of its own.
    #[must_use]
    pub fn spawn(backend: impl Backend + 'static) -> Self {
        Self::build(Box::new(backend), true)
    }

    /// Runs `backend` in the caller, each command finished before it
    /// returns: for the emulator and the tests, which must be deterministic.
    #[must_use]
    pub fn inline(backend: impl Backend + 'static) -> Self {
        Self::build(Box::new(backend), false)
    }

    fn send(&self, cmd: Cmd) {
        match &self.mode {
            Mode::Thread(tx) => {
                let _ = tx.send(cmd);
            }
            Mode::Inline(w) => {
                let mut w = w.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                w.run(cmd);
            }
        }
    }

    /// Counts every change of the view; compare with the last value seen.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.shared.generation.load(Ordering::SeqCst)
    }

    /// The view as it is now.
    #[must_use]
    pub fn view(&self) -> View {
        self.shared
            .view
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Turns the radio on or off (and scans after turning it on).
    pub fn power(&self, on: bool) {
        self.send(Cmd::Power(on));
    }

    /// Scans again.
    pub fn scan(&self) {
        self.send(Cmd::Scan);
    }

    /// Joins a network: `key` is `None` for a saved or an open one.
    pub fn join(&self, ssid: &str, key: Option<String>) {
        self.send(Cmd::Join(ssid.to_owned(), key));
    }

    /// Forgets a saved network.
    pub fn forget(&self, ssid: &str) {
        self.send(Cmd::Forget(ssid.to_owned()));
    }

    /// Reads the radio's state again.
    pub fn refresh(&self) {
        self.send(Cmd::Refresh);
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        if let Mode::Thread(tx) = &self.mode {
            let _ = tx.send(Cmd::Stop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_say_how_a_network_is_secured() {
        let s = Security::from_flags;
        assert_eq!(s("[ESS]"), Security::Open);
        assert_eq!(s("[WPA2-PSK-CCMP][ESS]"), Security::Wpa);
        assert_eq!(s("[WPA-PSK-TKIP][WPA2-PSK-CCMP][ESS]"), Security::Wpa);
        assert_eq!(s("[RSN-PSK+SAE-CCMP][ESS]"), Security::Wpa);
        assert_eq!(s("[RSN-SAE-CCMP][ESS]"), Security::Unsupported);
        assert_eq!(s("[WPA2-EAP-CCMP][ESS]"), Security::Unsupported);
        assert_eq!(s("[RSN-OWE-CCMP][ESS]"), Security::Unsupported);
        assert_eq!(s("[WEP][ESS]"), Security::Wep);
        assert!(Security::Wpa.needs_key() && Security::Wpa.locked());
        assert!(!Security::Unsupported.needs_key() && Security::Unsupported.locked());
        assert!(!Security::Open.locked() && !Security::Unknown.locked());
    }

    #[test]
    fn the_address_is_read_from_either_ifconfig() {
        let busybox = "eth0      Link encap:Ethernet  HWaddr 00:11:22:33:44:55\n          inet addr:192.168.1.5  Bcast:192.168.1.255  Mask:255.255.255.0\n";
        let net_tools = "eth0: flags=4163<UP>  mtu 1500\n        inet 10.0.0.7  netmask 255.255.255.0  broadcast 10.0.0.255\n";
        assert_eq!(
            parse_ifconfig_address(busybox).as_deref(),
            Some("192.168.1.5")
        );
        assert_eq!(
            parse_ifconfig_address(net_tools).as_deref(),
            Some("10.0.0.7")
        );
        assert_eq!(parse_ifconfig_address("eth0 Link encap:Ethernet\n"), None);
        assert_eq!(
            parse_ifconfig_address("  inet addr:169.254.3.4  Mask:255.255.0.0"),
            None
        );
        assert_eq!(
            parse_ifconfig_address("  inet addr:127.0.0.1  Mask:255.0.0.0"),
            None
        );
        assert_eq!(parse_ifconfig_address("  inet6 addr: fe80::1/64"), None);
        assert_eq!(parse_ifconfig_address(""), None);
    }

    #[test]
    fn signal_levels_make_one_to_four_bars() {
        assert_eq!(
            [-40, -55, -56, -65, -70, -75, -76, -90].map(bars),
            [4, 4, 3, 3, 2, 2, 1, 1]
        );
        let n = Network {
            ssid: "a".into(),
            signal: None,
            security: Security::Unknown,
            saved: true,
        };
        assert_eq!(n.bars(), 0, "out of range");
    }

    #[test]
    fn no_failure_text_carries_a_password_or_a_reason_from_the_system() {
        let e = JoinError::Failed("socket gone".into());
        assert_eq!(e.text(), "Wi-Fi did not answer");
        assert!(e.to_string().contains("socket gone"), "for the log");
        for e in [
            JoinError::BadKey,
            JoinError::Unsupported,
            JoinError::WrongKey,
            JoinError::NoAddress,
        ] {
            assert_eq!(e.to_string(), e.text());
        }
    }
}
