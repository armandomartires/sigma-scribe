# Changelog

All notable changes to Sigma Scribe. Versions are tagged after they pass on the Kobo Forma.

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
