# S16 — Japanese (kana input) — v1.3 (draft)

> **DRAFT — planning only (2026-10-21). No development until the user approves.**

**Feasible:** romaji → hiragana/katakana input method (table-driven, like Google/Mozc romaji rules), on the OSK (romaji keyboard + a kana toggle) and over USB; vertical text not planned.
**Big job, not planned:** kanji conversion (kana → kanji candidates) needs a dictionary IME (SKK/Mozc-class; tens of MB of data and a candidate UI). Kanji can still be **displayed** if typed elsewhere.
**Fonts:** Noto Sans JP / Serif JP: full ~16 MB; subset kana + JIS level 1 kanji (~3 000) ≈ 4–5 MB; kana only ≈ 0.3 MB. Binary today ~2.9 MB, so a level-1 subset would ~2.5× the package — or load the font from the notes partition at runtime.
**Text rules:** line breaking without spaces (kinsoku rules), word count → character count, Find on kana.
**Estimate:** 2–3 weeks for kana + display. **Risks:** high — size, wrapping rules, IME UI on e-ink.
