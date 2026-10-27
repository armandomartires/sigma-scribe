# Handover: Σscribe plugin system (written 2026-10-25)

For a new session picking this up. Read this first, then `CLAUDE.md` in
`/home/amartires/work/sigmarecon/` (workspace rules: never delete folders,
archive them to `99.ARCHIVE/`) and the memory notes in
`/home/amartires/.claude/projects/-home-amartires-work-sigmarecon-sigma-scribe/memory/`
(read-device-logs-myself, kobo-install-routine). This file is **not committed**.

## 1. What this work is

Σscribe (`sigma-scribe`) is a Rust Markdown notebook for the Kobo Forma
(static armv7 musl binary, FBInk, replaces Nickel while it runs). The user asked
for a plugin system and then moved the **Graph** screen out of the app into the
first code plugin. Plugins come from one place only: the user's own repo
`github.com/armandomartires/sigma-scribe-plugins` (pinned in the app:
`sigma_core::plugins::INDEX_URL`, `SOURCE_PREFIXES`).

Delivered so far:

| Area | Status |
|---|---|
| Plugin data model, registry, repo pinning, data packs (fonts, SKK, pinyin read from enabled plugins) | merged (PR 49) |
| Settings → Plugins page: list, On/Off switch per plugin | PR 53 |
| WASM screen plugins: wire format (`sigma-plugin-api`), wasmi host (`sigma-plugin-host`), generic plugin screen | merged up to phase 3 (PRs 50, 51); the rest in PR 52 |
| `graph-map` plugin (map + list + layout with cache, warm start, quadtree), equivalence test vs the built-in Graph | PR 52 |
| Graph icon follows the plugin; progress bar while the layout builds; per-plugin settings page | PR 54 |
| Global logging setting (Debug, Info, Warning, Error, Critical), rotating log file, plugin `log` imports | PR 55 |
| Online install / update from the plugins repo, signed index, Wi-Fi on demand | **not started** |

## 2. State at a glance

**App repo** `/home/amartires/work/sigmarecon/sigma-scribe` (remote `origin`
= github.com/armandomartires/sigma-scribe). Checked out: `logging-setting`.
`main` = `a71252c` (merge of PR 51; contains plugin phases 1 to 3 only).

Stacked PRs, all OPEN, each retargets when the one below merges (merge in order):

| PR | Branch (tip) | Base | Contents |
|---|---|---|---|
| #52 | `graph-plugin-host` (`0427a47`) | `main` | graph-map port, equivalence test, keep-loaded, quadtree layout, exact scale in the scene (phase 4) |
| #53 | `plugins-settings-page` (`6cd4295`) | `graph-plugin-host` | Settings → Plugins list and switch |
| #54 | `graph-plugin-polish` (`f04a6a7`) | `plugins-settings-page` | icon hides with plugin, progress bar (plugin API 2), per-plugin settings page |
| #55 | `logging-setting` (`e577270`) | `graph-plugin-polish` | logging (plugin ABI 3) |

CI: #52, #53, #54 green (test, package, armv7). #55 had no checks reported yet
at the time of writing (check `gh pr checks 55`). Auto-fix was enabled for the
session; there was nothing outstanding.

**Plugins repo** `/home/amartires/work/sigmarecon/sigma-scribe-plugins`
(remote `origin` = github.com/armandomartires/sigma-scribe-plugins, **empty on
GitHub: nothing has been pushed**). Local branch `graph-map`, tip `83e9011`
(graph-map 1.2.0). Pushing publishes it: ask the user first. Its `.gitignore`
excludes `.env*` (the `.env` only holds the repo URL, per the user).

**Pins to remember:** `plugins/graph-map/Cargo.toml` depends on
`sigma-plugin-api` and `sigma-core` by git rev
`ec3daec616c6056b2bb42dd180714ec49f60b12b`, a commit on the app's
`logging-setting` branch. After the stack merges, re-pin to a commit that is on
`main` and rebuild + refresh the fixture.

**Kobo Forma** (volume `KOBOeReader`, vfat on `/dev/sdc`, firmware 4.38.23828),
as last seen:
- app binary `501870df…` (the PR 55 build at `499e18e`; the later `e577270` log-timestamp fix is NOT on the device)
- plugin graph-map 1.2.0 installed; `plugins.state` says `graph-map=1.2.0,disabled` (the user switched it off while testing; they flip it often, always preserve it)
- plugin settings: `view=List`, `labels=All`, `layout=Best`; logging **on at Debug**
- notes root `/mnt/onboard/sigma-scribe/notes`: the user's notes plus the 300 test notes in `_test/graph-300/` (`test-n000`..`test-n299`, about 270 linked)
- backups of each pre-install state are in `/tmp/claude-1000/kobo-backup-*` (may not survive a reboot)

**Disk:** `/home` was full once (`target/` reached 35 GB). I removed
`sigma-scribe/target/debug` (build output; allowed by `CLAUDE.md`). About 23 GB
free now; if it fills again, clean `target/debug` (or `target/debug/incremental`).

## 3. Architecture in one page

**Plugin kinds** (`crates/core/src/plugins.rs`): data packs (font, dictionary,
hwr-model, word-list, theme, translation, layout, icons) and `screen`
(a WASM module, `entry=graph.wasm`). A plugin is a folder
`.adds/sigma-scribe/plugins/<id>/` with `plugin.conf` (key=value; may declare
`setting=key|Label|Option A,Option B|Default`, 2 to 4 options, max 8). On/off
and installed version live in `plugins/plugins.state` (`id=version,enabled|disabled`).
Per-plugin choices live in `<id>/settings`; the layout cache in `<id>/.cache/`.
Loaders consult `plugins::enabled_files()`; the Graph uses `enabled_screen_in()`.

**WASM screen plugins** (`crates/plugin-api`, `crates/plugin-host`, `app/src/plugin_screen.rs`):
- Wire format: host sends `Event` (Open(scene), Tap, Drag, Wheel, PageTurn, Back, **Step**), plugin answers a `Frame`: draw ops in 4 theme tones, hit regions that name emulator controls, one action (Close / OpenNote / PreviewNote), a redraw class, and **`pending`** (the host sends `Step` on idle ticks while true). Everything bounds-checked, size-limited.
- Plugin exports `sigma_abi`, `sigma_alloc`, `sigma_call`. **ABI is 3.** A module built for another ABI is refused, so the app and `graph.wasm` always ship together.
- Host imports (module `sigma`, no fs/network): `text_width`, `ascent`, `tr`, `cache_get`, `cache_put`, `log`, `log_level`.
- wasmi 2 with **`portable-dispatch`** (the default tail-call dispatch overflowed the stack in some build configurations). About 45x slower than native, so: fuel per call, 64 MB memory ceiling, instance poisoned after any failure.
- Open flow: tap Graph → the screen exists at once; if the module is not loaded, a host-drawn "Opening the graph…" picture shows and the module loads on the next idle `Tick`; the plugin builds the layout in about 12 `Step`s drawing its own progress bar ("Building the graph…"); Close/Esc leaves. A loaded plugin is kept between opens (reused while the file is unchanged). On any failure the **built-in Graph** takes over for the session.
- The title-bar Graph icon is shown only while the plugin is on (`EditorScreen::graph_on`; other icons close up). The built-in Graph code stays in the app as the plugin's test reference and as the failure fallback; `App::set_builtin_graph(true)` shows its icon (the emulator and old tests use it). **Removing the built-in Graph (the original "phase 5") was not done.**

**graph-map** (plugins repo `plugins/graph-map`): cold layout is the app's own Fruchterman-Reingold, bit for bit (tested against `sigma_core::graphmap`). Up to 150 linked notes exact; above that a Barnes-Hut quadtree (theta 1.2) with fewer iterations. Layout cached through the host (exact key; "latest" slot gives a warm start of 30 iterations when the library changed). Settings: Start on (Map/List), Labels (Auto/All), Layout (Quick/Best; Quick halves iterations and has its own cache key). Equivalence test: `app/tests/graph_plugin_equivalence.rs` (pixel-identical to the built-in Graph, exact regime).

**Logging** (`crates/core/src/log.rs`, Settings → Logging, category index 7): levels with Python's numbers; one global switch + level (`log=`, `log_level=`); file `.adds/sigma-scribe/log/sigma-scribe.log`, 512 KB then `.1`; line format `UTC +uptime LEVEL source: message`. Off: stderr gets INFO+ exactly as before (launcher log unchanged). On: file gets >= level, stderr only WARNING+. All former `eprintln!` go through `sigma_core::{debug,info,warn,error,critical}!` keeping their old prefixes (`usb:`, `light:`, `touch:`, `perf`…). Panic hook in `app/src/main.rs`. See `docs/logging.md`.

**Settings pages** (`crates/render/src/options.rs`): categories 0..8 are Writing, Display, Keyboard & input, Language, USB & devices, Experimental, **Plugins (6)**, **Logging (7)**, Help & about. `DRow` labels are `Cow` so plugin names can be rows; `SettingsPage::Plugin(i)` is a plugin's own page (locked while the plugin is off); `Change::Plugin`, `Change::PluginSetting`, `Change::Log`, `Change::LogLevel`.

## 4. Where things are

| What | Where |
|---|---|
| Plugin model, settings declarations, list/toggle | `crates/core/src/plugins.rs` |
| Logger | `crates/core/src/log.rs` |
| Settings struct and `Change` | `crates/core/src/settings.rs` |
| Wire format, ABI | `crates/plugin-api/src/lib.rs` |
| wasmi host, limits, imports, disk cache | `crates/plugin-host/src/{lib,cache,tests}.rs` |
| Settings pages | `crates/render/src/options.rs` |
| Plugin screen drawing, loading picture | `crates/render/src/plugin_screen.rs` |
| Title bar icons | `crates/render/src/editor.rs` (`graph_on`, `layout()`) |
| App wiring (loading, stepping, actions, settings, icon gate) | `app/src/plugin_screen.rs`; hooks in `app/src/lib.rs` (`plugin_step` on `Event::Tick`) |
| Plugin crate | `../sigma-scribe-plugins/plugins/graph-map/src/{lib,layout,view,progress,host,tests}.rs` |
| Test module fixture | `app/tests/fixtures/graph-map/` (`SOURCE.txt` has the plugin commit) |
| Docs | `docs/plugins.md`, `docs/logging.md`, CHANGELOG `## Unreleased` |
| Earlier plan (partly superseded) | `/home/amartires/.claude/plans/we-need-to-implement-imperative-stardust.md` |

Key tests: `app/tests/{plugin_screen,settings_plugins,logging,graph_plugin_equivalence,graph_map}.rs`,
ignored perf test `app/tests/graph_plugin_perf.rs`, emulator scripts
`tests/emu/v1.14-logging.script` (+ the 49 older ones).

## 5. How to work here

```bash
# in sigma-scribe/ ; CI runs exactly these
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings      # run BEFORE committing
cargo test --workspace                                      # 53 test binaries at last count
for s in tests/emu/*.script; do cargo run -q --release --bin sigma-emu -- --script "$s" --shots target/ci-shots --notes "target/ci-notes/$(basename "$s")" || break; done   # 50 scripts
cargo test --release -p sigma-scribe --test graph_plugin_perf -- --ignored --nocapture   # timings

# plugin changes: edit ../sigma-scribe-plugins, test + clippy there, commit, then
scripts/update-graph-plugin-fixture.sh        # rebuilds the wasm into app/tests/fixtures/graph-map
# (needs rustup target wasm32-unknown-unknown, already installed)

scripts/package-kobo.sh                       # armv7 package + check-package (FBInk lib at ~/src/FBInk)
```

The plugins repo: `cargo test` and `cargo clippy --all-targets -- -D warnings`
(33 tests at last count); `scripts/build-plugin.sh graph-map` builds `dist/graph-map/`.

**Installing on the Kobo:** follow the memory note `kobo-install-routine`:
user's explicit yes each time, backup first, install the package **and** the
plugin module together, preserve the plugin on/off state, settings file, cache
and notes, verify checksums, print the install log as text **before**
unmounting. To read device logs, mount read-only
(`udisksctl mount -b /dev/sdc -o ro`) and read both
`.adds/sigma-scribe/log/sigma-scribe.log` and the launcher's
`.adds/sigma-scribe/sigma-scribe.log`; no need to ask the user to send them.

## 6. Decisions the user made (and why)

- Plugins are **data packs first**, then code via **WASM** (chosen over native executables); install is **on-device download with Wi-Fi on demand** (planned, not built).
- All plugin sources (fonts, icons, dictionaries, the Graph) come only from **their own repo** `sigma-scribe-plugins`.
- Graph without its plugin: **hidden entirely** (icon gone); the other icons **close up**.
- Layout speed: keep WASM plus a **disk cache and warm start**; above ~150 linked notes use a **faster approximate repulsion** (exact below, so equivalence tests stay bit-exact).
- Graph plugin settings: all of Start on, Labels, Layout. Settings placement: dedicated pages; logging as a **new Logging category**.
- Logging: all five levels, own **rotating file** (512 KB + one backup), global setting.
- Working style: they test on the Forma between installs and report; they approve each install; they prefer concise status with what was verified and what was not; PRs stay stacked and unmerged until they say.

## 7. Open items, roughly in priority order

1. **Merge the stack** (#52, #53, #54, #55 in that order) when the user is ready; check CI on #55; then re-pin the plugins repo (section 2) and refresh the fixture.
2. **Cold Graph build timings on the Forma.** The first device log had a cache hit (303 notes, `loaded in 950 ms, opened in 76 ms`). I asked the user to force a cold build (switch Layout to Quick, or add a note) so the step timings show up in the log, or to let me clear `plugins/graph-map/.cache` on the device (needs their yes). Estimated 4 to 5 s for 300 notes cold; unmeasured.
3. **Push `sigma-scribe-plugins`** (empty on GitHub) once the user agrees; then Pages/Releases hosting, a build script that makes `.sgp` packages and a signed `index.json` (Ed25519, index serial against rollback), and CI. The index/package/URL rules already exist in `plugins.rs` (`Index`, `trusted_source`).
4. **In-app install/update/uninstall** from that repo: HTTPS client, Wi-Fi on demand (`launch.sh` currently kills `dhcpcd`), download + sha256 + signature check + atomic unpack, a Settings → Plugins "Get plugins" screen. Not started.
5. **Original phase 5:** decide whether to remove the built-in Graph from the app (it is the equivalence reference and failure fallback; removing it needs golden images instead) and bump the app version / write release notes (`docs/release/…`; `app/tests/release.rs` enforces a CHANGELOG heading and notes for the workspace version, currently 1.11.1 while the CHANGELOG already describes 1.13.x).
6. Smaller: the plugin module load costs ~950 ms on the Forma once per session (could be trimmed); the launcher's `sigma-scribe.log` is never rotated; the 29 new interface strings (plugins 14, Graph 7, logging 8) are machine-drafted in nine languages (native review needed); `e577270` (log timestamp order) is not installed on the device yet; real-device check of the progress bar, Settings → Plugins page and the Logging page is still pending.

## 8. Gotchas learned the hard way

- **Run clippy with `-D warnings` before committing.** One commit once went in with a failing lint because the commands were chained with `;`.
- **Fixture and ABI move together.** Changing the wire format or the imports means: bump `ABI`, push the app branch, re-pin the plugins repo's git rev, rebuild, run `update-graph-plugin-fixture.sh`. The test WAT modules use `sigma_plugin_api::ABI`, not a literal.
- **Equivalence tests need the built-in Graph:** the reference rig calls `set_builtin_graph(true)`; the plugin rig calls `plugin_settle()` after tapping Graph (it opens and builds on idle ticks).
- **Global state in tests:** the logger is process-global; `app/tests/logging.rs` takes turns with a mutex. Other test binaries never turn logging on.
- **rustfmt rewraps code**, so scripted multi-line `str.replace` edits can miss; edit by exact current text and let the assertions in the script catch misses.
- **Mutation-check new equivalence tests** (break the plugin on purpose, expect failure); it caught a 3 % glyph change once.
- **PR 51 merged before later commits were pushed to its branch**; always check `gh pr view` state before saying work is "on" a PR. Merges here are real merge commits, so stacked branches rebase cleanly.
- **FAT (vfat) Kobo volume:** rename-over-existing is handled in the log rotation by removing the backup first; use `tar --no-same-owner`.
- The package script overwrites `dist/sigma-scribe-1.11.1-kobo.tar.gz` each run (same version string).
