# S4 — Options & Line mode

**Release:** v0.4.0 · **Estimate:** 6–8 d · **Depends on:** S3

## Goal
Options sheet, themes, typography, Line mode, landscape keyboards (steps 4–5).

## Scope
- Options sheet rows (Paper, Type, Size, Leading, Margin, Tab, Tab width, Keyboard tone, Drawer, USB keyboard)
- Day/Night with full refresh
- Line mode fitting with real measurement; locked 1440 column + gutter in landscape
- Landscape keyboard: Centre / Split / Full + num/arrow pad — built on the S2.5 key model, modifiers and unit-grid renderer; Options "Keyboard" row (Compact/Full/…) supersedes the toggle key as the main switch
- Persist all settings

## Deliverables
- v0.4.0
- Line-mode math tests with fake measurer

## Definition of done
- All settings persist across reboot; Line holds N words in both orientations
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Night mode: no grey card, full refresh on switch
- [ ] Rotate with Line on: column locked, gutter shown
- [ ] Each landscape keyboard layout types correctly

Tasks: `tasks/S4-tasks.md`
