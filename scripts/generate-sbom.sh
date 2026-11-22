#!/bin/sh
# Emit a CycloneDX 1.6 JSON SBOM for the Kobo sigma-scribe binary into
# dist/sbom/. Requires cargo-cyclonedx on PATH (CI pins 0.5.10,
# Apache-2.0). This covers Cargo crates only; bundled non-crate assets
# (FBInk, Syncthing, fonts, dictionaries, plugin wasm) are listed in
# docs/dev/sbom.md and are not in this file.
set -eu
cd "$(dirname "$0")/.."

if ! command -v cargo-cyclonedx >/dev/null 2>&1 && ! cargo cyclonedx -V >/dev/null 2>&1; then
  echo "error: cargo-cyclonedx is not on PATH" >&2
  exit 1
fi

if [ -z "${SOURCE_DATE_EPOCH:-}" ]; then
  SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"
  export SOURCE_DATE_EPOCH
fi

mkdir -p dist/sbom

# --describe binaries also writes sibling SBOMs for sigma-emu and
# hwr-train; those are removed after we keep the product binary.
cargo cyclonedx \
  --manifest-path app/Cargo.toml \
  --format json \
  --spec-version 1.6 \
  --describe binaries \
  --no-default-features \
  --features kobo \
  --target armv7-unknown-linux-musleabihf \
  --target-in-filename

src="app/sigma-scribe_bin_armv7-unknown-linux-musleabihf.cdx.json"
if [ ! -f "$src" ]; then
  echo "error: expected $src" >&2
  exit 1
fi
mv "$src" dist/sbom/sigma-scribe-kobo.cdx.json
rm -f app/sigma-emu_bin_armv7-unknown-linux-musleabihf.cdx.json \
  tools/hwr-train/hwr-train_bin_armv7-unknown-linux-musleabihf.cdx.json

# Paths in the checksum file are relative to the repository root, so
# `sha256sum -c dist/sbom/sigma-scribe-kobo.cdx.json.sha256` works here.
sha256sum dist/sbom/sigma-scribe-kobo.cdx.json > dist/sbom/sigma-scribe-kobo.cdx.json.sha256
echo "wrote dist/sbom/sigma-scribe-kobo.cdx.json"
cat dist/sbom/sigma-scribe-kobo.cdx.json.sha256
