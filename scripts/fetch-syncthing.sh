#!/bin/sh
# Fetches the pinned Syncthing release for the Kobo (32-bit ARM Linux) into
# target/syncthing/ and prints the folder holding the unpacked files. The
# tarball is checked against the sha256 below before it is unpacked; a file
# that does not match is deleted and the script fails. See docs/dev/sync-design.md §1.
#
# To change the pin: edit both lines, run the Forma checklist in docs/dev/sync-design.md
# §12, and update the table in §1.
set -eu
cd "$(dirname "$0")/.."
SYNCTHING_VERSION=v2.1.7
SYNCTHING_SHA256=d8b59029e25992bc0d280fccc5626e5dca743b285fca1a36db1148fcf871b639
NAME="syncthing-linux-arm-$SYNCTHING_VERSION"
URL="https://github.com/syncthing/syncthing/releases/download/$SYNCTHING_VERSION/$NAME.tar.gz"
DIR=target/syncthing
TAR="$DIR/$NAME.tar.gz"
fail() { echo "fetch-syncthing: $*" >&2; exit 1; }
sum() { sha256sum "$1" | cut -d' ' -f1; }
mkdir -p "$DIR"
if [ -f "$TAR" ] && [ "$(sum "$TAR")" != "$SYNCTHING_SHA256" ]; then
  echo "fetch-syncthing: cached $TAR does not match the pin; fetching again" >&2
  rm -f "$TAR"
fi
if [ ! -f "$TAR" ]; then
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --retry 3 -o "$TAR.part" "$URL" || fail "download failed: $URL"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$TAR.part" "$URL" || fail "download failed: $URL"
  else
    fail "needs curl or wget"
  fi
  mv "$TAR.part" "$TAR"
fi
GOT=$(sum "$TAR")
if [ "$GOT" != "$SYNCTHING_SHA256" ]; then
  rm -f "$TAR"
  fail "sha256 mismatch for $NAME.tar.gz: got $GOT, want $SYNCTHING_SHA256"
fi
rm -rf "$DIR/$NAME"
tar -xzf "$TAR" -C "$DIR" --no-same-owner
for f in syncthing LICENSE.txt AUTHORS.txt; do
  [ -f "$DIR/$NAME/$f" ] || fail "$f is not in the tarball"
done
if command -v file >/dev/null 2>&1; then
  file "$DIR/$NAME/syncthing" | grep -q 'ARM.*statically linked' || fail "syncthing is not a static ARM binary"
fi
echo "$DIR/$NAME"
