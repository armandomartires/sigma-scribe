# Guidance for agents (and people) working on Σscribe

Start with [`CONTRIBUTING.md`](CONTRIBUTING.md). That file is the
source of truth for how to build, test, and change this repository.
This page is a short index so an automated agent can find the rules
quickly.

## What this repository is

Σscribe is a Rust Markdown notebook for the Kobo Forma. GitLab
(`sigmarecon/products/sigma-scribe/app`) is canonical; GitHub is a
push-mirror only. Do not push straight to GitHub.

The crate map and the Phase 2 platform seam are in
[`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md).
Architecture decision records are in [`docs/adr/`](docs/adr/README.md).
The cross-platform remediation plan lives in the
[dev repository](https://gtlb.sigmareconpt.duckdns.org/sigmarecon/products/sigma-scribe/dev/-/blob/main/docs/plans/2026-11-17-cross-platform-remediation.md).

## Commits and changelog

New commits use Conventional Commits: `type(scope): subject` (scope
optional). The `lint:commits` job, and `python3 scripts/check-commits.py`
locally, check only the commits on this branch against `main`. Existing
history is not rewritten. Details are in `CONTRIBUTING.md`.

User-visible changes go under `## Unreleased` in `CHANGELOG.md`, in the
Keep a Changelog headings (`### Added`, `### Changed`, `### Fixed`,
…). `git cliff --unreleased` (see `cliff.toml`) can draft that section
from Conventional Commits; review the draft before committing it.

## Checks

Run the checks in `CONTRIBUTING.md` before opening a merge request:
`cargo fmt`, `clippy -D warnings`, rustdoc `-D warnings`, and
`cargo test --workspace --locked`. The desktop build is not evidence
that the Kobo build works; that is `scripts/build-armv7.sh` (the
`kobo` feature) and the `package:kobo` job.

## House rules that often get missed

- One branch per change; open a GitLab merge request against `main`.
  Do not push to `main` or other protected branches, do not
  force-push, do not merge, and do not create tags or releases,
  without Armando's explicit approval in his own words.
- Development is emulator-first. Do not install on a device or write
  to a Forma without a separate, explicit approval, a
  checksum-verified backup, and a serial check. Mask device serials
  as `<FORMA-SERIAL>`.
- Licences of new dependencies must already be on the allow list in
  `deny.toml` (compatible with AGPL-3.0-only).
- `sigma-plugin-api` is a contract with plugins built elsewhere.
  Changing the encoding means bumping `ABI` and updating
  `docs/dev/plugin-api.md` and the plugins repository.

`CLAUDE.md` at the repository root is a short pointer at this file and
at `CONTRIBUTING.md`, so existing tools that look for `CLAUDE.md` still
load the rules.
