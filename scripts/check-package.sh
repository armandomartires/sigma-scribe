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
tar tzf "$OUT.tar.gz" | grep -q 'syncthing-home' && fail "syncthing-home in the package (it would replace the device's identity)"
tar tzf "$OUT.tar.gz" | grep -q 'plugins\.state$' && fail "plugins.state in the package (it would reset the user's choices)"
for f in .adds/nm/sigma-scribe .adds/sigma-scribe/sigma-scribe .adds/sigma-scribe/launch.sh \
         .adds/sigma-scribe/touch.conf .adds/sigma-scribe/LICENSE .adds/sigma-scribe/THIRD-PARTY.txt \
         .adds/sigma-scribe/OFL-Literata.txt .adds/sigma-scribe/OFL-AtkinsonHyperlegible.txt\
         .adds/sigma-scribe/OFL-NotoNaskhArabic.txt .adds/sigma-scribe/OFL-NotoSansHebrew.txt\
         .adds/sigma-scribe/OFL-NotoSansDevanagari.txt .adds/sigma-scribe/OFL-NotoSansBengali.txt\
         .adds/sigma-scribe/OFL-NotoSansTamil.txt .adds/sigma-scribe/OFL-NotoSansThai.txt\
         .adds/sigma-scribe/syncthing/syncthing .adds/sigma-scribe/syncthing/LICENSE.txt\
         .adds/sigma-scribe/syncthing/AUTHORS.txt .adds/sigma-scribe/syncthing/NOTICE.txt\
         .adds/sigma-scribe/dict/skk-jisyo.txt .adds/sigma-scribe/dict/SKK-JISYO.L-NOTICE.txt\
         .adds/sigma-scribe/dict/pinyin-hans.txt .adds/sigma-scribe/dict/pinyin-hant.txt\
         .adds/sigma-scribe/dict/PINYIN-NOTICE.txt .adds/sigma-scribe/dict/LGPL-3.0.txt\
         .adds/sigma-scribe/icons/my-apps.png .adds/sigma-scribe/icons/my-apps-active.png\
         .adds/sigma-scribe/plugins/graph-map/plugin.conf .adds/sigma-scribe/plugins/graph-map/graph.wasm\
         .adds/sigma-scribe/plugins/contents-pane/plugin.conf .adds/sigma-scribe/plugins/contents-pane/pane.wasm\
         .adds/sigma-scribe/plugins/todo-pane/plugin.conf .adds/sigma-scribe/plugins/todo-pane/pane.wasm\
         .adds/sigma-scribe/plugins/scratch-pane/plugin.conf .adds/sigma-scribe/plugins/scratch-pane/pane.wasm; do
  tar tzf "$OUT.tar.gz" | grep -qx "$f" || fail "missing $f"
done
file "$OUT/.adds/sigma-scribe/sigma-scribe" | grep -q 'ARM.*statically linked' || fail "binary is not ARM static"
file "$OUT/.adds/sigma-scribe/syncthing/syncthing" | grep -q 'ARM.*statically linked' || fail "syncthing is not ARM static"
# The notice names the pinned version (the source URL must be for these bits).
V=$(sed -n 's/^SYNCTHING_VERSION=//p' scripts/fetch-syncthing.sh)
grep -q "tree/$V" "$OUT/.adds/sigma-scribe/syncthing/NOTICE.txt" || fail "syncthing NOTICE.txt does not name $V"
grep -q "$(sed -n 's/^SYNCTHING_SHA256=//p' scripts/fetch-syncthing.sh)" "$OUT/.adds/sigma-scribe/syncthing/NOTICE.txt" || fail "syncthing NOTICE.txt has the wrong sha256"
( cd "$OUT" && sha256sum -c --quiet "../$(basename "$OUT").sha256" ) || fail "sha256 list mismatch"
echo "check-package: ok $(sha256sum "$OUT/.adds/sigma-scribe/sigma-scribe" | cut -c1-64)"
