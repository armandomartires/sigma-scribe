# Japanese input (v1.10)

## Using it
Settings → Keyboard & input → Layout → **Japanese (romaji)**. The Full on-screen keyboard or a USB keyboard.

- Type romaji; hiragana appears underlined. `nn` or `n'` is ん, a doubled consonant is っ, `-` ー, `,` 、 `.` 。 `[` 「 `]` 」.
- **Space** converts the reading; Space again goes to the next candidate (or tap one on the bar). **Enter** commits. **Esc** steps back: candidates, reading, nothing. **Backspace** takes back one kana.
- The key left of 1 (あ / A) switches between kana input and direct Latin input. AltGr types the US key in either mode.
- A digit ends the composition and is typed as it is.

## How conversion works
`crates/core/src/ja.rs`. The reading is cut into words by dynamic programming: each dictionary word costs a little less the longer it is, a kana no word covers costs more, particles never convert on their own, and endings such as ます ました ている stay kana. Verbs and adjectives match a stem plus the first kana of the ending, like SKK's okurigana entries. The candidate list is the best cut, the readings that are a single word, the hiragana, then the katakana.

## Dictionaries
- The built-in seed (`assets/ja/seed.skk`, `seed-okuri.txt`) was written for this project: about 440 words and 160 verb and adjective stems. It covers everyday text, not everything.
- A full SKK dictionary is read from `.adds/sigma-scribe/dict/skk-jisyo.txt` on the Kobo (or the file named by `SIGMA_SKK_DICT` on a computer). The format is SKK's: `よみ /候補/候補;注釈/`, with `;; okuri-ari entries.` and `;; okuri-nasi entries.` sections. **SKK-JISYO.L** (skk-dev/dict, GPL-2.0-or-later, about 3 MB) is the obvious choice: GPL-2.0-or-later may be combined with this AGPL-3.0 program, and the licence text and source notice must ship beside it. It has not been added to the repository or the package yet; that needs a download, so it waits for approval.
- Mozc's data (BSD-3, with NAIST and other dictionary licences) would convert whole sentences better, but it is a statistical model with its own build; the plan keeps it as the step after SKK.

## Fonts
Kanji and kana come from the CJK font pack (`fonts/cjk.ttf`, see font-packs.md). Without it they draw as empty boxes.
