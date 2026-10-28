//! The `wpa_supplicant` control interface (v1.18).
//!
//! `wpa_supplicant` listens on a Unix datagram socket,
//! `<dir>/<interface>` (`/var/run/wpa_supplicant/eth0` on the Forma). A
//! client binds a socket of its own, sends one command as text and reads
//! one reply. This is the whole of what `wpa_cli` does, so the app talks to
//! the socket itself and needs no `wpa_cli` binary.
//!
//! Commands used: `STATUS`, `SCAN`, `SCAN_RESULTS`, `LIST_NETWORKS`,
//! `ADD_NETWORK`, `SET_NETWORK`, `SELECT_NETWORK`, `ENABLE_NETWORK`,
//! `REMOVE_NETWORK`, `SAVE_CONFIG`, `RECONNECT`, `ATTACH` and `DETACH`.
//! Saved networks are whatever the supplicant's own config file holds (the
//! stock reader's, on a Kobo), and a new one is saved into it with
//! `SAVE_CONFIG`.
//!
//! A command is never logged and never put in an error: `SET_NETWORK ... psk`
//! carries the password.

use super::{JoinError, Network, Security};
use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// The biggest reply read: a scan of a crowded place is a few tens of KB.
const MAX_REPLY: usize = 128 * 1024;
/// A reply that does not come in this long means a hung supplicant.
const REPLY_TIMEOUT: Duration = Duration::from_secs(3);

/// A connection to the control socket: our own socket, bound to a private
/// path in the temporary folder, connected to the supplicant's.
pub struct Ctrl {
    sock: UnixDatagram,
    local: PathBuf,
}

impl Ctrl {
    /// Opens the control socket at `path`.
    ///
    /// # Errors
    /// The socket does not exist or cannot be reached.
    pub fn open(path: &Path) -> io::Result<Self> {
        static N: AtomicU32 = AtomicU32::new(0);
        let local = std::env::temp_dir().join(format!(
            "sigma-wpa-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&local);
        let sock = UnixDatagram::bind(&local)?;
        // From here the file is ours to remove, whatever fails.
        let me = Self { sock, local };
        me.sock.connect(path)?;
        me.sock.set_read_timeout(Some(REPLY_TIMEOUT))?;
        me.sock.set_write_timeout(Some(REPLY_TIMEOUT))?;
        Ok(me)
    }

    /// Sends `cmd` and returns the reply. Event messages (`<3>CTRL-EVENT-…`)
    /// that arrive first, after [`Ctrl::attach`], are skipped.
    ///
    /// # Errors
    /// The supplicant did not answer in time, or the socket failed.
    pub fn request(&self, cmd: &str) -> io::Result<String> {
        self.sock.send(cmd.as_bytes())?;
        let mut buf = vec![0u8; MAX_REPLY];
        loop {
            let n = self.sock.recv(&mut buf)?;
            let text = String::from_utf8_lossy(&buf[..n]).into_owned();
            if !text.starts_with('<') {
                return Ok(text);
            }
        }
    }

    /// Asks for unsolicited events, so [`Ctrl::wait_event`] can wait for a
    /// scan to end.
    ///
    /// # Errors
    /// As [`Ctrl::request`].
    pub fn attach(&self) -> io::Result<()> {
        ok(&self.request("ATTACH")?)
    }

    /// Waits up to `limit` for an event message containing `what`.
    /// Returns whether it came.
    #[must_use]
    pub fn wait_event(&self, what: &str, limit: Duration) -> bool {
        let end = Instant::now() + limit;
        let mut buf = vec![0u8; MAX_REPLY];
        let mut seen = false;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() || self.sock.set_read_timeout(Some(left)).is_err() {
                break;
            }
            match self.sock.recv(&mut buf) {
                Ok(n) if String::from_utf8_lossy(&buf[..n]).contains(what) => {
                    seen = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let _ = self.sock.set_read_timeout(Some(REPLY_TIMEOUT));
        seen
    }
}

impl Drop for Ctrl {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.local);
    }
}

/// `Ok` for the reply `OK`; anything else is an error naming the reply's
/// first line (which never contains the command).
///
/// # Errors
/// The reply was not `OK`.
pub fn ok(reply: &str) -> io::Result<()> {
    if reply.trim() == "OK" {
        Ok(())
    } else {
        let first = reply.lines().next().unwrap_or("").trim();
        Err(io::Error::other(format!(
            "wpa_supplicant replied {}",
            if first.is_empty() { "nothing" } else { first }
        )))
    }
}

/// The text of a name as `wpa_supplicant` prints it: `\xNN`, `\\`, `\"`,
/// `\n`, `\r`, `\t` and `\e` escapes undone, then read as UTF-8.
#[must_use]
pub fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let hex_at = |i: usize| b.get(i).and_then(|&c| (c as char).to_digit(16));
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' || i + 1 >= b.len() {
            out.push(b[i]);
            i += 1;
            continue;
        }
        match b[i + 1] {
            b'x' => {
                if let (Some(h), Some(l)) = (hex_at(i + 2), hex_at(i + 3)) {
                    out.push((h * 16 + l) as u8);
                    i += 4;
                } else {
                    out.push(b[i]);
                    i += 1;
                }
                continue;
            }
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'e' => out.push(0x1b),
            c @ (b'\\' | b'"') => out.push(c),
            _ => {
                out.push(b[i]);
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A name as the hex string `SET_NETWORK ssid` takes: no quoting to get
/// wrong for any name.
#[must_use]
pub fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

/// `STATUS`, the fields used.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Link {
    /// `wpa_state`: `COMPLETED`, `SCANNING`, `4WAY_HANDSHAKE`, …
    pub state: String,
    /// The network, when there is one.
    pub ssid: Option<String>,
    /// The address, when the supplicant knows it.
    pub ip: Option<String>,
}

impl Link {
    /// The handshake is complete.
    #[must_use]
    pub fn completed(&self) -> bool {
        self.state == "COMPLETED"
    }
}

/// Parses a `STATUS` reply.
#[must_use]
pub fn parse_status(reply: &str) -> Link {
    let mut l = Link::default();
    for line in reply.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k {
            "wpa_state" => l.state = v.to_owned(),
            "ssid" => l.ssid = Some(unescape(v)),
            "ip_address" if !v.is_empty() => l.ip = Some(v.to_owned()),
            _ => {}
        }
    }
    l
}

/// One network from `SCAN_RESULTS`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// Its name.
    pub ssid: String,
    /// Signal level, dBm.
    pub signal: i32,
    /// How it is secured.
    pub security: Security,
}

/// Parses a `SCAN_RESULTS` reply (`bssid / frequency / signal level /
/// flags / ssid`, tab separated, after a header). One entry per name, the
/// strongest; hidden networks (no name) are left out; strongest first.
#[must_use]
pub fn parse_scan(reply: &str) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    for line in reply
        .lines()
        .skip_while(|l| !l.starts_with("bssid"))
        .skip(1)
    {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
        let ssid = unescape(f[4]);
        // A hidden network is listed with an empty name or NULs.
        if ssid.is_empty() || ssid.contains('\0') || ssid.len() > 32 {
            continue;
        }
        let signal = f[2].trim().parse().unwrap_or(-100);
        let security = Security::from_flags(f[3]);
        match out.iter_mut().find(|n| n.ssid == ssid) {
            Some(n) if signal > n.signal => {
                n.signal = signal;
                n.security = security;
            }
            Some(_) => {}
            None => out.push(Found {
                ssid,
                signal,
                security,
            }),
        }
    }
    out.sort_by_key(|n| std::cmp::Reverse(n.signal));
    out
}

/// One network `wpa_supplicant` knows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    /// Its number in the supplicant.
    pub id: u32,
    /// Its name.
    pub ssid: String,
    /// It is the one joined.
    pub current: bool,
    /// Held off after failed handshakes (a wrong password).
    pub temp_disabled: bool,
}

/// Parses a `LIST_NETWORKS` reply (`network id / ssid / bssid / flags`).
#[must_use]
pub fn parse_list(reply: &str) -> Vec<Saved> {
    reply
        .lines()
        .skip_while(|l| !l.starts_with("network id"))
        .skip(1)
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let id = f.first()?.trim().parse().ok()?;
            let flags = f.get(3).copied().unwrap_or("");
            Some(Saved {
                id,
                ssid: unescape(f.get(1).copied().unwrap_or("")),
                current: flags.contains("[CURRENT]"),
                temp_disabled: flags.contains("[TEMP-DISABLED]"),
            })
        })
        .collect()
}

/// Whether `key` is one the network type takes: WPA 8 to 63 characters, or
/// 64 hex digits; WEP 5 or 13 characters, or 10 or 26 hex digits; no
/// control characters.
#[must_use]
pub fn valid_key(security: Security, key: &str) -> bool {
    if key.chars().any(char::is_control) {
        return false;
    }
    let hex_digits = key.bytes().all(|b| b.is_ascii_hexdigit());
    match security {
        Security::Wpa => (8..=63).contains(&key.len()) || (key.len() == 64 && hex_digits),
        Security::Wep => {
            matches!(key.len(), 5 | 13) || (matches!(key.len(), 10 | 26) && hex_digits)
        }
        _ => false,
    }
}

/// How long the steps of a join may take, and how often they are checked.
#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// Between two looks at the supplicant.
    pub poll: Duration,
    /// For the handshake to complete.
    pub associate: Duration,
    /// For an address after that. The first DHCP on a new network can be
    /// slow (20 s was too short on the Forma).
    pub address: Duration,
    /// For a scan to finish.
    pub scan: Duration,
}

impl Timing {
    /// What the Kobo uses.
    pub const DEFAULT: Self = Self {
        poll: Duration::from_millis(500),
        associate: Duration::from_secs(25),
        address: Duration::from_secs(40),
        scan: Duration::from_secs(10),
    };
}

/// What a join needs from the system beyond the supplicant.
pub trait Net {
    /// Asks DHCP for an address on the new network.
    fn kick_dhcp(&mut self);
    /// The interface's address, when it has one.
    fn address(&mut self) -> Option<String>;
}

/// One interface's supplicant.
#[derive(Clone, Debug)]
pub struct Supplicant {
    path: PathBuf,
    timing: Timing,
}

impl Supplicant {
    /// The supplicant of `iface`, whose socket is in `dir`.
    #[must_use]
    pub fn new(dir: &Path, iface: &str) -> Self {
        Self {
            path: dir.join(iface),
            timing: Timing::DEFAULT,
        }
    }

    /// The same with other timings (the tests').
    #[must_use]
    pub fn with_timing(mut self, t: Timing) -> Self {
        self.timing = t;
        self
    }

    /// The control socket exists.
    #[must_use]
    pub fn present(&self) -> bool {
        self.path.exists()
    }

    fn open(&self) -> io::Result<Ctrl> {
        Ctrl::open(&self.path)
    }

    /// The link state.
    ///
    /// # Errors
    /// The supplicant did not answer.
    pub fn link(&self) -> io::Result<Link> {
        Ok(parse_status(&self.open()?.request("STATUS")?))
    }

    /// The saved networks.
    ///
    /// # Errors
    /// The supplicant did not answer.
    pub fn saved(&self) -> io::Result<Vec<Saved>> {
        Ok(parse_list(&self.open()?.request("LIST_NETWORKS")?))
    }

    /// Scans, waits for the results, and lists them with the saved
    /// networks: found ones strongest first, then saved ones out of range.
    ///
    /// # Errors
    /// The supplicant did not answer.
    pub fn scan(&self) -> io::Result<Vec<Network>> {
        let c = self.open()?;
        let attached = c.attach().is_ok();
        let r = c.request("SCAN")?;
        if r.trim() == "OK" && attached {
            // Whether the event came or not, the results are read.
            let _ = c.wait_event("CTRL-EVENT-SCAN-RESULTS", self.timing.scan);
        } else if r.trim() == "FAIL-BUSY" {
            // A scan already running fills the same results.
            std::thread::sleep(self.timing.poll.min(Duration::from_secs(2)));
        }
        let found = parse_scan(&c.request("SCAN_RESULTS")?);
        let saved = parse_list(&c.request("LIST_NETWORKS")?);
        if attached {
            let _ = c.request("DETACH");
        }
        Ok(merge(found, &saved))
    }

    /// Removes every saved network called `ssid`, and saves the config.
    ///
    /// # Errors
    /// The supplicant did not answer or refused.
    pub fn forget(&self, ssid: &str) -> io::Result<()> {
        let c = self.open()?;
        for n in parse_list(&c.request("LIST_NETWORKS")?)
            .into_iter()
            .filter(|n| n.ssid == ssid)
        {
            ok(&c.request(&format!("REMOVE_NETWORK {}", n.id))?)?;
        }
        ok(&c.request("SAVE_CONFIG")?)
    }

    /// Joins `ssid`. `key` is `None` for an open network or a saved one;
    /// otherwise it is stored for the network (a saved network's key is
    /// replaced). Waits for the handshake and for an address, and saves the
    /// config once the handshake is done: a network that does not work is
    /// never kept.
    ///
    /// # Errors
    /// See [`JoinError`].
    pub fn join(
        &self,
        ssid: &str,
        security: Security,
        key: Option<&str>,
        net: &mut dyn Net,
    ) -> Result<(), JoinError> {
        if security == Security::Unsupported {
            return Err(JoinError::Unsupported);
        }
        if key.is_some_and(|k| !valid_key(security, k)) {
            return Err(JoinError::BadKey);
        }
        let c = self.open()?;
        let existing = parse_list(&c.request("LIST_NETWORKS")?)
            .into_iter()
            .find(|n| n.ssid == ssid);
        let added = existing.is_none();
        let id = match &existing {
            Some(n) => n.id,
            None => c
                .request("ADD_NETWORK")?
                .trim()
                .parse::<u32>()
                .map_err(|_| JoinError::Failed("ADD_NETWORK failed".into()))?,
        };
        let step = |r: io::Result<String>| -> Result<(), JoinError> {
            r.and_then(|s| ok(&s)).map_err(JoinError::from)
        };
        // A saved network joined without a key keeps the key it has.
        let configured = (|| -> Result<(), JoinError> {
            if added || key.is_some() {
                step(c.request(&format!("SET_NETWORK {id} ssid {}", hex(ssid))))?;
                match (security, key) {
                    (Security::Wpa, Some(k)) => {
                        // Raw between quotes: the supplicant takes everything
                        // up to the last quote, with no escapes.
                        step(c.request(&format!("SET_NETWORK {id} key_mgmt WPA-PSK")))?;
                        step(c.request(&format!("SET_NETWORK {id} psk \"{k}\"")))?;
                    }
                    (Security::Wep, Some(k)) => {
                        step(c.request(&format!("SET_NETWORK {id} key_mgmt NONE")))?;
                        let v = if matches!(k.len(), 10 | 26) {
                            k.to_owned()
                        } else {
                            format!("\"{k}\"")
                        };
                        step(c.request(&format!("SET_NETWORK {id} wep_key0 {v}")))?;
                        step(c.request(&format!("SET_NETWORK {id} wep_tx_keyidx 0")))?;
                    }
                    _ => step(c.request(&format!("SET_NETWORK {id} key_mgmt NONE")))?,
                }
            }
            step(c.request(&format!("SELECT_NETWORK {id}")))
        })();
        let outcome = configured.and_then(|()| self.wait_joined(&c, id, ssid, net));
        match &outcome {
            Ok(()) | Err(JoinError::NoAddress) => {
                // The handshake worked: the other networks may be tried
                // again, and this one is kept.
                let _ = c.request("ENABLE_NETWORK all");
                let kept = c.request("SAVE_CONFIG").and_then(|r| ok(&r));
                outcome.and(kept.map_err(JoinError::from))
            }
            Err(_) => {
                if added {
                    let _ = c.request(&format!("REMOVE_NETWORK {id}"));
                }
                let _ = c.request("ENABLE_NETWORK all");
                let _ = c.request("RECONNECT");
                outcome
            }
        }
    }

    fn wait_joined(
        &self,
        c: &Ctrl,
        id: u32,
        ssid: &str,
        net: &mut dyn Net,
    ) -> Result<(), JoinError> {
        let t = self.timing;
        let end = Instant::now() + t.associate;
        loop {
            let link = parse_status(&c.request("STATUS")?);
            if link.completed() && link.ssid.as_deref() == Some(ssid) {
                break;
            }
            // Repeated failed handshakes hold the network off: a wrong key.
            let held = parse_list(&c.request("LIST_NETWORKS")?)
                .iter()
                .any(|n| n.id == id && n.temp_disabled);
            if held || Instant::now() >= end {
                return Err(JoinError::WrongKey);
            }
            std::thread::sleep(t.poll);
        }
        net.kick_dhcp();
        let end = Instant::now() + t.address;
        loop {
            if net.address().is_some() {
                return Ok(());
            }
            if parse_status(&c.request("STATUS")?).ip.is_some() {
                return Ok(());
            }
            if Instant::now() >= end {
                return Err(JoinError::NoAddress);
            }
            std::thread::sleep(t.poll);
        }
    }
}

/// The scan's networks, strongest first, with a saved network marked as
/// such, and the saved networks that were not found (out of range) added at
/// the end so they can still be forgotten.
#[must_use]
pub fn merge(found: Vec<Found>, saved: &[Saved]) -> Vec<Network> {
    let is_saved = |s: &str| saved.iter().any(|n| n.ssid == s);
    let mut out: Vec<Network> = found
        .into_iter()
        .map(|f| Network {
            saved: is_saved(&f.ssid),
            ssid: f.ssid,
            signal: Some(f.signal),
            security: f.security,
        })
        .collect();
    for s in saved {
        if !s.ssid.is_empty() && !out.iter().any(|n| n.ssid == s.ssid) {
            out.push(Network {
                ssid: s.ssid.clone(),
                signal: None,
                security: Security::Unknown,
                saved: true,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex, MutexGuard};

    /// The tests share the temp dir, where the one that counts client
    /// sockets must see only its own: they take turns.
    static TURN: Mutex<()> = Mutex::new(());

    const SCAN: &str = "bssid / frequency / signal level / flags / ssid\n\
        aa:aa:aa:aa:aa:01\t2412\t-70\t[WPA2-PSK-CCMP][ESS]\tHome\n\
        aa:aa:aa:aa:aa:02\t5180\t-42\t[WPA2-PSK-CCMP][ESS]\tHome\n\
        bb:bb:bb:bb:bb:01\t2437\t-55\t[ESS]\tCafe\n\
        cc:cc:cc:cc:cc:01\t2462\t-80\t[WPA2-EAP-CCMP][ESS]\tOffice\n\
        dd:dd:dd:dd:dd:01\t2412\t-60\t[WPA2-PSK-CCMP][ESS]\t\n\
        ee:ee:ee:ee:ee:01\t2412\t-50\t[WPA2-PSK-CCMP][ESS]\tCaf\\xc3\\xa9 \\\"2\\\"\n";

    #[test]
    fn a_scan_is_one_entry_per_name_strongest_first_hidden_ones_left_out() {
        let r = parse_scan(SCAN);
        let names: Vec<&str> = r.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["Home", "Café \"2\"", "Cafe", "Office"]);
        assert_eq!(r[0].signal, -42, "the stronger of the two Home points");
        assert_eq!(r[2].security, Security::Open);
        assert_eq!(r[3].security, Security::Unsupported);
        assert!(parse_scan("").is_empty());
        assert!(parse_scan("FAIL\n").is_empty());
        assert!(parse_scan("bssid / x\nnot\tenough\n").is_empty());
    }

    #[test]
    fn status_gives_state_name_and_address() {
        let l = parse_status(
            "bssid=aa:bb\nfreq=2412\nssid=My\\x20Net\nid=0\nmode=station\nwpa_state=COMPLETED\nip_address=192.168.1.20\naddress=cc:dd\n",
        );
        assert!(l.completed());
        assert_eq!(l.ssid.as_deref(), Some("My Net"));
        assert_eq!(l.ip.as_deref(), Some("192.168.1.20"));
        let l = parse_status("wpa_state=SCANNING\naddress=cc:dd\n");
        assert!(!l.completed() && l.ssid.is_none() && l.ip.is_none());
        assert_eq!(parse_status("garbage"), Link::default());
    }

    #[test]
    fn the_network_list_marks_the_current_and_the_held_off() {
        let l = parse_list(
            "network id / ssid / bssid / flags\n0\tHome\tany\t[CURRENT]\n1\tCafe\tany\t[DISABLED]\n2\tBad\tany\t[TEMP-DISABLED]\n3\tOld\tany\n",
        );
        assert_eq!(l.len(), 4);
        assert!(l[0].current && !l[0].temp_disabled);
        assert!(l[2].temp_disabled);
        assert_eq!(
            (l[3].id, l[3].ssid.as_str(), l[3].current),
            (3, "Old", false)
        );
        assert!(parse_list("FAIL\n").is_empty());
        assert!(parse_list("network id / ssid / bssid / flags\nx\tno id\n").is_empty());
    }

    #[test]
    fn names_are_unescaped_and_sent_as_hex() {
        assert_eq!(unescape("a\\\\b\\\"c\\x41\\n"), "a\\b\"cA\n");
        assert_eq!(unescape("\\x"), "\\x", "a broken escape stays as it is");
        assert_eq!(unescape("\\xzz"), "\\xzz");
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape("end\\"), "end\\");
        assert_eq!(hex("Aé"), "41c3a9");
    }

    #[test]
    fn keys_are_checked_against_the_kind_of_network() {
        assert!(valid_key(Security::Wpa, "12345678"));
        assert!(!valid_key(Security::Wpa, "1234567"));
        assert!(valid_key(Security::Wpa, &"a".repeat(63)));
        assert!(valid_key(Security::Wpa, &"a".repeat(64)), "64 hex digits");
        assert!(!valid_key(Security::Wpa, &"g".repeat(64)), "64 but not hex");
        assert!(
            !valid_key(Security::Wpa, "tab\there!"),
            "control characters"
        );
        assert!(valid_key(Security::Wep, "12345"));
        assert!(valid_key(Security::Wep, "0123456789"));
        assert!(!valid_key(Security::Wep, "123456"));
        assert!(!valid_key(Security::Open, "anything"));
        assert!(
            valid_key(Security::Wpa, "say \"hi\" \\ now"),
            "quotes are fine"
        );
    }

    #[test]
    fn saved_networks_out_of_range_follow_the_found_ones() {
        let saved = parse_list(
            "network id / ssid / bssid / flags\n0\tHome\tany\t[CURRENT]\n1\tHolidays\tany\t\n",
        );
        let m = merge(parse_scan(SCAN), &saved);
        let names: Vec<&str> = m.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["Home", "Café \"2\"", "Cafe", "Office", "Holidays"]);
        assert!(m[0].saved && !m[1].saved);
        let h = &m[4];
        assert_eq!(
            (h.signal, h.security, h.saved),
            (None, Security::Unknown, true)
        );
        assert_eq!(h.bars(), 0);
    }

    /// A stand-in supplicant: a datagram socket with a little state,
    /// recording every command so a test can look at what was sent.
    struct Fake {
        dir: PathBuf,
        log: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        _turn: MutexGuard<'static, ()>,
    }

    #[derive(Clone)]
    struct Behaviour {
        /// The key the networks accept.
        key: &'static str,
        /// Saved networks to begin with (id, name).
        saved: Vec<(u32, &'static str)>,
        /// STATUS carries `ip_address` once joined.
        ip: bool,
    }

    const FAKE_SCAN: &str = "bssid / frequency / signal level / flags / ssid\n\
        aa\t2412\t-42\t[WPA2-PSK-CCMP][ESS]\tHome\n\
        bb\t2437\t-55\t[ESS]\tCafe\n\
        cc\t2462\t-61\t[WPA2-PSK-CCMP][ESS]\tOffice\n";

    impl Fake {
        fn start(tag: &str, b: Behaviour) -> Self {
            let turn = TURN
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let dir =
                std::env::temp_dir().join(format!("sigma-wpa-test-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let sock = UnixDatagram::bind(dir.join("wlan0")).unwrap();
            sock.set_read_timeout(Some(Duration::from_millis(30)))
                .unwrap();
            let log = Arc::new(Mutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let (l2, s2) = (log.clone(), stop.clone());
            std::thread::spawn(move || {
                // (id, name): the networks the config held, and the ones added.
                let mut nets: Vec<(u32, String)> =
                    b.saved.iter().map(|(i, n)| (*i, (*n).to_owned())).collect();
                let mut next_id = 10u32;
                let mut selected: Option<u32> = None;
                let mut psk: Option<String> = None;
                let mut attached: Option<PathBuf> = None;
                let mut buf = [0u8; 4096];
                while !s2.load(Ordering::SeqCst) {
                    let Ok((n, from)) = sock.recv_from(&mut buf) else {
                        continue;
                    };
                    let Some(to) = from.as_pathname().map(Path::to_path_buf) else {
                        continue;
                    };
                    let cmd = String::from_utf8_lossy(&buf[..n]).into_owned();
                    l2.lock().unwrap().push(cmd.clone());
                    let word = cmd.split_whitespace().next().unwrap_or("");
                    let wrong = psk.as_deref().is_some_and(|p| p != b.key);
                    let reply: String = match word {
                        "STATUS" => {
                            let name = nets
                                .iter()
                                .find(|(i, _)| Some(*i) == selected)
                                .map(|n| n.1.clone());
                            match name {
                                Some(name) if !wrong => format!(
                                    "wpa_state=COMPLETED\nssid={name}\n{}",
                                    if b.ip { "ip_address=10.0.0.5\n" } else { "" }
                                ),
                                _ => "wpa_state=SCANNING\n".into(),
                            }
                        }
                        "ATTACH" => {
                            attached = Some(to.clone());
                            "OK\n".into()
                        }
                        "DETACH" => {
                            attached = None;
                            "OK\n".into()
                        }
                        "SCAN" => {
                            let _ = sock.send_to(b"OK\n", &to);
                            if let Some(a) = &attached {
                                let _ = sock.send_to(b"<3>CTRL-EVENT-SCAN-STARTED \n", a);
                                let _ = sock.send_to(b"<3>CTRL-EVENT-SCAN-RESULTS \n", a);
                            }
                            continue;
                        }
                        "SCAN_RESULTS" => FAKE_SCAN.to_owned(),
                        "LIST_NETWORKS" => {
                            let mut s = String::from("network id / ssid / bssid / flags\n");
                            for (i, n) in &nets {
                                let flags = if Some(*i) == selected && wrong {
                                    "[TEMP-DISABLED]"
                                } else if Some(*i) == selected {
                                    "[CURRENT]"
                                } else {
                                    ""
                                };
                                s.push_str(&format!("{i}\t{n}\tany\t{flags}\n"));
                            }
                            s
                        }
                        "ADD_NETWORK" => {
                            next_id += 1;
                            nets.push((next_id, String::new()));
                            format!("{next_id}\n")
                        }
                        "SET_NETWORK" => {
                            let p: Vec<&str> = cmd.splitn(4, ' ').collect();
                            let id: u32 = p[1].parse().unwrap();
                            match p[2] {
                                "ssid" => {
                                    let raw: Vec<u8> = (0..p[3].len() / 2)
                                        .map(|i| {
                                            u8::from_str_radix(&p[3][2 * i..2 * i + 2], 16).unwrap()
                                        })
                                        .collect();
                                    if let Some(e) = nets.iter_mut().find(|e| e.0 == id) {
                                        e.1 = String::from_utf8(raw).unwrap();
                                    }
                                }
                                "psk" => psk = Some(p[3].trim_matches('"').to_owned()),
                                _ => {}
                            }
                            "OK\n".into()
                        }
                        "SELECT_NETWORK" => {
                            selected = cmd.split_whitespace().nth(1).and_then(|v| v.parse().ok());
                            "OK\n".into()
                        }
                        "REMOVE_NETWORK" => {
                            let id: u32 = cmd.split_whitespace().nth(1).unwrap().parse().unwrap();
                            nets.retain(|s| s.0 != id);
                            "OK\n".into()
                        }
                        "ENABLE_NETWORK" | "SAVE_CONFIG" | "RECONNECT" => "OK\n".into(),
                        _ => "UNKNOWN COMMAND\n".into(),
                    };
                    let _ = sock.send_to(reply.as_bytes(), &to);
                }
            });
            Self {
                dir,
                log,
                stop,
                _turn: turn,
            }
        }

        fn sent(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(80));
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    struct TestNet {
        kicked: u32,
        has_ip: bool,
        /// An address that comes this long after DHCP was asked.
        after: Option<Duration>,
        kicked_at: Option<Instant>,
    }
    impl TestNet {
        fn new(has_ip: bool) -> Self {
            Self {
                kicked: 0,
                has_ip,
                after: None,
                kicked_at: None,
            }
        }
    }
    impl Net for TestNet {
        fn kick_dhcp(&mut self) {
            self.kicked += 1;
            self.kicked_at = Some(Instant::now());
        }
        fn address(&mut self) -> Option<String> {
            let late =
                matches!((self.after, self.kicked_at), (Some(d), Some(t)) if t.elapsed() >= d);
            (self.has_ip || late).then(|| "10.0.0.5".to_owned())
        }
    }

    fn fast() -> Timing {
        Timing {
            poll: Duration::from_millis(10),
            associate: Duration::from_millis(400),
            address: Duration::from_millis(100),
            scan: Duration::from_millis(500),
        }
    }

    fn behaviour() -> Behaviour {
        Behaviour {
            key: "hunter2hunter2",
            saved: vec![(0, "Home")],
            ip: true,
        }
    }

    #[test]
    fn the_client_talks_to_a_supplicant_over_its_socket() {
        let f = Fake::start("talk", behaviour());
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        assert!(s.present());
        assert!(!s.link().unwrap().completed());
        let nets = s.scan().unwrap();
        let names: Vec<&str> = nets.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["Home", "Cafe", "Office"]);
        assert!(nets[0].saved && !nets[1].saved);
        assert!(f.sent().contains(&"SCAN".to_owned()));
        assert_eq!(f.sent().last().map(String::as_str), Some("DETACH"));
        let none = Supplicant::new(&f.dir, "wlan9");
        assert!(!none.present());
        assert!(none.link().is_err());
        // The client's own socket files are gone.
        let left = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("sigma-wpa-{}-", std::process::id()))
            })
            .count();
        assert_eq!(left, 0, "no client socket left behind");
    }

    #[test]
    fn a_new_network_is_added_selected_and_saved_once_it_has_an_address() {
        let f = Fake::start("join", behaviour());
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        let mut net = TestNet::new(true);
        s.join("Office", Security::Wpa, Some("hunter2hunter2"), &mut net)
            .unwrap();
        assert_eq!(net.kicked, 1, "DHCP was asked once the handshake was done");
        let sent = f.sent();
        let pos = |p: &str| sent.iter().position(|c| c.starts_with(p)).unwrap();
        assert!(pos("ADD_NETWORK") < pos("SET_NETWORK"));
        assert!(pos("SELECT_NETWORK") < pos("SAVE_CONFIG"));
        assert!(sent.contains(&format!("SET_NETWORK 11 ssid {}", hex("Office"))));
        assert!(sent.contains(&"SET_NETWORK 11 key_mgmt WPA-PSK".to_owned()));
        assert!(sent.contains(&"SET_NETWORK 11 psk \"hunter2hunter2\"".to_owned()));
        assert!(sent.contains(&"ENABLE_NETWORK all".to_owned()));
        assert_eq!(
            sent.iter().filter(|c| c.contains("hunter2")).count(),
            1,
            "the key is sent once, in SET_NETWORK psk"
        );
        assert!(s.saved().unwrap().iter().any(|n| n.ssid == "Office"));
    }

    #[test]
    fn a_wrong_key_removes_the_new_network_and_saves_nothing() {
        let f = Fake::start("wrong", behaviour());
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        let mut net = TestNet::new(true);
        let e = s
            .join("Office", Security::Wpa, Some("not-the-key"), &mut net)
            .unwrap_err();
        assert_eq!(e, JoinError::WrongKey);
        assert_eq!(net.kicked, 0);
        let sent = f.sent();
        assert!(sent.contains(&"REMOVE_NETWORK 11".to_owned()));
        assert!(sent.contains(&"RECONNECT".to_owned()));
        assert!(!sent.contains(&"SAVE_CONFIG".to_owned()), "never kept");
        assert!(s.saved().unwrap().iter().all(|n| n.ssid != "Office"));
    }

    #[test]
    fn a_saved_network_is_selected_without_a_key_and_keeps_the_key_it_has() {
        let f = Fake::start("saved", behaviour());
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        let mut net = TestNet::new(true);
        s.join("Home", Security::Wpa, None, &mut net).unwrap();
        let sent = f.sent();
        assert!(sent.contains(&"SELECT_NETWORK 0".to_owned()));
        assert!(!sent
            .iter()
            .any(|c| c.starts_with("ADD_NETWORK") || c.starts_with("SET_NETWORK")));
    }

    #[test]
    fn a_key_typed_again_replaces_the_one_a_saved_network_has() {
        let f = Fake::start("rekey", behaviour());
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        let mut net = TestNet::new(true);
        s.join("Home", Security::Wpa, Some("hunter2hunter2"), &mut net)
            .unwrap();
        let sent = f.sent();
        assert!(!sent.iter().any(|c| c.starts_with("ADD_NETWORK")));
        assert!(sent.contains(&"SET_NETWORK 0 psk \"hunter2hunter2\"".to_owned()));
    }

    #[test]
    fn a_network_with_no_address_is_kept_and_reported() {
        let f = Fake::start(
            "noip",
            Behaviour {
                ip: false,
                ..behaviour()
            },
        );
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        let mut net = TestNet::new(false);
        let e = s.join("Cafe", Security::Open, None, &mut net).unwrap_err();
        assert_eq!(e, JoinError::NoAddress);
        let sent = f.sent();
        assert!(sent.contains(&"SET_NETWORK 11 key_mgmt NONE".to_owned()));
        assert!(
            sent.contains(&"SAVE_CONFIG".to_owned()),
            "the handshake worked"
        );
    }

    #[test]
    fn the_kobo_waits_longer_for_an_address_than_the_20_s_that_failed_on_the_forma() {
        // 25 s is a first DHCP that is slow, not a failure.
        assert!(Timing::DEFAULT.address >= Duration::from_secs(35));
    }

    #[test]
    fn a_slow_address_still_counts_as_joined() {
        // The same shape as an address after 25 s of a 40 s wait.
        let f = Fake::start(
            "slow",
            Behaviour {
                ip: false,
                ..behaviour()
            },
        );
        let t = Timing {
            address: Duration::from_millis(400),
            ..fast()
        };
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(t);
        let mut net = TestNet::new(false);
        net.after = Some(Duration::from_millis(250));
        let started = Instant::now();
        s.join("Cafe", Security::Open, None, &mut net).unwrap();
        assert!(started.elapsed() >= Duration::from_millis(250), "it waited");
        assert!(f.sent().contains(&"SAVE_CONFIG".to_owned()));
    }

    #[test]
    fn an_address_the_status_reports_ends_the_wait_too() {
        let f = Fake::start("status-ip", behaviour());
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        let mut net = TestNet::new(false);
        s.join("Office", Security::Wpa, Some("hunter2hunter2"), &mut net)
            .unwrap();
    }

    #[test]
    fn a_wrong_key_is_refused_long_before_the_address_wait_would_end() {
        let f = Fake::start("wrong-fast", behaviour());
        let t = Timing {
            address: Duration::from_secs(30),
            ..fast()
        };
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(t);
        let mut net = TestNet::new(false);
        let started = Instant::now();
        let e = s
            .join("Office", Security::Wpa, Some("not-the-key"), &mut net)
            .unwrap_err();
        assert_eq!(e, JoinError::WrongKey);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(net.kicked, 0);
    }

    #[test]
    fn forgetting_removes_every_network_of_that_name_and_saves() {
        let f = Fake::start(
            "forget",
            Behaviour {
                saved: vec![(0, "Home"), (1, "Cafe"), (2, "Home")],
                ..behaviour()
            },
        );
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        s.forget("Home").unwrap();
        let left: Vec<String> = s.saved().unwrap().into_iter().map(|n| n.ssid).collect();
        assert_eq!(left, ["Cafe"]);
        let sent = f.sent();
        assert!(sent.contains(&"REMOVE_NETWORK 0".to_owned()));
        assert!(sent.contains(&"REMOVE_NETWORK 2".to_owned()));
        assert!(sent.iter().any(|c| c == "SAVE_CONFIG"));
    }

    #[test]
    fn a_bad_key_or_an_unsupported_network_is_refused_before_anything_is_sent() {
        let f = Fake::start("refuse", behaviour());
        let s = Supplicant::new(&f.dir, "wlan0").with_timing(fast());
        let mut net = TestNet::new(true);
        assert_eq!(
            s.join("Office", Security::Wpa, Some("short"), &mut net),
            Err(JoinError::BadKey)
        );
        assert_eq!(
            s.join("Corp", Security::Unsupported, None, &mut net),
            Err(JoinError::Unsupported)
        );
        assert!(f.sent().is_empty());
    }

    #[test]
    fn ok_names_the_reply_and_never_the_command() {
        assert!(ok("OK\n").is_ok());
        let e = ok("FAIL\n").unwrap_err().to_string();
        assert!(e.contains("FAIL"), "{e}");
        assert!(ok("").unwrap_err().to_string().contains("nothing"));
    }
}
