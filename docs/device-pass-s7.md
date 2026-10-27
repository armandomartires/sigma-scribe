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

## 7.4 Backlight colour: what does `tlc5947_bl` do?
The Forma lists two backlight devices, `mxc_msp430.0` (brightness, 0 to 100, confirmed) and `tlc5947_bl` (0 to 4096). Before a warm/cool slider can be built, someone has to see what the second one does. With your yes, from a shell on the Forma or over telnet/ssh:

    cat /sys/class/backlight/tlc5947_bl/brightness
    echo 2048 > /sys/class/backlight/tlc5947_bl/brightness   # watch the light
    echo 0 > /sys/class/backlight/tlc5947_bl/brightness      # put it back (or the value cat printed)

Note the result in `GOALS.md` (7.4): which end of the range is warm, and whether it needs `mxc_msp430.0` on to be visible.
