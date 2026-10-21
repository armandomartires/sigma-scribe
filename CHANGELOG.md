# Changelog

All notable changes to Sigma Scribe. Versions are tagged after they pass on the Kobo Forma.

## 1.0.0 — 2026-10-21 (release candidate: built, not installed, not tagged, not released)
- Release prep (sprint S9): reproducible tarball (fixed order, owner, mode, time; `gzip -n`), `scripts/check-package.sh` (only `.adds/`, no notes, required files, ARM static, hash list), `THIRD-PARTY.txt` in the package (AGPL, FBInk GPL-3.0+ at 886f25f1, OFL fonts, NickelMenu not included).
- `.github/workflows/release.yml`: builds and checks the package with the pinned FBInk on every PR and push to main (artifact = release candidate); on a `v*` tag pushed by the maintainer it checks tag = Cargo version and creates a **draft** GitHub Release. It never tags or publishes.
- README rewritten (features, screenshots, install/update/uninstall), `docs/install.md` for users, `docs/release/` (release notes draft, MobileRead draft, Forma developer notes draft, release checklist), v1.0 screenshot tour `tests/emu/v1.0-tour.script`.
- Graph map (sprint S9.1): the Graph sheet opens on a drawn map of notes (labelled boxes) and links (straight lines with arrowheads), with a Map / List toggle (the list stays). Deterministic layout in `sigma-core` (seeded force-directed, 160 iterations; orphans on an outer ring), cached until the links change. Opens fitted to the whole graph, centred on the current note (filled box). Tap a note to select it and emphasise its neighbours; tap it again, or **Open**, to open it in preview. Pan by dragging or with the page buttons (Partial), zoom − / + over 4 levels (Full). Mouse: wheel zooms, click selects, drag pans. Large graphs show dots with labels only where they fit (current, selected, neighbours); zoom in to read the rest. 300 notes: layout 22 ms on the desktop (~0.2 s estimated on the Forma, once per link change).
- Title bar (sprint S9.2): icons twice the size (glyphs and targets; 100 px portrait, 134 px landscape; bar 114 / 152 px) and in the mockup's order after the file name: Files, Preview, Find, Scratch, To-dos, Contents, Graph, Options, Help (was mirrored). Sheets' close X moved to the rightmost slot. Help is now drawn in preview too. Overflow rule for narrow panels: smaller gaps, then smaller icons (never below 44 px); not needed on the Forma.
- Emulator: `tap-on Map|List|Zoom+|Zoom-|Open|Node:PATH`, `expect-graph map|list`, `expect-selected PATH|none`, `expect-zoom N`, `make-notes N`, `perf-graph [MAX_MS]`; script `tests/emu/v1.0-graph-map.script`.
- Docs corrected: the keyboard that worked on this Forma before was a wired 60% keyboard (most likely with KOReader), not the K400 Plus; host-mode switch compared with KOReader's.
- Device binary sha256 (CI build, the release candidate, with S9.2) `@@BIN@@`; tarball `@@TAR@@`; local build `6d4316335944ae6b96726b16187d76cf7a6dad86fc0dc99442acb1a44bd24f9f`. (Earlier candidates: S9 `f2424114…f093d4`, S9.1 `22d8faac…7e52`.)

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
