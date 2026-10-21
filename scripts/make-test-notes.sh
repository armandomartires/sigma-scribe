#!/bin/sh
# Builds the test notes pack (S10.1) into dist/test-notes/ and a reproducible
# tarball dist/sigma-scribe-test-notes.tar.gz (+ .sha256).
#   scripts/make-test-notes.sh [--no-graph]
# Sources: tests/test-notes/ (hand-written notes, every name prefixed test-).
# Generated: _test/graph-300/test-n000..test-n299 — each note links to the
# next and to a hub ((i*7/10) mod 20); every tenth (i mod 10 = 9) is an
# orphan. Same rule as `sigma-emu make-notes`.
# Install on the Kobo (separate approval): copy _test/ to notes/_test/.
set -eu
cd "$(dirname "$0")/.."
graph=1
[ "${1:-}" = "--no-graph" ] && graph=0
out=dist/test-notes
rm -rf "$out"
mkdir -p "$out"
cp -R tests/test-notes/. "$out/"
if [ "$graph" = 1 ]; then
  g="$out/_test/graph-300"
  mkdir -p "$g"
  i=0
  while [ $i -lt 300 ]; do
    f=$(printf '%s/test-n%03d.md' "$g" $i)
    printf '# Note %d\n\n' $i > "$f"
    if [ $((i % 10)) -ne 9 ]; then
      next=$(((i + 1) % 300))
      [ $((next % 10)) -ne 9 ] && printf 'Next: [[test-n%03d]]\n' $next >> "$f"
      printf 'Hub: [[test-n%03d]]\n' $(((i * 7 / 10) % 20)) >> "$f"
    fi
    i=$((i + 1))
  done
fi
tar --sort=name --mtime='2026-10-21 00:00Z' --owner=0 --group=0 --numeric-owner \
  -C dist -cf - test-notes | gzip -n -9 > dist/sigma-scribe-test-notes.tar.gz
(cd dist && sha256sum sigma-scribe-test-notes.tar.gz > sigma-scribe-test-notes.tar.gz.sha256)
echo "$(find "$out" -type f | wc -l) files"
cat dist/sigma-scribe-test-notes.tar.gz.sha256
