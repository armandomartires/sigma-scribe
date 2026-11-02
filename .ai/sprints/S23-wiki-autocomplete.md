# S23 — Wiki-link autocomplete

Board: Sprint 11. Planned 2026-11-02 with the user, after S22.0 (PR #111) passed on the Forma. Status: DONE 2026-11-02, 1.20.0, merged in PR #125; Forma pass fine (keystroke p50 125 ms with the bar up). Tasks: `.ai/tasks/S23-tasks.md`. Starts after Sprint 10 (S22.1 Search across all notes) is merged: branch `wiki-autocomplete` from that `main`. Line numbers below are from `main` at 8ed3209 plus the `fixes-s22` branch; re-check them when the sprint starts, since S22.1 touches `events.rs`, `editor.rs` and `script.rs`.

**Version is open.** Ask the user when the sprint starts (S22.0 is 1.18.1; S22.1 has no number yet).

## Why
Typing `[[` gives no help today: you have to remember a note's exact name, and a typo makes a new empty note when the link is tapped. Autocomplete for wiki links is the first autocomplete in the app.

## Behaviour (decided with the user, 2026-11-02)
- **When:** in write mode, while the caret is inside an unclosed `[[` on its line (the text before the caret holds `[[` with no `]]` after it, and no `|` yet), a **suggestion bar** shows. It appears **right after `[[`**, before any letter is typed.
- **Notes:** the query is the text after `[[`. With an empty query, the most recently opened notes come first, then library order. Each letter narrows the list: **names that start with the query first, then names that contain it**, ignoring case and accents, matched on the note's file name without extension and on its path. A chip shows the name (`chapter-01`); when two notes share a name it shows and inserts the path (`drafts/chapter-01`).
- **Headings (in this version):** once the query has `#`, the note part is resolved with `links::resolve`, and the chips are that note's headings (`outline::headings`), filtered the same way by the text after `#`. A note that does not resolve shows no bar. A tap on such a link still opens the note at its top until "jump to heading" lands (S22-D5).
- **Picking:** a tap, or Enter / Tab on a USB keyboard (Left/Right move the selection as with the IME bar), replaces the query with the choice and closes the link: `[[chapter-01]]`, `[[chapter-01#Intro]]`. If `]]` already follows the caret it is not doubled. One Undo takes the pick back. To link a heading, type `#` after the name before picking; picking a note closes the link at once.
- **Closing:** Esc (USB) or moving the caret out of the link closes the bar.
- **Never shown:** in preview, rename, Find, the scratch pad, or while the Japanese or Chinese input method shows its own candidates (that bar wins; both use the same strip).
- **No new strings:** the chips are note names and headings.

## Decisions
- S23-D1: suggestions start right after `[[` (not after the first letter), with recent notes first.
- S23-D2: matching is prefix first, then contains; case- and accent-insensitive; no fuzzy matching.
- S23-D3: headings after `#` are in this sprint, not deferred.
- S23-D4: the bar reuses the IME candidate strip in the editor (`cand_bar`, `cand_chips`, `draw_cands`, `cand_reserve`), not a new widget, and not the handwriting bar.
- S23-D5: note names for suggestions come from a name-only walk of the library, cached while the bar is open. `Library::all_notes` reads every note to count words (`crates/core/src/library.rs:349`), which is too slow per keystroke on the Forma.
- S23-D6: the sprint comes after S22.1 Search, on its own branch and PR.

## Not in scope
- Fuzzy matching, `[[note|label]]` label suggestions, Markdown `[text](url)` links.
- Autocomplete for anything other than wiki links (words, tags). This sprint is meant to make that possible later, not to do it.
- Jumping to the heading when a `[[note#heading]]` link is tapped (S22-D5 follow-up).
- Creating a note from the bar ("New note: …" chip).

## Gate
- `cargo fmt`, `clippy -D warnings`, `cargo doc`, `cargo test --workspace` green. The new tests fail on the `main` the branch starts from.
- `tests/emu/<version>-wiki-complete.script` with screenshots in `docs/screenshots/<version>/`.
- `CHANGELOG.md` Unreleased, the wiki-links section in `docs/user/`, wiki resynced (`scripts/wiki-sync.sh`).
- Installed on the Forma with the usual routine. The user tries `[[` plus a few letters, a heading pick and typing with the bar up. Keystroke-to-refresh p50 in the log's `perf:` line stays near today's 116 ms (2026-11-02, 1.18.1 build).
