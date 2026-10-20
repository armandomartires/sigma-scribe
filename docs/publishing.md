# Publishing to the Cobalt Store

Sources: [CONTRIBUTING_APPS.md](https://github.com/BandarLabs/Cobalt/blob/main/docs/CONTRIBUTING_APPS.md), [APP_STORE.md](https://github.com/BandarLabs/Cobalt/blob/main/docs/APP_STORE.md), [DEVICES.md](https://github.com/BandarLabs/Cobalt/blob/main/docs/DEVICES.md), repo licence: AGPL-3.0 ([LICENSE](https://github.com/BandarLabs/Cobalt)). Read 2026-10-20.

## How it works
- Store apps live **inside the Cobalt monorepo** at `apps/<app-id>/` (copied from `templates/app/`). Cargo package must be `kobo-<app-id>`; runtime id passed to `kobo_sdk::run` = `<app-id>` → `sigma-scribe`.
- The only metadata is `apps/sigma-scribe/cobalt-app.json`: `id, display_name, short_label, summary, version, release_notes, glyph, capabilities` (+ optional `setup.steps`, `minimum_cobalt_version` floor).
- Local check: `node tools/app-contribute.mjs --manifest apps/sigma-scribe/cobalt-app.json --dry-run` → validates, fmt/tests/strict clippy, static ARMv7 hard-float build, test-signed package + catalog, generates `docs/apps/sigma-scribe/` page (commit those outputs).
- Open a PR with the app template **against `beta`**. Human review covers product policy: purpose, **licensing**, capabilities, setup requirements, public/demo evidence (README + `screenshots/`).
- After merge, CI builds and **signs** (Ed25519, project key; contributors never sign) and publishes to the Beta catalog. Maintainers then promote tested Beta to Stable with a human approval.
- Updates: bump to a strictly newer version with real release notes ("Bug fixes" is rejected).

## Implications for us
- Our repo `sigma-scribe` stays the dev home; for each Store release we sync the app crate into a fork of BandarLabs/Cobalt `apps/sigma-scribe/` and open a PR. Keep `core` dependency-light so it vendors cleanly.
- Version cadence v0.1 → v1.0 maps to Store versions; each needs release notes.
- Forma is not in the support matrix; consider contributing a Forma device row (probe + attended evidence) in S0/S7.
- Capabilities: offline app; USB-OTG keyboard (raw `/dev/input`) may need a new capability or may not be allowed — raise with maintainers early (S8 risk).

## Decisions
1. ✅ **Licence: AGPL-3.0** (resolved 2026-10-20) — same as the Cobalt monorepo; full text in `LICENSE`. Fonts Literata + Atkinson Hyperlegible are SIL OFL — ship their licence texts alongside.
2. ✅ **Author: Sigma Recon** (resolved 2026-10-20) — shown in Help → App info and on the Store page.
3. ✅ **Public screenshots/demo: allowed** (resolved 2026-10-20) — README + `screenshots/` for the Store listing.
4. Open: display name / short label / summary / glyph (decide in S9).
5. Forma device report (DEVICES.md row / issue): drafted in S0-T11, finalised in S9-T06 — **never posted without user approval**.
