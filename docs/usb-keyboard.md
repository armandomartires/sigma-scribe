# USB keyboard (v0.8) — hardware note and design

## Hardware
- Kobo Forma micro-USB port + OTG adapter + **powered** USB hub (or an OTG Y-cable with external power). The Forma does not supply VBUS.
- **Known to work on this Forma (user report, 2026-10-21):** a **Logitech K400 Plus** (keyboard + touchpad, Unifying receiver) used on this Forma, fw 4.38.23697, with other Kobo software — so the port's host mode, enumeration through the receiver and HID input work on this firmware. Not yet tried with Sigma Scribe; record the power arrangement (hub or powered OTG cable) and the `usb:` log lines at the first session.

## How it works
1. Options → USB keyboard On → `Device::usb_host(true)`: on the Kobo, `OtgSwitch` writes `host` to `/sys/kernel/debug/ci_hdrc.0/role` if it reads `gadget` (debugfs mounted first if missing). Off / exit → `gadget`, only if the app made the switch (`crates/device/src/otg.rs`).
2. A watcher thread polls `/proc/bus/input/devices` every second; a device with EV_KEY and the A, Z, Space and Enter bits is a keyboard (`usbkbd::parse_devices`). Keyboards are kept away from the touch decoder.
3. Each keyboard gets a reader thread and a `KeyMapper` (US layout, Shift/Caps/Ctrl/Alt, press + repeat). Plain characters → `Event::Key`; Ctrl/Alt combos and Shift+non-character → `Event::Chord`.
4. `App::chord` sets the on-screen keyboard's Ctrl/Shift/Alt to the chord's, runs the same key path as an on-screen tap (note, Scratch, Find, rename), and restores the on-screen modifiers.
5. A read error (unplug) ends that reader and sends `Event::Usb(false)` when none are left.

## Unverified on the device
That this app's debugfs role switch reaches host mode the same way the other software did (host mode itself is known to work, see Hardware); enumeration delay; Nickel also reading the keyboard (no EVIOCGRAB); Apple-mode keyboards (`hid ignore_special_drivers`). Each failure degrades to "no keyboard" with a log line.

## Export
Notes are plain `.md` files in `/mnt/onboard/sigma-scribe/notes` (shown as `sigma-scribe/notes` on the computer). Leave the app, connect over USB, tap Connect, copy the folder. Syncthing or similar: later, optional (brief: no cloud account).

## Mouse and touchpad (v0.8.1)
- Detection: `EV_REL` with `REL_X` and `REL_Y` (`usbkbd::parse_devices` → `pointer`). The K400 Plus may appear as one combined device (keys + REL) or as separate keyboard and mouse nodes; both work, one reader per node feeds the keymap and/or `pointer::PointerDecoder`.
- Decoder: `REL_X/Y` (summed per `SYN_REPORT`), `REL_WHEEL`, `BTN_LEFT/RIGHT/MIDDLE` → `PointerMove`, `PointerButton`, `Wheel`. Touchpad two-finger scroll works only if the receiver sends it as `REL_WHEEL` (unverified for the K400).
- Pointer: 12×19-cell outlined arrow (`render::pointer`), drawn last; the frame diff sends only the changed area as a Fast refresh (≥ 16×16); ≤ 1 redraw / 150 ms with pending motion flushed on the 50 ms tick; hidden after 3 s still, on typing and on touch.
- Click = tap at the press point; drag from the note selects (anchor at press); double-click (≤ 400 ms, ≤ 8 px) selects the word; right/middle click and long-press: nothing new.
- Wheel: note 3 rows per notch (the caret follows, so the view scrolls); preview, Files, Graph, Help: one page per wheel event; Options: nothing.
- No pointer acceleration; raw counts are screen pixels.
