# S1 — Editor core

**Release:** v0.1.0 · **Estimate:** 8–10 d · **Depends on:** S0

## Goal
Single-note editor with on-screen keyboard and autosave (brief step 1).

## Scope
- Text buffer with caret/selection, undo-safe ops
- Page render with tokens, Literata/Atkinson
- Title bar + rename
- Status bar (words, line, theme, Keys)
- Portrait on-screen keyboard + symbol layer + Shift
- Autosave (atomic write) + `saved` state
- Page-turn buttons scroll
- Seed content

## Deliverables
- v0.1.0 package installed on the Forma
- Unit tests for buffer, keyboard mapping, autosave

## Definition of done
- Typing on OSK works with partial refresh
- No data loss across app restart and reboot
- Matches mock (scaled) side by side
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `.ai/mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Type 500 words via OSK without missed keys
- [ ] Caret survives rerender/scroll
- [ ] Kill app mid-typing → last save intact
- [ ] Reboot → note and settings restored
- [ ] Page keys scroll up/down

Tasks: `.ai/tasks/S1-tasks.md`
