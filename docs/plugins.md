# Plugins

Plugins come from one place, the `sigma-scribe-plugins` repository, and are
installed under `.adds/sigma-scribe/plugins/<id>/`. `plugins.state` there
records what is installed and enabled. Format and index rules live in
`crates/core/src/plugins.rs`.

## Switching plugins on and off

Settings → Plugins lists every plugin folder under `.adds/sigma-scribe/plugins/`
that has a valid `plugin.conf` (its id is the folder's name) and gives it an
On/Off switch. The choice is written to `plugins.state` straight away.

- A **screen plugin** (the Graph) takes effect at once for its title-bar
  icon: **off removes the Graph icon from the top menu** (the other icons close
  up, Settings stays flush right) and **on puts it back**. The built-in Graph
  stays in the app as the reference the plugin is tested against, and as the
  fallback if the plugin is on but fails; the emulator shows it too.
- A **data pack** is read when the app starts, so its row says "restart to
  apply".
- A plugin that needs a newer app than this one is listed, greyed and without
  a switch.
- A plugin that is on disk but not yet in `plugins.state` shows as Off; its
  first switch-on adds it to the file.

### A plugin's own settings

A plugin can declare settings in `plugin.conf`, one line each:

    setting=<key>|<Label>|<Option A>,<Option B>[,...]|<Default>

(two to four options, at most eight settings; a bad line is skipped). While
the plugin is **on**, Settings → Plugins shows a row "<Name> · Settings" that
opens the plugin's own page, one choice per setting. While it is **off** the
row is greyed and says to turn the plugin on first, so the page cannot be
reached. Choices are kept in `<plugins>/<id>/settings` (`key=Option` lines) and
sent to the plugin in the scene each time its screen opens (`Scene.settings`),
so a change applies the next time the screen opens. The Graph declares
**Start on** (Map or List), **Labels** (Auto or All) and **Layout** (Quick or
Best).

Installing from the plugins repository is not in the app yet: copy a plugin's
folder to `.adds/sigma-scribe/plugins/<id>/`.

## Kinds

- **Data packs** (font, dictionary, hwr-model, word-list, theme, translation,
  layout, icons): files the app already knows how to read, found at the same
  relative paths as the loose files (`fonts/cjk.ttf`, `dict/skk-jisyo.txt`).
- **Screen plugins** (`kind=screen`, `entry=<module>.wasm`): a WebAssembly
  module that owns a full-screen body. Today: `graph-map`, the Graph screen.

## Screen plugins

The app sends events and gets a frame back (`crates/plugin-api`): a draw
list in the theme's four tones, hit regions that name controls, and
actions (close, open a note, preview a note). The app draws the title bar and
close button, owns the library data, and applies actions. A plugin keeps its
own state (map or list, zoom, pan, selection, page).

The module runs in wasmi (`crates/plugin-host`) with only seven imports:
`text_width`, `ascent`, `tr`, `cache_get`, `cache_put`, `log` and `log_level`
(see logging.md). No filesystem or
network. Every call gets a fuel budget and the memory has a ceiling; after any
failure the plugin is dropped and the built-in Graph serves the rest of the
session.

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

Expect roughly 8 to 10 times that on the Forma (docs/perf.md); these numbers
are not measured on the device yet.

### Tests

- `app/tests/plugin_screen.rs`: a tiny WAT plugin records every event, so the
  host's side of the protocol is checked.
- `app/tests/graph_plugin_equivalence.rs`: the real `graph-map` module and the
  built-in Graph draw identical pixels, refresh classes and states across
  zoom, selection, pan, list paging, the mouse path, Night, three screen sizes
  and 1 to 150 notes. For a library of 320 notes (approximate map layout) the
  list view, refresh classes and states are compared.
- The module is `app/tests/fixtures/graph-map/graph.wasm`, built from the
  plugins repo. Refresh it with `scripts/update-graph-plugin-fixture.sh`
  (it records the plugin commit and a sha256 in `SOURCE.txt`).
