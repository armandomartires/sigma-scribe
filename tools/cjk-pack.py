#!/usr/bin/env python3
"""Builds the CJK font pack (fonts/cjk.ttf): Japanese kana and kanji (JIS X
0208), Korean (KS X 1001 syllables, hanja, jamo), Chinese (GB 2312 and Big5),
punctuation and fullwidth forms, subset from Noto Sans CJK (face 0).

  python3 tools/cjk-pack.py NotoSansCJK-Regular.ttc cjk.ttf [dictionary ...]

Needs fontTools (`pip install fonttools`, or its wheel unpacked and on
PYTHONPATH). Optional dictionaries (assets/ja/skk-jisyo.txt,
assets/zh/pinyin-hans.txt, assets/zh/pinyin-hant.txt) add any character they
use. Noto Sans CJK is SIL OFL 1.1: ship its licence with the pack.
"""
import sys, tempfile
from fontTools.subset import main

def two_byte(codec, chars, trail=range(0xA1, 0xFF)):
    for a in range(0xA1, 0xFF):
        for b in trail:
            try:
                chars.add(bytes([a, b]).decode(codec))
            except UnicodeDecodeError:
                pass

src, out = sys.argv[1], sys.argv[2]
chars = set()
two_byte("euc_jp", chars)
two_byte("euc_kr", chars)
two_byte("gb2312", chars)
two_byte("big5", chars, list(range(0x40, 0x7F)) + list(range(0xA1, 0xFF)))
for lo, hi in [(0x20, 0x7F), (0x3000, 0x3100), (0x31F0, 0x3200), (0xFF01, 0xFFEF), (0x3131, 0x318F)]:
    chars.update(chr(c) for c in range(lo, hi))
for path in sys.argv[3:]:
    chars.update(c for c in open(path, encoding="utf-8").read() if ord(c) > 0x2FF)
chars = {c for c in chars if ord(c) <= 0xFFFF}   # the font has a BMP cmap only
# A scattered set of 20,000 characters has too many cmap segments for the
# format-4 table (65,535 bytes), so the unified ideographs are kept whole.
chars.update(chr(c) for c in range(0x4E00, 0xA000))
with tempfile.NamedTemporaryFile("w", encoding="utf-8", suffix=".txt") as t:
    t.write("".join(sorted(chars)))
    t.flush()
    main([src, "--font-number=0", f"--text-file={t.name}", f"--output-file={out}",
          "--layout-features=kern,palt,locl,vert,vrt2", "--no-hinting", "--desubroutinize"])
