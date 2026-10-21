# USB keyboard (v0.8) — hardware note and design

## Hardware
- Kobo Forma micro-USB port + OTG adapter + **powered** USB hub (or an OTG Y-cable with external power). The Forma does not supply VBUS.
- **Tested hub/cable: none yet** — no device access during S8. Fill in after the first session: hub model, cable, keyboard model, result.

## How it works
1. Options → USB keyboard On → `Device::usb_host(true)`: on the Kobo, `OtgSwitch` writes `host` to `/sys/kernel/debug/ci_hdrc.0/role` if it reads `gadget` (debugfs mounted first if missing). Off / exit → `gadget`, only if the app made the switch (`crates/device/src/otg.rs`).
2. A watcher thread polls `/proc/bus/input/devices` every second; a device with EV_KEY and the A, Z, Space and Enter bits is a keyboard (`usbkbd::parse_devices`). Keyboards are kept away from the touch decoder.
3. Each keyboard gets a reader thread and a `KeyMapper` (US layout, Shift/Caps/Ctrl/Alt, press + repeat). Plain characters → `Event::Key`; Ctrl/Alt combos and Shift+non-character → `Event::Chord`.
4. `App::chord` sets the on-screen keyboard's Ctrl/Shift/Alt to the chord's, runs the same key path as an on-screen tap (note, Scratch, Find, rename), and restores the on-screen modifiers.
5. A read error (unplug) ends that reader and sends `Event::Usb(false)` when none are left.

## Unverified on the device
Debugfs and the role file on fw 4.38.23697; built-in host/hub/HID drivers; enumeration delay; Nickel also reading the keyboard (no EVIOCGRAB); Apple-mode keyboards (`hid ignore_special_drivers`). Each failure degrades to "no keyboard" with a log line.

## Export
Notes are plain `.md` files in `/mnt/onboard/sigma-scribe/notes` (shown as `sigma-scribe/notes` on the computer). Leave the app, connect over USB, tap Connect, copy the folder. Syncthing or similar: later, optional (brief: no cloud account).
