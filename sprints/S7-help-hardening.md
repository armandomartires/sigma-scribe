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
- [x] `cargo test` for new core logic
- [x] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Pull power (hold power/force off) during save ×10 → no corruption
- [ ] All controls hittable
- [ ] Both orientations OK

Tasks: `tasks/S7-tasks.md`

## As built (2026-10-21, v0.7.0 — install PENDING APPROVAL, not tagged)
- Help: `crates/render/src/help.rs`; `?` icon leftmost in the title bar (write and preview); menu → App info / Markdown reference; laid out by the preview engine, paginated; Esc steps back; opening any sheet closes the others.
- Perf: no device access, so measured on the desktop (`docs/perf.md`: p95 ≈ 20 ms) and the app logs p50/p95/max on exit for the next device session.
- Robustness: `app/tests/hardening.rs` — kill during `write_atomic` ×10, unreadable note never overwritten, orientation round trip.
- Touch audit: automated over every sheet in portrait and landscape (Split/Full/Centre); fixed drawer heading rows and Filter.
- Rotation pass: the audit + round-trip test + landscape screenshots.

## Decisions
- S7-D1: Help reuses the preview layout engine; the reference is built in code, not a bundled .md.
- S7-D2: profiling on the Forma is replaced by on-device logging until the device is available.
- S7-D3: "Nickel restart instead of reboot" (plan.md row) is not in this sprint's scope doc and was not done.
- Not done: side-by-side check against the HTML mock.
