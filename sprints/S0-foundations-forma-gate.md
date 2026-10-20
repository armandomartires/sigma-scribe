# S0 — Foundations & Forma gate

**Release:** — · **Estimate:** 5–7 d · **Depends on:** plan approval

## Goal
Toolchain proven end to end and a go/no-go on Cobalt for the Forma.

## Scope
- Full backup of the Forma user partition
- Install Rust + Cobalt CLI + ARM cross compiler on the dev machine
- Cobalt app skeleton `sigma-scribe` (from `templates/app/`), workspace layout core/ui/app
- CI (fmt, clippy, test, ARMv7 build)
- Forma 1440×1920 simulator profile/metrics
- Install Cobalt + NickelMenu on the Forma (USB KoboRoot.tgz)
- Read-only doctor + attended device tests
- Spikes: text input/caret, measured text, partial refresh, page keys, rotation, raw evdev (USB-OTG)

## Deliverables
- `docs/backup/` record of the backup (checksums)
- Hello-world running on simulator and on the Forma
- `docs/s0-findings.md` with measured results
- Go/No-go decision recorded in plan.md
- Draft Forma device report `docs/cobalt-forma-device-report.md` (not posted without user approval)

## Definition of done
- Backup verified (file count + sha256 manifest match)
- Hello-world launches from NickelMenu/Cobalt and returns to Nickel cleanly
- Every spike has a written result
- Decision: Cobalt (Plan A) or FBInk+NickelMenu (Plan B)
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Cobalt shows untested-device notice and runs
- [ ] Touch maps correctly in portrait (and landscape if supported)
- [ ] Both page-turn buttons deliver events
- [ ] Partial refresh of a small rect < 150 ms, no heavy ghosting
- [ ] Exit returns to stock reader; reboot returns to stock reader
- [ ] Recovery procedure tested once (remove Cobalt, device boots to Nickel)

Tasks: `tasks/S0-tasks.md`
