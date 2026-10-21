#!/bin/sh
# Builds the device binary and assembles dist/sigma-scribe-<ver>-kobo/,
# a tree to copy onto the Kobo's USB drive (only under .adds/).
set -eu
cd "$(dirname "$0")/.."
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
./scripts/build-armv7.sh
OUT="dist/sigma-scribe-$VER-kobo"
rm -rf "$OUT" && mkdir -p "$OUT"
cp -r packaging/kobo/.adds "$OUT/"
cp target/armv7-unknown-linux-musleabihf/release/sigma-scribe "$OUT/.adds/sigma-scribe/"
cp LICENSE "$OUT/.adds/sigma-scribe/LICENSE"
cp assets/fonts/OFL-*.txt "$OUT/.adds/sigma-scribe/"
chmod 755 "$OUT/.adds/sigma-scribe/sigma-scribe" "$OUT/.adds/sigma-scribe/launch.sh"
( cd "$OUT" && find . -type f | sort | xargs sha256sum ) > "$OUT.sha256"
# Reproducible tarball: fixed order, owner, mode and time; gzip without name/time.
tar -C "$OUT" --sort=name --owner=0 --group=0 --numeric-owner \
  --mode='u+rwX,go+rX,go-w' --mtime='2026-01-01 00:00Z' -cf - .adds | gzip -9n > "$OUT.tar.gz"
./scripts/check-package.sh "$OUT"
echo "$OUT"; cat "$OUT.sha256"
