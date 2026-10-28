#!/bin/sh
# Fetches the pinned Mozilla CA bundle (as extracted by curl) into
# target/cacert/ and prints the file's path. Syncthing on the Kobo reads it
# through SSL_CERT_FILE: the Kobo has no root certificates Syncthing finds, so
# its relays and global discovery failed with "certificate signed by unknown
# authority". The file is checked against the sha256 below; a file that does
# not match is deleted and the script fails. See docs/sync.md §1.
#
# To change the pin: pick a dated file from https://curl.se/docs/caextract.html,
# edit both lines (curl publishes the sha256 beside it), and update the table
# in docs/sync.md §1.
set -eu
cd "$(dirname "$0")/.."
CACERT_DATE=2026-10-20
CACERT_SHA256=796f1389bfea13c0ed78f663b765baaff54a508e5483786bd6ece73c5e3e3cbe
NAME="cacert-$CACERT_DATE.pem"
URL="https://curl.se/ca/$NAME"
DIR=target/cacert
PEM="$DIR/$NAME"
fail() { echo "fetch-cacert: $*" >&2; exit 1; }
sum() { sha256sum "$1" | cut -d' ' -f1; }
mkdir -p "$DIR"
if [ -f "$PEM" ] && [ "$(sum "$PEM")" != "$CACERT_SHA256" ]; then
  echo "fetch-cacert: cached $PEM does not match the pin; fetching again" >&2
  rm -f "$PEM"
fi
if [ ! -f "$PEM" ]; then
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --retry 3 -o "$PEM.part" "$URL" || fail "download failed: $URL"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$PEM.part" "$URL" || fail "download failed: $URL"
  else
    fail "needs curl or wget"
  fi
  mv "$PEM.part" "$PEM"
fi
GOT=$(sum "$PEM")
if [ "$GOT" != "$CACERT_SHA256" ]; then
  rm -f "$PEM"
  fail "sha256 mismatch for $NAME: got $GOT, want $CACERT_SHA256"
fi
grep -q 'BEGIN CERTIFICATE' "$PEM" || fail "$NAME holds no certificates"
echo "$PEM"
