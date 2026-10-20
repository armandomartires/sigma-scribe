# S3 — Markdown preview

**Release:** v0.3.0 · **Estimate:** 5–6 d · **Depends on:** S2 (v0.2.0)

## Goal
Rendered preview with eye ↔ pencil (brief step 3).

## Scope
- Markdown parser → AST in core (headings, emphasis, lists, tasks, quote, rule, code, images, links, tables, strike)
- Renderer on sigma-render (real fonts, bold/italic faces)
- Preview toggle
- Off-UI-thread parsing for long notes

## Deliverables
- v0.3.0
- Parser unit + fuzz tests, golden render tests

## Definition of done
- Every syntax in brief §4 renders; 100 KB note opens < 1 s
- CI green (fmt, clippy, tests, ARMv7 build); docs updated; CHANGELOG entry
- Side-by-side check against `mockup/kobo-writing-mock.html` (scaled to 1440×1920)

## Test checklist
### Desktop / simulator
- [ ] `cargo test` for new core logic
- [ ] Simulator run at Forma metrics, portrait and landscape
### On-device (Kobo Forma, fw 4.38.23697)
- [ ] Seed notes render like the mock
- [ ] Toggle write↔preview keeps scroll position
- [ ] Long note does not block input

Tasks: `tasks/S3-tasks.md`
