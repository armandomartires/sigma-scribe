#!/usr/bin/env python3
"""Builds the CJK font pack (fonts/cjk.ttf): Japanese kana and kanji (JIS X
0208), Korean (KS X 1001 syllables, hanja, jamo), punctuation and fullwidth
forms, subset from Noto Sans CJK (face 0, the Japanese one).

  python3 tools/cjk-pack.py NotoSansCJK-Regular.ttc cjk.ttf [assets/ja/skk-jisyo.txt]

Needs fontTools (`pip install fonttools`, or its wheel unpacked and on
PYTHONPATH). The optional dictionary adds any character it uses. Result:
about 4.3 MB. Noto Sans CJK is SIL OFL 1.1: ship its licence with the pack.
"""
import sys, tempfile
from fontTools.subset import main

def two_byte(codec, chars):
    for a in range(0xA1, 0xFF):
        for b in range(0xA1, 0xFF):
            try:
                chars.add(bytes([a, b]).decode(codec))
            except UnicodeDecodeError:
                pass

src, out = sys.argv[1], sys.argv[2]
chars = set()
two_byte("euc_jp", chars)
two_byte("euc_kr", chars)
for lo, hi in [(0x20, 0x7F), (0x3000, 0x3100), (0x31F0, 0x3200), (0xFF01, 0xFFEF), (0x3131, 0x318F)]:
    chars.update(chr(c) for c in range(lo, hi))
if len(sys.argv) > 3:
    chars.update(c for c in open(sys.argv[3], encoding="utf-8").read() if ord(c) > 0x2FF)
with tempfile.NamedTemporaryFile("w", encoding="utf-8", suffix=".txt") as t:
    t.write("".join(sorted(chars)))
    t.flush()
    main([src, "--font-number=0", f"--text-file={t.name}", f"--output-file={out}",
          "--layout-features=kern,palt,locl,vert,vrt2", "--no-hinting", "--desubroutinize"])
