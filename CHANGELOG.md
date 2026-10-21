# Changelog

All notable changes to Sigma Scribe. Versions are tagged after they pass on the Kobo Forma.

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
