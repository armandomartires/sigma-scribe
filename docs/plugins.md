# Plugins

Plugins come from one place, the `sigma-scribe-plugins` repository, and are
installed under `.adds/sigma-scribe/plugins/<id>/`. `plugins.state` there
records what is installed and enabled. Format and index rules live in
`crates/core/src/plugins.rs`.

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

The module runs in wasmi (`crates/plugin-host`) with only five imports:
`text_width`, `ascent`, `tr`, `cache_get`, `cache_put`. No filesystem or
network. Every call gets a fuel budget and the memory has a ceiling; after any
failure the plugin is dropped and the built-in Graph serves the rest of the
session.

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
