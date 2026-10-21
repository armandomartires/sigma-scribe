# S3 tasks — Markdown preview (v0.3.0)

Acceptance made specific 2026-10-21 (see `sprints/S3-markdown-preview.md`). Status below.

### S3-T01 — Markdown parser in core ✅
- `sigma_core::markdown::parse` → blocks (heading 1–6, paragraph, list item {depth, number, task}, quote, fenced code, rule) with source line; `inline` → styled runs (strong, em, strike, code, web link, wiki link, image alt).
- Accept: unit tests for each construct; unmatched `*`, `_`, `~~`, `` ` ``, `[` stay literal; intraword `_` is literal; escapes; fuzz (4000 random inputs, no panic); 100 KB note parses in < 0.5 s (debug, desktop).

### S3-T02 — Preview layout and pagination in render ✅
- `sigma_render::preview::{layout, paginate, draw_page}`; faces Literata / Literata SemiBold / Literata Italic / Atkinson; sizes and spacing from the mock CSS.
- Accept: tests for faces per style, underline solid vs dashed (known vs missing wiki note), wrapping within width with hanging indents, overlong words cut, quote/code/rule decorations, pages ≤ column height covering every line, no spacer at page top, no orphan heading, `page_of_line`, 100 KB layout < 3 s debug.

### S3-T03 — Eye / pencil toggle and read-only preview in the app ✅
- Eye button left of Files; pencil in preview; Esc leaves; Files leaves preview; page turns page; everything else ignored (decision S3-D1).
- Accept: app tests — full refresh on entering/leaving, partial (never full-screen) on page turns, ends do nothing, typing/taps/holds change nothing, caret unchanged, file unchanged, entering from the end of a note lands on the last page.

### S3-T04 — Emulator script, screenshots, package ✅ / on-device ✅ (passed 2026-10-21, tagged `v0.3.0`)
- `tests/emu/v0.3-preview.script` (+ fixture), `docs/screenshots/v0.3-*.png`, `dist/sigma-scribe-0.3.0-kobo/`.
- On-device checklist in the sprint file (needs the user; no Kobo writes without approval).

### S3-T05 — Deferred
- Tables and images drawn; task box tap; wiki navigation (S6); Night preview (S4).
