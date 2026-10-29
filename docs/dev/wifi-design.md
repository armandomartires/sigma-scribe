# Wi-Fi design (the title-bar button)

How the button is used is in [../user/wifi.md](../user/wifi.md).

Status: written and tested on the desktop against a fake `wpa_supplicant`
and a pretend radio. **Not yet tried on a Forma.** §6 lists what the first
run must check; the two things most likely to need a change are the module
reload and the `wpa_supplicant` driver flag, and both live in
`wifi.sh`, a shell script that can be edited on the device without a
rebuild.

## 3. Design

```
title bar button ── menu (render: wifi_menu.rs) ── App (app/src/wifi.rs)
                                                     │  ticks take in the View
                                          sigma_device::wifi::Service
                                                     │  a thread, or inline
                                   Backend ── FakeWifi (desktop, emulator, tests)
                                           └─ KoboWifi (ARM only)
                                                  ├─ wpa::Supplicant  ── control socket
                                                  └─ wifi.sh on | off | dhcp
```

- **`sigma_device::wifi::wpa`** speaks the `wpa_supplicant` control interface:
  a Unix datagram socket, `/var/run/wpa_supplicant/<interface>`; the client
  binds a private socket in the temporary folder, sends one command as text and
  reads one reply. No `wpa_cli` is needed or run. It is plain Unix code, so the
  parsers *and* the whole flow are tested on the desktop against a fake
  supplicant on a real socket (`crates/device/src/wifi/wpa.rs`).
  - Scanning: `ATTACH`, `SCAN`, wait for the `CTRL-EVENT-SCAN-RESULTS` event
    (at most ten seconds), `SCAN_RESULTS`, `LIST_NETWORKS`, `DETACH`. A busy
    scan (`FAIL-BUSY`) is waited out briefly.
  - Joining a network that is not saved: `ADD_NETWORK`, `SET_NETWORK <id> ssid
    <hex>` (hex, so no name needs quoting), `key_mgmt WPA-PSK` and `psk "<key>"`
    (raw between quotes: the supplicant takes everything up to the last quote
    and has no escapes) or `key_mgmt NONE`, `SELECT_NETWORK`. Then it waits up
    to 25 s for `wpa_state=COMPLETED` on that name, asks DHCP for an address
    (`wifi.sh dhcp`) and waits up to 40 s for one (a first DHCP on a new network can be slow; 20 s was too short on the Forma). If none comes, the failure "gave no address" is shown and the network stays saved, but the service's regular poll (every 4 s) keeps looking: when an address appears the failure is cleared and the log says `wifi: address came late`. Only then, `ENABLE_NETWORK
    all` (a select disables the others) and `SAVE_CONFIG`. A network that the
    supplicant holds off after failed handshakes (`[TEMP-DISABLED]`) ends the
    wait at once as a wrong password.
  - A saved network joins with `SELECT_NETWORK` alone and keeps the key it has;
    typing a password for a saved one replaces it.
  - Forgetting: `REMOVE_NETWORK` for every network of that name, `SAVE_CONFIG`.
  - A failure removes the network it added, enables the others again and sends
    `RECONNECT`; nothing is saved.
- **Saved networks are the stock reader's own**: the supplicant is started
  with `/etc/wpa_supplicant/wpa_supplicant.conf`, the file Nickel uses, so
  what Nickel knew is known and what is added here is known to Nickel. That
  file needs `update_config=1` for `SAVE_CONFIG` to work (Kobo's has it);
  `wifi.sh on` logs a warning when it does not.
- **`wifi.sh`** (in the package, `.adds/sigma-scribe/wifi.sh`) is after KOReader's
  `platform/kobo/enable-wifi.sh`, `disable-wifi.sh` and `obtain-ip.sh`:
  - `on`: if the interface is missing, `insmod` `sdio_wifi_pwr.ko` (where there
    is one) and the driver `.ko` from `/drivers/$PLATFORM/wifi/`; wait for the
    interface; `ifconfig up`; start `wpa_supplicant -D wext -s -i <if> -O
    /var/run/wpa_supplicant -c <conf> -B` unless one runs; wait for its socket.
  - `off`: stop DHCP (`dhcpcd -k`, `udhcpc`), `killall wpa_supplicant`,
    `ifconfig down`, `rmmod` the driver and `sdio_wifi_pwr`.
  - `dhcp`: `dhcpcd -n` (rebind) if the stock `dhcpcd` still runs, else start
    `dhcpcd`, else `udhcpc`.
  - Its `wifi: …` lines go to the app's log. It never prints a network name or
    a key. `SIGMA_WPA_DRIVER` (default `wext`) and `SIGMA_WPA_CONF` override
    the driver flag and the config file.
- **The interface and driver** are the stock reader's environment,
  `INTERFACE` and `WIFI_MODULE` (the Forma: `eth0`, `8192es`). `launch.sh`
  keeps them (and defaults them to those values when it was not started from
  Nickel) and sets `SIGMA_WIFI_DIR` to the package folder.
- **`Service`** runs a backend on a thread of its own (never on the input
  loop; radio calls take seconds) and keeps the last `View`; the app takes it
  in on its ticks, as it does for Syncthing, and redraws only when something
  the screen shows changed. The thread also reads the radio every four
  seconds while idle, so a network that drops shows. In the emulator and the
  tests the service is *inline*: each command finishes before it returns, so
  they are deterministic.
- **Fake backend**: `sigma_device::wifi::fake::FakeWifi`. A WPA network takes any
  password of 8 or more characters except `wrong-password`. The desktop window
  starts with the radio off; the emulator uses `wifi-demo [online]`,
  `expect-wifi off|on|joined:NAME|saved:NAME|unsaved:NAME|failure|no-failure`
  and the controls `Wi-Fi`, `WifiSwitch`, `Rescan`, `Net:NAME`, `Forget:NAME`,
  `ShowPassword`.
- **Title bar**: one more icon slot (`TITLE_ICONS` is 9 with the light
  button); the order ends Graph, Light, Wi-Fi, Settings. The existing rule
  holds: the gaps shrink, then the icons, never below 44 brief px, and the
  name keeps at least 200 brief px. Tests cover portrait and landscape at UI
  scales 100, 150 and 200 %.
- **Logging** (source `wifi`, INFO unless said): `radio on|off` at start,
  `on`, `off`, `joining <name>`, `joined <name>`, `not connected`,
  `scan: <n> networks`, `off: Syncthing stopped`, `on: Syncthing started`,
  `interface …, module …` at start, the lines of `wifi.sh`; a failure is a
  WARNING (`failed: …`). **Never a password, a passphrase or an IP address.**
  A test reads the log after a join with a password and a forget.
- **Dependencies**: none new. Five Lucide icons (`wifi`, `wifi-low`,
  `wifi-off`, `lock`, `refresh-cw`; ISC) are baked in like the others.

## 4. Strings

Fifteen new texts are in the nine catalogs (`assets/i18n/`): Wi-Fi, Not
connected, Connected, Turning on…, Turning off…, Connecting…, Scanning…,
Scan again, No networks found, Password, and the five failure texts. Like the
others they are machine-drafted and need a native review. Network names and
addresses are never translated.

## 6. Checklist for the Forma (first run)

Not verified on the device. Install the package, turn logging on (Settings →
Logging) and read `sigma-scribe.log` after each step.

**The two things most likely to need changing**
1. **Module reload.** Turn Wi-Fi off in the menu, then on again. In the log:
   `wifi: off`, then `wifi: on (eth0, 8192es, driver wext)`. If it says
   `no 8192es.ko under /drivers`, find the module (`find /drivers -name
   '*.ko' | grep -i wifi`) and fix `find_ko` in `wifi.sh`; if `insmod`
   fails, a power module may be needed first (KOReader loads `sdio_wifi_pwr`
   on this family, which the script does when the file exists); if `eth0` does
   not appear, look at `dmesg`.
2. **The `wpa_supplicant` driver flag.** `wifi.sh` starts it with `-D wext`
   (what KOReader uses for Realtek on these Kobos). If the log shows
   `wpa_supplicant failed to start` or `no control socket`, or scans always
   come back empty, try `SIGMA_WPA_DRIVER=nl80211` (export it in `launch.sh`
   or edit `wifi.sh`).

**Then**
3. Start with the stock reader's Wi-Fi on: the button shows `wifi` (connected),
   the menu names the network and its address, and the `net:` and `wifi:`
   lines agree. Start with it off: the button shows `wifi-off`.
4. Switch on: it rejoins a saved network by itself and gets an address; the
   list fills.
5. Join an open network, a saved one, and one with a password typed on the
   on-screen keyboard (the eye shows the text; the field keeps the keyboard).
   Check the join takes the DHCP step (`wifi: dhcpcd rebound` or similar).
6. A wrong password: the message appears within about 30 s, the radio is back
   on the old network, and `/etc/wpa_supplicant/wpa_supplicant.conf` has no new
   network block (read it from a shell on the device).
7. A right password: the conf now has the network (`SAVE_CONFIG` worked); reboot
   into the stock reader and see it listed there as known.
8. Forget it: gone from the conf.
9. Settings → Sync on, then Wi-Fi off: Syncthing stops (the process is gone,
   `wifi: off: Syncthing stopped`); Wi-Fi on: it starts and syncs.
10. Settings → Plugins → Check for plugins works after joining from the menu.
11. Leave the app (power button): the stock reader boots and its Wi-Fi works.
    Wi-Fi turned off in Σscribe: after the reboot the stock reader's own
    Wi-Fi switch still works.
12. The title bar at 200 % and in landscape: nothing overlaps, the file name
    keeps room.
13. What `wifi.sh on` costs in time on the device (expect 5 to 15 s); the
    menu must stay responsive while it runs (it is on a thread).

Things that may differ on the Forma and are worth a note: `ip_address=` in the
supplicant's `STATUS` (the app falls back to `ifconfig`), BusyBox's `killall
-q`, and whether the stock `dhcpcd` is still running when Σscribe starts
(`launch.sh` logs `net: dhcpcd running`).

## 7. Tests

- `crates/device/src/wifi/wpa.rs`: the parsers on canned replies (scan, status,
  network list, escapes), key rules, and every flow (scan, join new, join
  saved, re-key, wrong key, no address, forget, refusal before sending) against
  a fake supplicant on a real Unix socket; checks that a key is sent once, only
  in `SET_NETWORK … psk`, and that no client socket file is left behind.
- `crates/device/src/wifi/mod.rs`, `fake.rs`: flags to security, signal bars, the
  `ifconfig` address (BusyBox and net-tools), the service (inline and threaded).
- `crates/render/src/wifi_menu.rs`: the title bar at three UI scales in both
  orientations, the menu layout and hits, the long list, the empty list, drawing
  and the password field. `crates/render/src/i18n.rs`: the new texts in all nine
  catalogs.
- `app/tests/wifi.rs`: the whole journey through the app with the pretend radio
  (switch, join, password with the on-screen and a physical keyboard, wrong
  password, forget, preview, Syncthing following the radio with a fake binary,
  the log, a threaded service on ticks).
- `tests/emu/v1.17-wifi.script`, screenshots in `docs/screenshots/v1.17/`.
- Not covered: the ARM-only `kobo.rs`, `wifi.sh`, a real supplicant, the Forma.

## 8. Notes moved from the user page

- **Pretend radio.** The button shows only when the app has a radio: the Kobo build always does, the desktop window gets a pretend one, the emulator only after `wifi-demo`.
- **Syncthing follows the radio** is logged as `wifi: off: Syncthing stopped` and `wifi: on: Syncthing started`.
- **Log lines** (INFO): `radio on|off` and `interface …, module …` at start, `on`, `off`, `joining <name>`, `joined <name>`, `not connected`, `scan: <n> networks`, `off: Syncthing stopped`, `on: Syncthing started`, and the `wifi:` lines of `wifi.sh` (the driver and supplicant steps). A failure is a WARNING (`wifi: failed: …`). Network names are logged; never a password, a passphrase or an IP address. The log file itself is described in [../user/logging.md](../user/logging.md).
- **Log folder on a desktop.** `SIGMA_LOG_DIR` overrides the log folder, and `target/emu-log` is the desktop and emulator default.
