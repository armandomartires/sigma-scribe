# S8 — USB-OTG keyboard & export

**Release:** v0.8.0 · **Estimate:** 4–6 d · **Depends on:** S7

## Goal
External USB keyboard via powered hub/Y-cable, export path (step 10).

## Scope
- evdev keyboard reader for `/dev/input/event*` (Forma USB host mode via powered hub)
- Keymap, modifiers, Ctrl+F etc.
- USB flag in status bar; in-app powered-hub note
- Export docs (notes are already .md on USB; Syncthing optional later)

## Deliverables
- v0.8.0
- Hardware note: tested hub/cable

## Definition of done
- USB keyboard types into editor and scratch; OSK unaffected
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `.ai/mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [x] `cargo test` for new core logic
- [x] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Hot-plug keyboard via powered hub
- [ ] Shortcuts work
- [ ] Unplug → no crash

Tasks: `.ai/tasks/S8-tasks.md`

## As built (2026-10-21, v0.8.0 — install PENDING APPROVAL, not tagged)
- `crates/device/src/usbkbd.rs` (detection, keymap), `otg.rs` (host-mode switch), watcher + readers in `fbink.rs`; `Event::Chord`, `Event::Usb`, `HwKey::Home/End/PageUp/PageDown`.
- `App::chord` routes USB combos through `Keyboard::press` (S2.5 shortcut code) in every text context; sheets ignore chords; Ctrl+F in preview opens Find.
- Status `USB` / `no USB`; Help → USB keyboard & export (powered-hub note, shortcuts, export).
- Tests: `usbkbd` (proc parsing 32/64-bit, keymap, repeat), `otg`, `app/tests/usb.rs` (shortcuts, Find, Scratch, OSK state, unplug, host mode on/off), emulator script.
- Docs: `docs/usb-keyboard.md` (hardware note — no hub tested yet), export in Help and that doc.

## Decisions
- S8-D1: USB input is accepted whenever a keyboard is found; the Options setting controls host mode and the flag (brief: "a setting").
- S8-D2: status shows `USB` when a keyboard is connected and `no USB` when the setting is on but none is found (brief only had `USB`).
- S8-D3: host mode via debugfs role switch, only while the setting is on, restored on Off/exit; no kernel modules shipped.
- S8-D4: US layout only; no exclusive grab (EVIOCGRAB) yet.
- Not done: side-by-side mock check; tested hub/cable (needs the device).
