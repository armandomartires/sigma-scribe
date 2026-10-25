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

The Graph layout is therefore cached through the host: an exact hit costs one
read, and a changed library warm-starts from the last picture (30 iterations
instead of 160). Desktop timings (`app/tests/graph_plugin_perf.rs`, release):

| Notes | First open | Unchanged | After one new note |
|---|---|---|---|
| 100 | 277 ms | 92 ms | 150 ms |
| 309 | 1173 ms | 111 ms | 339 ms |

Expect roughly 8 to 10 times that on the Forma (docs/perf.md). A warm picture
depends on history, not only on names and links; once cached it is stable.

### Tests

- `app/tests/plugin_screen.rs`: a tiny WAT plugin records every event, so the
  host's side of the protocol is checked.
- `app/tests/graph_plugin_equivalence.rs`: the real `graph-map` module and the
  built-in Graph draw identical pixels, refresh classes and states across
  zoom, selection, pan, list paging, the mouse path, Night, three screen sizes
  and 1 to 160 notes.
- The module is `app/tests/fixtures/graph-map/graph.wasm`, built from the
  plugins repo. Refresh it with `scripts/update-graph-plugin-fixture.sh`
  (it records the plugin commit and a sha256 in `SOURCE.txt`).
