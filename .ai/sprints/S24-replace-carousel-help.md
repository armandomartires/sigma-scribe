# S24 — Replace in all notes, screenshot carousel, help centre

Board: Sprint 13. Planned 2026-11-03 from `main` 7a7cd4f (1.20.0). Status: PLANNED, not started. Open decisions are marked **(ask)**; nothing is built until the user has answered them.

Three parts, independent of each other, three branches, three PRs. They touch disjoint files, so they can be built in parallel worktree sessions:

| Part | Branch | Touches | Ships as |
|---|---|---|---|
| S24.1 Replace in all notes | `replace-all-notes` | `crates/core`, `crates/render`, `app/`, `assets/i18n/`, `tests/emu/`, `docs/user/search.md` | app release (1.21.0) |
| S24.2 Screenshot carousel | `site-carousel` | `site/index.html`, `site/README.md` | Pages deploy, no release |
| S24.3 Help centre | `help-centre` | `docs/user/` (restructure), `scripts/`, `site/help/`, `.github/workflows/pages.yml` | Pages deploy, no release |

Order constraint: S24.1's user-doc change (`docs/user/search.md`) and S24.3's restructure of `docs/user/` collide. Merge S24.3 first and let S24.1 rebase, or S24.1 first and S24.3 moves the updated page. Either works; pick whichever is ready first.

"The gitlab page" is read as the project page on GitHub Pages (`site/index.html`, published by `.github/workflows/pages.yml`); the repository has no GitLab remote. **(ask)** if a page on the self-hosted GitLab is meant instead.

---

## S24.1 — Find and replace in all notes

### Today
- Find bar has a scope, This note | All notes (S22.1). All notes shows a paged results list (`crates/core/src/search.rs`: `search_notes`, `Searcher` worker; `app/src/find.rs`). Results are capped at 500 hits.
- Replace works in This note only: `find_replace_one` / `find_replace_all` return early in All scope (`app/src/find.rs:129`, `:147`), and the greyed field says "Replace: This note only" (12.7).
- S22-D3 kept it that way: replace across notes without a preview is a data-loss risk with no undo across files. This part lifts D3 by adding exactly those two things.

### Behaviour
1. In All notes the Replace field is enabled. Typing a replacement changes nothing on disk.
2. **Replace all** in All notes opens a **review sheet** in place of the results list: "Replace “lamp” with “lantern”: 52 matches in 3 notes", then one row per note with its match count and a checkbox (all ticked), and under each note its matched lines with the change shown (old struck, new bold), paged like the results list. Buttons: **Replace in N notes** and **Cancel**.
3. **Replace one** (the single-match button) in All notes replaces the match of the selected result row only, in that note, and moves to the next row. **(ask)** keep it, or offer only the reviewed Replace all.
4. The apply step rescans **without the 500 cap** (the review counts must be the real ones; if the uncapped count differs from the reviewed one because a note changed meanwhile, the sheet is refreshed instead of applying).
5. Applying runs on a worker, not the UI thread, one note at a time; the status line counts "Replacing… 2 of 3 notes". Each note is written temp-file-then-rename so a power cut leaves either the old or the new file.
6. The open note is not written behind the editor's back: its replacement goes through `buffer.replace_text` as one undo step, like the This-note Replace all, and is then saved.
7. **Undo**: before a note is rewritten its original goes to `.trash/replace-<yyyymmdd-hhmmss>/<note path>` with a small manifest (path, hash of the text written). After applying, a toast-style bar says "Replaced 52 in 3 notes · Undo". **Undo last replace** stays available from the Find bar menu (and the Trash screen lists the snapshot like any trashed set) until the next all-notes replace. Undo restores a note only if its current hash still matches what the replace wrote; notes edited since are listed and left alone.
8. Results, Graph, backlinks and the to-do index refresh after apply (the same invalidation a rename does).

### Edge cases to settle in the design
- **Links**: a match inside `[[…]]` changes a link target. Default: such matches are shown in the review with a "link" marker and are unticked by default; ticking them renames nothing, it only edits the text. **(ask)** alternative: skip link targets entirely.
- **Ink**: `.ink/` sidecars are not searched or touched; an ink image line (`![](.ink/…/ink-NNN.svg)`) must not match inside its path. Search already skips `.ink/`, but the image line itself is in the note text: exclude Markdown image/link URLs from matches in All-notes replace.
- **Code spans and fences**: replaced like any text (it is a text replace, not a Markdown one); say so in the docs.
- **Journal template** `journal/.template.md` and other hidden files: not searched today, not replaced.
- **Sync**: many notes change at once; Syncthing sends them as normal edits. A conflict copy created during apply is a separate note and is not touched by this run.
- **Case** follows the existing case option; no regex, no whole word (out of scope, as in S22).
- Empty replacement deletes the matches; the review shows that plainly ("delete 52 matches").
- Very large runs: cap the review at what the list pages show, but apply to all ticked notes; measure on the test-notes pack (303 notes): apply under 5 s for a three-letter query, UI responsive.

### Work
- T01 core: `replace_plan(root, query, case, with) -> Plan` (per note: path, ranges, preview lines, link/url flags), uncapped; `apply(plan, ticks) -> Applied` with snapshot, temp-rename writes, hash manifest; `undo(snapshot) -> Undone`. Unit tests: crash-safe write, undo refuses changed notes, URL and image paths excluded, link matches flagged, open-note path excluded from disk writes.
- T02 render: review sheet (rows, checkboxes, struck/bold preview, paging) in `l.page` with the keyboard up, as the results list does.
- T03 app wiring: enable Replace in All scope, review sheet events, worker polling, open-buffer path, refresh after apply, Undo bar and menu item, Back from the sheet returns to the results.
- T04 strings in the nine catalogs; emulator script `tests/emu/v1.21.0-replace-all.script` (search, review, untick a note, apply, undo); screenshots `docs/screenshots/v1.21/`.
- T05 user docs (`search.md` gains "Replace in all notes" and "Undoing a replace"), changelog, version 1.21.0, PR, Forma pass with a backup first, timing line in `docs/dev/perf.md`.

### Decisions
- S24-D1: replace across notes only through a review sheet; no one-tap blind replace (supersedes S22-D3).
- S24-D2: undo is a file snapshot in `.trash/`, not an in-memory undo, so it survives a restart and a crash, and empties with the trash.
- S24-D3: the plan is recomputed uncapped at apply time; the 500 cap is a display limit only.

---

## S24.2 — Screenshot carousel on the project page

### Today
`site/index.html` shows 13 screenshots in a two-column grid (`.gallery`, lines 61–64, 101–118), each 1440×1920 except `landscape.png` (1920×1440). The page is one static file, inline CSS, **no JavaScript** (`site/README.md`). The Pages workflow copies `docs/screenshots/readme/readme-*.png` in and fails if an `src="screenshots/…"` is missing.

### Design
- One slide visible at a time, the screenshot at the page's text width (height capped at about 80 % of the viewport on desktop, full width on a phone), caption under it as today ("Write", "Preview", …), and the long alt text kept.
- Slide counter "3 / 13" and Previous / Next buttons; a strip of small thumbnails (or dots) under it, current one outlined.
- Swipe on touch, arrow keys when focused, no autoplay (e-ink-looking screenshots, nothing should move by itself), `prefers-reduced-motion` turns off the smooth scroll.
- Landscape slide letterboxed to the same frame height so the layout does not jump.
- Accessibility: `role="region"` + `aria-roledescription="carousel"`, each slide `role="group"` `aria-label="3 of 13: Search all notes"`, buttons with labels, focus stays on the button.
- Light and dark as the page already themes (`--rule` etc.).

### Implementation choice **(ask)**
- **A (recommended): CSS scroll-snap track + ~40 lines of inline JS.** Without JS the track is a horizontally scrolling, swipeable strip with snap points: still a carousel, nothing lost. JS only adds the buttons, the counter, the thumbnail highlight and arrow keys. Changes the "no JavaScript" line in `site/README.md` to "no JavaScript needed; a small inline script adds the carousel controls".
- B: pure CSS (radio inputs / `:target`). Keeps "no JavaScript" literally, but `:target` jumps the page scroll and radio hacks are awkward for screen readers.

### Work
- T10 markup and CSS: replace `.gallery` with the track; keep every `src="screenshots/…"` attribute literal so the workflow's missing-image check still works.
- T11 script (if A), keyboard and touch, reduced motion.
- T12 check in the built-in browser at 375 px and desktop, light and dark, no JS; update `site/README.md`; PR; confirm on the live page after the Pages run.

---

## S24.3 — User documentation as a help centre (structure like help.boox.com)

### The reference
help.boox.com/653d4 (User Guide, E Ink Tablets): header with logo and search; a guide split into 12 **sections** (Set Up Your BOOX Tablet, Optimize Speed and Display, Split Screen, Reading, Note-Taking, Stylus, Apps, File Transfer, Data Syncing and Backup, Screenshot and Screen Recording, Troubleshooting and Customer Service, Lab Features); a **left sidebar** tree of the sections; a **section page** with a breadcrumb (Guide › Section) and a flat list of short task articles (e.g. "Firmware Updates", "WiFi and Bluetooth", "Sleep and Battery"); **article pages** one task each, with a table of contents. Their "Ask AI" panel is not copied.

### Today
`docs/user/` has 13 pages (595 lines), flat, indexed by `docs/user/README.md`; `scripts/wiki-sync.sh` turns them into the GitHub wiki. Pages are feature-sized, not task-sized (`getting-started.md` covers title bar, files, links, preview, settings and quitting in one page).

### Proposed sections and articles (first cut) **(ask: confirm or edit)**
1. **Set up Σscribe** — Install; Update; Uninstall; First start; Get your notes off the Kobo
2. **Find your way around** — The title bar; The drawer and its panes; Files and folders; Settings overview; Quit and restart
3. **Write and edit** — The editor and caret; Select, copy, paste; Undo; Preview; Line and Paper modes; Night and light
4. **Links and the Graph** — Wiki links; Link autocomplete; Backlinks; The Graph (map and list)
5. **Find, search and replace** — Find in a note; Search all notes; Replace in all notes (S24.1); Undo a replace
6. **Journal** — Today's note; The Calendar; Template; Folder layout and UTC offset
7. **Handwriting** — The Write panel; Modes and word mode; Training; Gestures
8. **Keyboards and languages** — On-screen keyboards and layouts; App language; Japanese; Chinese; Right-to-left; Font pack
9. **USB keyboard and mouse** — Connect; Shortcuts; Export
10. **Wi-Fi and Sync** — Join a network; Sync with Syncthing; Conflicts
11. **Plugins** — Switch on and off; Install and remove; Plugin settings
12. **Troubleshooting** — The log file; Common problems; Report a bug; Experimental features

### Source layout
- `docs/user/` stays the single source. New layout: `docs/user/<nn>-<section>/<article>.md`, each article with a short front block (`title`, `summary`, `order`) and one task per page; `docs/user/README.md` stays the index (sections and articles), so GitHub browsing still works.
- Existing pages are split, not rewritten; screenshots from `docs/screenshots/` reused, new ones taken with the emulator where an article has none.
- `scripts/wiki-sync.sh` learns the nested layout (one wiki page per article, the sidebar grouped by section). **(ask)** keep the wiki at all once the help centre exists, or point the wiki Home at it.

### Site
- Built at Pages time into `_site/help/` by `scripts/help-build.py` (Python 3 stdlib plus the `markdown` package installed from pip in the workflow at a pinned version; it is not a Rust dependency, so `cargo deny` does not see it).
- Pages: `help/index.html` (search box, section cards with their article titles), `help/<section>/index.html` (breadcrumb, article list with summaries), `help/<section>/<article>.html` (breadcrumb, sidebar tree with the current section open, on-page contents from `##` headings, Previous / Next article, "Edit this page on GitHub" link, last-changed date from git).
- Search: a JSON index (title, summary, headings, text) built with the pages; a small inline script filters it as you type. Works offline, no service. Without JS the search box is hidden and the section cards remain.
- Same look as `site/index.html` (Literata, its colour tokens, dark mode); the project page's "Documentation" section links to `help/`.
- The workflow's missing-image check extends to `help/`; a link check over the built pages fails the job on a dead internal link.

### Work
- T20 information architecture: final section and article list agreed with the user; mapping table old page → new articles.
- T21 split `docs/user/` into the new layout (content moves, no rewrites beyond headings and joins); README index.
- T22 `scripts/help-build.py`: pages, sidebar, breadcrumbs, contents, prev/next, search index; local preview command in `site/README.md`.
- T23 Pages workflow: build help, image and link checks.
- T24 `scripts/wiki-sync.sh` for the nested layout; resync the wiki.
- T25 new screenshots where an article has none; check in the browser at phone and desktop width; PR; confirm live.

---

## Gate
- S24.1: the four CI checks green, emulator script and screenshots, undo tested after a restart, Forma pass with backup, timing in `docs/dev/perf.md`, changelog.
- S24.2: page checked at 375 px and desktop, light and dark, with JS off; Pages run green.
- S24.3: every old `docs/user` page's content present in an article (mapping table ticked); Pages run green with image and link checks; wiki resynced.

## Not in scope
- Regex or whole-word replace; replace inside `.trash/`, hidden files or ink.
- Translating the help centre (English only, as `docs/user/` is today).
- An AI answer panel like Boox's.
