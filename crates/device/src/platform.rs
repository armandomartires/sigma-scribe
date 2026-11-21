//! The platform seam (ADR 0001): filesystem roots, Wi-Fi, and the hook
//! used after Syncthing is killed.
//!
//! The [`Device`](crate::Device) trait stays the drawing and input
//! surface; it is not widened to own paths or Wi-Fi.

use crate::paths::{self, Paths};
use crate::wifi::{self, Service};
use std::process::{Command, Stdio};

/// A concrete platform: the Kobo, or the desktop / emulator.
pub trait Platform: Send + Sync {
    /// Filesystem roots for this platform.
    fn paths(&self) -> &Paths;

    /// The Wi-Fi backend used by the title-bar button.
    fn wifi(&self) -> Service;

    /// Hook for [`sigma_core::sync::Config::after_kill`].
    ///
    /// The supervisor is not generic, so this is a function pointer. The
    /// Kobo implementation runs `killall -q syncthing` (the real Syncthing
    /// starts a child of its own). The desktop implementation is a no-op,
    /// so a development machine's own Syncthing is not killed.
    fn after_sync_kill(&self) -> fn();
}

/// The Kobo Forma: onboard paths, the real radio, and `killall` after a
/// Syncthing stop.
pub struct Kobo {
    paths: Paths,
}

impl Kobo {
    /// The stock Kobo layout, after environment overrides.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paths: paths::kobo(),
        }
    }
}

impl Default for Kobo {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for Kobo {
    fn paths(&self) -> &Paths {
        &self.paths
    }

    fn wifi(&self) -> Service {
        #[cfg(feature = "kobo")]
        {
            Service::spawn(wifi::kobo::KoboWifi::from_env())
        }
        #[cfg(not(feature = "kobo"))]
        {
            // The type exists so tests can inspect the Kobo path layout
            // without linking FBInk; there is no real radio here.
            Service::inline(wifi::fake::FakeWifi::demo())
        }
    }

    fn after_sync_kill(&self) -> fn() {
        kill_syncthing_by_name
    }
}

/// Desktop window and headless emulator: `target/emu-*` paths, a pretend
/// radio, and no `killall`.
pub struct Desktop {
    paths: Paths,
}

impl Desktop {
    /// The desktop / emulator layout, after environment overrides.
    #[must_use]
    pub fn new() -> Self {
        Self {
            paths: paths::desktop(),
        }
    }
}

impl Default for Desktop {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for Desktop {
    fn paths(&self) -> &Paths {
        &self.paths
    }

    fn wifi(&self) -> Service {
        Service::inline(wifi::fake::FakeWifi::demo())
    }

    fn after_sync_kill(&self) -> fn() {
        noop
    }
}

/// The platform selected by the `kobo` Cargo feature.
#[must_use]
pub fn current() -> Box<dyn Platform> {
    if cfg!(feature = "kobo") {
        Box::new(Kobo::new())
    } else {
        Box::new(Desktop::new())
    }
}

fn noop() {}

/// `killall -q syncthing`: the real Syncthing starts a child of its own,
/// which a kill of the parent would leave behind. Kobo only.
fn kill_syncthing_by_name() {
    let _ = Command::new("killall")
        .args(["-q", "syncthing"])
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::{current, Desktop, Kobo, Platform};

    #[test]
    fn host_current_is_desktop() {
        let p = current();
        assert_eq!(p.paths().adds, Desktop::new().paths().adds);
        // The hook must be the no-op, not killall.
        p.after_sync_kill()();
    }

    #[test]
    fn kobo_and_desktop_paths_differ() {
        assert_ne!(Kobo::new().paths().adds, Desktop::new().paths().adds);
    }
}
