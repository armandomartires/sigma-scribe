# S3 tasks — Markdown preview (v0.3.0)

High level; acceptance made specific when S3 starts. Status: all TODO.

### S3-T01 — Markdown parser → AST in core
- Headings, emphasis/strong, strike, lists, task lists, quotes, rules, code, links, `[[wiki]]` links (as text for now), tables, images (alt text). Pure core, unit-tested; parsing a 10 k-word note under budget on the Forma.

### S3-T02 — Preview renderer in sigma-render
- Literata regular/bold/italic, heading scale, list bullets, checkboxes, quote bar, code in a mono face (new OFL font), page-turn paging.

### S3-T03 — Eye / pencil toggle in the title bar
- Preview is read-only; task checkboxes tappable (writes `[x]` back to the file). Full refresh on toggle.

### S3-T04 — Performance + emulator script + screenshots `v0.3-*.png`, on-device check.
