# Logging

Settings → **Logging** turns on a log file that the app and its plugins write
to, and picks how much goes in it. The setting is global: one switch and one
level for everything.

## Levels

The levels are Python's `logging` levels, lowest to highest:

| Level | Number | What it is for |
|---|---|---|
| DEBUG | 10 | Detail: screens opening, each setting change, each layout step and timing |
| INFO | 20 | Milestones: start and exit, a plugin loaded, a layout built, USB and light |
| WARNING | 30 | Something odd that was handled: stray touch reports, a damaged cache entry |
| ERROR | 40 | Something failed: a plugin that did not load, a save that failed |
| CRITICAL | 50 | The app cannot go on: a panic, the device that would not open |

The file records the chosen level and everything above it. The Level row
appears once Logging is on. The default is INFO.

## The file

`.adds/sigma-scribe/log/sigma-scribe.log` on the Kobo. One line per entry:

    2026-10-25 11:15:19Z +12.345 WARNING  plugin/graph-map: layout: the saved picture is damaged
    └ UTC time           └ seconds since the app started    └ source: message

- **Time** is UTC. A Kobo whose clock is not set shows question marks
  (before 2020); the uptime next to it still orders the lines.
- **Source** is where the line came from: `app`, `ui`, `settings`, `usb`,
  `light`, `touch`, `perf`, `panic`, `files`, `wifi`, `plugin` (the app talking about a
  plugin), or `plugin/<id>` for a line the plugin wrote itself.
- `files` lines say what Files did, with paths and never note text, at INFO:
  `new note`, `new folder`, `rename a -> b`, `move a -> b`,
  `delete a -> .trash/<time>/…`, `restore a` (or `restore a as b (the name was
  taken)`) and `empty trash (n items)`. A failure is a WARNING with the
  reason (`files: delete a.md: …`). Opening the Trash is `ui: open trash` at
  DEBUG, like `ui: open files`.
- `wifi` lines say what the Wi-Fi button did: the radio turning on or off, joining and joining a network, scans, and failures as WARNINGs. Network names are logged; **never a password, a passphrase or an IP address.** See [wifi.md](wifi.md); the full list of lines is in [../dev/wifi-design.md](../dev/wifi-design.md#8-notes-moved-from-the-user-page).
- A multi-line message (a panic) stays one entry, its continuation lines
  indented.
- At **512 KB** the file becomes `sigma-scribe.log.1` (one backup is kept) and
  a new file starts, so the log cannot fill the Kobo's storage.
- Each time the file is opened a marker line says the level.

## What the launcher's log still gets

The launcher keeps its own `.adds/sigma-scribe/sigma-scribe.log`, which
collects everything on stderr (and the FBInk library's output). That does not
change:

- **Logging off:** nothing is written to the log file, and stderr gets INFO and
  above, which is what the app printed before there were levels (`usb:`,
  `light:`, `touch:`, `perf` lines read as before).
- **Logging on:** the file gets everything at or above the level; stderr only
  gets WARNING and above, so the launcher's log keeps the problems without the
  chatter.

A panic is written to the log file as CRITICAL (when logging is on) and, as
always, printed to stderr.

The log interface for plugin authors is in [../dev/plugin-api.md](../dev/plugin-api.md#for-plugin-authors-logging).
