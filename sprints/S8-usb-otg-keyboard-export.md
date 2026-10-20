# S8 — USB-OTG keyboard & export

**Release:** v0.8.0 · **Estimate:** 4–6 d · **Depends on:** S7

## Goal
External USB keyboard via powered hub/Y-cable, export path (step 10).

## Scope
- evdev reader for `/dev/input/event*` (Cobalt capability or platform code; Plan B native)
- Keymap, modifiers, Ctrl+F etc.
- USB flag in status bar; in-app powered-hub note
- Export docs (USB folder; Syncthing via Cobalt Sync optional)

## Deliverables
- v0.8.0
- Hardware note: tested hub/cable

## Definition of done
- USB keyboard types into editor and scratch; OSK unaffected
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Hot-plug keyboard via powered hub
- [ ] Shortcuts work
- [ ] Unplug → no crash

Tasks: `tasks/S8-tasks.md`
