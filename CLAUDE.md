# Σscribe

Rust Markdown notebook for the Kobo Forma. Engineering rules, layout and
conventions are in `CONTRIBUTING.md`; read it before changing code. Folder
rules (worktrees, never delete, archive) are in `../CLAUDE.md`.

Planning and working documents (plan, design brief, sprints, sprint board
`GOALS.md`, device-pass runbooks, handovers) are in `.ai/`; new ones go
there too. `CLAUDE.md` stays at the top level so Claude Code loads it.

## Checks (what CI runs)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo test --workspace
```

`cargo test --workspace` takes several minutes; run it in the background.

## Non-obvious rules

- Kobo-only code is behind `cfg(target_arch = "arm")` and is not compiled on
  the desktop. A green desktop build says nothing about it; the ARMv7 job in
  CI is the check.
- The MSRV is `rust-version` in `Cargo.toml` (1.88), checked by the `msrv`
  job in CI. Do not use newer language or library features, and do not
  change it, or update a dependency that needs a newer Rust, without asking.
- After adding or updating a dependency, run `cargo deny check` (the `deny`
  job in CI); licences must be on the allow list in `deny.toml`.
- `sigma-plugin-api` is a contract with plugins built elsewhere. Changing the
  encoding means bumping `ABI` and updating `docs/dev/plugin-api.md` and the
  plugins repository.
- The emulator (`sigma-emu`, `tests/emu/*.script`) is the end-to-end test
  layer and produces `docs/screenshots/`.
- Add user-visible changes to `CHANGELOG.md` under "Unreleased".
