# Plugin API and host (plugin authors, developers)

Switching plugins on and off, their settings and installing from the repository are in [../user/plugins.md](../user/plugins.md).

## Kinds

- **Data packs** (font, dictionary, hwr-model, word-list, theme, translation,
  layout, icons): files the app already knows how to read, found at the same
  relative paths as the loose files (`fonts/cjk.ttf`, `dict/skk-jisyo.txt`).
- **Screen plugins** (`kind=screen`, `entry=<module>.wasm`): a WebAssembly
  module that owns a full-screen body. Today: `graph-map`, the Graph screen.
- **Pane plugins** (`kind=pane`, `slot=contents|todos|scratch`,
  `entry=<module>.wasm`): a WebAssembly module that draws one pane of the side
  drawer. Today: `contents-pane`, `todo-pane` and `scratch-pane`.

## Screen plugins

The app sends events and gets a frame back (`crates/plugin-api`): a draw
list in the theme's four tones, hit regions that name controls, and
actions (close, open a note, preview a note). The app draws the title bar and
close button, owns the library data, and applies actions. A plugin keeps its
own state (map or list, zoom, pan, selection, page).

Every type and limit of the wire format is documented in the crate, with
examples: run `cargo doc -p sigma-plugin-api --open`.

The module runs in wasmi (`crates/plugin-host`) with only seven imports:
`text_width`, `ascent`, `tr`, `cache_get`, `cache_put`, `log` and `log_level`
(see [logging.md](../user/logging.md)). No filesystem or
network. Every call gets a fuel budget and the memory has a ceiling; after any
failure the plugin is dropped and the built-in Graph serves the rest of the
session.

## Pane plugins

The side drawer (a panel beside the page, about 48% wide) holds up to three
panes, top to bottom: Contents, To-dos, Scratch. Each is drawn by the enabled
plugin whose manifest says `kind=pane` and its `slot`. One title-bar button
(a panel on the drawer's side) opens the drawer menu, with a checkbox for each
pane whose plugin is on; switching a plugin off takes its row out of the menu
and closes its pane, and with no pane plugin on the button goes. The built-in panes stay in the app as
the test reference and the fallback: a plugin that fails to load or fails
while running is dropped for the session and the built-in pane draws instead.

The wire format is the screen's (API 4), with these differences:

- `Scene` carries the pane's rectangle (`left`, `top`, `width`, `bottom`), the
  open note's `headings` and the live `todos` list (every box in the library,
  the open note and the scratch pad taken from their buffers). The host sends
  it again whenever it, or the pane's place, changes; the plugin keeps its own
  state (the To-dos filter) across those.
- The plugin draws the whole pane, header included, and nothing outside it
  (the host clips); the host keeps the drawer's background, edge and rules.
- Only taps are sent (`Tap`), and a frame's actions all apply, in order, with
  the pane staying open: `ToggleTodo(n)`, `GotoTodo(n)`, `GotoHeading(n)`
  (indexes into the scene's lists). Hit regions name the emulator's controls
  (`Heading:TEXT`, `Todo:TEXT`, `TodoBox:TEXT`, `Filter`).
- Scratch is a text field the **host** owns: a frame's `field` places it and
  gives the placeholder. Typing, the on-screen keyboard, handwriting and
  saving `.scratchpad.md` stay in the host. Plugins never touch files.

The plugins repo has a small `sigma-plugin-sdk` crate (host imports, the three
exports, the header, hint and "N more" drawing every pane shares).

### Loading and progress

Opening a screen plugin must never look like a hang:

- The screen appears at once. If the module is not loaded yet, a loading
  picture (a title and an empty bar) shows while the next idle tick loads it;
  a plugin kept loaded from an earlier open skips that.
- A plugin can answer with a **pending** frame (plugin API 2): the frame is a
  progress picture, and the host sends `Step` on every idle tick until a frame
  is not pending. The Graph builds a new layout in about twelve steps, drawing a
  title, a bar and a percentage each time, with a partial refresh per step.
  Leaving (Close or Esc) stops the steps; a stray tap does not.
- A layout that is cached, or a library that has not changed, opens without
  any build.

### Dispatch and speed

wasmi is built with `portable-dispatch` (a plain interpreter loop). Its
default, tail-call dispatch, relies on the compiler turning calls into jumps
and overflowed the stack in some build configurations, which is not
acceptable on a device we cannot debug. The cost is about 45x native
(tail-call dispatch is about 16x).

The Graph is therefore made cheap in four ways:

- **The plugin stays loaded** between opens (the app keeps the instance while
  the module file is unchanged, and drops it after any failure), so starting
  wasmi and validating the module is paid once per session.
- **The layout is cached** through the host: an exact hit costs one read, and
  a changed library warm-starts from the last picture (30 iterations instead
  of 160).
- **Up to 150 linked notes** the layout is the built-in algorithm unchanged,
  bit for bit, so small libraries draw exactly what the built-in Graph draws.
- **Above 150 linked notes** the all-pairs repulsion (quadratic) is replaced by
  a Barnes-Hut quadtree (opening angle 1.2, mean force error under 5%), and a
  cold layout uses fewer iterations as the library grows (160 down to 60).
  Pictures of large libraries therefore differ from the built-in ones; they
  have the same character (checked in the plugin's tests) and are stable once
  cached. A warm picture depends on history, not only on names and links.

Desktop timings (`app/tests/graph_plugin_perf.rs`, release; first open runs
the layout, the other two show the cache and the warm start):

| Notes | First open | Unchanged | After one new note |
|---|---|---|---|
| 100 | 282 ms | 10 ms | 65 ms |
| 150 | 406 ms | 16 ms | 90 ms |
| 309 | 525 ms | 27 ms | 172 ms |
| 600 | 877 ms | 66 ms | 364 ms |
| 1000 | 1217 ms | 148 ms | 650 ms |

Expect roughly 8 to 10 times that on the Forma (docs/dev/perf.md); these numbers
are not measured on the device yet.

### Tests

- `app/tests/plugin_screen.rs`: a tiny WAT plugin records every event, so the
  host's side of the protocol is checked.
- `app/tests/pane_plugin_equivalence.rs`: the real pane modules and the
  built-in panes draw identical pixels, refresh classes and states: Contents
  jumps, the To-dos filter, check boxes and opening, Scratch typing, all three
  panes together, rows that do not fit, empty panes, preview and a smaller
  screen. `app/tests/plugin_pane.rs` covers the host's side (icons, switching
  off, a module that does not load, one that traps).
- `app/tests/graph_plugin_equivalence.rs`: the real `graph-map` module and the
  built-in Graph draw identical pixels, refresh classes and states across
  zoom, selection, pan, list paging, the mouse path, Night, three screen sizes
  and 1 to 150 notes. For a library of 320 notes (approximate map layout) the
  list view, refresh classes and states are compared.
- The modules are `app/tests/fixtures/<id>/`, built from the plugins repo. Refresh it with `scripts/update-plugin-fixtures.sh`
  (it records the plugin commit and a sha256 in `SOURCE.txt`).

## For plugin authors: logging

A plugin has two host imports (plugin API 3):

- `log(level, message)`: a line at a Python-style number (10, 20, 30, 40, 50).
  The host tags it `plugin/<id>` and keeps it only if the reader's level wants
  it.
- `log_level()`: the lowest level the host would keep (100 when none), so a
  plugin can skip building a message nobody will read.

The Graph plugin logs its layout cache (hit, miss, damaged entry), the layout
build with its iterations and warm start, each step, and what it was opened
with.

The log levels and the file are described in [../user/logging.md](../user/logging.md).

### Log folder and launcher lines

`SIGMA_LOG_DIR` overrides the log folder; on a desktop and in the emulator the
default is `target/emu-log`. Wi-Fi log lines, including those of `wifi.sh`, are
listed in [wifi-design.md](wifi-design.md#8-notes-moved-from-the-user-page).

## Plugin repository, packages and installation

Moved from the user page ([../user/plugins.md](../user/plugins.md)); the user-level behaviour is described there.

### State and format

Plugins come from the `sigma-scribe-plugins` repository and are installed under
`.adds/sigma-scribe/plugins/<id>/`. `plugins.state` there records what is
installed and enabled. Format and index rules live in
`crates/core/src/plugins.rs`.

### Settings

A plugin declares settings in `plugin.conf`, one line each:
`setting=<key>|<Label>|<Option A>,<Option B>[,...]|<Default>` (two to four
options, at most eight settings; a bad line is skipped). Choices are kept in
`<plugins>/<id>/settings` (`key=Option` lines) and sent to the plugin in the
scene each time its screen opens (`Scene.settings`), so a change applies the
next time the screen opens. The Graph declares **Start on** (Map or List),
**Labels** (Auto or All) and **Layout** (Quick or Best).

### Built-in Graph as reference

The built-in Graph stays in the app as the reference the plugin is tested
against (see Tests above), and as the fallback if the plugin is on but fails;
the emulator shows it too.

### Fetching from the repository

**Check for plugins** fetches the index (`INDEX_URL` in
`crates/core/src/plugins.rs`). The Kobo's Wi-Fi must be on in the Kobo's own
reader before Σscribe starts: the launcher keeps the network Nickel brought up
(it does not bring Wi-Fi up itself) and logs it as `net:` lines in
`.adds/sigma-scribe/sigma-scribe.log`. The app downloads in-process (ureq,
rustls, the bundled Mozilla roots; the Kobo has no `curl`), over https only,
redirects included, from the plugins repository only
(`plugins::trusted_source`). A fetch blocks the screen until it ends.

### Package format and install

A package is an uncompressed ustar archive of regular files with `plugin.conf`
at its root (`scripts/make-index.sh` in the plugins repository builds them and
the index). Nothing is unpacked until its size and sha256 match the index and
its `plugin.conf` agrees with the index on id and version
(`crates/core/src/pluginpkg.rs`). The plugin is written to a scratch folder and
renamed into place, so a failed install leaves the old version. A new plugin
starts on; an update keeps its on/off choice and its settings. An index older
than the last one seen (`plugins/index.serial`) and a package older than the
installed version are refused. The index is not signed; its integrity rests on
https.

### Offline mirror and tests

`SIGMA_PLUGIN_REPO=<folder>` reads `index.json` and the packages (by file name)
from a folder instead of the network: an offline mirror, and what
`app/tests/plugin_repo.rs` uses.

### Remove

Each installed plugin has a **Remove** row (`pluginpkg::uninstall_in`: its
folder and its `plugins.state` line). It is greyed while the plugin is on.
