# Wi-Fi (the title-bar button)

Σscribe stops the stock reader (Nickel) when it starts, and with it the thing
that turned Wi-Fi on and picked a network. Settings → Sync and Settings →
Plugins → Check for plugins need a network, so the app has a **Wi-Fi button**
in the title bar, just left of Settings (after the front-light button, when
the reader has one). Tapping it opens a menu under it.

Design, internals, strings and the first-run checklist for the Forma are in [../dev/wifi-design.md](../dev/wifi-design.md).

## What the reader sees

- The button's icon is the Lucide `wifi-off` while the radio is off, `wifi-low`
  (a dot and one arc) while it is on and no network is joined, and `wifi`
  while one is. It is shaded while the menu is open. It shows in write mode and
  in preview.
- The menu, top to bottom:
  1. **Wi-Fi**, with the same on/off switch as Settings uses. Off stops the
     radio; on loads the driver, starts the supplicant, lets it rejoin a
     saved network, and scans.
  2. **The state**: the name of the network joined and "Connected · its IP
     address"; or "Turning on…", "Turning off…", "Connecting…"; or "Not
     connected" / "Off"; and, under the name, the last failure in words
     ("Could not join: wrong password? Forget it and try again").
  3. **The networks** found by the last scan, strongest first, the joined one
     washed and in bold. Each row has four signal bars, a lock when it needs a
     password, and, when it is saved, a **×** that forgets it. A network that
     is saved but out of range is listed (without bars) so it can be
     forgotten. At most eight rows show, fewer when the screen is short.
  4. **Scan again.**
- A tap on a row: the joined network does nothing; an open or a saved network
  joins with no further question; a network with a password that is not saved
  opens the password field; a network the app cannot join (WPA-Enterprise,
  WPA3-only, enhanced open) says "This kind of network is not supported here".
- **The password** is typed in the title bar, the same field rename uses, with
  the on-screen keyboard (or a USB keyboard). It shows stars; the **eye** at
  the field's right end shows what was typed. While it is empty the field says
  "Password: <network>". **OK** (or Return) joins, **Cancel** (or Escape)
  does not. Either way the menu comes back. From preview, the field leaves
  preview, as Find does. A password of the wrong length (WPA: 8 to 63
  characters, or 64 hex digits) is refused before anything is sent.
- A wrong password is not kept: the network the app added is removed again,
  nothing is saved, and the radio goes back to what it had. To change the
  password of a saved network, forget it and join it again.
- A tap anywhere else, Escape, or the button closes the menu and does nothing
  else (no caret move, no typing), as the drawer menu does. The two menus are
  never open together.

## Syncthing follows the radio

With Settings → Sync on, **Wi-Fi off stops Syncthing** and **Wi-Fi on lets it start again**. The
Settings switch itself is not touched, and a radio turned on does not start
what Settings turned off.

## Limits

- WPA-Enterprise, WPA3-only (SAE) and enhanced open (OWE) networks are listed
  with a lock and cannot be joined. Hidden networks (no name) are not listed
  and there is no "add a network by name".
- No captive-portal sign-in.
- No "forget" confirmation: it is one tap on the ×, and joining again needs the
  password again.
- Wi-Fi cannot be turned on from Settings; the button is only in write mode and
  preview.
- The radio is not turned off when the app exits: the launcher reboots into the
  stock reader, which starts its own.
