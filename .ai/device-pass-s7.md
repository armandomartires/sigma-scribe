# Sprint 7 device pass

What is left of Sprint 7 needs the Forma in hand. Everything else in the sprint is built and tested on the desktop. Follow the install routine for each install (backup first, keep `plugins.state` and the notes, show the install log before unmounting).

## 7.1 Install the build and run the checklist
1. Build and install the current `main` (`scripts/package-kobo.sh`, then the install steps in `docs/install.md`). It carries this sprint's trash folder, drawer grip and preview timing log.
2. Run `docs/device-checklist-prerelease.md`; tick what passes and write down what does not. The open boxes there are the ones nobody has done on the device yet.
3. New in this sprint, to try as part of the pass:
   - [ ] Files → Delete (two taps) on a note: it disappears from the list, and `sigma-scribe/notes/.trash/<number>/<path>` holds it (check over USB). The last note still refuses to go.
   - [ ] Open the drawer (Contents), then drag the small grip on its inner edge across the page: the width follows to the nearest of the five stops when you lift, and it is still that width after a restart.
   - [ ] A tap on the page or on a pane near the grip, but not on it, still does what it did.

## 7.2 USB receiver and touchpad
With the Logitech K400 Plus receiver on the USB adapter (powered hub if the Forma does not power it):
- [ ] The USB flag shows and the keys type.
- [ ] The touchpad moves the arrow, a tap clicks, a double tap selects a word.
- [ ] Two fingers on the touchpad scroll the note, preview and Files; try Settings → Reverse scroll.
- [ ] If two-finger scroll does nothing, Settings → Logging at Debug level and send the `usb:` lines: it probably arrives as something other than wheel events.

## 7.3 Preview layout time
1. Settings → Logging on, Level Debug.
2. Open a large note (copy a 100 KB and a 500 KB Markdown file into `sigma-scribe/notes/` over USB), tap the eye.
3. Read `.adds/sigma-scribe/log/sigma-scribe.log`: `preview layout of <note> (<bytes> bytes): <n> ms`.
4. Put the numbers in `docs/perf.md` next to the desktop table (about 60 ms per 100 KB there; the Forma is expected to be 8 to 10 times slower, so about 0.5 s per 100 KB).

## 7.4 Backlight colour: Display → Warmth
KOReader's Forma ("Frost") entry settles what the second backlight device is for (`frontend/device/kobo/device.lua`, `KoboFrost`): the natural-light mixer is `/sys/class/backlight/tlc5947_bl/color`, range 0 to 10, **inverted** (10 is cold, 0 is warm), written independently of the white light on `mxc_msp430.0` (`frontend/device/sysfs_light.lua`, `setNaturalBrightness`). The slider is built on that; there is no shell on the Forma, so the slider is the device check.

With the build installed (not before your yes), Settings → Display → Light:
1. The Warmth row is there under Backlight, with Cool on the left and Warm on the right. If it is missing, the log has `light: no warmth mixer found` and the mixer file is not where KOReader says.
2. With the light on (Backlight about 50 %), move Warmth from Cool to Warm. **Cool should look white, Warm should look orange**, with the change visible by the middle. If it is the other way round, the inversion is wrong: report it, do not turn it round by hand.
3. Leave Settings, quit, start again: the light keeps its colour, and the slider shows the same stop (the step is saved and written at start).
4. Turn Backlight to Off, move Warmth, turn Backlight back up: the colour should follow (KOReader writes the mixer whatever the white light does).
5. In `sigma-scribe.log`: `light: warmth mixer /sys/class/backlight/tlc5947_bl/color (step …)` at start, just after the `logging on` line (`Some(n)` if the file reads back, `None` if not; Logging must be on before the start, or these lines are not written), one `light: warmth -> step n: Ok(())` per stop and no repeats while dragging.

Note in `.ai/GOALS.md` (7.4): which end is orange, and whether the file reads back (`step Some(..)` in the start line).
