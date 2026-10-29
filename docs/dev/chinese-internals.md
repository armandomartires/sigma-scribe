# Chinese input internals (v1.11)

How to type is in [../user/chinese.md](../user/chinese.md).

## Dictionaries
`dict/pinyin-hans.txt` and `dict/pinyin-hant.txt` in `.adds/sigma-scribe/` (about 3 MB each, 106,000 pinyin keys) are read at the first letter typed. They are built by `tools/zh-dict.py` from the Rime project's **luna-pinyin** and **essay** data (LGPL-3.0; `dict/PINYIN-NOTICE.txt`, `dict/LGPL-3.0.txt`). Words come from essay.txt (word frequencies, Traditional) with their pinyin put together from the readings of their characters, as Rime does; the Simplified file is the same data converted with OpenCC. Ranking is by frequency, and the sentence candidate is the cheapest cover of the input by words. Without the files a built-in seed of about 340 words and characters (`assets/zh/seed-*.txt`) is used.

A different dictionary can be used in the same format: `syllables<TAB>word weight<TAB>word weight…` per line, syllables separated by one space (`ni hao`), best first.

## Fonts
Needs the CJK font pack (`fonts/cjk.ttf`, `python3 tools/cjk-pack.py … assets/zh/pinyin-hans.txt assets/zh/pinyin-hant.txt`; now about 10.6 MB: it keeps the whole unified ideograph block). It is made from the Japanese face of Noto Sans CJK, so a few characters have the Japanese shape.
