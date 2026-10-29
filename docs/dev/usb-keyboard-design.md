# USB keyboard (v0.8) — design and hardware notes

How to use it is in [../user/usb-keyboard.md](../user/usb-keyboard.md).

## How it works
1. Options → USB keyboard On → `Device::usb_host(true)`: on the Kobo, `OtgSwitch` writes `host` to `/sys/kernel/debug/ci_hdrc.0/role` if it reads `gadget` (debugfs mounted first if missing). Off / exit → `gadget`, only if the app made the switch (`crates/device/src/otg.rs`).
2. A watcher thread polls `/proc/bus/input/devices` every second; a device with EV_KEY and the A, Z, Space and Enter bits is a keyboard (`usbkbd::parse_devices`). Keyboards are kept away from the touch decoder.
3. Each keyboard gets a reader thread and a `KeyMapper` (US layout, Shift/Caps/Ctrl/Alt, press + repeat). Plain characters → `Event::Key`; Ctrl/Alt combos and Shift+non-character → `Event::Chord`.
4. `App::chord` sets the on-screen keyboard's Ctrl/Shift/Alt to the chord's, runs the same key path as an on-screen tap (note, Scratch, Find, rename), and restores the on-screen modifiers.
5. A read error (unplug) ends that reader and sends `Event::Usb(false)` when none are left.

## Device-trial status
Moved from the user page. **Known to work on this Forma (user report, corrected 2026-10-21 13:55):** a regular wired 60% USB keyboard, used on this Forma (fw 4.38.23697) most likely with KOReader. So host mode on the micro-USB port, enumeration and HID keyboard input work on this firmware. Power arrangement used then: to be recorded at the first session. **Not tested on the device:** the Logitech K400 Plus (Unifying receiver, keyboard + touchpad); its receiver, its touchpad and its scroll behaviour are unverified.

Product brief for export: no cloud account required (Syncthing or similar was "later, optional"; it now exists, see [sync-design.md](sync-design.md)).

## Unverified on the device
That this app reaches host mode the way KOReader did (host mode itself is known to work with a wired keyboard, see [Hardware](../user/usb-keyboard.md#hardware) and the trial status below; the method matches KOReader's, see below); enumeration delay; Nickel also reading the keyboard (no EVIOCGRAB); Apple-mode keyboards (`hid ignore_special_drivers`). Each failure degrades to "no keyboard" with a log line.

## Mouse and touchpad (v0.8.1)

- Detection: `EV_REL` with `REL_X` and `REL_Y` (`usbkbd::parse_devices` → `pointer`). The K400 Plus may appear as one combined device (keys + REL) or as separate keyboard and mouse nodes; both work, one reader per node feeds the keymap and/or `pointer::PointerDecoder`.
- Decoder: `REL_X/Y` (summed per `SYN_REPORT`), `REL_WHEEL`, `BTN_LEFT/RIGHT/MIDDLE` → `PointerMove`, `PointerButton`, `Wheel`. Touchpad two-finger scroll works only if the receiver sends it as `REL_WHEEL` (unverified for the K400).
- No pointer acceleration; the raw counts are screen pixels.
- Pointer: 12×19-cell outlined arrow (`render::pointer`), drawn last; the frame diff sends only the changed area as a Fast refresh (≥ 16×16); ≤ 1 redraw / 150 ms with pending motion flushed on the 50 ms tick; hidden after 3 s still, on typing and on touch.

## Comparison with KOReader's ExternalKeyboard plugin (checked 2026-10-21, koreader master 24cf0b3)
KOReader: `plugins/externalkeyboard.koplugin/main.lua` + `frontend/device/kobo/device.lua` (`setupDebugFS`, `getOTGRole`, `setOTGRole`).

| Step | KOReader | Σscribe | Difference |
|---|---|---|---|
| Role file | `/sys/kernel/debug/ci_hdrc.0/role` (Chipidea; sunxi path for Elipsa/Sage) | same Chipidea path | none for the Forma |
| debugfs | if `/sys/kernel/debug` exists: check `/proc/mounts`, if not mounted and root, `mount -t debugfs none /sys/kernel/debug` | if the role file is missing: `mount -t debugfs none /sys/kernel/debug` (no `/proc/mounts` or uid check) | equivalent result; ours may run a redundant `mount` that fails harmlessly |
| Switch on | writes `host` when role is `gadget`, at start only if "Always enable OTG mode", else from its menu | writes `host` when role is `gadget`, while Options → USB keyboard is On (at start and when turned on) | same write; trigger differs |
| Switch off | `onExit`: writes `gadget` whenever the role is `host` | writes `gadget` on Off / exit **only if we made the switch** | a port already in host mode before launch stays in host mode; `launch.sh` reboots on exit anyway, which resets the role |
| Detecting keyboards | FBInk `fbink_input_scan(INPUT_KEYBOARD)` (ioctl capability bits), hot-plug via uevents (0.5 s delay) | `/proc/bus/input/devices` bitmap (EV_KEY + A, Z, Space, Enter), polled every 1 s | different classifier: a wired 60% keyboard has these keys, so both should accept it |
| Apple-mode keyboards | not handled (forum workaround: `hid ignore_special_drivers=1`) | not handled | none |
| Nickel | KOReader stops Nickel before running | `launch.sh` stops Nickel too (same kill list) and reboots on exit | none — Nickel does not see the keys |
| Layouts | several layout files | US only (decision 23) | by design |
