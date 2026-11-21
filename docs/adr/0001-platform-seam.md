# ADR 0001 — Platform seam: Cargo features and Paths

- **Status:** Accepted. The Cargo-feature half landed in P2-01. The
  `Paths` struct, device-layer constructors, and the `Platform` trait
  land in P2-02. Moving Kobo path literals out of `core` and `render`
  remains P2-03.
- **Date:** 2026-11-21 (Europe/Brussels)
- **Plan items:** [P2-01](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/app/-/issues/10)
  (this MR), [P2-02](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/app/-/issues/11),
  [P2-03](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/app/-/issues/12).
  Related: [P2-04](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/app/-/issues/13),
  [P2-05](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/app/-/issues/14).
- **Plan:** [2026-11-17-cross-platform-remediation.md](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md)
  (Rec 8). Tracking: [dev#1](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/issues/1).

## Context

Σscribe is a Markdown notebook for the Kobo Forma. The portable crates
(`sigma-core`, `sigma-render`, `sigma-hwr`, the plugin crates) should
build for other targets, but "is this a Kobo?" is encoded as
`cfg(target_arch = "arm")`. That is 32-bit ARM only, so Android and iOS
(`aarch64`) already skip the Kobo backend — yet any 32-bit ARM Linux
build, including a second e-ink device, would get FBInk, evdev, and
`/mnt/onboard` paths. A second ARM device cannot be built side by side.
`CONTRIBUTING.md` said "There are no Cargo feature flags."

This record audits every such switch and every hard-coded Kobo path, and
sets the shape of Phase 2.

## Audit (as of `4f0ffd3` on `main`)

### Platform switches (`target_arch = "arm"` standing in for "is Kobo")

These are the switches P2-01 replaces with `cfg(feature = "kobo")`:

| Location | What it selects |
|---|---|
| `app/src/main.rs` (5 `cfg`s) | The `window` module; `KoboDevice` vs the minifb window; `KoboWifi` vs `FakeWifi` |
| `app/src/bootstrap.rs` (2 `cfg!`) | Default notes and log directories (`/mnt/onboard/...` vs `target/emu-*`) |
| `app/src/sync.rs` (2 `cfg!`) | Default Syncthing install directory; `kill_by_name` (device `killall`) |
| `app/Cargo.toml` | `minifb` only on `not(target_arch = "arm")` |
| `crates/device/src/lib.rs` | Compiles the `fbink` module |
| `crates/device/src/wifi/mod.rs` | Compiles `wifi::kobo` and `log_line` |
| `crates/device/src/fbink.rs` | Docs only ("built only for `target_arch = "arm"`") |
| `crates/device/build.rs` | Links `libfbink.a` and the FFI shim when `CARGO_CFG_TARGET_ARCH` is `arm` |

Not platform switches, left alone:

- `#[cfg(unix)]` in `crates/core/src/replace.rs` and in Unix-only tests
  (`app/tests/sync_settings.rs`, `app/tests/wifi.rs`,
  `crates/core/tests/sync_supervisor.rs`): POSIX behaviour, not Kobo.
- CI target triples (`wasm32-unknown-unknown`, `aarch64-linux-android`,
  `x86_64-pc-windows-gnu`, `armv7-unknown-linux-musleabihf`): they name
  a compiler target, not a product.

### Hard-coded device paths

The plan's count (44 references in 15 non-test files; 6 of those in
core and render) still holds in spirit. The non-test Rust files that
embed Kobo or device absolute paths are:

**Must leave `sigma-core` and `sigma-render` (P2-03):**

| File | Paths |
|---|---|
| `crates/core/src/locale.rs` | `/mnt/onboard/.kobo/Kobo/Kobo eReader.conf` (`KOBO_CONF`, also the default of `conf_path()`) |
| `crates/core/src/plugins.rs` | `/mnt/onboard/.adds/sigma-scribe/plugins` (default of `plugins_root()`) |
| `crates/core/src/ja.rs` | `/mnt/onboard/.adds/sigma-scribe/dict/skk-jisyo.txt` |
| `crates/core/src/zh.rs` | `/mnt/onboard/.adds/sigma-scribe/dict/` |
| `crates/core/src/library.rs` | `/mnt/onboard/sigma-scribe/notes` in the module docs only (the type already forbids absolute paths) |
| `crates/render/src/fonts.rs` | `/mnt/onboard/.adds/sigma-scribe/fonts/cjk.ttf`, `.../korean.ttf`, and the desktop fallback `/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc#0` |

**App shell, will take a `Paths` value once it exists (P2-02 / P2-03):**

| File | Paths |
|---|---|
| `app/src/bootstrap.rs` | `/mnt/onboard/sigma-scribe/notes`, `/mnt/onboard/.adds/sigma-scribe/log` |
| `app/src/sync.rs` | `/mnt/onboard/.adds/sigma-scribe` |

**Kobo implementations, stay in the device layer (the Kobo `Paths` /
wifi / input backends may read them):**

| File | Paths |
|---|---|
| `crates/device/src/light.rs` | `/sys/class/backlight` |
| `crates/device/src/usbkbd.rs` | `/proc/bus/input/devices` |
| `crates/device/src/gsensor.rs` | `/proc/bus/input/devices`, `/dev/input/eventN` |
| `crates/device/src/fbink.rs` | `/dev/input/eventN`, `/proc/bus/input/devices`, mount of `/sys/kernel/debug` |
| `crates/device/src/wifi/kobo.rs` | `/mnt/onboard/.adds/sigma-scribe`, `/var/run/wpa_supplicant`, `/sys/class/net/<iface>` |
| `crates/device/src/wifi/wpa.rs` | `/var/run/wpa_supplicant/eth0` in docs |
| `crates/device/src/otg.rs` | `/sys/kernel/debug/ci_hdrc.0/role` |

**Out of scope for the seam:**

- `crates/core/src/sync/mod.rs` opening `/dev/urandom` (portable Unix,
  not a Kobo path).
- `/etc/passwd` in core tests, used only as a path that must be
  rejected.
- `/tmp/none` in render review tests (a dummy root).
- `/proc/<pid>` in Unix supervisor tests.
- Packaging and docs (`packaging/kobo/`, `scripts/`, `docs/`).

Environment overrides already exist for several of these
(`SIGMA_NOTES`, `SIGMA_LOG_DIR`, `SIGMA_PLUGINS`, `SIGMA_KOBO_CONF`,
`SIGMA_FONT_PACK`, `SIGMA_SKK_DICT`, `SIGMA_ZH_DICT`, `SIGMA_SYNCTHING`,
`SIGMA_SYNC_HOME`) and stay, applied in the constructors, not in core
defaults.

## Decision

### 1. Cargo features (P2-01)

A `kobo` feature on `sigma-device` and `sigma-scribe` only. No feature
flags on `sigma-core`, `sigma-render`, `sigma-hwr`, `sigma-plugin-api`,
or `sigma-plugin-host`.

On `sigma-scribe`:

- `default = ["desktop"]`
- `desktop` — pulls in `minifb` and builds the live window
  (`scripts/sim.sh`).
- `kobo` — enables `sigma-device/kobo` (FBInk + evdev, real Wi-Fi) and
  selects Kobo default paths and `kill_by_name` in the app shell.

The two features are mutually exclusive: enabling both is a
`compile_error`. The Kobo package is built with

```sh
cargo build --release --locked --target armv7-unknown-linux-musleabihf \
  -p sigma-scribe --bin sigma-scribe --no-default-features --features kobo
```

`scripts/build-armv7.sh` (and therefore `scripts/package-kobo.sh`,
GitLab `package:kobo`, and the GitHub ARMv7 / release jobs) pass those
flags. The default host build remains the desktop window and the
emulator. `sigma-emu` and the tests use default features and never
enable `kobo`.

On `sigma-device`, `kobo` is an empty feature that gates `fbink`,
`wifi::kobo`, `log_line`, and the FBInk link step in `build.rs`
(`CARGO_FEATURE_KOBO`). The feature can be turned on for any target
triple; linking still needs `FBINK_DIR` and a cross toolchain. A future
aarch64 Kobo, or a second 32-bit ARM device that is *not* a Kobo, can
be selected independently of `target_arch`.

#### Why not a `headless` feature

The plan suggested names `kobo`, `desktop`, and `headless`. Desktop vs
headless is already two binaries in the same package (`sigma-scribe`
and `sigma-emu`), not two backends that need to be compiled out of each
other. A package-level `headless` feature would not pull `minifb` out
of `sigma-emu` (features are per-package) and would complicate
`--all-targets` checks. `headless` is therefore not a feature.

### 2. `Paths` and `Platform` (P2-02, P2-03)

Core and render must not own filesystem roots or spawn processes. They
receive paths from the caller.

**`Paths` is a concrete struct**, not a trait, living in `sigma-core`
(a portable crate, no Kobo literals, no process spawning):

```rust
pub struct Paths {
    pub notes: PathBuf,
    pub log: PathBuf,
    pub adds: PathBuf,        // install root (`.adds/sigma-scribe` on a Kobo)
    pub locale_conf: PathBuf, // Nickel conf, or a dummy path on desktop
}

impl Paths {
    pub fn plugins(&self) -> PathBuf { self.adds.join("plugins") }
    pub fn fonts(&self) -> PathBuf { self.adds.join("fonts") }
    pub fn dict(&self) -> PathBuf { self.adds.join("dict") }
}
```

Constructors live outside core:

- `sigma_device::paths::kobo()` — the `/mnt/onboard/...` layout, after
  environment overrides.
- `sigma_device::paths::desktop()` — `target/emu-notes`,
  `target/emu-log`, `target/emu-sync`, and no Nickel conf unless
  `SIGMA_KOBO_CONF` is set.
- Tests and `sigma-emu` already pass explicit roots; they keep doing so.

The six core/render files take a `&Paths` (or a single `Path` they
already accept, with the default coming from `Paths`). A CI grep or a
unit test fails the build if `/mnt/onboard`, `/sys/class`, `/dev/input`,
or `wpa_supplicant` return to `crates/core` or `crates/render`.

**`Platform` is a small trait**, also in a portable crate (core, or a
thin `sigma-platform` if core should not mention wifi/sync process
control), for the rest of Rec 8:

- filesystem roots via `paths(&self) -> &Paths`
- wifi radio (power, scan, join, forget)
- sync-process control (start/stop Syncthing; no `killall` or
  `ifconfig` in the portable layer)

The Kobo implementation stays in `sigma-device` (and the Kobo app
shell). Desktop and headless implementations do not shell out to
`ifconfig` / `killall` unless they are clearly desktop-only adapters.
The existing `Device` trait stays the drawing and input surface; it is
not widened to own paths or wifi.

This ADR does not introduce a new crate yet. P2-02 / P2-03 can add
`sigma-platform` if putting `Platform` in `sigma-core` starts to drag
wifi/sync types into the editor core; the first cut should try to keep
`Paths` in core and `Platform` next to the device layer, with only a
narrow trait (or callbacks) crossing the boundary.

## Implementation (P2-02)

- `sigma_core::paths::Paths` is the concrete struct above.
- `sigma_device::paths::{kobo, desktop, current}` apply the environment
  overrides (`SIGMA_NOTES`, `SIGMA_LOG_DIR`, `SIGMA_KOBO_CONF`).
- `sigma_device::platform::Platform` owns `paths()`, `wifi()`, and
  `after_sync_kill()`. `Kobo` and `Desktop` implement it.
  `platform::current()` follows the `kobo` Cargo feature.
- The Syncthing supervisor no longer calls `killall`. It runs a
  `Config::after_kill` function pointer supplied by the platform
  (Kobo: `killall -q syncthing`; desktop: no-op). Starting the bundled
  binary (`Command::new(&cfg.binary)`) stays in the supervisor: that is
  a configured path, not a Kobo-specific process layout. Moving that
  spawn behind the trait can wait if a later target cannot host a child
  process at all.
- The app shell (`notes_dir`, `log_dir`, Syncthing's install root, and
  the Wi-Fi service in `main`) goes through `paths::current` /
  `platform::current`. The six core/render files still have their own
  defaults until P2-03.


## Consequences

- A second ARM target can be compiled without the Kobo backend, and the
  Kobo backend can be compiled for a future aarch64 device without
  pretending `target_arch = "arm"`.
- Host `cargo test` / `cargo clippy` still do not type-check `fbink.rs`
  or `wifi/kobo.rs`. The `package:kobo` job (and later, if wanted, an
  ARMv7 clippy job with `--features kobo`) remains the check. Same
  coverage as today.
- `cargo build --features kobo` on the host, without
  `--no-default-features`, is a compile error (do not silently pull in
  `minifb` and FBInk together). Packaging scripts must use the flags
  above.
- Moving paths out of core (P2-03) will change function signatures and
  needs its own reviewable MR, stacked on this one.
- P2-04 (wasm32 / aarch64 `cargo check` of the portable crates) is
  already largely in GitLab CI from P1-03 (`check:wasm32`,
  `check:android`, `check:windows`). Those jobs must keep building the
  portable crates with the `kobo` feature off. A cheap
  `aarch64-unknown-linux-gnu` check can be added in the P2-04 follow-up.
  iOS still needs a macOS runner; the job is documented as a follow-up,
  not added here.
- P2-05 (extracting `sigma-app` as a library) waits until the seam is
  stable.

## Rollback

Revert the MR. The previous `cfg(target_arch = "arm")` switches and the
target-specific `minifb` dependency return. No on-device state is
touched.
