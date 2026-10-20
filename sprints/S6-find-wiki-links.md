# S6 — Find & wiki links

**Release:** v0.6.0 · **Estimate:** 5–6 d · **Depends on:** S5

## Goal
Find/replace and wiki links, backlinks, graph (steps 7–8).

## Scope
- Find bar: count, prev/next, replace, all, Aa; seed from selection
- Wiki-link index, missing-note creation, backlinks
- Graph sheet (map or list — decision in S6 planning)

## Deliverables
- v0.6.0
- Find/replace + link index tests

## Definition of done
- Find/replace correct incl. case; wiki links navigate/create
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Replace All across 50 matches
- [ ] Tap missing [[note]] creates it
- [ ] Backlinks list correct

Tasks: `tasks/S6-tasks.md`
