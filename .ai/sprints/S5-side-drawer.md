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
- Side-by-side check against `.ai/mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Toggle a todo → file on USB shows `[x]`
- [ ] Scratch survives note switch and reboot
- [ ] Drawer side setting honoured

Tasks: `.ai/tasks/S5-tasks.md`

## As built (2026-10-21, v0.5.0 — install pending approval)
- `core::outline`: headings/to-dos (code fences skipped), byte-exact `toggle`, `scan` of all notes + `.scratchpad.md`, `Indexer` background thread (numbered requests, newest wins).
- `render::drawer`: panes, layout, hit-testing, empty states, icons; width min(48% page, 320 px); 1 px rules between panes.
- App: title-bar icons (scratch, to-dos, contents left of the eye, then `Aa`), Partial refresh on open/close, Fast for scratch typing, Full when opening another note.
- Tests: core outline (5), render drawer (3), app (3: cross-file toggle, scratch persistence/undeletable, regions ≥16 px), emulator script `tests/emu/v0.5-drawer.script`. Screenshots `docs/screenshots/v0.5/` (+ `eink/`).

### Decisions
- S5-D1: drawer is write-mode only; hidden in preview.
- S5-D2: lists that do not fit end with "N more…" (no scrolling yet).
- S5-D3: open panes are not persisted (not in the brief's persistence list).
- S5-D4: the open note and scratch are read live from their buffers; other notes come from the background indexer; toggling in another note writes atomically and reindexes.

## User approval (2026-10-21, 12:40 CEST)
Approved without device testing (never installed): v0.5 defaults stay (no drawer in preview, "N more…", panes not remembered); missing `[[link]]` → new note next to the linking note; known link → preview; Graph stays a list. Installs with v0.7 (0.4.0 → 0.7.0, pending approval).
