# Plan — Sigma Scribe: a Markdown notebook on the Kobo Forma

**Status:** ✅ APPROVED 2026-10-20 · **Platform: Plan B — native Rust + FBInk + NickelMenu** (decided 2026-10-20 22:14 CEST; Cobalt dropped) · **v0.1.0 passed on the Forma** (2026-10-20, tag `v0.1.0`) · **v0.2.0 passed on the Forma** (2026-10-21, tag `v0.2.0`).
**Licence:** AGPL-3.0-only (`LICENSE`) · **Author:** Sigma Recon · **Repo:** `sigma-scribe`
**Inputs:** `design-brief.md` (behaviour), `mockup/kobo-writing-mock.html` (look).

---

## 1. What the app is
A distraction-free Markdown/plain-text notebook for an e-reader (“the page is the product”): editor with autosave and on-screen keyboard, Markdown preview with wiki links/backlinks/graph, files and folders, options (Day/Night, Literata/Atkinson, size/leading/margin, words-per-line), side drawer (Contents, To-dos, Scratch), find & replace, help. Details: design-brief.md.

## 2. Target device (read-only probe 2026-10-20)
| Field | Value |
|---|---|
| Model | Kobo Forma (code 377, serial prefix N782, *frost*, Mark 7 / i.MX6 SLL, 512 MB) |
| Firmware | 4.38.23697 — **stay on 4.x** (NickelMenu does not support 5.x) |
| Panel | 8" 1440×1920 Carta 300 ppi, greyscale, rotation sensor |
| Input | capacitive touch, 2 page-turn buttons, power button; **no Bluetooth**; USB-OTG keyboard needs a powered hub / Y-cable |
| Storage | `/mnt/onboard` (vfat, 6.7 GB free); existing mod: NickelDBus 0.2.0 only |

## 3. Platform decision (S0)
**Cobalt dropped** because:
1. Cobalt's installer refuses the Forma: `kobo setup --dry-run` → “unsupported Kobo hardware: model code N782 has no reviewed Cobalt profile” (hard check in `crates/kobo-cli/src/setup.rs`; porting needs maintainer review + attended evidence).
2. Even if supported, the Cobalt SDK cannot meet the brief: no free-form drawing, no font choice, no colour/inversion (Night), no pixel layout (drawer, gutter, words-per-line), no multi-line text field (only a monospace terminal grid), notes in a key/value store (≤256 keys, ≤256 KiB) instead of `.md` files on USB, no raw input for USB keyboards. (docs/s0-findings.md)

**Plan B:** our own Rust app draws into an 8-bit canvas and pushes dirty rectangles to the panel with **FBInk** (NiLuJe, GPL-3.0-or-later, linked statically; compatible with AGPL-3.0). Touch and buttons are read from **evdev**. **NickelMenu** (pgaskin, v0.6.0, firmware 4.x) adds a menu entry that runs our launcher; the launcher stops Nickel, runs the app and **reboots back to the stock reader** on exit (the KOReader approach, simplified).

## 4. Architecture
```
crates/core    sigma-core   pure Rust, no platform deps: buffer, caret, measured wrap, keyboard model,
                            note store (atomic .md writes), autosave, text utils   [kept from S1]
crates/render  sigma-render 8-bit greyscale canvas, design tokens (brief §2 → grey), Literata + Atkinson
                            Hyperlegible (SIL OFL, embedded) via fontdue, editor screen layout/draw/hit-test
crates/device  sigma-device `Device` trait (size, present(canvas, rect, Refresh::{Fast,Partial,Full}),
                            next_event → Tap/PageTurn/Power/Key/Quit), evdev decoder + touch transform,
                            FBInk backend (arm only, C shim + libfbink.a), headless PNG backend
app            sigma-scribe App loop (generic over Device); bins: sigma-scribe (Kobo: FBInk; desktop: live
                            minifb window), sigma-emu (headless scripts → PNG)
packaging/kobo              .adds/sigma-scribe/{launch.sh,touch.conf}, .adds/nm/sigma-scribe
```
- Notes are plain files: `/mnt/onboard/sigma-scribe/notes/*.md` (visible over USB; export = plug in USB).
- Refresh policy: keystrokes/caret → `Fast` on the changed rectangle only; keyboard toggle/scroll → `Partial`; first paint, theme, orientation, sheets → `Full` (flashing GC16).
- Layout tokens are brief px ×(panel width/1264) = ×1.139 on the Forma (44 px targets → 50 px).

## 5. Toolchain
Rust stable + `armv7-unknown-linux-musleabihf` (static, `rust-lld`); ARM gcc: musl.cc toolchain in `~/.local/toolchains` (no sudo) or Debian `gcc-arm-linux-gnueabihf`; FBInk `make staticlib MINIMAL=true IMAGE=true KOBO=true CROSS_TC=armv7l-linux-musleabihf` in `~/src/FBInk`; NickelMenu v0.6.0 `KoboRoot.tgz` (sha256 `322ff9aa…d064af`). Scripts: `scripts/build-armv7.sh`, `scripts/package-kobo.sh`, `scripts/sim.sh`.

## 6. Testing
Unit tests (core/render/device), headless emulator end-to-end tests and scripts (`tests/emu/*.script`), live desktop simulator with optional e-ink look, on-device checklist per sprint. See docs/testing.md and docs/install-plan-forma.md.

## 7. Deployment & rollback
docs/deployment.md and docs/install-plan-forma.md. Only `/mnt/onboard` is touched by our files; NickelMenu's `KoboRoot.tgz` adds one library (`/usr/local/Kobo/imageformats/libnm.so`). Rollback: delete `.adds/sigma-scribe` + `.adds/nm/sigma-scribe`; uninstall NickelMenu by creating `.adds/nm/uninstall`; restore files from the backup.

## 8. Distribution (replaces the Cobalt Store)
GitHub Releases of `sigma-scribe` (AGPL-3.0, Sigma Recon): `sigma-scribe-<ver>-kobo.tar.gz` (the `.adds` tree, copied onto the USB drive) + sha256 + source; announcement thread on MobileRead (Kobo Developer's Corner). NickelMenu is a prerequisite users install from its own release. See docs/publishing.md.

## 9. Risks & open questions
- Touch transform on the Forma unverified (default `swap,mirror_x`, configurable in `touch.conf`; checklist §4).
- Stopping Nickel and rebooting on exit is robust but slow (~30 s); a later sprint can restart Nickel instead (as KOReader does).
- Rotation (gyro) handling and FBInk rotation quirks on Mark 7 — S4.
- USB-OTG keyboard: Forma host mode + powered hub, evdev keyboard mapping — S8 (the desktop window already feeds real keystrokes through the same `HwKey` path).
- Waveform choice for typing (AUTO vs DU) — measure on device.
- Never update the Forma to firmware 5.x.
- Deletes are permanent in v0.2 (armed confirm only). A `.trash/` folder could be added later if wanted.
- Open from the brief: edit-in-preview; graph map vs list; Syncthing export later.

## 10. Decisions log
1. Device Kobo Forma, fw 4.38.23697. 2. Plan B (native Rust + FBInk + NickelMenu) — 2026-10-20 after the Cobalt installer block and S0 limits. 3. Install NickelMenu + app on the Forma only after a verified backup, with the user present. 4. Keyboard: on-screen and USB-OTG; no Bluetooth. 5. v1.0 = all features, released incrementally. 6. Licence AGPL-3.0, author Sigma Recon, public screenshots OK. 7. Distribution: GitHub Releases + MobileRead (Cobalt Store dropped). 8. 2026-10-20: v0.1.0 passed on device (user report) and was tagged; v0.2 = library (scope §11). 9. 2026-10-21: v0.2.0 passed on device and tagged; v0.2.1 = Full on-screen keyboard (S2.5).

## 11. Sprints & version roadmap

### v0.2 scope (S2, decided 2026-10-20)
- Files sheet from the title-bar folder icon: folders first, then notes with word counts; enter folders, `← Up to …` crumb; open a note into the editor (current note saved first).
- New note / New folder (start in rename); More → Rename / Move / Delete; Delete needs a second tap (armed); folder delete removes its contents.
- Rename in place from the editor title bar (OK / Cancel / Return / blur).
- Safe storage: plain `.md` / `.txt` under `/mnt/onboard/sigma-scribe/notes`; paths validated (no `..`, absolute or hidden parts); names cleaned; collisions get ` 2`, ` 3`…; atomic writes; the last real note cannot be deleted; `.scratchpad.md` protected.
- Seed library only when there are no notes (keeps the v0.1 note); last-opened note restored on launch (`notes/.sigma-last-opened`).
- Out of scope: preview, drawer, search, sorting options, multi-select, undo of deletes (no trash in v0.2 — see §9).

### v0.2.1 scope (S2.5, requested 2026-10-21)
- "Full" Windows On-Screen-Keyboard style layout (Esc + number row, brackets, Caps, Enter, two Shifts, Ctrl, Alt, arrows, Home/End/PgUp/PgDn/Del) alongside the brief's Compact board; switch key on both; choice remembered. **Full is the default** (2026-10-21); a stored Compact choice is kept.
- Shift one-shot / double-tap lock, Caps toggle, Ctrl/Alt one-shot; Ctrl+A/C/X/V (internal clipboard), Ctrl+Z/Y (new undo/redo in core), Ctrl+←/→, Shift+navigation selection.
- Portrait fit: 1-unit keys 82×66 px, Full board 380 px → page 1420 px (Compact 307 → 1493). Only changed regions are refreshed.
- S4 landscape keyboards build on the same key model and renderer (see `sprints/S2.5-full-keyboard.md`). Alt has no bindings yet.

### Roadmap after v0.2 (confirmed)

| # | Sprint | Release | Contents |
|---|---|---|---|
| S0 | Foundations & Forma bring-up | — | ✅ backup, toolchain, FBInk/NickelMenu, install, device check (pass) |
| S1 | Editor core | **v0.1.0** ✅ | editor, OSK, autosave to .md, title/status bars, page keys, tap-to-place caret — passed on device |
| S2 | Library | **v0.2.0** ✅ | Files sheet, folders, new/rename/move/delete (armed), title-bar rename, seed, last-opened — passed on device |
| S2.5 | Full on-screen keyboard | **v0.2.1** ✅ | Windows-OSK style Full layout + Compact, sticky modifiers, Ctrl shortcuts, undo/redo, selection, remembered choice, Full default — passed on device |
| S3 | Markdown preview | v0.3.0 | parser + renderer, eye/pencil |
| S4 | Options & Line mode | v0.4.0 | Day/Night, faces, steppers, Line mode, rotation + landscape keyboards |
| S5 | Drawer | v0.5.0 | Contents, To-dos, Scratch |
| S6 | Find & wiki | v0.6.0 | find/replace, wiki links, backlinks, graph |
| S7 | Help & hardening | v0.7.0 | help pages, perf, power-loss tests, Nickel restart instead of reboot |
| S8 | USB-OTG keyboard & export | v0.8.0 | evdev keyboard via powered hub, export docs |
| S9 | Release | v1.0.0 | packaging, GitHub release, MobileRead thread |
