# Chinese input (v1.11)

Settings → Keyboard & input → Layout → **Chinese Simplified (pinyin)** or **Chinese Traditional (pinyin)**. The Full on-screen keyboard or a USB keyboard.

## Typing
- Type pinyin; it shows underlined, and the **candidate bar** above the keyboard updates with every letter. The first candidate is the best guess for the whole input (`woaizhongguo` → 我爱中国), then the words that start the input, longest first, then single characters.
- **Space** writes the chosen candidate (the first, unless you moved). **1–9** write that candidate. A tap on a candidate writes it. **Left / Right** move the choice.
- A candidate that covers only the start (你 for `nihao`) is written and the rest (`hao`) stays to be typed on or picked.
- **Punctuation** writes the chosen candidate first, then the mark in full width: `，。？！：；（）【】、《》`. After a digit, `.` `,` `:` stay as typed (3.14).
- `'` separates syllables (`xi'an` = 西安, `xian` = 先). ü is typed `v` (`nv` = 女, `lvse` = 绿色).
- **Enter** keeps the pinyin as typed (no newline). **Esc** removes it. **Backspace** takes back a letter.
- The key left of 1 shows 中 or A and switches to direct Latin input; AltGr types the US key.

## Dictionaries
`dict/pinyin-hans.txt` and `dict/pinyin-hant.txt` in `.adds/sigma-scribe/` (about 3 MB each, 106,000 pinyin keys) are read at the first letter typed. They are built by `tools/zh-dict.py` from the Rime project's **luna-pinyin** and **essay** data (LGPL-3.0; `dict/PINYIN-NOTICE.txt`, `dict/LGPL-3.0.txt`). Words come from essay.txt (word frequencies, Traditional) with their pinyin put together from the readings of their characters, as Rime does; the Simplified file is the same data converted with OpenCC. Ranking is by frequency, and the sentence candidate is the cheapest cover of the input by words. Without the files a built-in seed of about 340 words and characters (`assets/zh/seed-*.txt`) is used.

A different dictionary can be used in the same format: `syllables<TAB>word weight<TAB>word weight…` per line, syllables separated by one space (`ni hao`), best first.

## Fonts
Needs the CJK font pack (`fonts/cjk.ttf`, `python3 tools/cjk-pack.py … assets/zh/pinyin-hans.txt assets/zh/pinyin-hant.txt`; now about 10.6 MB: it keeps the whole unified ideograph block). It is made from the Japanese face of Noto Sans CJK, so a few characters have the Japanese shape.

## Not yet
Zhuyin (Bopomofo), Cangjie, abbreviations by initials (`nh` for 你好), learning the words you pick, fuzzy pinyin, paging beyond the 40 candidates.
