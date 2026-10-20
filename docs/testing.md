# Testing strategy

## Layers
1. **Unit (`cargo test --workspace`)** — core: buffer, wrap, keyboard, store/autosave, names; render: canvas, theme scale, fonts load/measure, layout fits 1440×1920, every key hittable, ≥50 px targets, tap-to-caret, scrolling; device: evdev decoding, touch transform, keys.
2. **Emulator end-to-end (`app/tests/emulator.rs`)** — types on the OSK, checks the `.md` file on disk, restart restores it, keystrokes use only `Fast` refreshes, power saves and stops.
3. **Emulator scripts (`tests/emu/*.script`)** — `sigma-emu --script FILE --shots DIR` (or `scripts/sim.sh --headless FILE DIR`). Steps: `tap X Y`, `key LABEL`, `type TEXT`, `hw TEXT` (hardware keyboard), `page fwd|back`, `shot NAME`, `expect TEXT`, `expect-status WORD`. Writes 1440×1920 greyscale PNGs; `docs/screenshots/` is produced this way.
4. **Live simulator** — `scripts/sim.sh [--eink] [--height N]`: minifb/X11 window, panel scaled to fit; mouse = touch, PageUp/PageDown = page keys, real keyboard = USB keyboard, Esc = power. `--eink` simulates a black flash on full refreshes and ~8 % ghosting on fast/partial ones (greyscale only). Compare with `mockup/kobo-writing-mock.html`.
5. **CI** — fmt, clippy -D warnings, tests, ARMv7 static build with FBInk.
6. **On-device (Kobo Forma, fw 4.38.23697)** — `docs/install-plan-forma.md` §5 checklist; later sprints extend it.
7. **Performance/robustness budgets** — keystroke-to-ink < 150 ms p95; 100 KB note opens < 1 s; atomic writes survive power-off during save.

## FBInk and desktop preview
FBInk has **no emulator or preview window**. Its README notes it "works perfectly fine on any kind of Linux framebuffer device" and offers `make linux` (generic `/dev/fb0`, e.g. a Linux text console, no e-ink refresh), plus `make ftrace` (a finger-trail utility for checking Kobo touch translation on the device). Third-party bridges listed there (fbink-xdamage for X11, a Qt5 QPA, InkVT) target devices, not desktops. Hence our own `Device` trait with a window and a headless backend; the FBInk backend is exercised only on the Kobo.
