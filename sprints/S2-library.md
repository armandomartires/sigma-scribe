# S2 — Library

**Release:** v0.2.0 · **Estimate:** 4–5 d · **Depends on:** S1

## Goal
Files sheet: folders and notes (brief step 2).

## Scope
- Folder/note model in core
- Files sheet: enter, up crumb, open, More → rename/move/armed delete
- New note / new folder (starts in rename)
- Name cleaning, last-note protection, word counts
- Notes stored as real files for USB export

## Deliverables
- v0.2.0 on device
- Model tests (create/rename/move/delete rules)

## Definition of done
- All file operations persist and appear on USB mount as .md/.txt
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Create nested folder, move note into it, rename, delete (armed)
- [ ] Last note can't be deleted
- [ ] Files visible over USB after exiting the app

Tasks: `tasks/S2-tasks.md`
