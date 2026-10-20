# S5 — Side drawer

**Release:** v0.5.0 · **Estimate:** 4–5 d · **Depends on:** S4

## Goal
Contents, To-dos, Scratch drawer (step 6).

## Scope
- Drawer left/right, ≤48%/scaled 320 px, no animation
- Contents pane with jump-to-line
- To-dos across all files, All/Open/Done filter, toggle rewrites line
- Scratch pane backed by hidden undeletable `.scratchpad.md`
- Background worker for outline/todo indexing

## Deliverables
- v0.5.0
- Tests: todo line rewrite, scratch persistence

## Definition of done
- 1–3 panes split height correctly; todos from scratch included
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Toggle a todo → file on USB shows `[x]`
- [ ] Scratch survives note switch and reboot
- [ ] Drawer side setting honoured

Tasks: `tasks/S5-tasks.md`
