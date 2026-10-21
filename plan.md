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
- USB-OTG keyboard / mouse (S8, S8.1, built): host mode with a wired 60% keyboard is known to work on this Forma (most likely via KOReader, whose role switch ours matches); the K400 Plus receiver and touchpad are untested, including whether two-finger scroll arrives as wheel events. Falls back to "no USB". See `docs/usb-keyboard.md`.
- Waveform choice for typing (AUTO vs DU) — measure on device.
- Never update the Forma to firmware 5.x.
- v0.2.2 touch: the Forma's lift signal (tracking id / pressure) and jitter while resting are unverified; if holds fire late or drags are jumpy, tune `Config` (hold_ms, slop) or the evdev MOVE_EPS. Palm / second-finger contacts are ignored (slot 0 only).
- Deletes are permanent in v0.2 (armed confirm only). A `.trash/` folder could be added later if wanted.
- Open from the brief: graph map vs list; Syncthing export later. (Edit-in-preview: decided read-only for v0.3, §10.12.)
- v0.3 preview speed on the Forma (parse + layout on toggle, on the UI thread) is unmeasured; a 100 KB note lays out in well under 1 s on a desktop debug build.

## 10. Decisions log
1. Device Kobo Forma, fw 4.38.23697. 2. Plan B (native Rust + FBInk + NickelMenu) — 2026-10-20 after the Cobalt installer block and S0 limits. 3. Install NickelMenu + app on the Forma only after a verified backup, with the user present. 4. Keyboard: on-screen and USB-OTG; no Bluetooth. 5. v1.0 = all features, released incrementally. 6. Licence AGPL-3.0, author Sigma Recon, public screenshots OK. 7. Distribution: GitHub Releases + MobileRead (Cobalt Store dropped). 8. 2026-10-20: v0.1.0 passed on device (user report) and was tagged; v0.2 = library (scope §11). 9. 2026-10-21: v0.2.0 passed on device and tagged; v0.2.1 = Full on-screen keyboard (S2.5). 10. 2026-10-21: v0.2.1 passed and tagged (8a54ab2); v0.2.2 = touch selection (S2.6). 11. 2026-10-21: v0.2.2 skipped (refresh crash found in the device log); v0.2.3 = v0.2.2 + region fix, installed instead; passed on device and tagged v0.2.3 (caaad3e). 12. 2026-10-21: S3 started (v0.3.0). **Preview is read-only in v0.3** — the brief's open question (tap in preview drops into write at that line?) is answered *no* for now; revisit after device use. 13. 2026-10-21: own small Markdown parser in core (no pulldown-cmark): the brief's subset plus `[[wiki]]`, source lines, literal fallbacks, dependency-free core. 14. 2026-10-21: Literata SemiBold 600 and Italic 400 static instances added (OFL, from the Literata variable fonts) for preview; no bold-italic; code in Atkinson as the mock. 15. 2026-10-21: preview status bar shows `page i of n` (not in the brief). 16. 2026-10-21: v0.3.0 passed on device (user report) and was tagged `v0.3.0`; S4 (v0.4.0) started. 17. 2026-10-21: S4 orientation is a manual Options row (no gyro yet), rendered by rotating the canvas in the app (not FBInk rotation); details and decisions S4-D1…D5 in `sprints/S4-options-line-mode.md`. 18. 2026-10-21: v0.4.0 passed the full on-device checklist; the landscape rotation direction (90° clockwise) is confirmed correct; tagged `v0.4.0` (89f3928). S5 (v0.5.0) started. 19. 2026-10-21: S5 drawer is write-mode only (hidden in preview); long lists are cut with "N more…" instead of scrolling; open panes are not persisted; the open note and scratch are indexed live from their buffers, other files by a background thread (S5-D1…D4 in `sprints/S5-side-drawer.md`). 20. 2026-10-21: the user cannot test on the Kobo for now; v0.5.0 stays uninstalled and untagged, its open questions (drawer in preview, scrolling lists, remembered panes) stay at the v0.5 defaults, and S6 (v0.6.0) is built on top. S6 decisions S6-D1…D5 in `sprints/S6-find-wiki-links.md` (Graph is a list; wiki taps only in preview; new notes beside the linking note; link index built synchronously; Find closes when a sheet or preview opens). 21. 2026-10-21: the user approved the v0.6 behaviour without device testing: a missing `[[link]]` creates the note next to the linking note; a known link opens in preview; Graph stays a list; the v0.5 defaults stay (no drawer in preview, "N more…" instead of scrolling, open panes not remembered). v0.5.0 and v0.6.0 were never installed; the device goes 0.4.0 → 0.7.0 (pending approval). S7 (v0.7.0) built; decisions S7-D1…D3 in `sprints/S7-help-hardening.md`. 22. 2026-10-21: S8 (v0.8.0) built without device access: USB host mode is switched through debugfs only while Options → USB keyboard is On and restored on Off/exit; no kernel modules; US layout; status `USB` / `no USB` (S8-D1…D4). v0.5–v0.7 never installed; the device goes 0.4.0 → 0.8.0 (pending approval). 23. 2026-10-21: the USB keyboard stays **US layout only** (user decision). 24. 2026-10-21 (corrected 13:55): a **regular wired 60% USB keyboard** has worked on this Forma before, most likely with KOReader, so USB host mode plus a wired keyboard is known to work on fw 4.38.23697; the S8 host-mode risk is lowered. The **Logitech K400 Plus and its Unifying receiver (and its touchpad) have never been tested on the device** (an earlier note said otherwise; that was wrong). Our host-mode switch was compared with KOReader's (`docs/usb-keyboard.md`). Mouse/touchpad support added as v0.8.1 in sprint S8.1 (decisions S8.1-D1…D5 in `sprints/S8.1-mouse-touchpad.md`). 25. 2026-10-21: S9 (v1.0.0) prepared without device access: the release candidate is the **CI-built** package (CI and local builds differ by toolchain; CI is reproducible with itself); release.yml makes only a draft release on a maintainer-pushed tag; no tag, release, public repo, MobileRead post or Cobalt contact without approval and a passing device test; the device goes 0.4.0 → 1.0.0 (pending approval). Decisions S9-D1…D4 in `sprints/S9-release.md`. 26. 2026-10-21: S9.1 Graph map added to the v1.0.0 scope (user request): the Graph is drawn as a map of notes and links by default, with the S6 list one tap away (supersedes S6-D1 "Graph is a list"). Decisions S9.1-D1…D5 in `sprints/S9.1-graph-map.md`: tap selects, second tap / Open opens in preview; dots with labels only for current/selected/neighbours when labels would overlap; layout on the UI thread with a cache; orphans on an outer ring; 4 zoom levels. 27. 2026-10-21: S9.2 title bar added to the v1.0.0 scope (user request): icons twice the size and in `mockup/kobo-writing-mock.html` order (Files, Preview, Find, Scratch, To-dos, Contents, Graph, Options, Help after the title — the earlier build had them mirrored); close X in the rightmost slot; fixed slots in preview; overflow by spacing then size (≥ 44 px), no ⋯ menu (S9.2-D1…D5 in `sprints/S9.2-title-bar.md`). 28. 2026-10-21 20:35: the user approved the 0.4.0 → 1.0.0 install; the CI binary `bb38a083…dad1` was installed (only the binary replaced, backup `2026-10-21-pre-v1.0.0` verified); the on-device v0.5–v1.0 checklist is next; fresh-install/uninstall tests, tag and release still need approval. 29. 2026-10-21 ~21:00: device test (user) of 1.0.0 with a Logitech K400 Plus on a charging USB splitter: **K400 keyboard, pointer and two-finger scroll (up/down) verified on the device**; the pointer leaves ghost trails (bug, fix planned for v1.0.1, S10). v1.1 planned (PLANNING ONLY, see "Post-1.0 plan"); no development until approved. 30. 2026-10-21 21:24: user answers to the plan — one release per feature (v1.0.1 … v1.8), UI scale default 200 %, Auto orientation on with fallback to the last manual one, Farsi + Arabic together, Japanese kana + kanji, Simplified Chinese, UI languages EN/FR/PT, pen mode = handwriting-to-text panel (no drawing). 31. 2026-10-21 21:31: user answers — (1) Grok Bot drafts the translations for **all** UI languages (EN, FR, PT; later fa, ar, ja, zh), marked **"machine-drafted, best effort"**, no external reviewer; (2) the UI language **follows the Kobo's language**: read-only from `/mnt/onboard/.kobo/Kobo/Kobo eReader.conf`, `[ApplicationPreferences] CurrentLocale=` (this Forma: `en`, from the 2026-10-20 backup), fall back to EN; (3) opt-in, on-device, local-only collection of the user's handwriting for training: yes; (4) character boxes are an acceptable v1.5 fallback; (5) SKK-style Japanese first: yes; (6) test notes prefixed `test-` (pack rebuilt); defaults kept: CJK common-character subsets + optional full font; Modern Standard Arabic first, Urdu letters a cheap later addition. 33. 2026-10-21 23:40: v1.0.2 = sprint S10.2 (Graph map look: round nodes, selection pill, plain links, labels beside dots only where they fit; Back navigation with a 50-entry session history; long-press history list deferred). Decisions S10.2-D1/D2 in `sprints/S10.2-map-look-back.md`. 32. 2026-10-21 22:32: the user approved tagging v1.0.0 (annotated tag on the rewritten S9.2 merge, the code installed on the Forma; draft release only, not published), merging the v1.1 plan, and building v1.0.1 (S10 + S10.1). v1.0.1 is built and pending install approval; not tagged. 34. 2026-10-22: v1.0.3 = sprint S10.3, both fixes approved by the user. **(a) S5-D1 amended (decision S10.3-D1):** the drawer (Contents, To-dos, Scratch) is allowed in write mode **and preview**, never over a sheet (Files, Options, Graph, Help, rename). In preview the page is laid out beside the drawer; Contents lists the preview's headings and a tap jumps to the preview page holding it; a To-do checkbox tap toggles it with the same atomic write + reindex as write mode and re-renders the preview — a **deliberate exception to read-only preview (decision 12)**; the Scratch pad stays editable, keys go to it only while it has focus, the note stays read-only. **(b) Pointer refresh strategy (decision S10.3-D2):** the 1.0.1 device log showed `perf pointer: gl16` p50 606 ms / p95 1091 ms and typing p95 up to ~187 ms (was 66–94 ms) — the GL16 updates and the blocking `fbink_wait_for_complete` every 20th move held up the input loop. Moving pointer = fast **DU** update, never waited for; one non-flashing **GL16** cleanup over the pointer's path when it hides (3 s idle, typing, touching); regions stay ≥ 16×16; no pointer code path waits for refresh completion. Device log: `perf pointer: du …` every 20th move, `perf pointer: gl16-cleanup …` per hide, and a `perf pointer:` p50/p95 summary at exit next to the typing `perf:` line. 35. 2026-10-22: **Icons → Lucide (user-approved, decision S11-D-icons).** The custom-drawn title-bar/drawer icons in `crates/render` (~11 `draw_*_icon` functions) are replaced by the Lucide icon pack (ISC licence). The SVGs are baked at build time with resvg into small greyscale images, embedded in the binary, one image per icon per UI-scale step (no runtime scaling), and drawn by one blit function. Options keeps its 'Aa' letters. Stroke drawn slightly thicker (~2 px at device size), no fine detail, for e-ink. Lucide's ISC LICENSE ships in the repo and package, with a credit line in the README and Help → About. Not urgent: scheduled alongside or right after v1.1 (UI scale).

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

### v0.4.0 scope (S4, built 2026-10-21 — ✅ passed on device, tagged `v0.4.0`)
- Options sheet (`Aa`): Paper, Type, Line, Size, Leading, Margin, Tab key, Tab width, Keyboard, Keyboard tone, Drawer, USB keyboard, Orientation; all persisted in `.sigma-settings`.
- Night mode (inverted tokens) with a full refresh; keyboard tone Theme/Light/Dark.
- Line mode 8–16 words, measured with the real face; locked 1440 px column + gutters in landscape.
- Landscape (manual) with Split · Full · Centre keyboards and number / arrow pad left or right.
- Details: `sprints/S4-options-line-mode.md`, `tasks/S4-tasks.md`.

### v0.5.0 scope (S5, built 2026-10-21 — install PENDING APPROVAL, not tagged)
- Drawer icons in the title bar (Contents, To-dos, Scratch); panes stack with equal height on the Options → Drawer side; write mode only.
- Contents jumps to headings; To-dos across all files with All/Open/Done and byte-exact line toggle; Scratch saved to hidden `.scratchpad.md`.
- Details: `sprints/S5-side-drawer.md`, `tasks/S5-tasks.md`.

### v0.6.0 scope (S6, built 2026-10-21 — install PENDING APPROVAL, not tagged)
- Find bar under the title bar: Find / Replace fields, count, Prev, Next, Replace, All, Aa; seeded from a one-line selection; Ctrl+F; opening from preview switches to write.
- Wiki links: tap in preview opens a known note (in preview) or creates a missing one; backlinks listed at the end of the preview.
- Graph sheet as a list of notes with → / ← links (decision S6-D1).
- v0.5 was never installed; the device goes 0.4.0 → 0.6.0 directly (`docs/install-plan-forma.md`).
- Details: `sprints/S6-find-wiki-links.md`, `tasks/S6-tasks.md`.

### v0.7.0 scope (S7, built 2026-10-21 — install PENDING APPROVAL, not tagged)
- Help sheet: App info + Markdown reference (paginated, preview engine).
- Robustness: kill-during-save test, unreadable notes never overwritten; touch targets ≥ 44 px everywhere; rotation pass.
- Perf: desktop p95 ≈ 20 ms; device logs p50/p95 on exit (`docs/perf.md`).
- Details: `sprints/S7-help-hardening.md`, `tasks/S7-tasks.md`.

### v0.8.0 scope (S8, built 2026-10-21 — install PENDING APPROVAL, not tagged)
- USB keyboard: host-mode switch (Options), hot-plug detection, US keymap, Ctrl/Shift chords through the shortcut code, `USB` / `no USB` flag.
- Help → USB keyboard & export; `docs/usb-keyboard.md`.
- Details: `sprints/S8-*.md`, `tasks/S8-tasks.md`.

### v0.8.1 scope (S8.1, built 2026-10-21 — install PENDING APPROVAL, not tagged)
- USB mouse / touchpad (incl. the K400 Plus touchpad): e-ink pointer, click = tap, drag-select, double-click word, wheel scroll; unplug safe.
- Details: `sprints/S8.1-mouse-touchpad.md`, `tasks/S8.1-tasks.md`.

### v1.0.0 scope (S9, built 2026-10-21 — install PENDING APPROVAL, not tagged, not released)
- Release workflow + reproducible, checked package with third-party notices; README, user install doc, release notes / MobileRead / Forma-notes drafts (files only); v1.0 screenshots.
- Details: `sprints/S9-release.md`, `tasks/S9-tasks.md`, `docs/release/checklist.md`.
- S9.1 (added 2026-10-21, decision 26): Graph map — notes and links drawn, Map / List toggle, select + open in preview, pan, zoom, mouse. Details: `sprints/S9.1-graph-map.md`, `tasks/S9.1-tasks.md`.
- S9.2 (added 2026-10-21, decision 27): title-bar icons doubled and in the mockup's order. Details: `sprints/S9.2-title-bar.md`, `tasks/S9.2-tasks.md`.

### Post-1.0 plan — one release per feature (PLANNING ONLY — updated 2026-10-21 21:24 with the user's answers; nothing implemented, no development until approved)

User decisions (2026-10-21 21:24): v1.0.1 = pointer trails + reverse scroll + "USB keyboard & mouse"; UI scale default **200 %**; Auto orientation **on** by default with fallback to the last manual orientation; Farsi **and** Arabic as one RTL release; Japanese needs kana **and** kanji conversion; Simplified Chinese wanted; UI languages EN/FR/PT; **no drawing** — pen mode becomes a handwriting-to-text panel; one release per feature.

| Release | Sprint | Feature | Estimate | Risk |
|---|---|---|---|---|
| **v1.0.1** | S10 (+ S10.1 docs) | Pointer ghost-trail fix (one merged old∪new region, GL16, cleanup on hide), Reverse scroll (Traditional/Natural), "USB keyboard & mouse" label; test-notes pack docs | 1.75 d (+0.25 d pack) | low–med (GL16 latency) |
| **v1.1** | S11 | UI scale 100–200 %, **default 200 %**, mm-based; Lucide icons (ISC) baked per scale step at build time, with or right after v1.1 (decision 35); fit check done: tight in landscape with the Full keyboard (~9 note rows), Options 5-value rows → cyclers, Find tool row in portrait | 4–5 d | medium |
| **v1.2** | S12 | Orientation: Auto (default) + 4 manual; accelerometer via MSC_RAW (KOReader method); fallback to last manual | 3 d + 0.5 d probe | medium |
| **v1.3** | S13 | Latin layouts US/FR/PT/DE/ES (+UK/IT/Nordic), dead keys, OSK + USB; **input pipeline + candidate bar** | 4–5 d | medium |
| **v1.4** | S14 | UI languages EN/FR/PT (Fluent), language setting, pseudo-locale tests, **RTL mirroring switch** | 4–6 d + review | low–med |
| **v1.5** | S15 | Handwriting input panel (Latin), finger/stylus/Auto, palm rejection, candidates; spike + go/fallback gate | 1 wk spike + 3–5 wk (word mode) or ~2 wk (character mode) | **high** |
| **v1.6** | S16 | Farsi + Arabic: shaping (rustybuzz), bidi, fonts, layouts, fa/ar UI mirrored | 4–6 wk | high |
| **v1.7** | S17 | Japanese: romaji→kana, SKK-style then Mozc-dictionary sentence conversion, CJK font files | 4–6 wk | high |
| **v1.8** | S18 | Simplified Chinese pinyin on the v1.7 framework, SC font | 2–3 wk | medium |

**Why this order:** the bug fix first (users hit it today). UI scale next because every later feature adds UI and must be laid out at the 200 % default — doing it later would mean re-laying out everything twice. Orientation is small and independent and benefits from the new scale. Latin layouts build the input pipeline and candidate bar that handwriting and the CJK IMEs reuse. i18n comes before RTL because the RTL release needs the string catalog and the mirroring switch (and fa/ar UI strings). Handwriting (Latin) before RTL/CJK: it proves the input-panel + candidate framework with the smallest model and alphabet, and its risky spike can fail without blocking the script releases (it is independent of them, so it can also move after v1.6 if the spike says "fallback"). RTL before CJK: shaping/bidi changes the text engine that CJK wrapping also touches; doing the deeper text-engine change once, earlier, avoids reworking CJK line breaking. Chinese last: it reuses the v1.7 converter, fonts and candidate UI.

**Key feasibility findings:** CJK — offline engines exist with AGPL-compatible licences (Mozc BSD-3 + permissive dictionary, SKK-JISYO GPL-2+, libkkc GPL-3, librime BSD-3 with LGPL-3 Rime data, libpinyin GPL-3); the practical route is an own Rust converter on Mozc (JP) and Rime (ZH) data, because Mozc/librime are heavy C++ cross-builds for armv7; dictionaries 10–40 MB, RAM 30–60 MB, conversion in ms on the A9 if mmap'd; fonts as separate files (JP/SC subsets ~2.5–3.5 MB per weight each, full Noto CJK ~19 MB) — storage is fine, the embedded binary is not. Handwriting — Tesseract (image OCR) and Google ML Kit Digital Ink (proprietary mobile SDK) are not usable; MyScript is commercial; the realistic open path is an own small BiLSTM/CTC model (1–2 M params, 1–8 MB) on tract, ~0.3–0.8 s per word on the Forma, with the main risk in training-data licences (IAM-OnDB and most online-ink sets are research-only); Zinnia (BSD) covers CJK single characters; fallbacks: word-at-a-time → single-character boxes → $P letter gestures.

**Answered 2026-10-21 21:31 (decision 31):** translations machine-drafted by Grok Bot for all UI languages (best effort, no reviewer); UI language follows the Kobo (`Kobo eReader.conf` → `[ApplicationPreferences] CurrentLocale`, read-only, EN fallback); opt-in local handwriting collection yes; character boxes OK as v1.5 fallback; SKK-style Japanese first; test notes prefixed `test-`. Defaults kept: CJK subsets + optional full font; Modern Standard Arabic first, Urdu letters later. **No open questions left in the plan.**

### Roadmap after v0.2 (confirmed)

| # | Sprint | Release | Contents |
|---|---|---|---|
| S0 | Foundations & Forma bring-up | — | ✅ backup, toolchain, FBInk/NickelMenu, install, device check (pass) |
| S1 | Editor core | **v0.1.0** ✅ | editor, OSK, autosave to .md, title/status bars, page keys, tap-to-place caret — passed on device |
| S2 | Library | **v0.2.0** ✅ | Files sheet, folders, new/rename/move/delete (armed), title-bar rename, seed, last-opened — passed on device |
| S2.5 | Full on-screen keyboard | **v0.2.1** ✅ | Windows-OSK style Full layout + Compact, sticky modifiers, Ctrl shortcuts, undo/redo, selection, remembered choice, Full default — passed on device |
| S2.6 | Touch selection | **v0.2.3** ✅ (v0.2.2 skipped) | long-press word select, handles, word-wise drag, action bar (Cut/Copy/Paste/Select all), double-tap; gesture recognizer + evdev touch phases; v0.2.3 pads refresh regions to ≥16 px and makes refresh failures non-fatal — v0.2.2 was never installed because the v0.2.1 log showed FBInk rejecting a 1×34 region and the app exiting (2026-10-21 00:25) — passed on device 2026-10-21 |
| S3 | Markdown preview | **v0.3.0** ✅ | eye/pencil toggle, read-only paged preview (headings, emphasis, lists, tasks, quotes, code, links, rules; wiki links shown), own parser, Literata SemiBold/Italic — passed on device 2026-10-21, tagged `v0.3.0` |
| S4 | Options & Line mode | **v0.4.0** ✅ | Options sheet, Day/Night, faces, steppers, Line mode, manual rotation + landscape keyboards (Centre/Split/Full + pads), all settings persisted — passed on device 2026-10-21 (landscape direction confirmed), tagged `v0.4.0` |
| S5 | Drawer | **v0.5.0** 🔨 | Contents, To-dos (All/Open/Done, cross-file toggle), Scratch (`.scratchpad.md`), background indexer — built and merged 2026-10-21, never installed (goes to the device with v0.8) |
| S6 | Find & wiki | **v0.6.0** 🔨 | Find bar (count, Prev/Next, Replace/All, Aa, seed from selection, Ctrl+F), wiki links open/create in preview, backlinks, Graph as a list — built and merged 2026-10-21, never installed (goes to the device with v0.8) |
| S7 | Help & hardening | **v0.7.0** 🔨 | Help (App info, Markdown reference), kill-during-save + unreadable-note guard, touch audit ≥44 px, rotation pass, perf logging — built and merged 2026-10-21, never installed (goes to the device with v0.8); Nickel restart not done |
| S8 | USB-OTG keyboard & export | **v0.8.0** 🔨 | host-mode switch, hot-plug evdev keyboard, US keymap, chords via the S2.5 shortcut code, USB flag, Help page, export docs — built and merged 2026-10-21, untested on the device, never installed (goes to the device with v0.8.1) |
| S8.1 | Mouse & touchpad | **v0.8.1** 🔨 | EV_REL pointers incl. the K400 Plus touchpad, e-ink pointer (Fast, ≥16 px, 150 ms throttle, 3 s hide), click = tap, drag-select, double-click word, wheel scroll — built and merged 2026-10-21, never installed (goes to the device with v1.0.0) |
| S9 | Release | **v1.0.0** 🔨 | release workflow (draft release on tag only), reproducible checked package, README / install / release-note and MobileRead drafts, v1.0 screenshots — built and merged 2026-10-21; device regression, tag, release and posts NOT done (need approval + passing device test) |
| S9.1 | Graph map | **v1.0.0** 🔨 | Graph drawn as nodes and links (deterministic cached layout in core), Map / List toggle, tap select / open in preview, drag and page-button pan, 4 zoom levels, mouse wheel/click/drag, 300-note perf — built 2026-10-21; not installed, not tagged |
| S9.2 | Title bar | **v1.0.0** 🔨 | icons ×2 (glyphs, targets, bar), mockup order, close X at the right, overflow rule, before/after + mockup comparison shots — built 2026-10-21; installed 2026-10-21 with 1.0.0, not tagged |
| S10 | Pointer, scroll, USB label (+S10.1 test-notes pack) | **v1.0.1** 🔨 | BUILT 2026-10-21 (approved 22:32); not installed, not tagged |
| S10.3 | Pointer lag fix + drawer in preview | **v1.0.3** 🔨 | BUILT 2026-10-22 (approved); not installed, not tagged |
| S11 | UI scale (default 200 %) | **v1.1** 📝 | PLANNED |
| S12 | Orientation (Auto default) | **v1.2** 📝 | PLANNED |
| S13 | Latin layouts + input pipeline | **v1.3** 📝 | PLANNED |
| S14 | UI languages EN/FR/PT | **v1.4** 📝 | PLANNED |
| S15 | Handwriting input (Latin) | **v1.5** 📝 | PLANNED, spike + gate |
| S16 | Farsi + Arabic (RTL) | **v1.6** 📝 | PLANNED |
| S17 | Japanese kana + kanji | **v1.7** 📝 | PLANNED |
| S18 | Simplified Chinese pinyin | **v1.8** 📝 | PLANNED |
