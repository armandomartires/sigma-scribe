# Changelog

All notable changes to Sigma Scribe. Versions are tagged after they pass on the Kobo Forma.

## 1.1.0 — 2026-10-22 (built, not installed, not tagged)
- UI scale (sprint S11, decision 38): Options → **UI scale** 100 / 125 / 150 / 175 / **200 %** (default 200 %, saved as `ui_scale=`). It scales the title bar, icons, sheets (Files, Options, Help, Graph), drawer, Find bar, status bar and keyboard together; the note text **Size**, margins, preview and selection handles keep the device scale. The keyboard takes at most 42 % of the height in portrait and 45 % in landscape. Touch targets stay ≥ 44 px at the device scale at every step.
- Icons are now **Lucide** (ISC, decision 35): the SVGs are baked with resvg at build time into greyscale masks, one per UI-scale step (stroke 2.25 for e-ink), and drawn by one blit; Options keeps its 'Aa'. Credit in the README, Help → App info and `THIRD-PARTY.txt`.
- Emulator: `tests/emu/v1.1-ui-scale.script`; screenshots `docs/screenshots/v1.1/`.
- Device binary sha256 (CI build) `<pending>`; tarball `<pending>`.

## 1.0.4 — 2026-10-22 (built, not installed, not tagged)
- Pointer regression fixed (sprint S10.4): pointer moves no longer redraw the whole screen (1.0.3 log: ~170 ms CPU per move, events queued behind it); a pointer-only frame restores and redraws just the arrow. Idle hiding uses processing time, so a backlog no longer makes the arrow flicker on and off. The Graph selection pill is GL16 again (1.0.3 sent it as DU).
- Device binary sha256 (CI build) `681a6d1f800a996ecef3ce8beea33434269fb43316af6f95c57bc14ea1b46b2e`; tarball `7da86927b84fbf69e96580827c677cd967a0094e003482caabeefd5a49f9a86d`.

## 1.0.3 — 2026-10-22 (built, not installed, not tagged)
- Pointer lag fixed (sprint S10.3, decision S10.3-D2): the moving pointer now uses the fast DU waveform and is never waited for; one non-flashing GL16 cleanup over its path runs when it hides (3 s idle, typing, touching), clearing trails. The blocking `fbink_wait_for_complete` every 20th move (1.0.1/1.0.2) is gone, so pointer updates can no longer hold up typing. Regions stay ≥ 16×16. Device log: `perf pointer: du …` every 20th move, `perf pointer: gl16-cleanup …` per hide, and a pointer p50/p95 summary at exit.
- Drawer in preview (decision S10.3-D1, amends S5-D1): Contents, To-dos and Scratch stay open in preview; the page is laid out beside the drawer. Contents jumps to the preview page of a heading; a to-do checkbox toggles (atomic write + reindex) and re-renders the preview — the one edit allowed from preview; the Scratch pad stays editable while focused, the note stays read-only. Not shown over Files, Options, Graph, Help or rename.
- Emulator: `tests/emu/v1.0.3-drawer-preview.script`; screenshots `docs/screenshots/v1.0.3/`.
- Device binary sha256 (CI build) `339b62613cf13b51c7e140ac9c85887b5780677028b759799aa6d04178b49be0`; tarball `b6d4dd52bd56aa07ebfcb005f7c651b111a5ad3e1f261f72ae6f209f7d07a6d4`.

## 1.0.2 — 2026-10-21 (built, not installed, not tagged)
- Graph map look (sprint S10.2): links are plain lines (no arrowheads); notes are small circles (rounded rectangles, radius = height/2); the selected note grows into a pill with its full name, in one non-flashing GL16 region; the current note stays filled; the selection's neighbours keep a larger, thicker ring and ink links. Labels sit beside the dots: always for the current note and the neighbours, for every note only at zooms with room (decision S10.2-D1). Every node hits within ≥ 44 × 44 brief px.
- Back navigation: a session history (note, write/preview, caret, scroll, preview page; cap 50) is pushed when a note is opened from a wiki link or backlink in preview, the Graph (map Open / second tap, list rows), To-dos, Contents jumps and Files. A Back arrow appears at the far left of the title bar (before Files) only while the history is not empty; Esc (when nothing else is open) and Alt+Left on a USB keyboard also go back. The title bar reserves the slot, so it still fits in portrait and landscape.
- Emulator: `tests/emu/v1.0.2-map-back.script`, control `Back`.
- Device binary sha256 (CI build) `5e14620bfeb40ca937f7ff06f9b9bf08b2df4af0c556f2a49c41e9c668703376`; tarball `1b9c2192c6f0e6d858a8ba5803cb93080dc7e663e91091b26a302da2f098e22f`.

## 1.0.1 — 2026-10-21 (built, not installed, not tagged)
- Pointer ghost trails fixed (sprint S10): every pointer move is now ONE update over the union of the old and new arrow (≥ 16×16) on a new `Refresh::Pointer` class mapped to FBInk `WFM_GL16` (grey-accurate, no flash) instead of `WFM_AUTO` (DU/A2, which left residue on the grey paper). When the arrow hides (3 s idle, typing, touching) one non-flashing GC16 partial covers its whole path since the last cleanup. Throttle stays 150 ms. Device log: `perf pointer: gl16 WxH submit+complete N ms` every 20th update.
- Reverse scroll: Options → **Scroll: Traditional / Natural** (default Traditional = 1.0). Natural flips every wheel / two-finger notch: note, preview, sheets, Graph zoom. Saved as `scroll=natural|traditional`.
- Label: Options → **USB keyboard & mouse**; Help → **USB keyboard, mouse & export**; About text updated. Settings key `usb` unchanged.
- Test notes pack (S10.1): sources in `tests/test-notes/`, `scripts/make-test-notes.sh` builds `dist/test-notes/` (316 files, incl. generated `graph-300/`) and a reproducible tarball; emulator `copy-notes DIR` and `tests/emu/v1.0.1-test-notes.script`.
- Emulator e-ink model: small Fast updates to mid greys now keep 30 % of the old pixel (DU-like), so trails show in `SIGMA_EINK=1`.
- Device binary sha256 (CI build) `ffc2e119e569e4364e565b7779d4e650183bb6107bc46e4bc982356650306567`; tarball `3aa18e6b2d0514f9b446a3e520d05307d7632b0ac40ba0b6376db6b8137ff03c`.

## 1.0.0 — 2026-10-21 (release candidate: built, not installed, not tagged, not released)
- Release prep (sprint S9): reproducible tarball (fixed order, owner, mode, time; `gzip -n`), `scripts/check-package.sh` (only `.adds/`, no notes, required files, ARM static, hash list), `THIRD-PARTY.txt` in the package (AGPL, FBInk GPL-3.0+ at 886f25f1, OFL fonts, NickelMenu not included).
- `.github/workflows/release.yml`: builds and checks the package with the pinned FBInk on every PR and push to main (artifact = release candidate); on a `v*` tag pushed by the maintainer it checks tag = Cargo version and creates a **draft** GitHub Release. It never tags or publishes.
- README rewritten (features, screenshots, install/update/uninstall), `docs/install.md` for users, `docs/release/` (release notes draft, MobileRead draft, Forma developer notes draft, release checklist), v1.0 screenshot tour `tests/emu/v1.0-tour.script`.
- Graph map (sprint S9.1): the Graph sheet opens on a drawn map of notes (labelled boxes) and links (straight lines with arrowheads), with a Map / List toggle (the list stays). Deterministic layout in `sigma-core` (seeded force-directed, 160 iterations; orphans on an outer ring), cached until the links change. Opens fitted to the whole graph, centred on the current note (filled box). Tap a note to select it and emphasise its neighbours; tap it again, or **Open**, to open it in preview. Pan by dragging or with the page buttons (Partial), zoom − / + over 4 levels (Full). Mouse: wheel zooms, click selects, drag pans. Large graphs show dots with labels only where they fit (current, selected, neighbours); zoom in to read the rest. 300 notes: layout 22 ms on the desktop (~0.2 s estimated on the Forma, once per link change).
- Title bar (sprint S9.2): icons twice the size (glyphs and targets; 100 px portrait, 134 px landscape; bar 114 / 152 px) and in the mockup's order after the file name: Files, Preview, Find, Scratch, To-dos, Contents, Graph, Options, Help (was mirrored). Sheets' close X moved to the rightmost slot. Help is now drawn in preview too. Overflow rule for narrow panels: smaller gaps, then smaller icons (never below 44 px); not needed on the Forma.
- Emulator: `tap-on Map|List|Zoom+|Zoom-|Open|Node:PATH`, `expect-graph map|list`, `expect-selected PATH|none`, `expect-zoom N`, `make-notes N`, `perf-graph [MAX_MS]`; script `tests/emu/v1.0-graph-map.script`.
- Docs corrected: the keyboard that worked on this Forma before was a wired 60% keyboard (most likely with KOReader), not the K400 Plus; host-mode switch compared with KOReader's.
- Device binary sha256 (CI build, the release candidate, with S9.2) `bb38a083b0925ced9adc61690430d0f768509a1cf284a0c533cf5839e570dad1`; tarball `edba7f6fb63d0147f0464addb8249ecb1946da4514e0823ac9cf6575e07b8c58`; local build `6d4316335944ae6b96726b16187d76cf7a6dad86fc0dc99442acb1a44bd24f9f`. (Earlier candidates: S9 `f2424114…f093d4`, S9.1 `22d8faac…7e52`.)

## 0.8.1 — 2026-10-21 (built, not yet installed or tagged)
- USB mouse and touchpad (sprint S8.1), including the touchpad half of a combined device such as the Logitech K400 Plus on its Unifying receiver: detected by EV_REL + REL_X/REL_Y; one reader per device feeds both keymap and pointer decoder. Touch panel and buttons are never pointers.
- On-screen arrow (outlined, Paper and Night): Fast partial refreshes of the changed area only (≥ 16×16), at most one redraw per 150 ms, hidden after 3 s still and when typing or touching.
- Left click = tap everywhere; drag on the note selects (same selection and bar as touch); double-click selects a word; right/middle click do nothing.
- Wheel / two-finger scroll (when sent as REL_WHEEL): note moves 3 rows per notch; preview, Files, Graph and Help turn a page.
- Unplug safe (also mid-drag); Help → USB keyboard & export has a Mouse and touchpad section.
- Emulator: `pointer plug|unplug`, `pointer-to`, `pointer-to-word`, `pointer-before`, `pointer-after`, `pointer-to-control`, `pointer-move`, `click`, `double-click`, `right-click`, `mouse-down`, `mouse-up`, `wheel`, `expect-pointer`; script `tests/emu/v0.8.1-pointer.script`.
- Device binary sha256 `21103122de5602e77c954ffb6dc0c303d645bf5cf91e6ea1f2129f048579f562`.

## 0.8.0 — 2026-10-21 (built, not yet installed or tagged)
- USB keyboard (Options → USB keyboard On): switches the Forma's port to USB host mode while on (back to normal on Off or exit), finds keyboards in `/proc/bus/input/devices` with hot-plug (1 s poll), reads them through a US keymap with Shift, Caps Lock, Ctrl, Alt and auto-repeat.
- Ctrl / Shift / Alt combinations go through the on-screen keyboard's shortcut code: Ctrl+Z/Y/A/C/X/V/F, Shift+arrows, Ctrl+arrows, Home/End, Ctrl+Home/End, PgUp/PgDn; in the note, Scratch, Find and rename fields. The on-screen keyboard is unaffected.
- Status bar: `USB` when a keyboard is connected, `no USB` when the setting is on but none is found.
- Help → USB keyboard & export: powered-hub note, shortcuts, export (copy the notes folder over USB), troubleshooting.
- Unplugging is safe; any failure (no host mode, no keyboard) is logged and leaves touch and buttons working.
- Desktop window: Ctrl+letter, Shift/Ctrl+arrows, Home/End send the same chords.
- Emulator: `usb plug|unplug`, `usb-type`, `usb-key`, `expect-usb`; script `tests/emu/v0.8-usb-keyboard.script`.
- Device binary sha256 `b533680ed86d175b49821cdb1312e9dc279bc123c557d8146423edbef903be27`.

## 0.7.0 — 2026-10-21 (built, not yet installed or tagged)
- Help sheet (`?`, leftmost in the title bar, write and preview): App info (version, licence, author, notes folder, buttons, USB keyboard, credits) and a paginated Markdown reference rendered by the preview engine; "← Back to Help", Esc goes back, page buttons turn.
- Hardening: a note that cannot be read (e.g. invalid UTF-8) opens read-only and is never overwritten; atomic save verified by a kill-during-save test (10 SIGKILLs, file always whole).
- Touch-target audit: every control ≥ 44 px scaled in portrait and landscape; drawer heading rows and the To-dos Filter button enlarged.
- Rotation pass: orientation round trip keeps text, caret and valid refresh regions; landscape screenshots of every sheet.
- Performance: key-event latency logged on exit (`perf: n … p50 … p95 … max`) to the device log; emulator `perf-type N`; report in `docs/perf.md`.
- Emulator: `expect-mode help`, `expect-help`, `perf-type`; script `tests/emu/v0.7-help-hardening.script`.
- Device binary sha256 `b8d76998234d71b6cf5e8bb626dda66738365f14ab198b0e8cbdfba7fe2f7789`.

## 0.6.0 — 2026-10-21 (built, not yet installed or tagged)
- Find bar (magnifier in the title bar, or Ctrl+F): Find and Replace fields, count (`Find` / `None` / `i of n`), Prev, Next, Replace, All, Aa (match case, off by default). A one-line selection seeds the query; Esc or the X closes it; opening it from preview switches to write. Current note only; Replace All is one undo step.
- Wiki links in preview: tap a known `[[note]]` to open it (in preview); tap a missing one (dashed) to create it, empty, beside the linking note (or in the folder the link names) and open it in write mode.
- Backlinks at the bottom of the preview: the notes that link here, each tappable.
- Graph sheet (connected-nodes icon): a list of every note with its outgoing (→) and incoming (←) links; tap a row to open the note. The scratch pad is not a node.
- Title bar order now Files, Preview, Find, Scratch, To-dos, Contents, Graph, Options (brief §3; Help comes in S7).
- Emulator: `expect-find`, `expect-mode find|graph`, `tap-on Link:LABEL`, `tap-on Graph:PATH`, Find-bar controls; script `tests/emu/v0.6-find-wiki.script`.
- Device binary sha256 `a9c2922daceefbbeae9b870e11ceb12e04cf5e1bbd2030733f59d361cf8f734c`.

## 0.5.0 — 2026-10-21 (built, not yet installed or tagged)
- Side drawer in write mode: three title-bar icons (Contents, To-dos, Scratch), any combination open, stacked with equal heights; side from Options → Drawer.
- Contents: the open note's headings, indented by level; tap jumps the caret to that line.
- To-dos: every `- [ ]` / `- [x]` across all notes and the scratch pad, filter All / Open / Done; the box toggles (only that line is rewritten on disk), the text opens the note at that line.
- Scratch: a side buffer typed with the keyboard, saved to the hidden `notes/.scratchpad.md` (not in Files, cannot be deleted); kept across note switches and restarts.
- Background indexer for to-dos across files; refresh regions stay ≥ 16×16.
- Emulator: `expect-scratch`, `expect-file-contains`, `expect-line`; script `tests/emu/v0.5-drawer.script`.
- Device binary sha256 `1ada5ae93504d287bb685e15e9cb3a15288cb65073dc233b220cd4293ec01f8f`.

## 0.4.0 — 2026-10-21
- Options sheet (`Aa` in the title bar): Paper, Type, Line, Size, Leading, Margin, Tab key, Tab width, Keyboard, Keyboard tone, Drawer, USB keyboard, Orientation.
- Night mode: the same page inverted, full refresh on switch; keyboard tone Theme / Light / Dark.
- Line mode: 8–16 words per line, fitted with the real face; Size and Margin move each other.
- Landscape (Options → Orientation): locked portrait-width column with gutters when Line is on; keyboards Centre / Split / Full with a number or arrow pad on either side.
- Every setting persists in `notes/.sigma-settings` (v0.2.1 files still load).
- Tab key follows Options (spaces 2/4/8 or a tab).
- Emulator: `--eink` also for headless runs; new script `tests/emu/v0.4-options.script`.
- Device binary sha256 `689b84c4e763119cbdfa19eb07ade52d5709cdf034bf971fc2e5116e97c02043`.

## 0.3.0 — 2026-10-21
- Markdown preview: eye / pencil toggle, read-only, paged with the page-turn buttons.

## 0.2.3 — 2026-10-21
- Touch selection (long-press, handles, action bar, double-tap); refresh regions ≥ 16×16 px and non-fatal refresh failures. (0.2.2 skipped.)

## 0.2.1 — 2026-10-21
- Full Windows-style on-screen keyboard (default), sticky modifiers, Ctrl shortcuts, undo/redo.

## 0.2.0 — 2026-10-20
- Library: Files sheet, folders, new / rename / move / delete, last-opened note.

## 0.1.0 — 2026-10-20
- Editor with on-screen keyboard and autosave to plain `.md`.
