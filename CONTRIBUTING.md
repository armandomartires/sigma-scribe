# Contributing to Σscribe

How to build, check and change this repository. The user-facing overview is
in `README.md`; the test layers are in `docs/testing.md`.

## Layout

| Path | Crate | What it is |
|---|---|---|
| `crates/core` | `sigma-core` | Editor core with no platform dependency: buffer, wrapping, keyboard layouts, store, settings, plugins registry, logging. |
| `crates/render` | `sigma-render` | Software renderer and every screen, into an 8-bit greyscale canvas. |
| `crates/device` | `sigma-device` | The `Device` trait, the desktop PNG backend, and the Kobo FBInk + evdev backend. |
| `crates/hwr` | `sigma-hwr` | Offline handwriting recognition, no dependencies. |
| `crates/plugin-api` | `sigma-plugin-api` | Wire format between the app and its WASM plugins. Plugin authors depend on it. |
| `crates/plugin-host` | `sigma-plugin-host` | Runs plugins under `wasmi` with fuel, memory and message limits. |
| `app` | `sigma-scribe` | The app, plus the `sigma-emu` headless emulator. |
| `tools/hwr-train` | `hwr-train` | Builds and measures the default handwriting model. |

No crate is published to crates.io (`publish = false`).

## Toolchain and targets

- Edition 2021. Day-to-day work and the main CI job use current stable
  Rust.
- The minimum supported Rust version (MSRV) is `rust-version` in the root
  `Cargo.toml`, currently 1.88. It is set by the dependencies: `wasmparser`
  0.262, which `wasmi` 2 uses, needs 1.88.
- The `msrv` job in CI checks the workspace with exactly that version. Do
  not use language or library features newer than it.
- To change the MSRV, change `rust-version` and the toolchain in the `msrv`
  job together, and note it in `CHANGELOG.md`. A dependency update that
  needs a newer Rust is an MSRV change.
- Changing the MSRV, the edition or a dependency's major version is a
  deliberate decision: say so in the pull request and in `CHANGELOG.md`.
- Two targets are supported:
  - **Desktop Linux (x86_64)** for development, tests and the simulator.
    The live window needs `libxkbcommon-dev` (Debian/Ubuntu).
  - **Kobo**: `armv7-unknown-linux-musleabihf`, a static binary linked
    with `rust-lld` against FBInk. Build it with `scripts/build-armv7.sh`.
    It needs the musl cross toolchain and a pinned FBInk build; see
    `.github/workflows/ci.yml` and `docs/s0-findings.md`.
- Code for the Kobo only sits behind `#[cfg(target_arch = "arm")]`. The
  FBInk backend is never compiled on the desktop, so a desktop build passing
  is not evidence that the ARM build does. CI runs both.
- There are no Cargo feature flags.

## Checks before a pull request

CI runs all of these on every pull request. Run them locally first:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo test --workspace
```

`cargo test --workspace` includes the doctests. CI also runs, and you can run
when relevant:

```sh
cargo test -q -p hwr-train --tests -- --nocapture
scripts/sim.sh --headless tests/emu/v1.0-tour.script target/shots
```

The first prints the handwriting accuracy and latency figures. The second is
one emulator script; CI runs every `tests/emu/*.script` this way.

**Dependencies.** The `deny` job in CI runs `cargo deny check` against
`deny.toml`: RustSec advisories, licences, banned crates and sources. To run
it locally, install it once with `cargo install --locked cargo-deny`.
- Licences must be on the allow list in `deny.toml`. All are permissive
  and compatible with AGPL-3.0-only. Do not add a dependency whose licence
  is not.
- A new advisory does not block a pull request, but fix it or add it to
  `ignore` in `deny.toml` with a reason.
- `rustybuzz` and `ttf-parser` (the font stack) are unmaintained and listed
  there. They have no known vulnerability.

A UI change also follows `docs/ui-rulebook.md`. A release follows
`docs/release/checklist.md`.

## Code conventions

These describe how the code is already written. Keep new code consistent.

**Errors.**
- Expected failures return `Result`.
- File and device code uses `io::Result`.
- A crate whose callers need to tell failures apart has its own error type
  with `Display` and `std::error::Error`. See `sigma_plugin_host::Error` and
  `sigma_plugin_api::DecodeError`.
- There is no `thiserror` or `anyhow`. Do not add them for this.

**Panics.**
- Do not `unwrap`, `expect` or `panic!` on anything that can fail at run
  time: files, user input, plugin output, device input.
- `expect` is fine for invariants the program guarantees, such as parsing
  data embedded with `include_bytes!`. The message says what is assumed.
- Tests may unwrap freely.

**Untrusted input.**
- Plugins, note files, settings files and USB devices are untrusted.
- Decode them with bounds and size limits before allocating, as
  `sigma-plugin-api` does.

**Logging.**
- Use the `sigma_core` macros (`debug!`, `info!`, `warning!`, `error!`,
  `critical!`) with a target such as `"app"` or `"usb"`. See
  `docs/logging.md`.
- Do not log note text or what the reader types.
- `println!` and `eprintln!` belong only in tools, build scripts and
  `sigma-emu`.

**`unsafe`.**
- `unsafe` is forbidden in the workspace.
- The one exception is the FBInk FFI in `crates/device/src/fbink.rs`.
- Each `unsafe` block there states why it is sound.

**Lints.**
- Clippy's default set is an error in CI.
- An `#[allow(...)]` covers the smallest item it can and has a comment
  giving the reason.
- `crates/device/Cargo.toml` restates the workspace lint tables. Keep it in
  step when they change.

**Dependencies.**
- Few, with default features off where possible.
- A new dependency needs a reason in the pull request: what it replaces and
  what it costs in binary size on the Kobo.

**Threads.** There is no async. Background work uses `std::thread` and
channels, as in `crates/core/src/outline.rs`. The input loop must never
block on the e-ink panel.

## Documentation

**Crate and module docs.** Every crate root has `//!` docs saying what the
crate is for.

**Public items.**
- Public items get `///` docs. The first sentence says what the item is.
- Add `# Errors` when a function returns `Result`, and `# Panics` if it can
  panic.
- Comments in the body explain why, not what.

**The two plugin crates.**
- `sigma-plugin-api` and `sigma-plugin-host` are fully documented and set
  `#![warn(missing_docs)]`, which CI turns into an error.
- Keep them that way. The wire format is a contract with plugins built
  outside this repository.
- Their examples are doctests, so they must compile and pass. Do not mark
  them `ignore`.
- Browse them with `cargo doc -p sigma-plugin-api --open`.

**Other crates.**
- `sigma-core`, `sigma-render`, `sigma-device` and `sigma-hwr` are also
  fully documented and set `#![warn(missing_docs)]`. Keep them that way.
- `sigma-device`'s `fbink.rs` is built only for ARM, so the desktop checks
  do not see it. Document its public items by hand.
- The app and `hwr-train` are binaries and not fully documented. Document
  what you add or change.

**User-facing docs.**
- User-facing behaviour goes in `README.md`, `docs/` and `CHANGELOG.md`
  (under "Unreleased").
- Keep them in step with the code in the same pull request.

## Tests

**Where tests go.**
- Unit tests sit next to the code in a `#[cfg(test)] mod tests`.
- App behaviour is tested through the emulator device in `app/tests/`.
- Emulator scripts in `tests/emu/` cover longer journeys and produce the
  screenshots in `docs/screenshots/`.

**Bugs.** A fixed bug gets a regression test named after what it checks.

**Determinism.**
- Tests must be deterministic: fixed seeds, temporary folders named per
  process, no network.
- Some tests assert time budgets, such as handwriting latency and the graph
  layout. Do not loosen a budget to make a test pass. Find out why it got
  slower.
- `#[ignore]` is only for tests that print measurements, with a reason
  string.

**Data.**
- Plugin fixtures in `app/tests/fixtures/` are built from the plugins
  repository by `scripts/update-plugin-fixtures.sh`. Do not edit them by
  hand.

## Repository folders

The workspace this checkout lives in has its own rules: work branches are
`git worktree`s named `sigma-scribe.wt-<topic>`, and folders are archived,
never deleted. See the `CLAUDE.md` one level up.
