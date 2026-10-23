#!/usr/bin/env python3
"""Decode orientation-probe.sh output: python3 tools/decode-probe.py <probe-dir>
Prints every EV_MSC/MSC_RAW event (32-bit ARM input_event, 16 bytes)."""
import struct, sys, pathlib

d = pathlib.Path(sys.argv[1])
dev = d / "devices.txt"
if dev.exists():
    for blk in dev.read_text().split("\n\n"):
        if "MSC=" in blk:
            print(blk.strip(), "\n")
for f in sorted(d.glob("event*.bin")):
    b = f.read_bytes()
    hits = []
    for i in range(0, len(b) - 15, 16):
        sec, usec, typ, code, val = struct.unpack_from("<IIHHi", b, i)
        if typ == 4 and code == 3:
            hits.append((sec, usec, val))
    print(f"{f.name}: {len(b)//16} events, {len(hits)} MSC_RAW")
    t0 = hits[0][0] if hits else 0
    for sec, usec, val in hits:
        print(f"  t={sec - t0:4d}.{usec // 1000:03d}s  0x{val:02x}")
