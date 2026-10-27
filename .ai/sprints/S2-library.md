# S2 — Library

**Release:** v0.2.0 · **Estimate:** 4–5 d · **Depends on:** S1

## Goal
Files sheet: folders and notes, as plain files under `/mnt/onboard/sigma-scribe/notes` (brief step 2).

## Scope (v0.2.0)
- Library model with safe paths (no escapes, hidden files ignored), `.md`/`.txt` only
- Files sheet: folders first, word counts, enter / up crumb, open into the editor
- More → Rename / Move / Delete with armed confirmation; folder delete removes contents
- New note / new folder (start in rename); rename in place from the title bar
- Name cleaning and collision suffixes; last real note cannot be deleted
- Seed library on first run only; last-opened note restored on launch
- v0.1 editor/OSK/autosave behaviour unchanged

## Deliverables
- v0.2.0 on device
- Model tests (create/rename/move/delete rules)

## Definition of done
- All file operations persist and appear on USB mount as .md/.txt
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `.ai/mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [x] `cargo test` for new core logic (library tests incl. unsafe names, collisions)
- [x] `tests/emu/v0.2-library.script` + v0.1 script pass
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Create nested folder, move note into it, rename, delete (armed)
- [ ] Last note can't be deleted
- [ ] Files visible over USB after exiting the app

Tasks: `.ai/tasks/S2-tasks.md`
