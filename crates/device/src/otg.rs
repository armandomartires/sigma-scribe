//! USB OTG role switch (v0.8). The Forma's Chipidea controller boots in
//! *gadget* mode (USB storage for Nickel); a keyboard needs *host* mode,
//! switched through debugfs (`ci_hdrc.0/role`, as KOReader's
//! ExternalKeyboard plugin does). Only done while Options → USB keyboard is
//! On, and switched back to gadget when that is turned off or the app
//! leaves. A reboot always returns the port to gadget mode.
//!
//! Unverified on the Forma: that its kernel has debugfs and this role file,
//! and that host + hub + HID drivers are built in. If anything is missing
//! the switch fails, is logged, and the app goes on without a keyboard.

use std::fs;
use std::io;
use std::path::PathBuf;

/// The Chipidea role file on Clara / Forma-generation Kobos.
pub const CHIPIDEA_ROLE: &str = "/sys/kernel/debug/ci_hdrc.0/role";

/// The USB OTG role file and whether this app switched it to host.
#[derive(Clone, Debug)]
pub struct OtgSwitch {
    /// The role file (normally [`CHIPIDEA_ROLE`]).
    pub path: PathBuf,
    /// We changed gadget → host and owe a switch back.
    pub switched: bool,
}

impl OtgSwitch {
    /// A switch on the role file at `path`, not yet switched.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            switched: false,
        }
    }

    /// Current role (`host` / `gadget`), if the file is readable.
    #[must_use]
    pub fn role(&self) -> Option<String> {
        fs::read_to_string(&self.path)
            .ok()
            .map(|s| s.trim().to_owned())
    }

    /// Host mode on (`true`) or back to gadget (`false`). Turning it off
    /// only undoes our own switch: a port someone else put in host mode is
    /// left alone.
    ///
    /// # Errors
    /// The role file is missing or not writable.
    pub fn set_host(&mut self, on: bool) -> io::Result<()> {
        if on {
            let role = self
                .role()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no OTG role file"))?;
            if role == "gadget" {
                fs::write(&self.path, "host")?;
                self.switched = true;
            }
        } else if self.switched {
            fs::write(&self.path, "gadget")?;
            self.switched = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_and_restores_only_its_own_change() {
        let p = std::env::temp_dir().join(format!("sigma-otg-{}", std::process::id()));
        fs::write(&p, "gadget\n").unwrap();
        let mut o = OtgSwitch::new(&p);
        o.set_host(true).unwrap();
        assert_eq!(o.role().as_deref(), Some("host"));
        o.set_host(false).unwrap();
        assert_eq!(o.role().as_deref(), Some("gadget"));
        // Already host (someone else): left as it is.
        fs::write(&p, "host").unwrap();
        o.set_host(true).unwrap();
        o.set_host(false).unwrap();
        assert_eq!(o.role().as_deref(), Some("host"));
        let _ = fs::remove_file(&p);
        assert!(OtgSwitch::new("/nonexistent/role").set_host(true).is_err());
        assert!(OtgSwitch::new("/nonexistent/role").set_host(false).is_ok());
    }
}
