# S15 — Farsi (RTL) — v1.2 (draft)

> **DRAFT — planning only (2026-10-21). No development until the user approves.**

**Needs:** Arabic-script shaping (contextual forms, ligatures, ZWNJ) → **rustybuzz** (pure Rust HarfBuzz port); bidirectional text (mixed Farsi/Latin/digits) → **unicode-bidi**; a font: **Vazirmatn** (OFL, ~0.4–0.6 MB per weight); Farsi OSK + USB table (ISIRI 9147), Persian digits option.
**Impact:** the editor today assumes left-to-right glyph runs with per-char advance: wrapping, caret movement, hit-testing, selection handles and Find all become bidi/cluster-aware. Preview and Markdown rendering need run shaping. This is a **major editor/render change**.
**Estimate:** 3–4 weeks. **Risks:** high — caret/selection in mixed runs, performance on the Forma (shaping per line, needs caching), e-ink partial regions per visual run. Arabic and Urdu come almost free afterwards.
