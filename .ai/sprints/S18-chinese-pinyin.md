# S18 — v1.8: Simplified Chinese input (pinyin) (draft)

> **DRAFT — planning only (updated 2026-10-21 21:24 with the user's answers). No development until the user approves.**

**Does Japanese give us Chinese?** Partly. **Shared** with v1.7: the IME framework (composer, candidate bar, learning store), CJK font loading and line breaking, handwriting panel hooks (Zinnia-style per-character models exist for both). **Not shared:** Chinese **pinyin needs its own dictionary and language model** (pinyin syllables → hanzi words, with word frequencies/bigrams) and a Simplified Chinese font (Han glyph shapes differ by region: SC forms ≠ JP forms, so one font cannot serve both well).

| Engine / data | Licence | AGPL ok | Size | RAM | ARM effort |
|---|---|---|---|---|---|
| **librime** + rime-luna-pinyin / rime-essay | BSD-3 code, data **LGPL-3.0** | yes | ~20–40 MB compiled | ~30–60 MB | hard C++ cross-build |
| **rime-ice** (雾凇拼音) data | GPL-3.0 | yes | ~30–60 MB | | data only |
| **libpinyin** (ibus-libpinyin) | GPL-3.0 | yes | ~10–20 MB | ~30 MB | medium–hard (C, GLib) |
| **Own Rust converter** on luna_pinyin/essay frequency data | LGPL-3 data | yes | ~10–20 MB | ~20–40 MB | medium (reuses v1.7 Viterbi) |

**Recommendation:** reuse the v1.7 Rust converter with pinyin syllable segmentation and Rime's LGPL word-frequency data; consider embedding librime only if quality is insufficient. **Font:** Noto Sans SC GB2312 subset (~6 800 hanzi) ≈ 2.5–3.5 MB/weight as a separate file; full SC ~8–16 MB optional.
**Defaults kept:** GB2312 subset + optional full font; zh UI strings machine-drafted, best effort (decision 31).
**Estimate:** 2–3 wk on top of v1.7. **Risks:** medium — data licence audit, candidate ranking quality, font size.
