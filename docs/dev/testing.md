# Testing strategy

## Layers
1. **Unit (`cargo test --workspace`)** — core: buffer, wrap, keyboard, store/autosave, names; render: canvas, theme scale, fonts load/measure, layout fits 1440×1920, every key hittable, ≥50 px targets, tap-to-caret, scrolling; device: evdev decoding, touch transform, keys.
2. **Emulator end-to-end (`app/tests/emulator.rs`)** — types on the OSK, checks the `.md` file on disk, restart restores it, keystrokes use only `Fast` refreshes, power saves and stops.
3. **Emulator scripts (`tests/emu/*.script`)** — `sigma-emu --script FILE --shots DIR` (or `scripts/sim.sh --headless FILE DIR`). Steps: `tap X Y`, `key LABEL`, `type TEXT`, `hw TEXT` (hardware keyboard), `page fwd|back`, `shot NAME`, `expect TEXT`, `expect-status WORD`, and for v0.3 `open-fixture FILE` (copy a note into the library and open it at the top), `expect-mode write|preview`, `expect-page N`, `expect-pages-at-least N`; `tap-on Eye` / `tap-on Pencil` hit the preview toggle; v0.4 adds `tap-on Options`, `tap-on Row/Cell` inside Options (e.g. `Dark mode/On`, `Size/+`, `Line/-`), `tap-on Split|Full|Centre|Arrows|123|Pad left|Pad right` (landscape bar), `expect-setting KEY=VALUE` (`.sigma-settings` line), `expect-words N` (Line sample words per row), `expect-screen WxH` and `restart` (power off/on with the same library). Coordinates are screen coordinates (landscape 1920×1440); shots are written upright. `SIGMA_EINK=1` (or `scripts/sim.sh --eink --headless …`) applies 16 grey levels and ghosting to headless shots too. Writes 1440×1920 greyscale PNGs; `docs/screenshots/` is produced this way.
4. **Live simulator** — `scripts/sim.sh [--eink] [--height N]`: minifb/X11 window, panel scaled to fit; mouse = touch, PageUp/PageDown = page keys, real keyboard = USB keyboard, Esc = power. `--eink` simulates a black flash on full refreshes and ~8 % ghosting on fast/partial ones (greyscale only). Compare with `.ai/mockup/kobo-writing-mock.html`.
5. **CI** — fmt, clippy -D warnings, tests, ARMv7 static build with FBInk.
6. **On-device (Kobo Forma, fw 4.38.23697)** — `.ai/install-plan-forma.md` §5 checklist; later sprints extend it.
7. **Performance/robustness budgets** — keystroke-to-ink < 150 ms p95; 100 KB note opens < 1 s; atomic writes survive power-off during save.

## FBInk and desktop preview
FBInk has **no emulator or preview window**. Its README notes it "works perfectly fine on any kind of Linux framebuffer device" and offers `make linux` (generic `/dev/fb0`, e.g. a Linux text console, no e-ink refresh), plus `make ftrace` (a finger-trail utility for checking Kobo touch translation on the device). Third-party bridges listed there (fbink-xdamage for X11, a Qt5 QPA, InkVT) target devices, not desktops. Hence our own `Device` trait with a window and a headless backend; the FBInk backend is exercised only on the Kobo.

## Emulator commands added in v0.5
- `tap-on Panels` (the drawer button; v1.16), then `tap-on Contents|To-dos|Scratch` (rows of the open drawer menu; a tap anywhere else closes it); `pane Contents|To-dos|Scratch` does all three taps (open, tick, close). `press-on DrawerHandle` / `release` twice is a double tap on the grip (closes the drawer); `tap-on DrawerTab` reopens it from the edge tab. `tap-on Heading:TEXT`, `tap-on Todo:TEXT`, `tap-on TodoBox:TEXT`, `tap-on Filter`, `tap-on ScratchPane`.
- `expect-scratch TEXT`, `expect-file-contains PATH|TEXT`, `expect-line N`.
- The to-do indexer is settled after every step, so scripts are deterministic.

## Emulator commands added in v0.6
- `tap-on Find|Graph` (title-bar icons); Find bar: `tap-on FindField|ReplaceField|Prev|Next|Replace|All|Aa|FindClose`.
- `tap-on Link:LABEL` (a wiki link or backlink on the shown preview page); `tap-on Graph:PATH` (a Graph row), `tap-on Close` on the Graph sheet.
- `expect-find LABEL` (`Find`, `None`, `i of n`); `expect-mode` also knows `find` and `graph`.
- Graph map (v1.0): `tap-on Map|List|Zoom+|Zoom-|Open`, `tap-on Node:PATH` (a visible map node), `expect-graph map|list`, `expect-selected PATH|none`, `expect-zoom N`, `make-notes N` (N linked notes in `map/`, every tenth an orphan; opens `map/n000.md`), `perf-graph [MAX_MS]` (layout with an empty cache; prints `layout_ms`, `draw_ms`). The Graph opens on the map, so list scripts `tap-on List` first.
- Fixture `tests/emu/fixtures/lamps.md`: 50 matches of "lamp" and a missing link.

## v0.7 emulator commands
- `expect-mode help`, `expect-help TITLE` (e.g. `Help / App info`); `expect-page N` also checks Help pages.
- `perf-type N` — taps N keys and prints p50/p95/max handling time (use a release build; see `docs/dev/perf.md`).
- Script: `tests/emu/v0.7-help-hardening.script` (screenshots in `docs/screenshots/v0.7/`, `eink/` versions with `--eink`).

## v0.8 emulator commands
- `usb plug|unplug` — hot-plug events; `expect-usb on|off`.
- `usb-type TEXT` — text as raw evdev press/release codes through the USB keymap (US layout, Shift added as needed).
- `usb-key CHORD` — e.g. `ctrl+f`, `shift+left`, `ctrl+home`, `pgdn`, `esc` (pressed in order, released in reverse).
- Script: `tests/emu/v0.8-usb-keyboard.script` (screenshots in `docs/screenshots/v0.8/`, `eink/`).
- The live window (`scripts/sim.sh`) sends the same chords for Ctrl+letter, Shift/Ctrl+arrows, Home/End.

## v0.8.1 emulator commands (pointer)
- `pointer plug|unplug`; `pointer-to X Y`, `pointer-to-word W`, `pointer-before W`, `pointer-after W`, `pointer-to-control NAME`, `pointer-move DX DY` — all sent as raw `REL_X/REL_Y` through the evdev pointer decoder.
- `click`, `double-click`, `right-click`, `mouse-down`, `mouse-up`, `wheel N` (+ = up); `expect-pointer visible|hidden`; `wait MS` drives the throttle and the 3 s hide.
- Script: `tests/emu/v0.8.1-pointer.script` (screenshots in `docs/screenshots/v0.8.1/`, `eink/`).

## v1.16 sync (Settings → Sync)
- `crates/core/src/sync/*` unit tests: the JSON reader, the REST reply reader, the status model from the documented Syncthing answers, `.stignore`, conflict counting.
- `crates/core/tests/sync_supervisor.rs`: the supervisor against a fake REST API (a thread on the loopback with canned answers) and a fake `syncthing` (a shell script that sleeps or exits). No real Syncthing is run anywhere in the tests. Unix only.
- `app/tests/sync_settings.rs`: the page, the switch, Accept and Ignore; and the app → supervisor → fake API path. `app/tests/sync_notes.rs`: a note changed on disk is taken in, and never overwritten.
- Emulator steps `sync-demo STATE` (`off`, `missing`, `starting`, `failed`, `alone`, `pending-device`, `pending-folder`, `syncing`, `idle`, `offline`), `expect-sync STATUS` and `expect-sync-pending DEVICES,FOLDERS`; script `tests/emu/v1.16-sync.script` (screenshots in `docs/screenshots/v1.16/`).
- Not covered by any test: a real Syncthing, and the Forma. See `docs/dev/sync-design.md` §12.

## v1.17 Wi-Fi (title-bar button)
- `crates/device/src/wifi/wpa.rs`: the control-socket protocol (parsers on canned replies, and every join / scan / forget flow against a fake `wpa_supplicant` on a real Unix datagram socket). `crates/device/src/wifi/{mod,fake}.rs`: the service, the pretend radio. `crates/render/src/wifi_menu.rs`: the title bar and menu layout in both orientations at three UI scales. `app/tests/wifi.rs`: the journey through the app, with the pretend radio (and a fake `syncthing` for Syncthing following the radio).
- Emulator steps `wifi-demo [online]` and `expect-wifi off|on|joined:NAME|saved:NAME|unsaved:NAME|failure|no-failure`; controls `Wi-Fi`, `WifiSwitch`, `Rescan`, `Net:NAME`, `Forget:NAME`, `ShowPassword`; script `tests/emu/v1.17-wifi.script` (screenshots in `docs/screenshots/v1.17/`).
- Not covered by any test: `kobo.rs` (ARM only), `wifi.sh`, a real supplicant, the Forma. See `docs/dev/wifi-design.md` §6.

## v1.0 release checks
- `scripts/check-package.sh dist/sigma-scribe-<ver>-kobo` (run by `package-kobo.sh` and the release workflow).
- `app/tests/release.rs`; tour script `tests/emu/v1.0-tour.script` (screenshots `docs/screenshots/v1.0/`, `eink/`).
