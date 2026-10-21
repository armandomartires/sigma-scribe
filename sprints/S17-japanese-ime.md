# S17 — v1.7: Japanese input (kana + kanji conversion) (draft)

> **DRAFT — planning only (updated 2026-10-21 21:24 with the user's answers). No development until the user approves.**

**Scope (user):** romaji → kana and **kana → kanji conversion** with candidates, on the OSK, USB keyboard and (later) handwriting; Japanese fonts; line breaking (kinsoku), character counts.

## Engines and data (offline)
| Engine / data | Licence | AGPL-3.0 compatible? | Size on device | RAM | ARM effort | Notes |
|---|---|---|---|---|---|---|
| **Mozc** (OSS) | BSD-3 code; dictionary data mostly IPAdic-derived/permissive (to verify per file) | yes | compiled system dict ~30–50 MB | 50–100 MB | **hard**: Bazel, large C++, no armv7 Linux target | best conversion quality |
| **Mozc dictionary + own Rust converter** (Viterbi over the dict's word costs + connection matrix) | data permissive | yes | ~20–40 MB (compressed trie) | ~30–60 MB | medium (own code) | **recommended**: good quality, static Rust build |
| **libkkc** (+ SKK dicts, n-gram LM) | GPL-3.0 | yes (GPL-3 + AGPL-3 combine via §13) | ~10–20 MB | ~30 MB | medium–hard (C/Vala, GLib) | decent sentence conversion |
| **SKK** style + **SKK-JISYO.L** | dict GPL-2.0-or-later | yes (as GPL-3) | ~4 MB text | small | **easy** (own Rust) | user marks word boundaries (SKK habit); good fallback/first step |
| **librime** + rime-japanese | BSD-3 / data varies | yes | ~20 MB + deps | ~30–60 MB | hard (Boost, leveldb, yaml-cpp, OpenCC) | Japanese support weak |

**Recommendation:** shared Rust IME framework (composer + candidate bar from v1.3) with (1) romaji→kana tables, (2) **SKK-style conversion first** (fast to ship, tiny), then (3) **sentence conversion on the Mozc OSS dictionary with an own Viterbi converter**. Learning (user history) stored next to the notes.
**Speed on the A9:** dictionary lookups are fast; a Viterbi over a 20-character segment is milliseconds to tens of ms; loading must be mmap'd (no 50 MB parse at start).

## Fonts and storage
Noto Sans/Serif CJK (OFL). Full multi-region OTC ~19 MB per weight (variable ~30 MB+); single-region OTF (Noto Sans JP) ~4–5 MB; **JIS X 0208 subset (~6 900 chars) ≈ 2–3 MB per weight**. Storage is not the problem (6.7 GB free-ish user partition); the binary is (2.9 MB today, fonts embedded). → Ship CJK fonts as **separate files** in `.adds/sigma-scribe/fonts/`, loaded at runtime and mmap'd; default the JP subset (one weight, ~3 MB), with an optional full font for rare kanji.
**Estimate:** 4–6 wk (kana 1 wk, SKK 1 wk, Viterbi converter 2–3 wk, fonts/wrapping 1 wk). **Risks:** high — dictionary licence audit, conversion quality, candidate UI on e-ink, RAM.
