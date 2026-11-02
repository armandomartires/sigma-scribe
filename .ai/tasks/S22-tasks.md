# S22 tasks — three fixes, Search across all notes

Status: planned 2026-11-02, nothing started. Sprint file: `.ai/sprints/S22-fixes-and-search.md`. Estimates are desktop days; the Forma pass is extra. Version numbers are open (see the sprint file's head); `<version>` below is a placeholder.

Order: T01 → T02 → T03 → T04 (one PR), then T10 → T16 (one PR). T01–T03 are independent of each other and may be done in any order or in parallel on separate branches, but merge as one (S22-D6).

## S22.0 — fixes (branch `fixes-s22`)

### S22-T01 — `[[note#heading]]` resolves
- `crates/core/src/links.rs`: in `resolve`, split the target on the first `#` before trimming the extension. The parser already splits `|label` (`markdown.rs:333-338`), so `resolve` never sees one. Same strip in `new_note_place` (`links.rs:142`), which gets the raw target from `follow_link` and would otherwise create `new#h.md` for a missing note. `Graph::from_texts` (`:198`, `:204`) goes through `resolve` and needs nothing more.
- `app/src/preview.rs:33`: the `known` set used to draw links as present or missing is built from `note_names()` (`nav.rs:11`) by exact lowercase match; strip `#` there or route it through `resolve`, else `[[note#h]]` keeps drawing as missing.
- Decide `[[#heading]]` (bare fragment): treat as the current note; today it would try to create a note.
- `app/src/nav.rs:76` `follow_link`: no change expected once the two helpers are right.
- Tests: `links.rs` unit tests `resolve("note#h")`, `resolve("folder/note#h")`; `new_note_place` with `new#h` yields `new`; an app test in `app/tests/write_nav.rs` or `history.rs` (that file uses a private `Rig` with a temp dir and `fs::write` of `a.md`/`b.md`/`c.md`, `:13-29`): tap `[[chapter-01#Intro]]` in preview opens `chapter-01.md` and creates nothing.
- **Estimate:** 0.4 d

### S22-T02 — On-screen Ctrl shortcuts on non-Latin layouts
- `crates/core/src/keyboard.rs:726`: resolve the shortcut letter through the layout: find `i` with `self.lang.chars(i).0 == Some(c)` over `CODES` order (`layouts.rs:136`, `chars` at `:955`), then `self.lang.shortcut_char(CODES[i])` (`:992`); fall back to `c.to_ascii_lowercase()`. Consider a small `KeyLayout::shortcut_for_char(c)` next to `shortcut_char` so the USB path and the on-screen path share it.
- Only the Full board has a Ctrl key (`Key::Ctrl` at `keyboard.rs:403`; Compact and Kobo-style rows have none), so nothing changes on those boards.
- Tests: next to `ctrl_shortcuts_clipboard_undo` (`keyboard.rs:1024`) add the same flow on `KeyLayout::Ru`. Ru's bottom row is `ячсмитьбю.` (`layouts.rs:743`), so on the Z, X, C, V keys: Ctrl+я undoes, Ctrl+ч cuts, Ctrl+с copies, Ctrl+м pastes; take A and F from the layout's home and top rows the same way. One test on `Ar` and one on `Ko` (Hangul composer: Ctrl must bypass composition; `self.composer.forget()` is already in the arm). A `layouts.rs` test for the new helper if added.
- **Estimate:** 0.4 d

### S22-T03 — Back history after rename, move and delete
- `app/src/nav.rs`: `fn remap_history(&mut self, from: &NotePath, to: &NotePath)` (exact match and `within` prefix, same rule as `finish_rename` uses for `current` at `files.rs:257-266`, so a folder rename or move remaps every entry under it) and `fn prune_history(&mut self)` that drops entries whose note no longer exists (`Library::exists`, `library.rs:318`). `reorganise_journal` in `app/src/settings.rs:229-240` already does both inline (`h.note = to.clone()` and `history.retain(|h| h.note == *cur || lib.exists(&h.note))`): factor it onto the new helpers.
- `go_back` (`nav.rs:50`): pop until an existing note or an empty stack; the title-bar Back slot follows `history.is_empty()` (`render.rs:95`, `events.rs:373`, `keyboard.rs:35`), so either prune eagerly at the change sites (preferred) or return `None` when nothing valid is left.
- `app/src/files.rs`: `remap_history` from `finish_rename` (`:254`) and from the move, which happens in `FilesHit::Open(i)` when `f.moving` is set (`:423-461`, `lib.move_to` at `:429`, `current` remap at `:437-447`; `FilesHit::Move` at `:496` only arms it); `prune_history` after the delete at `:523` and after Trash → Empty if it can remove paths in history (restore needs nothing).
- Not needed: a last-note case. `Library::trash` refuses the last note (`library.rs:592`, `:617`), so `next == None` at `:526` is unreachable.
- Tests in `app/tests/history.rs` (private `Rig`, temp dir, `fs::write` of `a.md`/`b.md`/`c.md`, `:13-29`): open A, open B, rename A → A2, Back lands on A2 and no `a.md` exists after typing; open A, open B, delete A, Back goes to the entry before A (or the Back slot disappears) and no `a.md` reappears; move A into a folder, Back opens `folder/a.md`.
- **Estimate:** 0.5 d

### S22-T04 — S22.0 wrap
- `CHANGELOG.md` Unreleased: three lines, user-facing wording ("Tapping `[[note#heading]]` now opens the note…", "Ctrl shortcuts on the Russian, Arabic… boards", "Back after renaming or deleting a note").
- Version: ask the user. `Cargo.toml` is 1.15.0, Unreleased holds 1.16, 1.17 and the Wi-Fi items, and "v1.18" already names Wi-Fi in code comments; do not pick a number alone.
- Full checks; PR; install on the Forma with the routine in memory (`kobo-install-routine`); the user tries the three cases; board and `.ai/plan.md` table row updated.
- **Estimate:** 0.25 d

## S22.1 — Search across all notes (branch `search-all-notes`, from `main` after S22.0)

### S22-T10 — Core: library search
- `crates/core/src/find.rs` (or a new `search.rs`): `pub struct SearchHit { note: NotePath, line: usize, byte: usize, len: usize, text: String }` and `pub fn search_notes(root, notes: &[NotePath], query, case: bool, cap) -> Vec<SearchHit>`. `find::matches(text, query, case) -> Vec<(usize, usize)>` (`find.rs:10`) returns byte ranges over the whole text: run it on the whole note and derive the line from the offset, so `byte` is already the absolute caret offset for open-at-line. There is no whole-word option anywhere; `case` is the only flag. Skip `.trash/`, `.ink/`, hidden parts (`NotePath::parse` already rejects them; use `Library::all_notes`); unreadable or non-UTF-8 notes skipped and counted.
- Worker: `outline::Indexer` (`outline.rs:198`) is not generic: `request(root: PathBuf)`, `pending()`, `poll() -> Option<Vec<TodoItem>>`, `wait()`. Add a sibling `Searcher` with the same shape whose request carries root, query, case and cap, and a request id so a newer request's result wins. The worker reads from disk: the app must save the open note before each request (or the test "results update after an edit" is flaky).
- Tests in `find.rs`/`search.rs`: hits across three notes in library order, cap honoured, case flag, `.trash/` skipped, a non-UTF-8 file skipped, line and byte of a hit on the third line; a `Searcher` test that a second request wins.
- **Estimate:** 0.75 d

### S22-T11 — Find bar: scope toggle
- `crates/render/src/findbar.rs`: the bar is two rows (find and replace fields plus close; the count plus five tool keys). `FindModel.scope: Scope { Note, All }`, `FindHit::Scope(Scope)`, `find_control("All notes")` for the emulator. Where it goes: the count rect left of Prev is the only free spot on the left and it is narrow; the realistic options are (a) a two-segment control in the count's place with the count moved into the status line of the results list, or (b) a third row. Decide with the user before drawing (handover question 1). Whichever, the bar must fit at 100–200 % portrait and landscape: `app/tests/languages.rs` opens Find at `:60` and checks the screens fit in Fr, Pt, pseudo and En, so the label must fit there at 200 %.
- Replace field and Replace keys hidden or greyed in `All` (S22-D3).
- New touch targets must be listed in `touch_targets` (`app/src/script.rs:392`), which `languages.rs` checks.
- **Estimate:** 0.5 d

### S22-T12 — Results list screen
- New `crates/render/src/search.rs`. Borrow the row and paging arithmetic from `graph.rs` (`GraphModel.first`, `visible`, "page i of n" at `:236-247`) but not its layout: `graph_layout` (`:94-137`) is a full-screen sheet built from `layout(false, keyboard)` with its own title and close button, while the results must sit inside the editor layout's `l.page` with the on-screen keyboard up and the Find bar above. Rows of two lines (path · line n; the matched line with the hit bold, trimmed around the hit), 44 px minimum hit height, page arrows, status line "N matches in M notes · page i of n" / "first 500 shown" / "Type to search N notes" / "Searching…" and the singular forms.
- Draw in place of the page while scope is `All` and the Find bar is open; the drawer stays as it is. The bar is drawn only in the editor branch of `app/src/render.rs:154-170`; the list draws there too.
- Tests: layout fits at the four scales and both orientations; hit regions map to rows; empty, searching and capped states render their strings.
- **Estimate:** 0.75 d

### S22-T13 — App wiring
- `app/src/find.rs`: `FindState.scope`, `results`, `first`, `selected`; save the note and request on every query or case change in `All`; poll where the `Indexer` is polled, the idle-tick branch at `app/src/events.rs:174` (`drawer.rs:412` is `reindex`, request plus wait); refresh class on new results. In `All`, `query_changed` (`:165`), `select_match` (`:56`) and `find_label` must not touch the open buffer's selection or scroll.
- Touch: `app/src/events.rs:307` routes only taps inside the bar; add arms for row taps and the page arrows, and keep page turns (`:301`) off the list area.
- Open at line: `fn open_at(&mut self, note, byte)` = `push_history` + `open` + `set_caret(byte)` + `screen.follow_caret` (`crates/render/src/editor.rs:948`, as `select_match` does at `find.rs:66`). `go_back` restores a stored `scroll`, it does not follow the caret.
- Back to the results: `go_back` (`nav.rs:50-62`) calls `close_find()` unconditionally and `HistEntry` derives `Eq` (`nav.rs:97`). Store the search state (scope, query, case, `first`) in the entry as an `Option`, and have `go_back` reopen the bar in `All` and re-request when it is set. Small design, decide before T13 starts.
- Keys: Enter opens the first or selected result; Up/Down move `selected` in `find_hw` (`find.rs:235-241`, returns `None` today) and `find_key` (`:205`); Esc closes the bar and the list.
- Emulator: `tests/emu/<version>-search.script` (open Find, tap All notes, type, screenshot list, tap a row, screenshot note, Back, screenshot list again); add the needed `find_control` names and `touch_targets` entries.
- Tests: `app/tests/search.rs` with a `Rig` like `history.rs`: the flow above; Back returns to the list with the same page; edit a note, re-run the query, results change.
- **Estimate:** 1.25 d

### S22-T14 — Strings and languages
- New entries in the nine catalogs `assets/i18n/{fr,pt,de,es,it,nl,ca,da,sv}.txt` (English is the key, no file): "This note", "All notes", "{n} matches in {m} notes", "1 match in 1 note" and the other singular forms as their own entries, "first {n} shown", "Type to search {n} notes", "Searching…". `line {n}` and `page {n} of {m}` already exist (`fr.txt:46`, `:52`). At most two numbers per string; the " · " split translates each part on its own. Machine-drafted; the completeness test is `catalogs_parse_and_keep_placeholders` in `crates/render/src/i18n.rs:340-364` (every catalog has exactly fr's keys, placeholders match), and `app/tests/languages.rs` checks that the screens fit.
- **Estimate:** 0.25 d

### S22-T15 — Performance on the Forma
- On the test-notes pack (`.ai/sprints/S10.1-test-notes-pack.md`) and a 500 KB note: time `search_notes` for a 1-letter and a 3-letter query, desktop and Forma; record in `docs/dev/perf.md` like the preview table. Nothing there measures the to-do indexer today, so this is the first number for a whole-library read on the device. Target: under 2 s for 3 letters on the Forma, UI responsive meanwhile. If over, the first lever is reading notes once per request in the worker and keeping their text in memory for the next keystroke (same request root, notes unchanged by mtime).
- **Estimate:** 0.25 d + device time

### S22-T16 — S22.1 wrap
- `docs/user/search.md` (what it searches, what it skips, the cap, open-at-line, Back), the Find row in `docs/user/getting-started.md:22`, `docs/user/README.md` index, `scripts/wiki-sync.sh` rerun after merge; screenshots `docs/screenshots/<version>/`; `CHANGELOG.md` Unreleased; `Cargo.toml` to the agreed version; `.ai/plan.md` §11 row and a decision entry pointing at S22-D1…D6; board.
- Full checks, PR, install on the Forma, user pass (search a journal day, a word from a synced note, a word written by hand and inserted).
- **Estimate:** 0.5 d

Total: S22.0 ≈ 1.6 d, S22.1 ≈ 4.3 d, plus two Forma passes.

## Parallel plan for S22.1 (2026-11-02)
S22.0 is in PR #111 (version 1.18.1, chosen there). S22.1 is 1.19.0 unless the user says otherwise: `tests/emu/v1.19.0-search.script`, `docs/screenshots/v1.19/`.

Wave 1, five sessions at once, each in its own worktree from `main`, disjoint files, each ending in its own PR against `main`:
- **core** T10 — `crates/core/src/search.rs` (new) + `Searcher` worker, `lib.rs` module line. No render or app changes.
- **findbar** T11 — `crates/render/src/findbar.rs` only. Placement decided there by mocking at 200 % portrait Fr: (a) the segmented control in the count rect's place with the count moved to the right end of the query field, else (b) a third row.
- **results** T12 — `crates/render/src/search.rs` (new) + `lib.rs` module line. Takes a plain row model (`path: String, line: usize, text: String, hit: (usize, usize)`), no dependency on T10's types.
- **strings** T14 — the nine `assets/i18n/*.txt` only, with the planned English keys.
- **docs** T16 (text part) — `docs/user/search.md`, the Find row in `getting-started.md`, `docs/user/README.md` index. No screenshots yet.

Wave 2, one session after PR #111 and the five wave-1 PRs are merged: T13 app wiring (`app/src/{find,events,render,nav}.rs`, Back-to-results stores the search state in `HistEntry`), emulator script, `touch_targets`, app tests.
Wave 3: T15 on the Forma, T16 screenshots and wiki sync, changelog, version, board.

Reviewed against the code 2026-11-02 (read-only): the three diagnoses hold; the task details above were corrected after that review (Ru letters, last-note delete removed, move branch location, nine catalogs, no whole-word option, poll site, `follow_caret`, `touch_targets`, `docs/dev/perf.md`).
