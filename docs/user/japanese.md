# Japanese input

## Using it
Settings → Keyboard & input → Layout → **Japanese (romaji)**. This layout is experimental: it appears in the list only with Settings → Experimental features on. The Full on-screen keyboard or a USB keyboard.

- Type romaji; hiragana appears underlined. `nn` or `n'` is ん, a doubled consonant is っ, `-` ー, `,` 、 `.` 。 `[` 「 `]` 」.
- **Space** converts the reading; Space again goes to the next candidate (or tap one on the bar). **Enter** commits. A sentence converts in phrases (私は | 学生です): **Left / Right** choose the phrase, **Space** cycles its candidates, **Shift + Left / Right** move its end by one kana; a tap on a candidate picks it and moves to the next phrase. A phrase you pick by hand is remembered for the run. **Esc** steps back: candidates, reading, nothing. **Backspace** takes back one kana.
- The key left of 1 (あ / A) switches between kana input and direct Latin input. AltGr types the US key in either mode.
- A digit ends the composition and is typed as it is.

## Fonts
Kanji and kana come from the CJK font pack (`fonts/cjk.ttf`, see font-packs.md). Without it they draw as empty boxes.

How conversion works and the dictionaries are in [../dev/japanese-internals.md](../dev/japanese-internals.md).
