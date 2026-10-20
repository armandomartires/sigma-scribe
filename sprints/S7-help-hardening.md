# S7 — Help & hardening

**Release:** v0.7.0 · **Estimate:** 3–4 d · **Depends on:** S6

## Goal
Help pages and quality pass (step 9).

## Scope
- Help: App info + Markdown reference
- Performance profiling on Forma
- Power-loss/robustness tests
- Touch-target audit (≥44 px scaled)
- Rotation pass

## Deliverables
- v0.7.0
- Perf report in docs

## Definition of done
- Keystroke-to-ink < 150 ms p95; no data loss in fault tests
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Pull power (hold power/force off) during save ×10 → no corruption
- [ ] All controls hittable
- [ ] Both orientations OK

Tasks: `tasks/S7-tasks.md`
