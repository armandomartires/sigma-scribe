# S9 — Release & Store

**Release:** v1.0.0 · **Estimate:** 3–5 d · **Depends on:** S8

## Goal
Package, sign via Cobalt Store pipeline, publish (step 11).

## Scope
- Final regression on Forma
- cobalt-app.json, README + screenshots for Store
- `node tools/app-contribute.mjs --dry-run` clean
- PR to BandarLabs/Cobalt `beta`; beta smoke; stable promotion
- Install/rollback docs final
- Store metadata: licence AGPL-3.0, author Sigma Recon, public screenshots
- Finalise Forma device report (post only with user approval)

## Deliverables
- v1.0.0 in Cobalt Store (beta → stable)
- Release notes

## Definition of done
- App installs from Store on the Forma and updates cleanly
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Install from Store over Wi-Fi
- [ ] Update from previous version keeps notes
- [ ] Remove app → stock reader and notes files intact

Tasks: `tasks/S9-tasks.md`
