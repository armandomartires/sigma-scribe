# Script font packs (v1.7.1)

Literata and Atkinson cover Latin, Greek, Cyrillic and Vietnamese. Other scripts come from an optional **font pack**, a single TrueType/OpenType file the app loads at start and uses for any character the bundled faces lack (plan §12.6).

## Where the app looks
1. `SIGMA_FONT_PACK` — a path; `path#N` picks face N of a collection (development).
2. `/mnt/onboard/.adds/sigma-scribe/fonts/korean.ttf` — the Kobo.
3. `/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc#1` (Noto Sans CJK KR) — a development machine only; the simulator uses it, which is why Korean shows in emulator screenshots.

No pack: the script's characters draw as empty boxes.

## Building the Korean pack (not yet done)
Noto Sans CJK is about 19 MB per weight, too big to ship whole. Subset it to KS X 1001's 2,350 syllables plus compatibility jamo (about 1.5–2 MB), with fontTools (SIL OFL 1.1; ship the licence text beside it):

```bash
pyftsubset NotoSansCJKkr-Regular.otf --unicodes="U+3131-318E,U+AC00-D7A3" --layout-features='*' --output-file=korean.ttf
```

The full Hangul block (`U+AC00-D7A3`, 11,172 syllables) is about 3–4 MB; for the 2,350-syllable set, pass the KS X 1001 syllable list instead. Copy the result to `.adds/sigma-scribe/fonts/korean.ttf`. The build host here has no fontTools, so the pack has not been made; installing the Korean layout on the device waits for it.
