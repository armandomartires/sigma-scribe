//! A pretend radio for the desktop window, the emulator and the tests.
//!
//! Deterministic: the networks are a fixed list, a scan finds the same ones,
//! and a WPA network takes any password of 8 or more characters except
//! `wrong-password`. Nothing touches the system.

use super::{Backend, JoinError, Network, Security, Status};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The password the pretend networks refuse.
pub const WRONG_PASSWORD: &str = "wrong-password";

/// An in-memory [`Backend`].
#[derive(Clone, Debug)]
pub struct FakeWifi {
    powered: bool,
    joined: Option<String>,
    networks: Vec<Network>,
    /// The next power-on fails (the tests' way to see a failure).
    pub fail_power: bool,
    /// Whether the networks hand out addresses. Cleared, a join succeeds
    /// without one (a slow DHCP); set again, the address comes. Shared, so a
    /// test keeps a handle on the radio the service owns.
    pub lease: Arc<AtomicBool>,
}

impl FakeWifi {
    /// A radio that is off, with a handful of networks around: a saved home
    /// network, an open café, two others with passwords, one the app cannot
    /// join, and a saved network out of range.
    #[must_use]
    pub fn demo() -> Self {
        let n = |ssid: &str, signal: Option<i32>, security, saved| Network {
            ssid: ssid.to_owned(),
            signal,
            security,
            saved,
        };
        Self {
            powered: false,
            joined: None,
            networks: vec![
                n("Home", Some(-42), Security::Wpa, true),
                n("Café Central", Some(-58), Security::Open, false),
                n("Office-5G", Some(-63), Security::Wpa, false),
                n("Neighbour", Some(-78), Security::Wpa, false),
                n("Campus", Some(-81), Security::Unsupported, false),
                n("Holiday house", None, Security::Unknown, true),
            ],
            fail_power: false,
            lease: Arc::new(AtomicBool::new(true)),
        }
    }

    /// The same, already on and joined to `Home`.
    #[must_use]
    pub fn demo_online() -> Self {
        let mut f = Self::demo();
        f.powered = true;
        f.joined = Some("Home".to_owned());
        f
    }
}

impl Backend for FakeWifi {
    fn status(&mut self) -> io::Result<Status> {
        Ok(Status {
            powered: self.powered,
            ssid: self.joined.clone(),
            ip: self
                .joined
                .as_ref()
                .filter(|_| self.lease.load(Ordering::SeqCst))
                .map(|_| "192.168.1.42".to_owned()),
        })
    }

    fn set_power(&mut self, on: bool) -> io::Result<()> {
        if on && std::mem::take(&mut self.fail_power) {
            return Err(io::Error::other("the driver did not load"));
        }
        self.powered = on;
        if !on {
            self.joined = None;
        } else if self.joined.is_none() {
            // Back on, the radio rejoins a saved network in range.
            self.joined = self
                .networks
                .iter()
                .find(|n| n.saved && n.signal.is_some())
                .map(|n| n.ssid.clone());
        }
        Ok(())
    }

    fn scan(&mut self) -> io::Result<Vec<Network>> {
        if !self.powered {
            return Err(io::Error::other("Wi-Fi is off"));
        }
        Ok(self.networks.clone())
    }

    fn join(&mut self, ssid: &str, key: Option<&str>) -> Result<(), JoinError> {
        let Some(n) = self.networks.iter_mut().find(|n| n.ssid == ssid) else {
            return Err(JoinError::Failed("no such network".into()));
        };
        match (n.security, key) {
            (Security::Unsupported, _) => return Err(JoinError::Unsupported),
            (Security::Wpa | Security::Wep, Some(k)) if k.len() < 8 => {
                return Err(JoinError::BadKey)
            }
            (Security::Wpa | Security::Wep, Some(k)) if k == WRONG_PASSWORD => {
                return Err(JoinError::WrongKey)
            }
            // A saved network joins on the key it has; one that is not saved
            // cannot be joined without one.
            (Security::Wpa | Security::Wep, None) if !n.saved => return Err(JoinError::WrongKey),
            _ => {}
        }
        n.saved = true;
        self.joined = Some(ssid.to_owned());
        if !self.lease.load(Ordering::SeqCst) {
            return Err(JoinError::NoAddress);
        }
        Ok(())
    }

    fn forget(&mut self, ssid: &str) -> io::Result<()> {
        for n in self.networks.iter_mut().filter(|n| n.ssid == ssid) {
            n.saved = false;
        }
        if self.joined.as_deref() == Some(ssid) {
            self.joined = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Busy, Service, View};
    use super::*;

    #[test]
    fn a_service_over_the_fake_follows_the_radio() {
        let s = Service::inline(FakeWifi::demo());
        let v = s.view();
        assert!(!v.powered && v.ssid.is_none() && v.networks.is_empty());
        let g = s.generation();
        s.power(true);
        assert!(s.generation() > g);
        let v = s.view();
        assert!(v.powered && v.scanned);
        assert_eq!(v.ssid.as_deref(), Some("Home"), "rejoins the saved network");
        assert_eq!(v.ip.as_deref(), Some("192.168.1.42"));
        assert_eq!(v.busy, Busy::Idle);
        assert_eq!(v.networks.len(), 6);
        s.power(false);
        let v = s.view();
        assert!(!v.powered && v.ssid.is_none() && v.networks.is_empty() && !v.scanned);
    }

    #[test]
    fn joining_with_a_wrong_or_missing_key_fails_and_with_the_right_one_saves() {
        let s = Service::inline(FakeWifi::demo_online());
        s.scan();
        s.join("Office-5G", Some(WRONG_PASSWORD.into()));
        assert_eq!(s.view().failure, Some(JoinError::WrongKey));
        assert_eq!(s.view().ssid.as_deref(), Some("Home"), "still on Home");
        s.join("Office-5G", Some("short".into()));
        assert_eq!(s.view().failure, Some(JoinError::BadKey));
        s.join("Campus", None);
        assert_eq!(s.view().failure, Some(JoinError::Unsupported));
        s.join("Office-5G", Some("correct horse".into()));
        let v = s.view();
        assert_eq!(v.failure, None, "a new action clears the last failure");
        assert_eq!(v.ssid.as_deref(), Some("Office-5G"));
        assert!(v.networks.iter().any(|n| n.ssid == "Office-5G" && n.saved));
        s.join("Café Central", None);
        assert_eq!(s.view().ssid.as_deref(), Some("Café Central"));
    }

    #[test]
    fn an_address_that_comes_after_the_join_gave_up_clears_the_failure() {
        let f = FakeWifi::demo_online();
        let lease = f.lease.clone();
        let s = Service::inline(f);
        s.scan();
        lease.store(false, Ordering::SeqCst);
        s.join("Office-5G", Some("correct horse".into()));
        let v = s.view();
        assert_eq!(v.failure, Some(JoinError::NoAddress));
        assert_eq!(v.ssid.as_deref(), Some("Office-5G"));
        assert!(v.ip.is_none());
        assert!(v.networks.iter().any(|n| n.ssid == "Office-5G" && n.saved));
        // Still no address: the failure stays through the regular poll.
        s.refresh();
        assert_eq!(s.view().failure, Some(JoinError::NoAddress));
        lease.store(true, Ordering::SeqCst);
        let g = s.generation();
        s.refresh();
        let v = s.view();
        assert!(s.generation() > g);
        assert_eq!(v.failure, None);
        assert_eq!(v.ip.as_deref(), Some("192.168.1.42"));
    }

    #[test]
    fn the_regular_poll_of_a_thread_picks_the_late_address_up() {
        let f = FakeWifi::demo_online();
        let lease = f.lease.clone();
        let s = Service::spawn(f);
        s.scan();
        lease.store(false, Ordering::SeqCst);
        s.join("Office-5G", Some("correct horse".into()));
        let wait = |done: &dyn Fn(&View) -> bool| {
            let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !done(&s.view()) && std::time::Instant::now() < end {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };
        wait(&|v| v.failure == Some(JoinError::NoAddress));
        assert_eq!(s.view().failure, Some(JoinError::NoAddress));
        lease.store(true, Ordering::SeqCst);
        // No command: the thread's own poll (every few seconds) notices.
        wait(&|v| v.failure.is_none());
        assert_eq!(s.view().failure, None);
        assert!(s.view().ip.is_some());
    }

    #[test]
    fn forgetting_unsaves_and_drops_a_network_out_of_range() {
        let s = Service::inline(FakeWifi::demo_online());
        s.scan();
        s.forget("Holiday house");
        assert!(s.view().networks.iter().all(|n| n.ssid != "Holiday house"));
        s.forget("Home");
        let v = s.view();
        assert!(v.ssid.is_none(), "forgetting the joined network leaves it");
        assert!(v.networks.iter().any(|n| n.ssid == "Home" && !n.saved));
    }

    #[test]
    fn a_failing_power_on_says_so_and_leaves_the_radio_off() {
        let mut f = FakeWifi::demo();
        f.fail_power = true;
        let s = Service::inline(f);
        s.power(true);
        let v = s.view();
        assert!(!v.powered);
        assert!(matches!(v.failure, Some(JoinError::Failed(_))));
        s.power(true);
        assert!(s.view().powered, "the next try works");
    }

    #[test]
    fn a_threaded_service_gets_there_too() {
        let s = Service::spawn(FakeWifi::demo());
        s.power(true);
        let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !s.view().scanned && std::time::Instant::now() < end {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(s.view().powered && s.view().scanned);
    }
}
