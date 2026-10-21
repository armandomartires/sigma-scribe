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
crates/render  sigma-render 8-bit greyscale canvas, design tokens (brief §2 → grey), Literata (+ SemiBold, Italic) + Atkinson
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
- v0.2.2 touch: the Forma's lift signal (tracking id / pressure) and jitter while resting are unverified; if holds fire late or drags are jumpy, tune `Config` (hold_ms, slop) or the evdev MOVE_EPS. Palm / second-finger contacts are ignored (slot 0 only).
- Deletes are permanent in v0.2 (armed confirm only). A `.trash/` folder could be added later if wanted.
- Open from the brief: graph map vs list; Syncthing export later. (Edit-in-preview: decided read-only for v0.3, §10.12.)
- v0.3 preview speed on the Forma (parse + layout on toggle, on the UI thread) is unmeasured; a 100 KB note lays out in well under 1 s on a desktop debug build.

## 10. Decisions log
1. Device Kobo Forma, fw 4.38.23697. 2. Plan B (native Rust + FBInk + NickelMenu) — 2026-10-20 after the Cobalt installer block and S0 limits. 3. Install NickelMenu + app on the Forma only after a verified backup, with the user present. 4. Keyboard: on-screen and USB-OTG; no Bluetooth. 5. v1.0 = all features, released incrementally. 6. Licence AGPL-3.0, author Sigma Recon, public screenshots OK. 7. Distribution: GitHub Releases + MobileRead (Cobalt Store dropped). 8. 2026-10-20: v0.1.0 passed on device (user report) and was tagged; v0.2 = library (scope §11). 9. 2026-10-21: v0.2.0 passed on device and tagged; v0.2.1 = Full on-screen keyboard (S2.5). 10. 2026-10-21: v0.2.1 passed and tagged (8a54ab2); v0.2.2 = touch selection (S2.6). 11. 2026-10-21: v0.2.2 skipped (refresh crash found in the device log); v0.2.3 = v0.2.2 + region fix, installed instead; passed on device and tagged v0.2.3 (caaad3e). 12. 2026-10-21: S3 started (v0.3.0). **Preview is read-only in v0.3** — the brief's open question (tap in preview drops into write at that line?) is answered *no* for now; revisit after device use. 13. 2026-10-21: own small Markdown parser in core (no pulldown-cmark): the brief's subset plus `[[wiki]]`, source lines, literal fallbacks, dependency-free core. 14. 2026-10-21: Literata SemiBold 600 and Italic 400 static instances added (OFL, from the Literata variable fonts) for preview; no bold-italic; code in Atkinson as the mock. 15. 2026-10-21: preview status bar shows `page i of n` (not in the brief). 16. 2026-10-21: v0.3.0 passed on device (user report) and was tagged `v0.3.0`; S4 (v0.4.0) started.

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

### v0.2.2 scope (S2.6, requested 2026-10-21)
- Android-style touch selection: long-press (500 ms, 30 px tolerance) selects a word, or places the caret with Paste on blank space; two draggable handles with char snapping; long-press + slide extends by words; floating action bar Cut / Copy / Paste / Select all (shared clipboard + undo); double-tap selects a word; tap elsewhere dismisses.
- New core gesture recognizer; device events gain `Touch { Down | Move | Up, t }` and `Tick`; evdev follows MT protocol B on slot 0; desktop window simulates with mouse hold/drag.
- E-ink: no animation, partial fast refresh of changed regions only, drag redraws throttled to ~150 ms plus lift.
- Device risk: hold reliability depends on the Forma reporting a steady contact; see §9.

### v0.3.0 scope (S3, approved 2026-10-21 — ✅ done, passed on device)
- Eye ↔ pencil toggle in the title bar (left of Files); Esc leaves preview.
- Rendered with the real faces as the mock: headings (Literata 600, 1.45/1.22/1.05 em), **strong** (Literata 600), *em* (Literata Italic), ~~strike~~, lists with nesting, task boxes, quotes, inline and fenced code (Atkinson on wash), web links underlined, rules.
- `[[wiki]]` / `[[note|label]]` shown (solid if the note exists, dashed if not); navigation, backlinks, graph stay in S6.
- Page-turn buttons page through the preview; status shows `· preview · page i of n`.
- Read-only (decision 12). Entering keeps the place (page of the top source line); leaving restores write mode untouched.
- E-ink: full refresh entering/leaving preview, partial refreshes when paging.
- Deferred: tables and images drawn (shown as text / `[image: alt]`), task box tap, Night preview (S4).
- Details and acceptance: `sprints/S3-markdown-preview.md`, `tasks/S3-tasks.md`.

### Roadmap after v0.2 (confirmed)

| # | Sprint | Release | Contents |
|---|---|---|---|
| S0 | Foundations & Forma bring-up | — | ✅ backup, toolchain, FBInk/NickelMenu, install, device check (pass) |
| S1 | Editor core | **v0.1.0** ✅ | editor, OSK, autosave to .md, title/status bars, page keys, tap-to-place caret — passed on device |
| S2 | Library | **v0.2.0** ✅ | Files sheet, folders, new/rename/move/delete (armed), title-bar rename, seed, last-opened — passed on device |
| S2.5 | Full on-screen keyboard | **v0.2.1** ✅ | Windows-OSK style Full layout + Compact, sticky modifiers, Ctrl shortcuts, undo/redo, selection, remembered choice, Full default — passed on device |
| S2.6 | Touch selection | **v0.2.3** ✅ (v0.2.2 skipped) | long-press word select, handles, word-wise drag, action bar (Cut/Copy/Paste/Select all), double-tap; gesture recognizer + evdev touch phases; v0.2.3 pads refresh regions to ≥16 px and makes refresh failures non-fatal — v0.2.2 was never installed because the v0.2.1 log showed FBInk rejecting a 1×34 region and the app exiting (2026-10-21 00:25) — passed on device 2026-10-21 |
| S3 | Markdown preview | **v0.3.0** ✅ | eye/pencil toggle, read-only paged preview (headings, emphasis, lists, tasks, quotes, code, links, rules; wiki links shown), own parser, Literata SemiBold/Italic — passed on device 2026-10-21, tagged `v0.3.0` |
| S4 | Options & Line mode | v0.4.0 | Day/Night, faces, steppers, Line mode, rotation + landscape keyboards |
| S5 | Drawer | v0.5.0 | Contents, To-dos, Scratch |
| S6 | Find & wiki | v0.6.0 | find/replace, wiki links, backlinks, graph |
| S7 | Help & hardening | v0.7.0 | help pages, perf, power-loss tests, Nickel restart instead of reboot |
| S8 | USB-OTG keyboard & export | v0.8.0 | evdev keyboard via powered hub, export docs |
| S9 | Release | v1.0.0 | packaging, GitHub release, MobileRead thread |
