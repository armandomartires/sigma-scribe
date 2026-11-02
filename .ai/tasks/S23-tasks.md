# S23 tasks — Wiki-link autocomplete

Status: planned 2026-11-02, nothing started. Sprint file: `.ai/sprints/S23-wiki-autocomplete.md`. Estimates are desktop days; the Forma pass is extra. `<version>` is a placeholder until the user picks the number. Line numbers come from a read-only map of `main` (8ed3209) on 2026-11-02; re-check them after S22.1 is merged.

Order: T01 → T02 → T03 → T04 → T05, one branch (`wiki-autocomplete`), one PR.

### S23-T01 — Core: link query and suggestions
- `crates/core/src/links.rs` (or a new `complete.rs` beside it if it grows): `pub fn link_query(before_caret: &str) -> Option<(usize, &str)>`, giving the byte offset where the query starts and the query itself. It is `None` outside an open `[[`, after a `|`, or once `]]` has closed the link. It takes the line's text up to the caret (`Buffer::line_start`, `buffer.rs:468`).
- `pub fn suggest_notes(query, notes: &[NotePath], recent: &[NotePath], cap) -> Vec<Suggestion>`, where `Suggestion { label, insert }`. It folds case and accents, puts prefix hits before contains hits, puts recent notes first when the query is empty, and uses the path as label and insert when two notes share a name.
- `pub fn suggest_headings(text, query, cap)` over `outline::headings` (`outline.rs:104`).
- `Library::note_paths()` (`library.rs`): the same walk as `all_notes` (`:395`, through `folders` `:369` and `list` `:327`) but **without reading files** (`list` reads each note to count words, `:349-351`). Skip the same paths: `.trash`, `.ink`, hidden.
- Tests: `link_query` (open link, closed link, after `|`, two links on one line, inside a code span); `suggest_notes` (prefix before contains, accents, a shared name gives the path, cap, recent first on an empty query); `suggest_headings`; `note_paths` gives the same set as `all_notes`.
- **Estimate:** 0.5 d

### S23-T02 — Render: the suggestion bar on the IME strip
- `crates/render/src/editor.rs`:
  - Add `Model.suggest: Option<(&[String], usize)>`. `Model` (`:18`) is built at `app/src/render.rs:136`, `app/src/editor.rs:223` and `app/src/events.rs:~380`.
  - `draw_cands` (`:1040`) takes its chips from the composer when it has any, else from `suggest`. Reuse `cand_bar` (`:998`), `cand_chips` (`:1003`) and `cand_reserve` (`:291`, used by `text_area` at `:925`) so the caret line is never hidden behind the bar.
  - Add `Hit::Suggest(usize)` beside `Hit::Candidate` (`:94`), resolved in `Screen::hit` next to it (`:1198`).
- Tests: the bar fits at UI scale 100–200 % in both orientations; hit regions map to chips; the IME's candidates win when both are present.
- **Estimate:** 0.5 d

### S23-T03 — App wiring
- New `app/src/suggest.rs`:
  - `App.suggest: Option<SuggestState>` holds the query span, the chips, the selection and the cached note paths.
  - `refresh_suggest()` is called from `after_key` (`app/src/keyboard.rs:103`), from the end of the USB key arm (`events.rs:375`) and after caret moves (taps on the text, arrows). It returns early when `buffer.revision()` and the caret have not changed.
  - It is off in preview, rename, Find and the scratch pad, and while `composer.candidates()` is `Some` (`layouts.rs:1651`).
  - The cached paths are dropped when the bar closes and on create, rename, move, delete and sync.
- Recent notes: `.sigma-last-opened` and the Back history (`App.history`). No new file.
- `pick_suggest(i)`: `Buffer::select(start, caret)` (`buffer.rs:289`), then `insert(choice + "]]")` (`:363`), one undo step. Skip the `]]` when the text after the caret already starts with `]]`. Then `save()` and `Refresh::Partial`. Use `Refresh::Fast` while the bar only changes its chips.
- Taps: a `Hit::Suggest(i)` arm beside `Hit::Candidate` (`events.rs:436`). USB: Left/Right, Enter, Tab and Esc beside the IME arrows (`events.rs:311-319`).
- Emulator: control names `Suggest0`, `Suggest1`, … in `control_centre` and `touch_targets` (`app/src/script.rs:42`, `:392`), like `Cand0` (`:82`), so the minimum-target checks cover them.
- Tests in `app/tests/suggest.rs`, with a `Rig` like `app/tests/history.rs`:
  - `[[ch` shows `chapter-01`.
  - Tapping `Suggest0` gives `[[chapter-01]]` with the caret after it, and one Undo gives back `[[ch`.
  - `[[chapter-01#` shows heading chips, and picking one gives `[[chapter-01#Intro]]`.
  - An existing `]]` after the caret is not doubled.
  - Esc and moving the caret away close the bar.
  - No bar in preview, rename, Find, or with Japanese candidates showing.
  - USB Enter and Tab pick.
  - An empty query lists recent notes first.
- **Estimate:** 1 d

### S23-T04 — Emulator, docs, performance
- `tests/emu/<version>-wiki-complete.script` (type `[[`, then a few letters, screenshot, pick, screenshot; `[[note#`, screenshot headings, pick) with screenshots in `docs/screenshots/<version>/`. Pattern: `tests/emu/v1.10.0-japanese.script`. Fixtures: `tests/test-notes/_test`.
- `docs/user/`: the wiki-links section gets the autocomplete; rerun `scripts/wiki-sync.sh` after the merge.
- Performance on the test-notes pack (`.ai/sprints/S10.1-test-notes-pack.md`):
  - Time `note_paths` and one `suggest_notes` call on the desktop and the Forma.
  - On the Forma, keystroke-to-refresh with the bar up, from the log's `perf:` line, against the 116 ms p50 measured 2026-11-02.
  - Record both in `docs/dev/perf.md`.
- **Estimate:** 0.5 d + device time

### S23-T05 — Wrap
- `CHANGELOG.md` Unreleased; `Cargo.toml` to the agreed version; `.ai/plan.md` §11 row; board.
- Full checks, PR, install on the Forma with the routine in memory (`kobo-install-routine`), the user's pass. Read the device logs after the pass.
- **Estimate:** 0.25 d

Total ≈ 2.75 d plus a Forma pass.
