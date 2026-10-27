# Σscribe — Markdown notebook for the Kobo Forma

<!--
Each "## " heading is a sprint. Optional lines under it: Goal:, Dates:, Status:.
Each top-level list item is a story; its checkbox is its status:
  plain item = Backlog, [ ] To do, [>] In progress, [?] In review, [!] Blocked, [x] Done
Indented lines under a story are its details, shown when you hover its dot.
-->

Status as of 2026-10-27: v1.15.0 is tagged and CI builds the release. Everything from v0.1 to v0.4 and v1.0.0 to v1.0.2 passed on the Forma. v1.0.3 to v1.4.6 also passed on the Forma. v1.5 to v1.15 (handwriting, languages, plugins) are accepted as complete; 524 tests pass on the desktop and in the emulator. Sources: `plan.md`, `sprints/`, `CHANGELOG.md`, git tags.

## Sprint 1 — Editor, library, preview, options (v0.1 – v0.4)
Goal: A usable notebook on the Forma: write, organise, preview, configure
Status: Done, passed on the Forma and tagged v0.1.0 to v0.4.0
- [x] S0 Foundations and Forma bring-up (platform decision, Plan B)
- [x] S1 Editor core: buffer, wrapping, status bar, on-screen keyboard, autosave
- [x] S2 Library: Files sheet, folders, rename in place, atomic writes, last note restored
- [x] S2.5 Full keyboard: Esc and number row, Shift/Ctrl/Alt, undo and redo, region-only refresh
- [x] S2.6 Touch selection: long-press word select, handles, caret and Paste
- [x] S3 Markdown preview: rendered note, wiki links shown, paging
- [x] S4 Options and Line mode: settings page, night mode, landscape keyboards, 8 to 16 words per line

## Sprint 2 — Drawer, find, links, help, USB, release (v0.5 – v1.0)
Goal: Navigate a large library and ship the first release
Status: Done; tagged v1.0.0, v1.0.1 and v1.0.2
- [x] S5 Side drawer: Contents, To-dos, Scratch
- [x] S6 Find and replace, wiki links, backlinks, Graph list
- [x] S7 Help sheet and hardening: unreadable notes are never overwritten
- [x] S8 USB-OTG keyboard and export, S8.1 mouse and touchpad
- [x] S9 Release packaging, S9.1 Graph map, S9.2 title bar
- [x] S10 v1.0.1 pointer trails, reverse scroll, USB label
- [x] S10.2 v1.0.2 map look and Back navigation
  Built 2026-10-21.

## Sprint 3 — Display, input and languages (v1.0.3 – v1.4)
Goal: The app fits any scale, orientation and Latin layout, in nine UI languages
Status: Done, passed on the Forma (confirmed 2026-10-27)
- [x] S10.3 and S10.4 pointer lag fix and regression fix (v1.0.3, v1.0.4)
- [x] S11 UI scale (v1.1)
- [x] S12 Orientation: four rotations plus Auto from the accelerometer (v1.2)
- [x] S13 Latin keyboard layouts for on-screen and USB (v1.3)
- [x] S14 Interface languages, Settings redesign, keyboard size slider (v1.4 to v1.4.6)

## Sprint 4 — Handwriting (v1.5 – v1.15)
Goal: Write with a finger or stylus and have it recognised, offline
Status: Done (accepted 2026-10-27)
- [x] S15.0 and S15.1 Ink panel, ink in preview and rename
- [x] S15.2 Letter boxes with candidates, enrolment and Train mode
- [x] S15.3 Word mode and free-area writing
  A second network for word pieces brought word error to 7.4 to 9.5 % after enrolment. The 15 % gate now passes on the synthetic fonts.
- [x] S15.4 Gestures (scratch-out, caret tap)
- [x] S15.5 Language override
- [x] Learn from corrections, with a Settings switch (1.15)
- [x] Check handwriting accuracy and speed on the Forma
  Accepted as complete 2026-10-27.

## Sprint 5 — Language coverage (v1.6 – v1.11)
Goal: Layouts and shaping for the languages Kobo supports
Status: Done (accepted 2026-10-27); only the native-speaker review is left, to come later if ever
- [x] S16 Latin layouts, Cyrillic and Greek
- [x] S17 Vietnamese and Korean composition
- [x] S18 Right-to-left: Arabic, Farsi, Urdu, Hebrew
- [x] S19 Indic scripts and Thai
- [x] S20 Japanese IME, phrases
- [x] S21 Chinese pinyin
- [ ] Native review of the machine-drafted UI strings and layouts
  Not blocking: deferred, may never happen. The Asian and right-to-left layouts stay behind the Experimental switch.

## Sprint 6 — Plugins and platform (v1.8 – v1.15)
Goal: Graph and drawer panes as WASM plugins, plus a maintainable codebase
Status: In progress; 6.1–6.6 done (bundled in 1.15.0), cleanups and the backlight check open
- [x] Plugin API 4: Graph and drawer-pane plugins, Settings → Plugins, plugin settings pages
- [x] Logging (Settings → Logging) with levels, rotation and plugin log imports
- [x] Bundle `graph-map`, `contents-pane`, `todo-pane` and `scratch-pane` in the package (1.15.0)
- [x] New font stack: harfrust, skrifa and zeno replace rustybuzz and fontdue
- [x] Document every public item in every library crate; `cargo deny` in CI; MSRV 1.88
- [x] Split `lib.rs` by feature
  Branch `split-lib-by-feature`, commit 6a74c46: state moved beside its feature module, `lib.rs` from 781 to 425 lines, no behaviour change. fmt, clippy, doc and the full test suite pass. Not merged yet.
- [>] Open a pull request for `split-lib-by-feature`
- [ ] Replace `use super::*` with explicit imports, then consider moving `App` out of `lib.rs`
- [ ] Split `ink.rs` (933 lines) into state, gestures, recognition and training
- [ ] Backlight: confirm which sysfs device is the white light on the Forma
  Taken from KOReader's tables, not measured. The first run's `light:` log lines settle it.

## Sprint 7 — Device verification and next
Goal: One pass on the Forma over everything built since v1.0.2
Status: To do
- [ ] Install 1.15.0 on the Forma and run `docs/device-checklist-prerelease.md`
  Follow the install routine: keep plugin on/off state, settings and notes, and read the device logs.
- [ ] Verify the USB K400 Plus receiver and touchpad, including two-finger scroll
- [ ] Measure preview layout time on a large note on the device
- Draggable handle on the drawer edge
- Install plugins from the plugins repository (copied by hand today)
- Syncthing export
- Trash folder for deleted notes (deletes are permanent today)
