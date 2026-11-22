# Architecture

Σscribe is a Markdown notebook for the Kobo Forma. This page is the
crate map and the Phase 2 platform seam. Engineering rules are in
`CONTRIBUTING.md`; decisions that shaped the seam are in
`docs/adr/`. The cross-platform plan lives in the
[dev repository](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md).

## Crate map

The workspace is a set of libraries plus two thin binaries. No crate is
published to crates.io (`publish = false`). Licence: AGPL-3.0-only.

| Path | Crate | Role |
|---|---|---|
| `crates/core` | `sigma-core` | Portable editor core: buffer, wrapping, layouts, store, settings, plugin registry, logging, and the `Paths` / `SearchPaths` types. No Kobo path literals and no `cfg(target_arch = "arm")`. |
| `crates/render` | `sigma-render` | Software renderer and every screen, into an 8-bit greyscale canvas. Also portable. |
| `crates/device` | `sigma-device` | `Device` and `Platform` traits. Desktop (minifb / PNG) and Kobo (FBInk + evdev) backends. `Paths` constructors `kobo()`, `desktop()`, and `current()` live here. |
| `crates/hwr` | `sigma-hwr` | Offline handwriting recognition, no platform dependency. |
| `crates/plugin-api` | `sigma-plugin-api` | Wire format between the app and WASM plugins. Plugin authors depend on it. |
| `crates/plugin-host` | `sigma-plugin-host` | Runs plugins under `wasmi` with fuel, memory and message limits. |
| `crates/sigma-app` | `sigma-app` | Portable UI state machine (screens, settings, editor, plugins, sync hooks). |
| `app` | `sigma-scribe` | Thin shells: the `sigma-scribe` device/desktop binary and the `sigma-emu` headless emulator. |
| `tools/hwr-train` | `hwr-train` | Builds and measures the default handwriting model. |

```
                 sigma-scribe / sigma-emu
                         │
                    sigma-app
                   /     |     \
          sigma-render  sigma-device  sigma-hwr
                 \       |        /
                  sigma-core
                         |
              sigma-plugin-host ── sigma-plugin-api
```

The shells still depend on `sigma-core`, `sigma-render` and
`sigma-device` for logging, the desktop window canvas, and emulator
script commands. Further thinning of those imports can wait.

## Platform seam (Phase 2)

The seam is recorded in [ADR 0001](../adr/0001-platform-seam.md). In short:

- **Build switches, not architecture.** The `sigma-scribe` and
  `sigma-device` packages have mutually exclusive Cargo features:
  `desktop` (the default, minifb) and `kobo` (FBInk). There is no
  separate `headless` feature; `sigma-emu` is already its own binary.
  Portable crates have no `kobo` / `desktop` features and must not
  use `cfg(target_arch = "arm")` to mean "this is a Kobo".
- **Paths.** `sigma_core::paths::Paths` is a plain structure with no
  Kobo literals in it. The device layer fills it in. `SearchPaths` is
  installed at start-up from that structure; locale, plugins,
  dictionaries and font packs take their roots from it. Environment
  overrides (`SIGMA_NOTES`, `SIGMA_PLUGINS`, `SIGMA_KOBO_CONF`, and the
  others listed in the ADR) still work.
- **Platform.** A small `Platform` trait in `sigma-device` covers
  wifi and sync-process hooks. The Syncthing supervisor still launches
  `Command::new(&cfg.binary)` from a configured path, not a device
  literal.
- **CI targets.** Portable crates are `cargo check`'d for wasm32,
  `aarch64-linux-android`, `x86_64-pc-windows-gnu`, and
  `aarch64-unknown-linux-gnu`. An iOS check still needs a macOS runner
  and is a follow-up. The Kobo package is a separate
  `armv7-unknown-linux-musleabihf` musl build with the `kobo` feature.

## Plugins

Plugins are WASM modules described by `docs/dev/plugin-api.md`. The
host (`sigma-plugin-host`) is the sandbox; `sigma-plugin-api` is the
contract. Changing the encoding means bumping `ABI` and updating both
that document and the plugins repository. Moving the plugins SDK off
`sigma-core` internals is Phase 3 (P3-03, P3-04) and is not described
here yet.

## Tests

`docs/dev/testing.md` is the test-layer guide. The important split is
that unit tests live next to the code, app behaviour is exercised
through `sigma-emu` in `crates/sigma-app/tests/` and `tests/emu/`, and
the Kobo package job is the check that the device feature builds.
