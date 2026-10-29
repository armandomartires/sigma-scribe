#!/bin/sh
# Builds the device binary and assembles dist/sigma-scribe-<ver>-kobo/,
# a tree to copy onto the Kobo's USB drive (only under .adds/).
set -eu
cd "$(dirname "$0")/.."
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
BUNDLED_PLUGINS="graph-map contents-pane todo-pane scratch-pane"
./scripts/build-armv7.sh
OUT="dist/sigma-scribe-$VER-kobo"
rm -rf "$OUT" && mkdir -p "$OUT"
cp -r packaging/kobo/.adds "$OUT/"
cp target/armv7-unknown-linux-musleabihf/release/sigma-scribe "$OUT/.adds/sigma-scribe/"
cp LICENSE "$OUT/.adds/sigma-scribe/LICENSE"
cp assets/fonts/OFL-*.txt "$OUT/.adds/sigma-scribe/"
mkdir -p "$OUT/.adds/sigma-scribe/dict"
cp assets/ja/skk-jisyo.txt "$OUT/.adds/sigma-scribe/dict/skk-jisyo.txt"
cp assets/ja/SKK-JISYO.L-NOTICE.txt "$OUT/.adds/sigma-scribe/dict/"
cp assets/zh/pinyin-hans.txt assets/zh/pinyin-hant.txt assets/zh/PINYIN-NOTICE.txt assets/zh/LGPL-3.0.txt "$OUT/.adds/sigma-scribe/dict/"
# The bundled plugins (the Graph and the drawer's three panes), as built from
# the plugins repository (SOURCE.txt names the commit). No plugins.state: an
# update must not overwrite the user's on/off choices; launch.sh writes one
# with these on when there is none (a new install).
for id in $BUNDLED_PLUGINS; do
  mkdir -p "$OUT/.adds/sigma-scribe/plugins/$id"
  cp "app/tests/fixtures/$id/plugin.conf" "app/tests/fixtures/$id/SOURCE.txt" \
     "app/tests/fixtures/$id/"*.wasm "$OUT/.adds/sigma-scribe/plugins/$id/"
done
# Syncthing for Settings -> Sync (docs/dev/sync-design.md): the pinned release's binary,
# unmodified, with its licence (MPL-2.0) and the notice from packaging/. No
# syncthing-home/: that is the device's identity and database, kept across
# updates.
ST=$(./scripts/fetch-syncthing.sh)
mkdir -p "$OUT/.adds/sigma-scribe/syncthing"
cp "$ST/syncthing" "$ST/LICENSE.txt" "$ST/AUTHORS.txt" "$OUT/.adds/sigma-scribe/syncthing/"
# Its root certificates (the Kobo has none it finds): the pinned Mozilla
# bundle, passed as SSL_CERT_FILE when the app starts it.
cp "$(./scripts/fetch-cacert.sh)" "$OUT/.adds/sigma-scribe/syncthing/ca-certificates.crt"
chmod 755 "$OUT/.adds/sigma-scribe/sigma-scribe" "$OUT/.adds/sigma-scribe/launch.sh" "$OUT/.adds/sigma-scribe/wifi.sh" "$OUT/.adds/sigma-scribe/syncthing/syncthing"
( cd "$OUT" && find . -type f | sort | xargs sha256sum ) > "$OUT.sha256"
# Reproducible tarball: fixed order, owner, mode and time; gzip without name/time.
tar -C "$OUT" --sort=name --owner=0 --group=0 --numeric-owner \
  --mode='u+rwX,go+rX,go-w' --mtime='2026-01-01 00:00Z' -cf - .adds | gzip -9n > "$OUT.tar.gz"
./scripts/check-package.sh "$OUT"
echo "$OUT"; cat "$OUT.sha256"
