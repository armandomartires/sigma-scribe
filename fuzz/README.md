# Fuzz targets (P3-02)

libFuzzer targets for the three untrusted-input surfaces in the plan:
plugin-api `Frame` and `Event` decoders, `pluginpkg::read_tar` (package
unpacking), and `Settings::parse`.

This directory is its own Cargo workspace. It is not a member of the
main workspace, so `cargo test --workspace` on stable does not build it,
and it does not inherit the main tree's `unsafe_code = "forbid"`.

## Run locally

cargo-fuzz needs a nightly toolchain (libFuzzer / sanitizers). CI pins
`nightly-2026-11-19` and cargo-fuzz 0.13.2; any recent nightly should
do on a desktop:

```sh
rustup toolchain install nightly --component rust-src
cargo install cargo-fuzz --locked --version 0.13.2
cargo +nightly fuzz run plugin_api_frame
cargo +nightly fuzz run plugin_api_event
cargo +nightly fuzz run pluginpkg_tar
cargo +nightly fuzz run settings_parse
```

Crashes are written under `fuzz/artifacts/` (gitignored). File one and
treat it as a release blocker once a baseline is recorded; a crash does
not have to block every merge request before that. Small seeds live in
`fuzz/corpus/<target>/`. Do not check in a large corpus.

## CI

The `fuzz:smoke` job installs the pinned nightly and cargo-fuzz binary,
then runs each target for a few seconds (`-max_total_time=8 -runs=256`).
That is a smoke test, not a long campaign. It uses the same bookworm
image as the other jobs, plus `rustup toolchain install nightly-2026-11-19`.
The job sets `RUSTUP_TOOLCHAIN` to that nightly (so cargo-fuzz's inner
`cargo build` is not pinned back to stable by `rust-toolchain.toml`) and
passes `--target x86_64-unknown-linux-gnu` because the pinned cargo-fuzz
binary is musl and would otherwise try to fuzz on musl, which is
incompatible with AddressSanitizer.
