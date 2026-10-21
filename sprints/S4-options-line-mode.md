# S4 — Options & Line mode

**Status:** 🔨 BUILT 2026-10-21 (desktop/emulator green; awaiting install approval and on-device check) · **Release:** v0.4.0 · **Estimate:** 6–8 d · **Depends on:** S3 (v0.3.0) · **Branch:** `feat/v0.4-options-line-mode`

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

## As built (v0.4.0)
- **Options sheet** — `Aa` button in the title bar (left of the eye; also in preview). Full-page overlay, rows in the brief's order with a hint each: Paper (Day/Night), Type (Literata/Atkinson), Line (Off/On, − N words +), Size, Leading, Margin (steppers), Tab key (Tab/Spaces), Tab width (2/4/8), Keyboard (Compact/Full), Keyboard tone (Theme/Light/Dark), Drawer (Left/Right, used from S5), USB keyboard (On/Off; shows `USB` in the status bar; driver in S8), Orientation (Portrait/Landscape). Close with X or Esc.
- **Day/Night** — Night is the same drawing inverted (brief tokens), used by write, preview, Files and Options; switching is a full (flashing) refresh. Keyboard tone can keep a light keyboard under a night page and vice versa.
- **Line mode** (`crates/core/src/linefit.rs`) — measures the first N and N+1 words of a fixed prose sample with the real face and aims half-way, so a row holds N words; Size moves Margin, Margin moves Size; ranges size 14–40, margin 8–400 brief px. Refit on face, word count, size, margin or orientation change. Tests: fake measurer (core) and real Literata/Atkinson for N = 8…16 in both orientations (render).
- **Landscape** — the screen turns 90° on the portrait panel in software (canvas rotated in the app, touches mapped back), so FBInk's rotation quirks are avoided and the ≥16×16 refresh rule still holds (width/height swap). With Line on, the page column is locked to the portrait width (1440 px) with gutters (`#e0dcd4` day / `#242424` night); title, status and keyboard stay full width.
- **Landscape keyboards** — bar `Split · Full · Centre` (fixed place) above the keys; in Full, `Arrows`/`123` and `Pad left`/`Pad right` at the right edge. Centre = the chosen board (Compact or Full), centred, portrait key size. Split = the same board cut in two with a 3-unit gap; Space appears on both halves. Full = board + number pad (7 8 9 / 4 5 6 / 1 2 3 / 0 .) or direction pad (↑ / ← → / ↓) on either side. Keyboard re-layouts use a full refresh (no ghosting).
- **Persistence** — every setting in `notes/.sigma-settings` (`key=value`, atomic write); v0.2.1 files (`keyboard=…` only) still load; unknown keys / bad values fall back to defaults. Tab key follows Options (on-screen and hardware Tab).

### Decisions
- **S4-D1** Orientation is an Options row (manual), not the gyro: reading the Forma's accelerometer is device-specific and untested; auto-rotation can follow later.
- **S4-D2** Landscape turns the picture 90° clockwise on the panel. Which way the user holds the Forma is to be confirmed on device; the other direction is a small change (mapping in `App::present` / `App::to_logical`).
- **S4-D3** Preview keeps Literata faces even when Type = Atkinson (preview typography from the mock); preview follows Night, Size, Leading, Margin and Line.
- **S4-D4** Keyboard (Compact/Full) is an Options row; the on-board switch key stays as a shortcut.
- **S4-D5** Side-by-side check against `mockup/kobo-writing-mock.html` not done (no browser render of the mock in this run); screenshots in `docs/screenshots/v0.4/`.

### Screenshots
`docs/screenshots/v0.4/` (plain) and `docs/screenshots/v0.4/eink/` (`scripts/sim.sh --eink`, 16 greys + ghosting), from `tests/emu/v0.4-options.script`.
