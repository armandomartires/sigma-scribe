# Σscribe — Markdown notebook for the Kobo Forma

<!--
Each "## " heading is a sprint. Optional lines under it: Goal:, Dates:, Status:.
Each top-level list item is a story; its checkbox is its status:
  plain item = Backlog, [ ] To do, [>] In progress, [?] In review, [!] Blocked, [x] Done
Indented lines under a story are its details, shown when you hover its dot.
-->

Status as of 2026-10-27: v1.15.0 is tagged and CI builds the release. Everything from v0.1 to v0.4 and v1.0.0 to v1.0.2 passed on the Forma. v1.0.3 to v1.4.6 also passed on the Forma. v1.5 to v1.15 (handwriting, languages, plugins) are accepted as complete; 524 tests pass on the desktop and in the emulator. Sources: `.ai/plan.md`, `.ai/sprints/`, `CHANGELOG.md`, git tags.

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
- [x] Native review of the machine-drafted UI strings and layouts
  Closed for now (2026-10-27, the user's call): no native review planned. The Asian and right-to-left layouts stay behind the Experimental switch.

## Sprint 6 — Plugins and platform (v1.8 – v1.15)
Goal: Graph and drawer panes as WASM plugins, plus a maintainable codebase
Status: Done; PRs #76, #77 and #78 merged 2026-10-27
- [x] Plugin API 4: Graph and drawer-pane plugins, Settings → Plugins, plugin settings pages
- [x] Logging (Settings → Logging) with levels, rotation and plugin log imports
- [x] Bundle `graph-map`, `contents-pane`, `todo-pane` and `scratch-pane` in the package (1.15.0)
- [x] New font stack: harfrust, skrifa and zeno replace rustybuzz and fontdue
- [x] Document every public item in every library crate; `cargo deny` in CI; MSRV 1.88
- [x] Split `lib.rs` by feature
  Branch `split-lib-by-feature`, commit 6a74c46: state moved beside its feature module, `lib.rs` from 781 to 425 lines, no behaviour change. fmt, clippy, doc and the full test suite pass. Merged (PR #76).
- [x] Open a pull request for `split-lib-by-feature`
- [x] Replace `use super::*` with explicit imports, then consider moving `App` out of `lib.rs`
- [x] Split `ink.rs` (933 lines) into state, gestures, recognition and training
- [x] Backlight: confirm which sysfs device is the white light on the Forma
  Taken from KOReader's tables, not measured. The first run's `light:` log lines settle it.

## Sprint 7 — Device verification and next
Goal: One pass on the Forma over everything built since v1.0.2
Status: Everything that can be done without the Forma is built and merged to `main` (PRs #80 to #85, #89); and the drawer button (PR #90). `main` (a247885, 1.15.0) was installed on the Forma on 2026-10-27; the device pass is the user's, then the logs are read. `main` 5b976fe (Warmth, Trash, Sync; still labelled 1.15.0) installed on the Forma 2026-10-28 01:35 with the perf notes (backup kept, 39 files checked, notes and plugin state unchanged). Then `main` b587310 (PRs #94 loopback for Sync, #95 light probe in the log, #96 Files More fix and Files log lines) installed 2026-10-28 02:27, same checks. Then `main` b2e46e8 (PR #97, Sync device ID as a QR code) installed 2026-10-28 08:14, same checks. Then `main` 14649bf (PR #98, root certificates for Syncthing) installed 2026-10-28 18:50, same checks (40 files). Then `main` a6dac4e (PR #99, Accept shares the notes folder) installed 2026-10-28 19:32, same checks. Then `main` f3b7b21 (PR #100, share with devices accepted earlier) installed 2026-10-28 20:11, same checks. Then `main` a9fb5f5 (PR #101, "Waiting for <device> to accept") installed 2026-10-28 20:49, same checks. Then `main` bade18e (PRs #102 Last sync, #103 light button, #104 Wi-Fi button) installed 2026-10-28 22:42, same checks (41 files). Then `main` 764ee5e (PR #105, light read before the first frame) installed 2026-10-28 22:56, same checks. Runbook: `.ai/device-pass-s7.md`.
- [x] Install 1.15.0 on the Forma and run `docs/device-checklist-prerelease.md`
  `main` is installed on the Forma (2026-10-27, backup kept, package checked). Closed by the user 2026-10-28: the tests that matter were done on the device during the sprint; the checklist is not run.
- [x] Verify the USB K400 Plus receiver and touchpad, including two-finger scroll
  Done by the user several times on the Forma: the K400 types and the touchpad is acceptable for an e-ink device (2026-10-28).
- [x] Measure preview layout time on a large note on the device
  Done 2026-10-28 on the Forma (5b976fe): 100 KB in 929 ms, 500 KB in 1,905 ms (desktop 60 / 337 ms); the first preview carries warm-up. Table in `docs/perf.md`.
- [?] Backlight colour slider (warm to cool)
  Merged (PR #91, 2026-10-28): Display → Warmth, 11 stops, saved and applied at start. KOReader (`device.lua` KoboFrost) drives the Forma's warmth through `tlc5947_bl/color`, 0 to 10, inverted (10 cold, 0 warm), never read back. On the Forma 2026-10-28 (5b976fe): the mixer is there, all 11 steps write Ok in both directions, the user tried it and left it at 5. The start-up probe lines are now logged (PR #95; seen 2026-10-28: mixer `tlc5947_bl/color`, warmth 5 applied at start). Was blocked on one device check: what `tlc5947_bl` (0 to 4096) does when written. Probe in `.ai/device-pass-s7.md`. Then: a second slider under Display → Backlight, a `Device` call for it, one string in the nine catalogs, tests.
- [x] Draggable handle on the drawer edge
  Merged (PR #82): drag the grip, the nearest Drawer width stop applies on lift and is saved. Tests in `app/tests/drawer_handle.rs`. Used on the Forma 2026-10-27 (width changes in the log): OK.
- [x] Install plugins from the plugins repository (copied by hand today)
  Merged (PR #84 and plugins repo #1, #2; Remove is #85): Settings → Plugins → Repository → Check for plugins, Install / Update, Remove (off plugins only). Packages are checked against the index's size and sha256 and installed atomically. The index is live at `https://armandomartires.github.io/sigma-scribe-plugins/index.json` (GitHub Pages workflow, 4 plugins). On the Forma (2026-10-27) Remove worked, Check for plugins failed: the Kobo has no `curl`, and the launcher stopped `dhcpcd`. Branch `plugin-fetch-on-device`: in-app https (ureq + rustls), the launcher keeps Wi-Fi and logs `net:` lines; tried on the Forma 2026-10-27 18:48: Wi-Fi kept (eth0 192.168.99.244, dhcpcd running), index fetched (4 plugins), scratch-pane installed from the repository and loaded in 72 ms. Merged (PR #89). The index is not signed.
- [x] One drawer button with a menu to show or hide each pane
  Branch `drawer-menu`: one title-bar button (Lucide panel on the drawer's side) replaces the Contents, To-dos and Scratch icons; a tap opens a menu with a checkbox per pane, rows toggle panes and the menu stays open, a tap elsewhere or Escape closes it and does nothing else. Write mode and preview. Tests in `app/tests/drawer_menu.rs`, screenshots in `docs/screenshots/v1.16/`. Same branch, asked for after: a double tap on the grip or a drag to the screen edge closes the drawer and a tab left of the scroll bar reopens the same panes (tests in `app/tests/drawer_handle.rs`); the scroll bar keeps its strip and a drawer on the right ends left of it; the grip is outlined, grey, with three dots. Tried on the Forma 2026-10-27 (06618b8): all OK. Merged (PR #90).
- [>] Syncthing export
  Decided 2026-10-28: Syncthing runs on the Kobo. Bundle an ARMv7 binary, Σscribe starts it while open (Wi-Fi kept), it syncs the notes folder, Settings shows device ID and status. Merged (PR #93, 2026-10-28): bundled Syncthing v2.1.7 (pinned, sha256), Settings → Sync with device ID, status, Accept/Ignore pairing, a guard so a save never overwrites a note Syncthing delivered; design in `docs/sync.md`. On the Forma it first failed (loopback down); fixed by PR #94: on b587310 the launcher brings `lo` up, Syncthing starts and reports no devices (log 2026-10-28). Its relay lookup fails TLS ("certificate signed by unknown authority": no CA roots for Syncthing on the Kobo), so only the local network is expected to work. Fixed by PR #98 (merged and installed 2026-10-28 18:50, not yet run): the package carries Mozilla's CA bundle and Syncthing gets it as `SSL_CERT_FILE`. Device ID as a QR code merged (PR #97), installed 08:14. Tried 2026-10-28: the phone scanned the QR code and paired (LAN, relays joined, no x509 lines: #98 confirmed), but no folder was shared, because the Kobo only accepted offers, and the page said Up to date. PR #99 (merged, installed 19:32): Accept now shares a Kobo-made "Σscribe notes" folder, and the page says "Not shared yet". A device accepted before #99 (the phone) is covered by PR #100 (merged, installed 20:11): at start the Kobo shares the folder with devices already accepted. Tried 2026-10-28 20:13: the Kobo made "Σscribe notes" (4ervi-bwlw6), shared it with the phone and connected over a relay; the offer was on the phone (as from "kobo", Web GUI / notification) and the user accepted it. Kobo to phone confirmed 2026-10-28 (the user saw the notes on the phone; "Waiting for AM-S23 to accept" then Up to date in the log); phone to Kobo not yet tried. "Last sync" says Never, because it counts only received files; fix in progress. PR #101 (merged, installed 20:49): the page says "Waiting for <device> to accept" while a connected device has not accepted the folder.
- [?] Trash folder for deleted notes (deletes are permanent today)
  Merged (PR #80), installed on the Forma: deletes move to `.trash/<time>/<path>`, not listed or indexed, copy back over USB. The user saw no trash in the app (2026-10-28), so a Trash screen in Files is merged (PR #92, 2026-10-28): list with deletion time, Restore to the old path (free name on a clash), Empty trash (two taps). Tried on the Forma 2026-10-28: delete, restore and empty all work. Follow-ups merged (PR #96): Files' More menu now shows on the last visible row, and Trash actions are logged (`files: delete / restore / empty trash` lines seen on the Forma 2026-10-28). The More fix is not yet confirmed on the device.
- [?] Light button in the title bar (brightness and warmth)
  Merged (PR #103, 2026-10-28), installed 22:42: a sun icon after Find opens Backlight and Warmth sliders (same changes as Settings → Display). Found while merging #104: the icon is missing until the first redraw after start (the light is read after the first frame); fixed by PR #105 (merged, installed 22:56). Not tried on the Forma.
- [?] Wi-Fi button in the title bar (switch, networks, password, forget)
  Merged (PR #104, 2026-10-28), installed 22:42: Wi-Fi switch, network and address, scan list, password in the title-bar field, forget; Syncthing follows the radio. Talks to wpa_supplicant's socket; `wifi.sh` loads and unloads the module after KOReader. Title bar fits with both new buttons at 100 to 200 %, portrait and landscape (tests, emulator). Not tried on the Forma: module reload, the `-D wext` driver flag, Nickel's Wi-Fi after leaving with Wi-Fi off (`docs/wifi.md` §6).
