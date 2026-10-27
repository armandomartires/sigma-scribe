# Handover: plugin distribution (written 2026-10-25)

For a new session picking this up. Read this first, then `CLAUDE.md` in
`/home/amartires/work/sigmarecon/` (workspace rules: never delete folders,
archive them to `99.ARCHIVE/`) and the memory notes in
`/home/amartires/.claude/projects/-home-amartires-work-sigmarecon-sigma-scribe/memory/`
(read-device-logs-myself, kobo-install-routine). This file is **not committed**.
It continues `.ai/handover-plugins-2026-10-25.md` (also uncommitted), whose
sections 1 to 3 describe the plugin system itself; this one covers only what
is left: **getting plugins from the plugins repo onto a Kobo**.

## 1. Where things stand

Plugins exist and work, but they are installed by hand: a person copies a
plugin's folder and edits `plugins.state` on the Kobo (that is what every
install in this project has done so far). Nothing is hosted, signed, built by
CI or downloadable.

| Piece | State |
|---|---|
| Plugin model, manifest, `plugins.state`, Settings → Plugins list/switch | done, on `main` |
| Four code plugins (`graph-map` 1.3.0, `contents-pane`, `todo-pane`, `scratch-pane`), plugin API 4 | done, on `main` (app) and `pane-plugins` (plugins repo) |
| Index format, URL trust rules, `availability()` (not installed / update / up to date / incompatible) | done in `crates/core/src/plugins.rs`, unit-tested, **unused by any front end** |
| Package format (`.sgp`) | **undefined**: the extension appears only in tests |
| Build script making packages and an index | **not started** (`scripts/build-plugin.sh <id>` makes an unpacked `dist/<id>/` only) |
| Index signing (Ed25519) and rollback protection | **not started** (the `serial` field is parsed, nothing checks it) |
| Hosting (GitHub Pages and/or Releases) | **not set up**; the app's pinned URLs point at places that do not exist yet |
| CI in the plugins repo | **none** (no `.github/`) |
| In-app install, update, uninstall, "Get plugins" screen, Wi-Fi on demand | **not started** |

### Repos (state at the end of the session)

- **App** `/home/amartires/work/sigmarecon/sigma-scribe`, remote `github.com/armandomartires/sigma-scribe`. `main` = `89a5d42` (merge of PR 58). The whole plugin stack is on `main`. The checkout is on `fixtures-pin` (merged); switch to `main` and pull before working. Worktrees `sigma-scribe.wt-ui` and `sigma-scribe.wt-write-nav` are other work, leave them.
- **Plugins** `/home/amartires/work/sigmarecon/sigma-scribe-plugins`, remote `github.com/armandomartires/sigma-scribe-plugins`. Only branch `pane-plugins` (`a044e3f`) is pushed; **there is no default branch or `main` yet** (the repo was empty). Its plugins depend on the app's `sigma-plugin-api` and `sigma-core` by git rev `c870b04` (on app `main`). The local branch `graph-map` is older history already contained in `pane-plugins`.
- Merging mishap to know about: PRs #53 to #56 were merged into their own base branches, so only #52 reached `main`; PR 57 (`logging-setting` into `main`) landed the rest. Retarget stacked PRs, or merge top-down, next time.

### What the app already pins (do not change lightly)

`crates/core/src/plugins.rs`:

- `INDEX_URL = https://armandomartires.github.io/sigma-scribe-plugins/index.json`
- `SOURCE_PREFIXES`: `https://armandomartires.github.io/sigma-scribe-plugins/` and `https://github.com/armandomartires/sigma-scribe-plugins/releases/download/`. Every package URL in an index must start with one of them and contain no `..`; entries that fail are skipped (`trusted_source`).
- Index text: a header block with `serial=<n>`, then one block per plugin, blocks separated by a blank line. Each block is the plugin's manifest fields plus `url=`, `size=`, `sha256=` (64 lowercase hex). Unknown keys are ignored; bad entries are skipped so one broken plugin never hides the rest.
- `Manifest` fields: `id`, `name`, `version`, `kind`, `description`, `min_app`, `api` (the format version, currently 1), `entry` (screen and pane), `slot` (pane), `setting=` lines.
- `State` (`plugins.state`): `id=version,enabled|disabled` per line; `install`, `set_enabled`, `remove`, `enabled()`.
- The README of the plugins repo already says `index.json` is "generated, signed" and that the app "downloads from here and nowhere else".

## 2. The plan the user agreed to (design is open where marked)

Decisions already made: plugins come **only** from the user's own repo; install is **on the device with Wi-Fi on demand**; code plugins are WASM; the hosting target is GitHub Pages (index and packages) and/or Releases (packages), as the two prefixes say.

### Step A: the plugins repo as a publishable source

1. Agree with the user how to make the repo's default branch (`main`): push the same history as `main`, or merge `pane-plugins`. Ask first; pushing is outward-facing.
2. Define the **`.sgp` package**. Open design question; propose and confirm. Constraints: a data pack's files sit at the same relative paths the app reads from `.adds/sigma-scribe/` (`fonts/cjk.ttf` ...); a code plugin carries `plugin.conf` and its module; the unpacker must be tiny and safe on a Kobo (no path traversal, size limits, atomic). A tar.gz or zip with a plain manifest first is the likely answer; the app has `png` and hand-written parsers but no archive crate yet, so a minimal in-tree reader (or one small dependency) is needed.
3. `scripts/build-package.sh <id>` (extends `build-plugin.sh`): builds, packs `dist/<id>-<version>.sgp`, prints size and sha256.
4. `scripts/build-index.sh`: for every plugin, one block (manifest fields from `plugin.conf` plus `url`, `size`, `sha256`), a header with a **serial that only rises** (derive it from a counter file or the commit count; never from the clock alone), then sign it.
5. **Signing (Ed25519)**: the app must carry the public key (compile-time constant) and reject an unsigned or wrongly signed index and any index whose `serial` is lower than the last one it accepted (store that serial under `.adds/sigma-scribe/plugins/`). Decide where the private key lives (the user's machine and a GitHub Actions secret are the usual choices; never commit it, never put it in a log). The signature can be a detached `index.json.sig`. sha256 per package is already in the index, so the signature covers the packages transitively.
6. **CI** (`.github/workflows` in the plugins repo): on PR, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and a wasm build of every plugin. On a version tag or manual dispatch: build packages and index, sign with the secret, publish to Pages and/or a Release (never auto-publish without the user's tag, as the app's `release.yml` does: it only drafts). The app repo's `.github/workflows/ci.yml` and `release.yml` are the model; they run in the container `ghcr.io/rust-cross/rust-musl-cross:armv7-musleabihf`.
7. Turn on GitHub Pages for the plugins repo (a user action in GitHub settings or via `gh api`; ask first).
8. Keep `scripts/update-plugin-fixtures.sh` in the app repo working: it builds the four plugins from a plugins checkout into `app/tests/fixtures/<id>/`; the plugin commit goes in each `SOURCE.txt`. Whenever the app's `plugin-api` or `core` change, re-pin the plugins' git rev and refresh the fixtures (the order: merge the app change, then re-pin to a commit on `main`).

### Step B: in-app install, update, uninstall

All of this is new code in the app; `sigma-core` stays free of I/O and the network (its header comment says so), so put the download and unpack code in the app or `device` crate.

1. **Network.** `launch.sh` kills `dhcpcd-dbus` and `dhcpcd` (the line `killall ... dhcpcd-dbus dhcpcd fmon`) and Nickel, so the Kobo has no Wi-Fi while Σscribe runs. "Wi-Fi on demand" means bringing the interface up from the app: `wpa_supplicant` and `dhcpcd` (or `udhcpc`) against the network Nickel already knows (`/mnt/onboard/.kobo/Kobo/Kobo eReader.conf` holds saved networks; KOReader's `enable-wifi.sh`/`obtain-ip.sh` are the reference for the exact commands on this firmware, 4.38.23828, kernel 4.1.15). **This must be probed on the device first** (read-only script like `orientation-probe.sh`, output to `.adds/sigma-scribe/probe/`); do not guess. Turn Wi-Fi off again after use. Date matters for TLS: check whether the clock is right.
2. **HTTPS client** for a static armv7 musl binary. The binary is built with a C cross toolchain already (FBInk via `cc`), so `rustls` plus a pure-Rust or cross-compiled crypto backend is feasible; weigh binary size (it ships on a small partition) and build time. Only the two pinned prefixes may be fetched; refuse redirects off them; set size and time limits from the index's `size`.
3. **Flow** (all steps verified, atomic, and logged with the existing `sigma_core::{debug,info,warn,error}!` macros, category `plugin`): fetch index and signature; verify signature and serial; compare with `plugins.state` using `availability()`; for an install or update download the package to a temp file under `.adds/sigma-scribe/plugins/`, check `size` and `sha256`, unpack into a temp folder, validate `plugin.conf` (same `Manifest::parse` rules, `id` equals the index id, `min_app`, `api`, a screen or pane plugin has its module), then swap into `plugins/<id>/` by rename (FAT: remove the old folder first, as the log rotation does for its backup) and write `plugins.state` last. A failure at any point leaves the old version untouched. **Keep the user's `settings` file and `.cache` in a plugin's folder across updates**, and keep its on/off state (`State::install` already keeps `disabled`). Uninstall removes the folder and the state line (never touch notes).
4. **UI**: a "Get plugins" row in Settings → Plugins (the page and its rows are in `crates/render/src/options.rs`, the wiring in `app/src/plugin_screen.rs`, which already has `refresh_plugin_list`, `set_plugin`, `set_plugin_setting`). Rows: name, version, size, Install / Update / Remove, "needs a newer app" greyed (the existing list already greys incompatible plugins). Show progress for downloads (the Graph's progress bar picture and the `Step`-on-idle pattern are the model for work that spans several ticks; a download must not freeze the UI or block input). Strings go through `tr()` and into all nine catalogs under `assets/i18n/` (English line, then `= translation`; machine-drafted, flagged for native review).
5. A pane or screen plugin that is installed or updated while its screen is open must be reloaded cleanly: `App::set_plugins_root` and `sync_plugin_icons` show how plugin state is re-read; loaded modules are keyed by file stamp (`Stamp`), so a changed module is loaded afresh.
6. **Bootstrap**: with no pane plugin installed the drawer icons are gone (by design), and with no `graph-map` the Graph icon is gone. A fresh Kobo therefore needs the four plugins before the app is useful. Decide, with the user: ship the four in the Kobo package (`scripts/package-kobo.sh`, then `check-package`), or make the first run offer a one-time "Get plugins" prompt, or both. Today the package ships none and the install routine copies them from `app/tests/fixtures/`.

### Tests to write (CI parity with the rest of the project)

- core: signature and serial rules, package manifest validation, path traversal and size rejection (unit tests; pure functions fed bytes).
- app: a fake index and package served from a temp folder through a pluggable fetch trait, so the whole flow (install, update keeping settings and cache, remove, bad sha256, bad signature, rolled-back serial, truncated package, disk-full style failure mid-unpack leaves the old plugin) runs without a network. Existing helpers: `app/tests/settings_plugins.rs`, `app/tests/plugin_pane.rs`, `app/tests/plugin_screen.rs`.
- emulator script for the Get plugins screen (`tests/emu/`; the emulator turns the built-in Graph and panes on with `set_builtin_graph(true)` and `set_builtin_panes(true)`).
- a real-device check by the user at the end, as always.

## 3. How to work here

```bash
# in sigma-scribe/ ; CI runs exactly these (run clippy BEFORE committing)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast          # 508 tests at last count
rm -rf target/ci-notes target/ci-shots         # a stale notes folder fails emulator scripts
for s in tests/emu/*.script; do cargo run -q --release --bin sigma-emu -- --script "$s" --shots target/ci-shots --notes "target/ci-notes/$(basename "$s")" || break; done   # 50 scripts

# in sigma-scribe-plugins/
cargo test && cargo clippy --all-targets -- -D warnings
scripts/build-plugin.sh <id>                   # dist/<id>/ with plugin.conf and the module
# back in sigma-scribe/: scripts/update-plugin-fixtures.sh   # all four plugins into app/tests/fixtures/
scripts/package-kobo.sh                        # armv7 package + check-package (FBInk lib at ~/src/FBInk)
```

- Installing on the Kobo follows the memory note `kobo-install-routine`: the user's explicit yes each time, backup first, install the package **and all four plugins** together (plugin API 4: an older module is refused), keep `plugins.state` choices and plugin settings and cache, verify checksums, print the install log before unmounting. To read device logs mount read-only (`udisksctl mount -b /dev/sdc -o ro`) and read `.adds/sigma-scribe/log/sigma-scribe.log` and the launcher's `.adds/sigma-scribe/sigma-scribe.log`; if the device is not attached, say so and wait.
- PRs: the user merges them. Open stacked work with the right base, merge top-down or retarget, and check `git merge-base --is-ancestor <tip> origin/main` before saying something is "on main". Pushing a branch, opening a PR, publishing the plugins repo and enabling Pages are outward-facing: ask first.
- Plugin API changes (the wire format, imports, or Scene/Frame) mean: bump `sigma_plugin_api::ABI`, merge the app change, re-pin the plugins repo's git rev to a commit on `main`, rebuild, refresh the fixtures.

## 4. Gotchas learned

- A new `Frame`/`Scene` field breaks every initialiser in the plugins repo and `crates/plugin-host/src/tests.rs`; the compiler lists them.
- `Fonts` is shared between the editor screen and all plugins (an `Rc`); parsing it costs about a second on the Forma, so never create another one.
- A pane plugin draws in screen coordinates; the host clips it to its pane by drawing into a pane-sized canvas (`EditorScreen::draw_plugin_pane`).
- Equivalence tests (`app/tests/pane_plugin_equivalence.rs`, `graph_plugin_equivalence.rs`) compare the real module with the built-in code pixel for pixel; mutation-check any new one by breaking the plugin on purpose and expecting a failure.
- The Kobo volume is FAT: rename-over-existing fails, use `tar --no-same-owner`, remove first.
- `cargo test` stops at the first failing test binary unless `--no-fail-fast`.
- Disk: `/home` has filled before (`target/` reached 35 GB). Clean `target/debug` (build output; allowed) if it fills again.

## 5. Other open items (not distribution)

- Removing the built-in Graph and drawer panes from the app (they are the equivalence reference and the failure fallback; removal needs golden images instead).
- The app version is still 1.11.1 while the CHANGELOG describes 1.13.x and an `## Unreleased` section; release notes (`docs/release/…`; `app/tests/release.rs` enforces a CHANGELOG heading and notes for the workspace version).
- The launcher's `sigma-scribe.log` is never rotated.
- All interface strings added for plugins, Graph, logging and Drawer width are machine-drafted in nine languages and need native review.
- A draggable handle on the drawer's edge (the user's first idea for resizing; the Drawer width slider, five snap positions, was built instead).
