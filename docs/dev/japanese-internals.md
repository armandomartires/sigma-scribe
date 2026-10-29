# Japanese input internals (v1.10)

How to type is in [../user/japanese.md](../user/japanese.md).

## How conversion works
`crates/core/src/ja.rs`. The reading is cut into words by dynamic programming: each dictionary word costs a little less the longer it is, a kana no word covers costs more, particles never convert on their own, and endings such as ます ました ている stay kana. Verbs and adjectives match a stem plus the first kana of the ending, like SKK's okurigana entries. The candidate list is the best cut, the readings that are a single word, the hiragana, then the katakana.

## Dictionaries
- The built-in seed (`assets/ja/seed.skk`, `seed-okuri.txt`) was written for this project: about 500 words and 170 verb and adjective stems. It steers the ranking of the full dictionary.
- **SKK-JISYO.L** (skk-dev/dict, GPL-2.0-or-later, 160,000 words) ships in the package as `.adds/sigma-scribe/dict/skk-jisyo.txt`, with its notice (`SKK-JISYO.L-NOTICE.txt`), and is read at the first conversion (about 2 s on the Forma, estimated; 0.7 s on a computer). The repository holds the UTF-8 copy in `assets/ja/skk-jisyo.txt` (the original is EUC-JP, sha256 of the download `c791f578…967e40`); it is loaded from the device path, from `SIGMA_SKK_DICT`, or from `assets/ja/` in a checkout. GPL-2.0-or-later may be combined with this AGPL-3.0 program; it is data in a separate file.
- Any other SKK-format dictionary can replace it: `よみ /候補/候補;注釈/`, with `;; okuri-ari entries.` and `;; okuri-nasi entries.` sections.
- Ranking: SKK gives no frequencies, so the project's seed words (about 440) and endings are preferred over the long tail, and `Space` cycles four alternative cuts of the reading, then the whole-word candidates, the hiragana and the katakana.
- Mozc's data (BSD-3, with NAIST and other dictionary licences) would convert whole sentences better, but it is a statistical model with its own build; the plan keeps it as the step after SKK.
