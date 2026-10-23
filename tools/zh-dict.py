#!/usr/bin/env python3
"""Builds the pinyin dictionaries (assets/zh/pinyin-hant.txt and pinyin-hans.txt)
from Rime's luna-pinyin (LGPL-3.0) and essay (LGPL-3.0) data:

  python3 tools/zh-dict.py luna_pinyin.dict.yaml essay.txt [min-freq, default 500]

Words come from essay.txt (Traditional, with frequencies); their pinyin is put
together from the readings of their characters in luna_pinyin.dict.yaml (the
way Rime's preset vocabulary does), plus luna's own word entries. Simplified
output is made with OpenCC t2s (libopencc, as tools/zh-hant.py). Format, one
line per pinyin key:  syllables<TAB>word weight<TAB>word weight...  (best first,
at most 12 words per key; a key is its syllables joined by one space, ü as v).
"""
import ctypes, itertools, os, sys
from collections import defaultdict

luna, essay = sys.argv[1], sys.argv[2]
MIN = int(sys.argv[3]) if len(sys.argv) > 3 else 500
out = os.path.join(os.path.dirname(__file__), "..", "assets", "zh")

freq = {}
for line in open(essay, encoding="utf-8"):
    p = line.rstrip("\n").split("\t")
    if len(p) >= 2 and p[1].isdigit():
        freq[p[0]] = int(p[1])

chars = defaultdict(list)   # char -> [(syllable, share 0..1)]
own = []                    # luna's word entries
body = False
for line in open(luna, encoding="utf-8"):
    if not body:
        body = line.startswith("...")
        continue
    p = line.rstrip("\n").split("\t")
    if len(p) < 2 or p[0].startswith("#"):
        continue
    share = float(p[2].rstrip("%")) / 100 if len(p) > 2 and p[2].endswith("%") else 1.0
    if len(p[0]) == 1:
        if share > 0:
            chars[p[0]].append((p[1], share))
    else:
        own.append((p[0], p[1]))

table = defaultdict(dict)   # key -> {word: weight}
def add(key, word, w):
    t = table[key]
    t[word] = max(t.get(word, 0), w)

for c, rs in chars.items():
    f = freq.get(c, 1)
    for s, share in rs:
        add(s, c, max(1, int(f * share)))
for w, f in freq.items():
    if len(w) < 2 or len(w) > 8 or f < MIN or not all(c in chars for c in w):
        continue
    combos = sorted(itertools.product(*[chars[c] for c in w]),
                    key=lambda r: -eval("*".join(str(x[1]) for x in r)))[:3]
    for r in combos:
        share = 1.0
        for _, sh in r:
            share *= sh
        if share >= 0.05:
            add(" ".join(x[0] for x in r), w, max(1, int(f * share)))
for w, k in own:
    add(k, w, freq.get(w, 1) or 1)

lib = ctypes.CDLL("libopencc.so.1.1")
lib.opencc_open.restype = ctypes.c_void_p
lib.opencc_open.argtypes = [ctypes.c_char_p]
lib.opencc_convert_utf8.restype = ctypes.c_void_p
lib.opencc_convert_utf8.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_size_t]
lib.opencc_convert_utf8_free.argtypes = [ctypes.c_void_p]
h = lib.opencc_open(b"/usr/share/opencc/t2s.json")
def t2s(s):
    b = s.encode()
    p = lib.opencc_convert_utf8(h, b, len(b))
    r = ctypes.string_at(p).decode()
    lib.opencc_convert_utf8_free(p)
    return r

def write(name, conv):
    n = 0
    with open(os.path.join(out, name), "w", encoding="utf-8") as f:
        f.write("# Sigma Scribe pinyin dictionary (%s), built by tools/zh-dict.py from Rime luna-pinyin\n"
                "# and essay (LGPL-3.0; see PINYIN-NOTICE.txt). key<TAB>word weight<TAB>word weight...\n" % name)
        for key in sorted(table):
            seen, items = set(), []
            for w, wt in sorted(table[key].items(), key=lambda x: -x[1]):
                w = conv(w)
                if w not in seen:
                    seen.add(w)
                    items.append("%s %d" % (w, wt))
                if len(items) == 12:
                    break
            f.write(key + "\t" + "\t".join(items) + "\n")
            n += 1
    print(name, n, "keys", os.path.getsize(os.path.join(out, name)) // 1024, "KB")

write("pinyin-hant.txt", lambda s: s)
write("pinyin-hans.txt", t2s)
