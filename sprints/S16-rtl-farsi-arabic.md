# S16 — v1.6: Farsi and Arabic (RTL + shaping), one release (draft)

> **DRAFT — planning only (updated 2026-10-21 21:24 with the user's answers). No development until the user approves.**

**Scope (user: both wanted):** Farsi and Arabic text in notes and preview; OSK and USB layouts (Farsi ISIRI 9147, Arabic 101/PC); Persian/Arabic-Indic digits option; UI in fa and ar with mirrored layout (uses the v1.4 i18n mirroring switch).
**Needs:** shaping (contextual forms, lam-alef ligatures, ZWNJ for Farsi, kashida not needed) → **rustybuzz** (MIT); bidirectional text (mixed RTL/LTR/digits/Markdown markers) → **unicode-bidi** (MIT/Apache); fonts **Vazirmatn** (OFL, Farsi, ~0.4–0.6 MB/weight) and **Noto Naskh/Sans Arabic** (OFL, ~0.2–0.5 MB/weight), loaded from files.
**Impact (major editor/render change):** today the editor lays out LTR runs with per-char advances; wrapping, caret movement (logical vs visual), hit-testing, selection handles, Find highlighting and the Markdown preview all become bidi- and cluster-aware; shaping results cached per line for the A9. Line mode word counts unaffected.
**Estimate:** 4–6 wk. **Risks:** high — mixed-direction caret/selection, performance, e-ink partial regions per visual run, reviewers for fa/ar strings.
