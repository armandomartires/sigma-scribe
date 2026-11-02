# S22 — Three fixes and Search across all notes

Board: Sprint 10. Planned 2026-11-02 from a read-only review of `main` (fb7e836) after the Journal merge (#110). Status: PLANNED, not started. Tasks: `.ai/tasks/S22-tasks.md`. Handover for the session that builds it: `.ai/handover-s22-2026-11-02.md`.

**Version numbers are open.** `Cargo.toml` is 1.15.0, `CHANGELOG.md` Unreleased holds the 1.16 (write panel), 1.17 (journal) and Wi-Fi items, and "v1.18" is already the Wi-Fi feature's name in code comments (`app/src/wifi.rs:1`, `lib.rs:167`, `events.rs:103`). The user decides the numbering before S22.0 is tagged; this file says "S22.0" and "S22.1" and the emulator script, screenshot folder and version lines below are placeholders to rename then.

Two parts, two branches, two PRs:

- **S22.0 — fixes** (`fixes-s22`): three defects found by the review, each with a regression test. Small, independent of each other, mergeable one by one if preferred.
- **S22.1 — Search across all notes** (`search-all-notes`): the Find bar gets a scope, "This note" or "All notes", and a paged results list. First new feature since Journal.

## S22.0 scope

### F1 — `[[note#heading]]` links never resolve and create stray notes
- `crates/core/src/links.rs:37` `resolve()` does not strip `#fragment`; `rewrite()` at `links.rs:88` does, and `CHANGELOG.md` lists `[[note#part]]` among the kept forms.
- `app/src/nav.rs:79` `follow_link` therefore falls through to `new_note_place` and `create_note`, writing an empty `chapter-01#Intro.md` and opening it. The Graph and backlinks show the link as broken.
- Fix: strip `#…` in `resolve`, so `Graph::from_texts` (`links.rs:198`, `:204`) and `follow_link` agree with `rewrite`. Two more places see the raw target and need the same strip: `new_note_place` (`links.rs:142`, else a missing `[[new#h]]` still creates `new#h.md`) and the preview's `known` set (`app/src/preview.rs:33`, built from `note_names()` at `nav.rs:11` by exact match, else the link still draws as missing). The parser already splits `|label` (`markdown.rs:333-338`), so `resolve` never sees a label; the alias form in `rewrite` is `note#h|label`. Decide what a bare `[[#heading]]` does (same note; today it would try to create a note). Out of scope: jumping to the heading in preview; a tap opens the note at its top. That is a follow-up (see below).

### F2 — On-screen Ctrl shortcuts are dead on every non-Latin layout
- `crates/core/src/keyboard.rs:726`: `Key::Char(c) if ctrl => self.shortcut(c.to_ascii_lowercase(), …)`. On the Full board `c` is the layout's own glyph (`full_rows`, `keyboard.rs:383`: `self.lang.chars(i).0`), so on Ru, Ua, Gr, Ko, Ar, Fa, Ur, He, Deva, Bn, Ta, Th, Ja, Zh the match in `shortcut` (`keyboard.rs:823`) never hits.
- The USB path is right: `crates/device/src/usbkbd.rs:274` uses `KeyLayout::shortcut_char(code)` (`layouts.rs:992`), which falls back to the US letter of the same key.
- Fix: in the Ctrl arm, find the `CODES` index `i` whose `self.lang.chars(i).0 == Some(c)` and call `self.lang.shortcut_char(CODES[i])`; fall back to `c.to_ascii_lowercase()` when there is no such key (punctuation). Only the Full board has a Ctrl key (`Key::Ctrl` at `keyboard.rs:403`; Compact and Kobo-style rows have none), so the arm never fires elsewhere. The only existing test, `ctrl_shortcuts_clipboard_undo` (`keyboard.rs:1024`), runs on US only.

### F3 — Back after a rename or delete resurrects a phantom note
- `app/src/files.rs:254` `finish_rename` and the delete branch at `files.rs:523` update `self.current` but never `self.history` (`app/src/lib.rs:279`, `HistEntry` in `app/src/nav.rs:32`).
- `go_back` (`nav.rs:50`) opens the stale path; `open()` (`lib.rs:462`) does not check existence and `FsNoteStore::load` (`crates/core/src/store.rs:88`) maps NotFound to `Ok(None)`, so the user gets an empty **writable** note under the old name; typing recreates the file beside the renamed one.
- Not a problem: deleting the last note. `Library::trash` refuses it (`library.rs:592`, `:617`, "the last note cannot be deleted"), so the `next == None` branch at `files.rs:526` is unreachable.
- Fix: one helper that remaps history after a rename or move (exact and `within`-prefix rewrite, as `finish_rename` does for `current`) and prunes it after a delete; `go_back` skips entries whose note no longer exists (`Library::exists`, `library.rs:318`). Precedent to factor out: `reorganise_journal` in `app/src/settings.rs:229-240` already remaps `h.note` and does `history.retain(|h| h.note == *cur || lib.exists(&h.note))`; Journal should use the new helper too. The move is performed in `FilesHit::Open(i)` when `f.moving` is set (`files.rs:423-461`, `lib.move_to` at `:429`, `current` remap at `:437-447`); `FilesHit::Move` (`:496`) only arms it.

## S22.1 scope — Search across all notes

Today Find is "in the note" only (`docs/user/getting-started.md:22`; `crates/core/src/find.rs`, `crates/render/src/findbar.rs`, `app/src/find.rs`). With Journal (a note a day), Sync (notes from the phone) and handwriting feeding text in, "where did I write that" is now the main gap on a device with no desktop search. There is no cross-note search anywhere in `crates/core` or `app` (checked 2026-11-02).

### Behaviour
- The Find bar gets a **scope toggle** at its left end: **This note** | **All notes**. Default: This note. The toggle is remembered for the session, not saved in settings (S22-D2).
- In **All notes**, typing in the query field searches the whole library (notes under the root, not `.trash/`, not `.ink/`, not hidden files) and shows a **results list** in place of the page: one row per match line, `note path · line n` above the matched line with the hit in bold, grouped by note in library order, paged like the Graph list ("N matches in M notes · page i of n"). Replace is disabled in this scope (S22-D3).
- A tap on a row opens that note in write mode at that line (caret on the match, page scrolled to it) and pushes Back history, like a Graph list row. Back returns to the results with the same query, scope and page.
- Case follows the Find bar's existing option (the only one: `FindState.case`, `app/src/find.rs:259-266`; there is no whole-word option). Empty query: empty list with the note count ("Type to search 312 notes").
- Back from an opened result returns to the results with the same query, scope and page. This needs a design: `go_back` (`nav.rs:50-62`) calls `close_find()` unconditionally and `HistEntry` derives `Eq`; the search state has to live in the entry (or beside it) and `go_back` has to reopen the bar.
- Search runs **off the UI thread** (design brief: never block input) through the same worker pattern as the to-do `Indexer` (`crates/core/src/outline.rs:198`): a request carries root + query + options, the result is polled each frame; a newer request supersedes an older one. A results cap (first 500 matches) keeps a one-letter query cheap; the status line says "first 500 shown".
- Keyboard: Enter in All notes = jump to the first result; Up/Down (USB) move the selection; Esc closes as today.
- Strings go in `assets/i18n/<lang>.txt` for the nine catalogs (fr, pt, de, es, it, nl, ca, da, sv; English is the key and has no file), machine-drafted as before; native review stays deferred.
- Layout constraints the Graph list does not have: the results list must sit inside the editor layout's page (`l.page`) with the on-screen keyboard up while a query is typed; `graph_layout` is a full-screen sheet with its own title and close button and cannot be copied as is. The bar is drawn only in the editor branch of `app/src/render.rs:154-170`, and `app/src/events.rs:307` routes only taps inside the bar, so row taps and page turns (`:301`) need new arms. In All scope `query_changed`, `select_match` and `find_label` (`app/src/find.rs:56`, `:165`) must stop moving the open buffer's selection. The worker reads from disk, so save the open note before each request (or overlay the live buffer as `Graph::build` does).

### Not in scope (S22.1)
- Regular expressions, fuzzy matching, searching ink that was never recognised, a persistent index on disk, search inside `.trash/`.
- Replace across notes (S22-D3).
- A separate Search screen or drawer pane: the Find bar is the entry point (S22-D1).

## Decisions
- S22-D1: cross-note search extends the Find bar with a scope instead of adding a screen or a pane. One entry point, no new title-bar button (the bar is already full at 200 % landscape), no plugin ABI change.
- S22-D2: the scope is session state, not a setting. Opening Find always starts on This note.
- S22-D3: Replace is for one note only. Replace across notes without a preview of each change is a data-loss risk on a device with no undo across files; revisit after the feature has been used.
- S22-D4: search is a live scan, not a stored index. The to-do indexer already reads every note on each request; whether that is fast enough on the Forma for a per-keystroke search is **not measured** (`docs/dev/perf.md` has graph and preview timings only), so T15 measures it; a stored index is a follow-up if too slow.
- S22-D5: `[[note#heading]]` opens the note at the top in S22.0; jumping to the heading is a follow-up once search-to-line (S22.1) exists, because both need the same "open note at line" step.
- S22-D6: the three fixes ship as one PR before the feature, so the feature branch starts from a `main` that has them (F3's history work and S22.1's open-at-line touch the same code).

## Gate
- S22.0: `cargo fmt`, `clippy -D warnings`, `cargo doc`, `cargo test --workspace` green; the four new tests fail on `main` and pass on the branch; `CHANGELOG.md` Unreleased lists the three fixes; installed on the Forma with the usual routine and a Ru Ctrl+C, a `[[note#h]]` tap and a rename-then-Back tried by the user.
- S22.1: same checks; `tests/emu/<version>-search.script` with screenshots in `docs/screenshots/<version>/`; `docs/user/getting-started.md` Find row and a new `docs/user/search.md`; wiki resynced (`scripts/wiki-sync.sh`); measured on the Forma on the test-notes pack (`.ai/sprints/S10.1-test-notes-pack.md`): a three-letter query over the pack returns in under 2 s; typing stays responsive while it runs; table in `docs/dev/perf.md`.

## Follow-ups found by the same review (not in S22)
- Plugin `Op::Text.size` and shape sizes reach the rasteriser unclamped (`crates/render/src/plugin_screen.rs:118` → `fonts.rs:285`; `graphmap.rs:443` loops over `r.h`); a broken plugin can ask for a multi-GB glyph on a 512 MB device. Clamp in the host before painting; test an oversized frame.
- Plugin fuel default 20 000 000 000 (`crates/plugin-host/src/lib.rs:72`) on the UI thread: a runaway plugin freezes e-ink for minutes before the trap. Measure wasmi on the Forma and cut the default to seconds.
- Wi-Fi: a new key typed for a saved network is set before the handshake and not restored on failure (`crates/device/src/wifi/wpa.rs:481-521`); reachability from the UI not shown.
- `rewrite` and `outline::prose_lines` track code spans and fences differently from the parser (`links.rs:61-80` vs `markdown.rs:109-132`, `:317-327`): a stray backtick can leave one link stale after a rename.
- Case-only rename can overwrite on a case-sensitive filesystem (`library.rs:487-495`); not reachable on the Kobo's vfat.
- Wi-Fi password field: a tap outside the field submits the half-typed key (`editor.rs:1222`, "blur confirms" is right for rename, questionable for a password).
- Review coverage gaps: `crates/render/src/{preview,files,ink,wifi_menu}.rs`, `crates/core/src/sync/{model,json}.rs` were not read.
