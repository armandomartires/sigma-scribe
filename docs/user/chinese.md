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

## Not yet
Zhuyin (Bopomofo), Cangjie, abbreviations by initials (`nh` for 你好), learning the words you pick, fuzzy pinyin, paging beyond the 40 candidates.

Dictionaries, the font pack and how they are built are in [../dev/chinese-internals.md](../dev/chinese-internals.md).
