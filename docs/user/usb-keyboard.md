# USB keyboard and mouse

Design, how it works, what is unverified on the device and the comparison with KOReader are in [../dev/usb-keyboard-design.md](../dev/usb-keyboard-design.md).

## Hardware
- Kobo Forma micro-USB port + OTG adapter + **powered** USB hub (or an OTG Y-cable with external power). The Forma does not supply VBUS.
- **Known to work:** a regular wired USB keyboard on the Forma's micro-USB port.
- Other keyboards, wireless receivers and touchpads may work but have not been checked on the device. How the keyboard is detected and what is still unverified are in the [design page](../dev/usb-keyboard-design.md#unverified-on-the-device).

## Export
Notes are plain `.md` files in `/mnt/onboard/sigma-scribe/notes` (shown as `sigma-scribe/notes` on the computer). Leave the app, connect over USB, tap Connect, copy the folder. To keep notes in step with other computers without cables, see [sync.md](sync.md).

## Mouse and touchpad (v0.8.1)

- Click = tap at the press point; drag from the note selects (anchor at press); double-click (≤ 400 ms, ≤ 8 px) selects the word; right/middle click and long-press: nothing new.
- Wheel: note 3 rows per notch (the caret follows, so the view scrolls); preview, Files, Graph, Help: one page per wheel event; Options: nothing.
- The pointer moves at a fixed speed, with no acceleration.
