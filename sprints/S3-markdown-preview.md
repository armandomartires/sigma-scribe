# S3 — Markdown preview

**Status:** 🔨 BUILT 2026-10-21 (awaiting on-device check) · **Release:** v0.3.0 · **Estimate:** 5–6 d · **Depends on:** S2.6 (v0.2.3) · **Branch:** `feat/v0.3-preview`

## Goal
Read a note rendered (brief §4 "Preview mode", step 3 of §11): eye ↔ pencil in the title bar, the page-turn buttons page through it, and nothing in write mode changes.

## Scope (v0.3.0)
- **Toggle.** An eye button left of the Files folder in write mode; in preview it becomes a pencil (brief §3: "eye in write mode, pencil in preview"). Esc also leaves preview. Opening Files from preview leaves preview.
- **Rendered with the real faces** (mock `.preview` CSS, scaled ×1.139):
  - Headings `#` `##` `###` (and deeper, shown as `###`): Literata 600 at 1.45 / 1.22 / 1.05 em, line height 1.25, 0.4 em below.
  - `**strong**` / `__strong__` → Literata 600; `*em*` / `_em_` → Literata Italic; `~~strike~~` → 1 px line through.
  - Lists `-` `*` `+` and `1.` / `1)`, nested by indent (2 spaces a level), 1.2 em per level, hanging indent on wrapped lines; bullets • (even levels) / – (odd levels); numbers as typed.
  - Task lists `- [ ]` / `- [x]`: 14 px box, outlined or filled, text after an 8 px gap.
  - Block quote `>`: 2 px ink bar, muted ink, 0.8 em inset; consecutive `>` lines are one quote.
  - Inline `code` and fenced ```` ``` ```` / `~~~` blocks: Atkinson Hyperlegible at 0.86 em on the wash tone (the mock's UI face; no mono face is added); code keeps its leading spaces and wraps.
  - Web links `[label](url)`: label underlined. They do not open (no browser on the device in v0.3).
  - Horizontal rule `---` / `***` / `___`: 1 px ink across the column.
  - Blocks are 0.8 em apart (list items in one list are not).
- **Wiki links** `[[note]]` / `[[note|label]]` are **shown**: the label, solid underline when a note of that name exists, dashed and muted when it does not (as the mock). Tapping does nothing; navigation, creating missing notes, backlinks and the graph are **S6**.
- **Paging.** The preview is cut into pages of the page column (keyboard hidden). Next / Prev page-turn buttons move one page; at either end they do nothing. A page never starts with block spacing and a heading is never left alone at the bottom. Status bar: `N words · preview · page i of n` (`· page …` only when there is more than one page), right side `Paper · saved`, no Keys button.
- **Read-only (decision S3-D1).** In preview, typing, taps on the page, double-taps, holds and drags do nothing; the file is not written; the editor's caret, selection and scroll are unchanged when going back.
- **Keeps the place.** Entering preview opens the page that shows the source line at the top of the write page.
- **E-ink.** Entering and leaving preview: one full (GC16) refresh. Page turns: partial refresh of the changed regions only (never full-screen flashes).
- **Unchanged:** all v0.2.3 behaviour (editor, keyboards, touch selection, Files sheet, refresh-region padding, non-fatal refresh failures).

### Out of scope for v0.3 (brief §4 items moved)
- Tables and images: a table is shown as its text; an image as `[image: alt]` in muted ink. Drawing them: later sprint (S7 polish unless asked sooner).
- Tapping a task box to toggle it (would write the file — conflicts with read-only), wiki navigation, backlinks, graph → S6.
- Preview in Night theme / other faces → S4 (preview already takes the theme and type settings).
- Off-thread parsing: not needed at measured speeds (see S3-T04); revisit if the Forma check is slow.

## Decisions (recorded in plan.md §10)
- **S3-D1** Preview is read-only in v0.3. The brief's open question "should a tap on preview drop into write at that line?" is answered *no* for v0.3; revisit after on-device use.
- **S3-D2** Small dependency-free parser in `sigma-core` instead of pulldown-cmark: the brief's subset is small, we need `[[wiki]]` links and source line numbers, unmatched markers must stay literal, and core stays free of dependencies. (pulldown-cmark would also have built statically for ARM; not needed.)
- **S3-D3** Faces: Literata SemiBold (600) and Literata Italic (400) are added as static instances of the OFL Literata variable fonts (google/fonts `ofl/literata`), as the mock uses; no bold-italic face (strong wins). Code uses Atkinson Hyperlegible, as the mock.
- **S3-D4** Page indicator `· page i of n` added to the status bar (not in the brief) so paging is visible on e-ink.

## Deliverables
- v0.3.0 package `dist/sigma-scribe-0.3.0-kobo/` (binary sha256 `d86b3d5a9abefb95d55b6ac124ffbb307539744dcadce848615376168ba18716`)
- Screenshots `docs/screenshots/v0.3-*.png`; script `tests/emu/v0.3-preview.script` with fixture `tests/emu/fixtures/markdown-tour.md`

## Definition of done
- Unit tests: parser (every construct, nesting, literals, escapes, fuzz, 100 KB speed), layout (faces, decorations, wrapping, indents, pagination, place keeping, drawing), screen (eye/pencil hits, no keyboard in preview), app (read-only, full refresh on toggle, partial on paging, Files from preview); all emulator scripts; ARMv7 build — CI green
- On-device checklist below passes

## Test checklist
### Desktop / simulator
- [x] `cargo test` (core 59, render 26, device 5, app 10)
- [x] `tests/emu/v0.3-preview.script` and all earlier scripts
- [x] Screenshots compared with the mock's preview styles
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Eye opens preview with one full flash; the seed note looks like `docs/screenshots/v0.3-seed-preview.png`
- [ ] Headings, bold, italic, lists, tasks, quote, code, links, rule are crisp in the real faces
- [ ] Next / Prev page through a long note with partial refreshes; no ghosting storm; ends do nothing
- [ ] Typing / tapping the page in preview changes nothing; pencil returns to write with the caret where it was
- [ ] Entering preview on a long note is instant enough (< 1 s)
- [ ] Editor, keyboards, selection, Files work as in v0.2.3

Tasks: `tasks/S3-tasks.md`
