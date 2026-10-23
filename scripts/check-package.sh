#!/bin/sh
# Checks a packaged release tree + tarball (dist/sigma-scribe-<ver>-kobo):
# only .adds/ paths, no notes, required files, ARM static binary, sha256
# list matches. Used by package-kobo.sh and the release workflow.
set -eu
OUT="${1:?usage: check-package.sh dist/sigma-scribe-<ver>-kobo}"
fail() { echo "check-package: $*" >&2; exit 1; }
[ -f "$OUT.tar.gz" ] || fail "missing $OUT.tar.gz"
tar tzf "$OUT.tar.gz" | grep -v '^\.adds/' && fail "entries outside .adds/"
tar tzf "$OUT.tar.gz" | grep -qi 'notes' && fail "notes in the package"
for f in .adds/nm/sigma-scribe .adds/sigma-scribe/sigma-scribe .adds/sigma-scribe/launch.sh \
         .adds/sigma-scribe/touch.conf .adds/sigma-scribe/LICENSE .adds/sigma-scribe/THIRD-PARTY.txt \
         .adds/sigma-scribe/OFL-Literata.txt .adds/sigma-scribe/OFL-AtkinsonHyperlegible.txt\
         .adds/sigma-scribe/OFL-NotoNaskhArabic.txt .adds/sigma-scribe/OFL-NotoSansHebrew.txt\
         .adds/sigma-scribe/OFL-NotoSansDevanagari.txt .adds/sigma-scribe/OFL-NotoSansBengali.txt\
         .adds/sigma-scribe/OFL-NotoSansTamil.txt .adds/sigma-scribe/OFL-NotoSansThai.txt; do
  tar tzf "$OUT.tar.gz" | grep -qx "$f" || fail "missing $f"
done
file "$OUT/.adds/sigma-scribe/sigma-scribe" | grep -q 'ARM.*statically linked' || fail "binary is not ARM static"
( cd "$OUT" && sha256sum -c --quiet "../$(basename "$OUT").sha256" ) || fail "sha256 list mismatch"
echo "check-package: ok $(sha256sum "$OUT/.adds/sigma-scribe/sigma-scribe" | cut -c1-64)"
