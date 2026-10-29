//! The Kobo's radio (ARM only): the supplicant's control socket for
//! everything about networks, and `wifi.sh` (in the package, after
//! KOReader's `enable-wifi.sh` and `disable-wifi.sh`) for loading the
//! driver and starting `wpa_supplicant`.
//!
//! The interface and the driver come from the stock reader's environment
//! (`INTERFACE`, `WIFI_MODULE`; the Forma: `eth0`, `8192es`), which
//! `launch.sh` keeps. Nothing in the device tree here is verified on a
//! Forma yet; `docs/dev/wifi-design.md` lists what the first run must check.

use super::wpa::{Net, Supplicant};
use super::{Backend, JoinError, Network, Security, Status};
use std::collections::HashMap;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Where the package keeps `wifi.sh`.
const ADDS: &str = "/mnt/onboard/.adds/sigma-scribe";
/// `wpa_supplicant`'s control sockets.
const CTRL_DIR: &str = "/var/run/wpa_supplicant";
/// How long the script may take to load a driver and start the supplicant.
const POWER_ON: Duration = Duration::from_secs(45);
/// How long it may take to stop them.
const POWER_OFF: Duration = Duration::from_secs(25);
/// How long DHCP may take.
const DHCP: Duration = Duration::from_secs(35);
/// How long a radio just turned on gets to rejoin a saved network.
const REJOIN: Duration = Duration::from_secs(15);

/// The real radio.
pub struct KoboWifi {
    iface: String,
    module: String,
    script: PathBuf,
    sup: Supplicant,
    /// How the networks of the last scan are secured, for joining them.
    known: HashMap<String, Security>,
}

impl KoboWifi {
    /// The radio as the environment describes it: `INTERFACE` (default
    /// `eth0`), `WIFI_MODULE` (default `8192es`), `SIGMA_WIFI_DIR` for
    /// `wifi.sh` (default the package folder), `SIGMA_WPA_DIR` for the
    /// control sockets.
    #[must_use]
    pub fn from_env() -> Self {
        let var = |k: &str, d: &str| {
            std::env::var(k)
                .ok()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| d.to_owned())
        };
        let iface = var("INTERFACE", "eth0");
        let dir = PathBuf::from(var("SIGMA_WPA_DIR", CTRL_DIR));
        crate::wifi::log_line(&format!(
            "interface {iface}, module {}",
            var("WIFI_MODULE", "8192es")
        ));
        Self {
            sup: Supplicant::new(&dir, &iface),
            module: var("WIFI_MODULE", "8192es"),
            script: PathBuf::from(var("SIGMA_WIFI_DIR", ADDS)).join("wifi.sh"),
            iface,
            known: HashMap::new(),
        }
    }

    /// The interface exists and is up.
    fn iface_up(&self) -> bool {
        let base = format!("/sys/class/net/{}", self.iface);
        std::fs::read_to_string(format!("{base}/flags"))
            .ok()
            .and_then(|f| u32::from_str_radix(f.trim().trim_start_matches("0x"), 16).ok())
            .is_some_and(|f| f & 1 != 0)
    }

    /// `wifi.sh <what>`, with a time limit. Its `wifi: …` lines go to the
    /// log. Returns whether it exited with success.
    fn script(&self, what: &str, limit: Duration) -> io::Result<bool> {
        let mut child = Command::new("sh")
            .arg(&self.script)
            .arg(what)
            .env("INTERFACE", &self.iface)
            .env("WIFI_MODULE", &self.module)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut out = child.stdout.take();
        let reader = std::thread::spawn(move || {
            let mut s = String::new();
            if let Some(o) = out.as_mut() {
                let _ = o.read_to_string(&mut s);
            }
            s
        });
        let end = Instant::now() + limit;
        let status = loop {
            if let Some(st) = child.try_wait()? {
                break Some(st);
            }
            if Instant::now() >= end {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let text = reader.join().unwrap_or_default();
        for line in text.lines().filter(|l| l.starts_with("wifi:")) {
            crate::wifi::log_line(line.trim_start_matches("wifi:").trim());
        }
        match status {
            Some(st) => Ok(st.success()),
            None => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("wifi.sh {what} took more than {} s", limit.as_secs()),
            )),
        }
    }

    /// The interface's address, read from `ifconfig`.
    fn ifconfig_address(&self) -> Option<String> {
        let out = Command::new("ifconfig").arg(&self.iface).output().ok()?;
        crate::wifi::parse_ifconfig_address(&String::from_utf8_lossy(&out.stdout))
    }
}

/// What a join needs of the system: the script's DHCP step and `ifconfig`.
struct KoboNet<'a>(&'a KoboWifi);

impl Net for KoboNet<'_> {
    fn kick_dhcp(&mut self) {
        // It reports its own failure in the log; the join waits for an
        // address either way.
        let _ = self.0.script("dhcp", DHCP);
    }
    fn address(&mut self) -> Option<String> {
        self.0.ifconfig_address()
    }
}

impl Backend for KoboWifi {
    fn status(&mut self) -> io::Result<Status> {
        let up = self.iface_up() && self.sup.present();
        if !up {
            return Ok(Status {
                powered: false,
                ssid: None,
                ip: None,
            });
        }
        let link = self.sup.link().unwrap_or_default();
        let joined = link.completed().then(|| link.ssid.clone()).flatten();
        let ip = joined
            .as_ref()
            .and_then(|_| link.ip.clone().or_else(|| self.ifconfig_address()));
        Ok(Status {
            powered: true,
            ssid: joined,
            ip,
        })
    }

    fn set_power(&mut self, on: bool) -> io::Result<()> {
        if !on {
            self.script("off", POWER_OFF)?;
            return if self.iface_up() && self.sup.present() {
                Err(io::Error::other("Wi-Fi did not turn off"))
            } else {
                Ok(())
            };
        }
        if !self.script("on", POWER_ON)? {
            return Err(io::Error::other("wifi.sh on failed (see the log)"));
        }
        // A saved network in range is joined by the supplicant on its own;
        // give it a moment, then ask DHCP for an address.
        let end = Instant::now() + REJOIN;
        while Instant::now() < end {
            if self.sup.link().is_ok_and(|l| l.completed()) {
                let _ = self.script("dhcp", DHCP);
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        Ok(())
    }

    fn scan(&mut self) -> io::Result<Vec<Network>> {
        let nets = self.sup.scan()?;
        self.known = nets.iter().map(|n| (n.ssid.clone(), n.security)).collect();
        Ok(nets)
    }

    fn join(&mut self, ssid: &str, key: Option<&str>) -> Result<(), JoinError> {
        let security = self.known.get(ssid).copied().unwrap_or(if key.is_some() {
            Security::Wpa
        } else {
            Security::Open
        });
        let mut net = KoboNet(self);
        let sup = self.sup.clone();
        sup.join(ssid, security, key, &mut net)
    }

    fn forget(&mut self, ssid: &str) -> io::Result<()> {
        self.sup.forget(ssid)
    }
}
