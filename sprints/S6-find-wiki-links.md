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

## As built (2026-10-21, v0.6.0 — install PENDING APPROVAL, not tagged; v0.5 never installed)
- `core::find`: case-insensitive (default) or exact matching on char boundaries, non-overlapping; next/prev with wrap; `replace_all`; count label. `Buffer::replace_text` makes Replace All one undo step. `KeyAction::Find` for Ctrl+F.
- `core::links`: link targets from the Markdown parser (code is not linked), resolution by file name or path, any case, with or without extension; `Graph` (notes, outgoing links, backlinks, edges); `new_note_place` for missing notes.
- `render::findbar`: the bar (fields, close, count, Prev, Next, Replace, All, Aa); `EditorScreen.find_open` moves the page down by the bar height. `render::graph`: the Graph sheet as a list. Preview pieces carry their wiki target; `preview_link_at` hit-tests them; `build_preview_with` appends Backlinks.
- App: Find icon/Ctrl+F, fields take the keyboard (Tab switches field, Enter = Next, Esc closes), tapping the page gives the caret back; Graph sheet; link taps in preview. Refresh: Full when the bar opens/closes or a note/sheet changes, Partial for Prev/Next/Replace/All and query edits, Fast for field caret moves.
- Tests: core find (5) and links (4); render find bar, graph and preview links (3); app (2: Replace All across 50 + undo + regions ≥ 16 px; missing link creates a note, backlinks, scratch not a node); emulator `tests/emu/v0.6-find-wiki.script`. Screenshots `docs/screenshots/v0.6/` (+ `eink/`).

### Decisions
- S6-D1: Graph is a list of notes with → / ← links (the brief allows it on the device); no drawn map.
- S6-D2: wiki links are tappable in preview only (write mode is a plain textarea, brief §4).
- S6-D3: a missing note is created beside the linking note, or in the folder the link names if that folder exists; name cleaned like any new note; it opens in write mode. A known note opens in preview.
- S6-D4: the link index is built synchronously when preview or Graph opens (reads every note; fine for a personal library — can move to the S5 indexer thread if it gets slow).
- S6-D5: the Find bar closes when preview, Files, Options or Graph opens; the query is not kept after closing (brief §8: need not persist).
- Not done: side-by-side check against the mockup (as S4/S5).
