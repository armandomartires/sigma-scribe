//! Wi-Fi in the app (v1.18, `docs/wifi.md`): the title-bar button, its
//! menu, the password typed into the title bar, and Syncthing following
//! the radio.
//!
//! The radio is a [`sigma_device::wifi::Service`]: on the Kobo it runs on a
//! thread of its own, and the app takes in what it reports on its ticks,
//! as it does for Syncthing. With no service the button is not shown, so
//! the emulator scripts and the tests that never mention Wi-Fi look as
//! they did.

use crate::files::RenameTarget;
use crate::App;
use sigma_core::Buffer;
use sigma_device::wifi::{Busy, Network, Service, View};
use sigma_device::Refresh;
use sigma_render::wifi_menu::{Activity, NetRow, WifiHit, WifiModel, WifiState};

/// The rows of the menu, joined network first, and which of the view's
/// networks each stands for.
fn rows(v: &View) -> (Vec<NetRow>, Vec<usize>) {
    let mut order: Vec<usize> = (0..v.networks.len()).collect();
    let joined = |n: &Network| v.ssid.as_deref() == Some(n.ssid.as_str());
    // The joined network on top; the rest keep the scan's order.
    order.sort_by_key(|&i| !joined(&v.networks[i]));
    let rows = order
        .iter()
        .map(|&i| {
            let n = &v.networks[i];
            NetRow {
                ssid: n.ssid.clone(),
                bars: n.bars(),
                locked: n.security.locked(),
                saved: n.saved,
                connected: joined(n),
            }
        })
        .collect();
    (rows, order)
}

fn activity(b: &Busy) -> Activity {
    match b {
        Busy::Idle => Activity::Idle,
        Busy::Power(true) => Activity::TurningOn,
        Busy::Power(false) => Activity::TurningOff,
        Busy::Scanning => Activity::Scanning,
        Busy::Joining(_) => Activity::Joining,
    }
}

fn state_of(v: &View) -> WifiState {
    match (v.powered, &v.ssid) {
        (false, _) => WifiState::Off,
        (true, None) => WifiState::On,
        (true, Some(_)) => WifiState::Connected,
    }
}

impl App {
    /// Gives the app a Wi-Fi radio: the title bar gets the Wi-Fi button.
    /// The Kobo binary passes the real one, the desktop window and the
    /// emulator a pretend one.
    pub fn set_wifi(&mut self, service: Service) {
        self.wifi = Some(service);
        self.screen.wifi_on = true;
        let v = self.wifi.as_ref().map(Service::view).unwrap_or_default();
        sigma_core::info!(
            "wifi",
            "radio {}{}",
            if v.powered { "on" } else { "off" },
            v.ssid
                .as_ref()
                .map_or(String::new(), |s| format!(", joined {s}"))
        );
        self.wifi_view = v;
        self.wifi_gen = self.wifi.as_ref().map_or(0, Service::generation);
        self.screen.wifi_state = state_of(&self.wifi_view);
    }

    /// What the last Wi-Fi report said (tests and the emulator).
    #[must_use]
    pub fn wifi_view(&self) -> &View {
        &self.wifi_view
    }

    /// Wi-Fi is off, so Syncthing is held: it has no network to use.
    pub(crate) fn wifi_blocks_sync(&self) -> bool {
        self.wifi.is_some() && !self.wifi_view.powered
    }

    /// Takes in what the radio reported since the last look. Returns
    /// whether the view changed, and applies what follows: the button's
    /// state, Syncthing stopping with the radio, and the log lines.
    pub(crate) fn wifi_take(&mut self) -> bool {
        let Some(s) = &self.wifi else {
            return false;
        };
        let g = s.generation();
        if g == self.wifi_gen {
            return false;
        }
        self.wifi_gen = g;
        let new = s.view();
        let old = std::mem::replace(&mut self.wifi_view, new.clone());
        self.log_wifi_change(&old, &new);
        self.screen.wifi_state = state_of(&new);
        if old.powered != new.powered {
            let had = self.sync.is_some();
            self.apply_sync();
            match (had, self.sync.is_some()) {
                (true, false) => sigma_core::info!("wifi", "off: Syncthing stopped"),
                (false, true) => sigma_core::info!("wifi", "on: Syncthing started"),
                _ => {}
            }
        }
        true
    }

    /// One INFO line for each thing that changed; never a key, never an
    /// address.
    fn log_wifi_change(&self, old: &View, new: &View) {
        if old.powered != new.powered {
            sigma_core::info!("wifi", "{}", if new.powered { "on" } else { "off" });
        }
        if old.ssid != new.ssid {
            match &new.ssid {
                Some(s) => sigma_core::info!("wifi", "joined {s}"),
                None if new.powered => sigma_core::info!("wifi", "not connected"),
                None => {}
            }
        }
        if new.scanned && (!old.scanned || old.networks != new.networks) {
            sigma_core::info!("wifi", "scan: {} networks", new.networks.len());
        }
        if new.failure != old.failure {
            if let Some(f) = &new.failure {
                sigma_core::warn!("wifi", "failed: {f}");
            }
        }
    }

    /// On every tick: takes in the radio's news. Returns the refresh a
    /// visible change needs: the menu open, or the button changing.
    pub(crate) fn wifi_tick(&mut self) -> Option<Refresh> {
        let before = self.screen.wifi_state;
        if !self.wifi_take() {
            return None;
        }
        (self.screen.wifi_menu || self.screen.wifi_state != before).then_some(Refresh::Partial)
    }

    /// Both menus are never open together; a tap while one is open goes to it.
    pub(crate) fn any_menu_open(&self) -> bool {
        self.screen.drawer_menu || self.screen.wifi_menu
    }

    /// The Wi-Fi button: opens or closes the menu. Opening it looks at the
    /// radio again, and scans when nothing has since it came on.
    pub(crate) fn toggle_wifi_menu(&mut self) -> Refresh {
        self.screen.drawer_menu = false;
        self.screen.wifi_menu = !self.screen.wifi_menu;
        self.sel_ui = None;
        if self.screen.wifi_menu {
            if let Some(s) = &self.wifi {
                s.refresh();
            }
            self.wifi_take();
            let v = &self.wifi_view;
            if v.powered && !v.scanned && v.busy == Busy::Idle {
                if let Some(s) = &self.wifi {
                    s.scan();
                }
                self.wifi_take();
            }
        }
        Refresh::Partial
    }

    /// The menu as the screen draws it and tests taps in it.
    fn with_wifi_model<R>(&self, f: impl FnOnce(&WifiModel<'_>, &[usize]) -> R) -> Option<R> {
        self.wifi.as_ref()?;
        let v = &self.wifi_view;
        let (rows, order) = rows(v);
        let m = WifiModel {
            powered: v.powered,
            activity: activity(&v.busy),
            ssid: v.ssid.as_deref(),
            ip: v.ip.as_deref(),
            note: v.failure.as_ref().map(sigma_device::wifi::JoinError::text),
            networks: &rows,
        };
        Some(f(&m, &order))
    }

    /// Draws the Wi-Fi menu when it is open (over the drawer, under the ink
    /// panel). Leaving write mode and preview closes it.
    pub(crate) fn draw_wifi_menu(&mut self, c: &mut sigma_render::Canvas) {
        if !self.drawer_allowed() {
            self.screen.wifi_menu = false;
        }
        if !self.screen.wifi_menu {
            return;
        }
        let l = self.title_layout();
        let t = self.theme();
        self.with_wifi_model(|m, _| self.screen.draw_wifi_menu(c, &l, m, t));
    }

    /// A tap while the Wi-Fi menu is open: the switch, a network, the cross
    /// of a saved one, or Scan again; the button or any other place closes
    /// it, and the tap does nothing else.
    pub(crate) fn wifi_menu_tap(&mut self, x: i32, y: i32) -> Option<Refresh> {
        let l = self.title_layout();
        let hit = self
            .with_wifi_model(|m, order| (self.screen.wifi_menu_hit(&l, m, x, y), order.to_vec()))?;
        let (hit, order) = hit;
        let net = |i: usize| self.wifi_view.networks.get(*order.get(i)?).cloned();
        match hit {
            WifiHit::Switch => {
                let on = !self.wifi_view.powered;
                if let Some(s) = &self.wifi {
                    s.power(on);
                }
            }
            WifiHit::Rescan => {
                if self.wifi_view.busy == Busy::Idle {
                    if let Some(s) = &self.wifi {
                        s.scan();
                    }
                }
            }
            WifiHit::Forget(i) => {
                if let (Some(n), Some(s)) = (net(i), &self.wifi) {
                    s.forget(&n.ssid);
                }
            }
            WifiHit::Net(i) => {
                let n = net(i)?;
                let joined = self.wifi_view.ssid.as_deref() == Some(n.ssid.as_str());
                if joined || self.wifi_view.busy != Busy::Idle {
                    return Some(Refresh::Partial);
                }
                if n.security.needs_key() && !n.saved {
                    self.start_wifi_password(n.ssid);
                    return Some(Refresh::Partial);
                }
                self.wifi_join(&n.ssid, None);
            }
            WifiHit::Inside => return Some(Refresh::Partial),
            WifiHit::Button | WifiHit::Outside => {
                self.screen.wifi_menu = false;
                return Some(Refresh::Partial);
            }
        }
        self.wifi_take();
        Some(Refresh::Partial)
    }

    /// Asks the radio to join `ssid`: with no key for a saved or an open
    /// network. The name goes in the log, the key nowhere.
    fn wifi_join(&mut self, ssid: &str, key: Option<String>) {
        sigma_core::info!("wifi", "joining {ssid}");
        if let Some(s) = &self.wifi {
            s.join(ssid, key);
        }
    }

    /// The password of `ssid` is typed in the title bar (the rename field),
    /// with the keyboard up. Leaves preview, as Find does.
    fn start_wifi_password(&mut self, ssid: String) {
        if self.preview.is_some() {
            self.exit_preview();
        }
        self.screen.wifi_menu = false;
        self.wifi_show = false;
        self.start_rename(RenameTarget::WifiPassword(ssid));
    }

    /// OK, Return or Cancel of the password field: joins with what was
    /// typed, or leaves the network alone, and the menu opens again.
    pub(crate) fn finish_wifi_password(&mut self, ssid: &str, field: Buffer, ok: bool) {
        self.screen.wifi_menu = true;
        if ok {
            self.wifi_join(ssid, Some(field.text().to_owned()));
            self.wifi_take();
        }
    }

    /// The eye of the password field was tapped.
    pub(crate) fn wifi_eye_at(&self, x: i32, y: i32) -> bool {
        matches!(&self.rename, Some((RenameTarget::WifiPassword(_), _)))
            && self
                .screen
                .wifi_eye(&self.screen.layout(true, &self.keyboard))
                .contains(x, y)
    }

    /// Shows or hides the typed password.
    pub(crate) fn wifi_toggle_show(&mut self) -> Refresh {
        self.wifi_show = !self.wifi_show;
        Refresh::Fast
    }

    /// The typed password as the title bar shows it: stars unless the eye
    /// is on, with the caret where it is.
    pub(crate) fn wifi_field_view(&self, field: &Buffer) -> Buffer {
        let mut b = if self.wifi_show {
            Buffer::new(field.text())
        } else {
            Buffer::new("*".repeat(field.text().chars().count()))
        };
        let before = field.text()[..field.caret()].chars().count();
        let caret = if self.wifi_show {
            field.caret()
        } else {
            before
        };
        b.set_caret(caret);
        b
    }

    /// Draws what the password field adds to the title bar.
    pub(crate) fn draw_wifi_password(
        &self,
        c: &mut sigma_render::Canvas,
        l: &sigma_render::editor::Layout,
    ) {
        if let Some((RenameTarget::WifiPassword(ssid), field)) = &self.rename {
            self.screen.draw_wifi_field(
                c,
                l,
                self.theme(),
                ssid,
                field.text().is_empty(),
                self.wifi_show,
            );
        }
    }

    /// The centre of a Wi-Fi control, for the emulator and the tests:
    /// `Wi-Fi` (the button), and with the menu open `WifiSwitch`, `Rescan`,
    /// `Net:<name>` and `Forget:<name>`; with a password being typed,
    /// `ShowPassword`.
    #[must_use]
    pub fn wifi_control(&self, label: &str) -> Option<(i32, i32)> {
        let c = |r: sigma_render::Rect| (r.x + r.w / 2, r.y + r.h / 2);
        let l = self.title_layout();
        match label {
            "Wi-Fi" if self.screen.wifi_on => return Some(c(l.wifi_button)),
            "ShowPassword" if self.wifi_eye_at_all() => return Some(c(self.screen.wifi_eye(&l))),
            _ => {}
        }
        if !self.screen.wifi_menu {
            return None;
        }
        self.with_wifi_model(|m, _| {
            let g = self.screen.wifi_menu_layout(&l, m);
            match label {
                "WifiSwitch" => Some(c(g.switch_row)),
                "Rescan" => g.rescan.map(c),
                _ => {
                    let (forget, name) =
                        match (label.strip_prefix("Net:"), label.strip_prefix("Forget:")) {
                            (Some(n), _) => (false, n),
                            (_, Some(n)) => (true, n),
                            _ => return None,
                        };
                    let (_, row, cross) = g
                        .nets
                        .iter()
                        .find(|(i, _, _)| m.networks[*i].ssid == name)?;
                    if forget {
                        cross.map(c)
                    } else {
                        Some((row.x + row.w / 3, row.y + row.h / 2))
                    }
                }
            }
        })?
    }

    fn wifi_eye_at_all(&self) -> bool {
        matches!(&self.rename, Some((RenameTarget::WifiPassword(_), _)))
    }

    /// Syncthing runs when it is on in Settings and the radio is not off.
    pub(crate) fn sync_wanted(&self) -> bool {
        self.settings.sync && !self.wifi_blocks_sync()
    }
}
