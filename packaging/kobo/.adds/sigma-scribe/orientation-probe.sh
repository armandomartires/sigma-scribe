#!/bin/sh
# Σscribe g-sensor probe (S12 device check, read-only).
# Leaves Nickel running and the app untouched. Copies the input device list
# and records the raw bytes of every /dev/input/event* node for 60 s, so the
# PC can decode which node reports MSC_RAW and which code each way of holding
# the Forma gives. Writes only under /mnt/onboard/.adds/sigma-scribe/probe/.
DIR=/mnt/onboard/.adds/sigma-scribe
OUT="$DIR/probe"
SECS=60
mkdir -p "$OUT"
rm -f "$OUT"/event* 2>/dev/null
{
  echo "=== $(date) probe start"
  cat /mnt/onboard/.kobo/version 2>/dev/null; echo
  uname -a
} >"$OUT/probe.log" 2>&1
cat /proc/bus/input/devices >"$OUT/devices.txt" 2>&1
PIDS=""
for n in /dev/input/event*; do
  [ -c "$n" ] || continue
  b=$(basename "$n")
  head -c 2000000 "$n" >"$OUT/$b.bin" 2>/dev/null &
  PIDS="$PIDS $!"
  echo "recording $n" >>"$OUT/probe.log"
done
sleep "$SECS"
kill $PIDS >/dev/null 2>&1
sleep 1
for f in "$OUT"/event*.bin; do
  echo "$(basename "$f") $(wc -c <"$f") bytes" >>"$OUT/probe.log"
done
echo "=== $(date) probe end" >>"$OUT/probe.log"
sync
