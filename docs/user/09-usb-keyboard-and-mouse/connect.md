---
title: Connect
summary: Connect a USB keyboard through a powered hub and turn the USB keyboard on.
order: 1
---

# Connect

Design, how it works, what is unverified on the device and the comparison with KOReader are in [../dev/usb-keyboard-design.md](../../dev/usb-keyboard-design.md).

## Hardware
- Kobo Forma micro-USB port + OTG adapter + **powered** USB hub (or an OTG Y-cable with external power). The Forma does not supply VBUS.
- **Known to work:** a regular wired USB keyboard on the Forma's micro-USB port.
- Other keyboards, wireless receivers and touchpads may work but have not been checked on the device. How the keyboard is detected and what is still unverified are in the [design page](../../dev/usb-keyboard-design.md#unverified-on-the-device).

Settings → **USB & devices** → **USB keyboard & mouse** On, then a keyboard / mouse through a **powered** hub or a powered OTG cable (the Kobo does not power its port). While it is on, a computer cannot see the Kobo; leave the app first. The keys follow the keyboard Layout you chose in Settings. See Help → USB keyboard, mouse & export.
