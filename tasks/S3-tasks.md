# S3 tasks — Markdown preview

Estimates are working days. Status: all TODO.

### S3-T01 — Markdown parser → AST in core (headings, emphasis, lists, tasks, quote, rule, code, images, links, tables, strike)
- **Description:** Markdown parser → AST in core (headings, emphasis, lists, tasks, quote, rule, code, images, links, tables, strike).
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S2 done
- **Estimate:** 1 d

### S3-T02 — Renderer to Cobalt screens
- **Description:** Renderer to Cobalt screens.
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S3-T01
- **Estimate:** 1 d

### S3-T03 — Preview toggle
- **Description:** Preview toggle.
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S3-T02
- **Estimate:** 0.5–1 d

### S3-T04 — Off-UI-thread parsing for long notes
- **Description:** Off-UI-thread parsing for long notes.
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S3-T03
- **Estimate:** 0.5–1 d
