# S14 — v1.4: Interface languages (i18n) (draft)

> **DRAFT — planning only (updated 2026-10-21 21:24 with the user's answers). No development until the user approves.**

**Scope:** UI in **English, Français, Português** (PT-PT; PT-BR variants where they differ). Options → Language (explicit choice; default English, or the Kobo's language if we can read it from Nickel's config — open question).
**String catalog:** **Fluent** (`fluent-bundle`, Apache-2.0/MIT, pure Rust): `.ftl` files per language embedded in the binary; handles plurals ("1 note / 2 notes"), variables, gender-free phrasing; English is the source and the fallback for missing keys. (gettext considered: fine too, but plural/variable handling in Fluent is cleaner and it needs no C libintl.)
**Work:** extract every UI string (sheets, Help pages, Markdown reference, status, errors, emulator labels stay English IDs), a pseudo-locale (+40 % length, accents) in tests to catch overflow at UI scale 200 %, date/number formats (word counts with thin-space thousands for FR/PT), Help/Markdown reference translated per language.
**RTL needs (for v1.6, designed now):** a `direction` per language; mirrored layouts for fa/ar: title bar order reversed (title on the right), drawer side default flipped, chevrons/arrows flipped, text alignment right, page-turn semantics unchanged; numbers stay LTR inside RTL text. v1.4 builds the mirroring switch and tests it with a pseudo-RTL locale; real fa/ar strings ship in v1.6.
**Who supplies translations:** EN by us; FR and PT first drafts by Grok Bot, reviewed by Armando (PT/FR) or a native reviewer he names before release; fa/ar (v1.6) need native reviewers — open question. Community contributions later via the `.ftl` files in the repo.
**Estimate:** 4–6 d + review time. **Risks:** low–medium — string expansion at 200 %, Help text volume.
